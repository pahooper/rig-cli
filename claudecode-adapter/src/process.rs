use crate::error::ClaudeError;
use crate::types::{OutputFormat, RunConfig, RunResult};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::timeout;

// Resource management constants
const CHANNEL_CAPACITY: usize = 100;
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn run_claude(
    path: &std::path::Path,
    prompt: &str,
    config: &RunConfig,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, ClaudeError> {
    let args = crate::cmd::build_args(prompt, config);
    let start_time = Instant::now();

    // Build and configure command
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    // Spawn subprocess with proper error handling
    let mut child = cmd.spawn().map_err(|e| ClaudeError::SpawnFailed {
        stage: "subprocess spawn".to_string(),
        source: e,
    })?;

    // Take stdout/stderr pipes with proper error handling
    let stdout = child.stdout.take().ok_or(ClaudeError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(ClaudeError::NoStderr)?;

    // Get PID for tracking
    let pid = child.id().ok_or(ClaudeError::NoPid)?;

    // Create bounded internal channels for stdout/stderr communication
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);

    // Track reader tasks in JoinSet for proper lifecycle management
    let mut tasks = JoinSet::new();

    // Spawn stdout reader task
    let format = config.output_format;
    let stream_sender = sender.clone();
    tasks.spawn(async move {
        drain_stdout_bounded(stdout, stdout_tx, stream_sender, format, MAX_OUTPUT_BYTES).await
    });

    // Spawn stderr reader task
    tasks.spawn(async move { drain_stderr_bounded(stderr, stderr_tx, MAX_OUTPUT_BYTES).await });

    // Main execution loop with timeout
    let execution = async {
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut stream_events = Vec::new();
        let mut stdout_done = false;
        let mut stderr_done = false;

        // Drain both channels until they close
        while !stdout_done || !stderr_done {
            tokio::select! {
                result = stdout_rx.recv(), if !stdout_done => {
                    match result {
                        Some(line) => {
                            // Parse JSONL stream events if in StreamJson mode
                            if format == Some(OutputFormat::StreamJson) {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                                    stream_events.push(val);
                                }
                            }
                            stdout_lines.push(line);
                        }
                        None => {
                            stdout_done = true;
                        }
                    }
                }
                result = stderr_rx.recv(), if !stderr_done => {
                    match result {
                        Some(line) => {
                            stderr_lines.push(line);
                        }
                        None => {
                            stderr_done = true;
                        }
                    }
                }
            }
        }

        // Wait for subprocess to exit
        let status = child.wait().await.map_err(|e| ClaudeError::SpawnFailed {
            stage: "child.wait()".to_string(),
            source: e,
        })?;

        // Await all reader tasks to ensure they completed
        while let Some(result) = tasks.join_next().await {
            result.map_err(|e| ClaudeError::StreamFailed {
                stage: "reader task join".to_string(),
                source: e,
            })??;
        }

        let duration = start_time.elapsed();
        let final_stdout = stdout_lines.join("\n");
        let final_stderr = stderr_lines.join("\n");

        // Parse JSON output if requested
        let json = if config.output_format == Some(OutputFormat::Json) {
            serde_json::from_str(&final_stdout).ok()
        } else {
            None
        };

        Ok::<RunResult, ClaudeError>(RunResult {
            stdout: final_stdout,
            stderr: final_stderr,
            exit_code: status.code().unwrap_or(-1),
            duration_ms: duration.as_millis() as u64,
            json,
            stream_events,
            structured_output: None,
        })
    };

    // Run with timeout
    match timeout(config.timeout, execution).await {
        Ok(result) => result,
        Err(_) => {
            // Timeout occurred - perform graceful shutdown
            let partial_stdout = collect_remaining(&mut stdout_rx).await;
            let partial_stderr = collect_remaining(&mut stderr_rx).await;

            // Attempt graceful shutdown (SIGTERM -> SIGKILL)
            let _ = graceful_shutdown(&mut child, pid).await;

            // Abort all reader tasks
            tasks.abort_all();

            Err(ClaudeError::Timeout {
                elapsed: config.timeout,
                pid,
                partial_stdout,
                partial_stderr,
            })
        }
    }
}

/// Drain stdout with bounded memory, parse JSONL, forward stream events
async fn drain_stdout_bounded(
    stdout: impl tokio::io::AsyncRead + Unpin,
    tx: mpsc::Sender<String>,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
    format: Option<OutputFormat>,
    max_bytes: usize,
) -> Result<(), ClaudeError> {
    let mut reader = BufReader::new(stdout).lines();
    let mut total_bytes = 0;

    while let Some(line) = reader.next_line().await.map_err(|e| ClaudeError::SpawnFailed {
        stage: "stdout read".to_string(),
        source: e,
    })? {
        let line_bytes = line.len();
        total_bytes += line_bytes;

        // Enforce output size limit
        if total_bytes > max_bytes {
            return Err(ClaudeError::OutputTruncated {
                captured_bytes: total_bytes,
                limit_bytes: max_bytes,
            });
        }

        // Forward stream events if in StreamJson mode
        if format == Some(OutputFormat::StreamJson) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ref stream_tx) = sender {
                    if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val) {
                        // Ignore send errors (receiver may have dropped)
                        let _ = stream_tx.send(event).await;
                    }
                }
            }
        }

        // Send line on internal channel
        if tx.send(line).await.is_err() {
            // Receiver dropped - stop reading
            break;
        }
    }

    Ok(())
}

/// Drain stderr with bounded memory
async fn drain_stderr_bounded(
    stderr: impl tokio::io::AsyncRead + Unpin,
    tx: mpsc::Sender<String>,
    max_bytes: usize,
) -> Result<(), ClaudeError> {
    let mut reader = BufReader::new(stderr).lines();
    let mut total_bytes = 0;

    while let Some(line) = reader.next_line().await.map_err(|e| ClaudeError::SpawnFailed {
        stage: "stderr read".to_string(),
        source: e,
    })? {
        let line_bytes = line.len();
        total_bytes += line_bytes;

        // Enforce output size limit
        if total_bytes > max_bytes {
            return Err(ClaudeError::OutputTruncated {
                captured_bytes: total_bytes,
                limit_bytes: max_bytes,
            });
        }

        // Send line on internal channel
        if tx.send(line).await.is_err() {
            // Receiver dropped - stop reading
            break;
        }
    }

    Ok(())
}

/// Graceful shutdown: SIGTERM, wait grace period, then SIGKILL
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    // Send SIGTERM for graceful shutdown
    let nix_pid = Pid::from_raw(pid as i32);
    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| ClaudeError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        source: e,
    })?;

    // Wait for grace period
    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(status)) => Ok(status),
        Ok(Err(e)) => Err(ClaudeError::SpawnFailed {
            stage: "graceful_shutdown wait".to_string(),
            source: e,
        }),
        Err(_) => {
            // Grace period expired - force kill
            child.kill().await.map_err(|e| ClaudeError::SpawnFailed {
                stage: "SIGKILL".to_string(),
                source: e,
            })?;
            child.wait().await.map_err(|e| ClaudeError::SpawnFailed {
                stage: "post-SIGKILL wait".to_string(),
                source: e,
            })
        }
    }
}

/// Collect any remaining buffered lines from a channel (non-blocking)
async fn collect_remaining(rx: &mut mpsc::Receiver<String>) -> String {
    let mut lines = Vec::new();
    while let Ok(line) = rx.try_recv() {
        lines.push(line);
    }
    lines.join("\n")
}
