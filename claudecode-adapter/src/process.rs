//! Subprocess execution with streaming, timeouts, and signal handling.

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

/// Bounded channel capacity for internal stdout / stderr pipes.
const CHANNEL_CAPACITY: usize = 100;
/// Maximum bytes captured from a single pipe before truncation.
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB
/// Time to wait for a graceful SIGTERM exit before sending SIGKILL.
const GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Executes the Claude CLI as a subprocess, optionally streaming events.
///
/// # Errors
///
/// Returns `ClaudeError` when the subprocess cannot be spawned, an I/O pipe
/// fails, the configured timeout expires, or output exceeds the size limit.
pub async fn run_claude(
    path: &std::path::Path,
    prompt: &str,
    config: &RunConfig,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, ClaudeError> {
    let args = crate::cmd::build_args(prompt, config);
    let start_time = Instant::now();

    let mut child = spawn_child(path, &args, config)?;

    let stdout = child.stdout.take().ok_or(ClaudeError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(ClaudeError::NoStderr)?;
    let pid = child.id().ok_or(ClaudeError::NoPid)?;

    let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);

    let mut tasks = JoinSet::new();
    let format = config.output_format;

    tasks.spawn(async move {
        drain_stdout_bounded(stdout, stdout_tx, sender, format, MAX_OUTPUT_BYTES).await
    });
    tasks.spawn(async move { drain_stderr_bounded(stderr, stderr_tx, MAX_OUTPUT_BYTES).await });

    let execution = execute_and_collect(
        &mut child,
        &mut stdout_rx,
        &mut stderr_rx,
        &mut tasks,
        format,
        start_time,
        config.output_format,
    );

    if let Ok(result) = timeout(config.timeout, execution).await {
        result
    } else {
        handle_timeout(&mut child, pid, &mut stdout_rx, &mut stderr_rx, &mut tasks, config).await
    }
}

/// Spawns the Claude CLI subprocess with the given arguments and config.
fn spawn_child(
    path: &std::path::Path,
    args: &[std::ffi::OsString],
    config: &RunConfig,
) -> Result<tokio::process::Child, ClaudeError> {
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    cmd.spawn().map_err(|e| ClaudeError::SpawnFailed {
        stage: "subprocess spawn".to_string(),
        source: e,
    })
}

/// Drains both stdout/stderr channels, waits for the child to exit, then
/// joins all reader tasks and assembles the final `RunResult`.
async fn execute_and_collect(
    child: &mut tokio::process::Child,
    stdout_rx: &mut mpsc::Receiver<String>,
    stderr_rx: &mut mpsc::Receiver<String>,
    tasks: &mut JoinSet<Result<(), ClaudeError>>,
    format: Option<OutputFormat>,
    start_time: Instant,
    output_format: Option<OutputFormat>,
) -> Result<RunResult, ClaudeError> {
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut stream_events = Vec::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            result = stdout_rx.recv(), if !stdout_done => {
                if let Some(line) = result {
                    if format == Some(OutputFormat::StreamJson) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                            stream_events.push(val);
                        }
                    }
                    stdout_lines.push(line);
                } else {
                    stdout_done = true;
                }
            }
            result = stderr_rx.recv(), if !stderr_done => {
                if let Some(line) = result {
                    stderr_lines.push(line);
                } else {
                    stderr_done = true;
                }
            }
        }
    }

    let status = child.wait().await.map_err(|e| ClaudeError::SpawnFailed {
        stage: "child.wait()".to_string(),
        source: e,
    })?;

    while let Some(result) = tasks.join_next().await {
        result.map_err(|e| ClaudeError::StreamFailed {
            stage: "reader task join".to_string(),
            source: e,
        })??;
    }

    let duration = start_time.elapsed();
    let final_stdout = stdout_lines.join("\n");
    let final_stderr = stderr_lines.join("\n");

    let json = if output_format == Some(OutputFormat::Json) {
        serde_json::from_str(&final_stdout).ok()
    } else {
        None
    };

    Ok(RunResult {
        stdout: final_stdout,
        stderr: final_stderr,
        exit_code: status.code().unwrap_or(-1),
        duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        json,
        stream_events,
        structured_output: None,
    })
}

/// Handles a timeout by collecting remaining output, gracefully shutting down
/// the child process, and returning a `ClaudeError::Timeout`.
async fn handle_timeout(
    child: &mut tokio::process::Child,
    pid: u32,
    stdout_rx: &mut mpsc::Receiver<String>,
    stderr_rx: &mut mpsc::Receiver<String>,
    tasks: &mut JoinSet<Result<(), ClaudeError>>,
    config: &RunConfig,
) -> Result<RunResult, ClaudeError> {
    let partial_stdout = collect_remaining(stdout_rx);
    let partial_stderr = collect_remaining(stderr_rx);

    let _ = graceful_shutdown(child, pid).await;
    tasks.abort_all();

    Err(ClaudeError::Timeout {
        elapsed: config.timeout,
        pid,
        partial_stdout,
        partial_stderr,
    })
}

/// Drains stdout with bounded memory, parses JSONL, and forwards stream events.
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
        total_bytes += line.len();

        if total_bytes > max_bytes {
            return Err(ClaudeError::OutputTruncated {
                captured_bytes: total_bytes,
                limit_bytes: max_bytes,
            });
        }

        if format == Some(OutputFormat::StreamJson) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ref stream_tx) = sender {
                    if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val) {
                        let _ = stream_tx.send(event).await;
                    }
                }
            }
        }

        if tx.send(line).await.is_err() {
            break;
        }
    }

    Ok(())
}

/// Drains stderr with bounded memory.
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
        total_bytes += line.len();

        if total_bytes > max_bytes {
            return Err(ClaudeError::OutputTruncated {
                captured_bytes: total_bytes,
                limit_bytes: max_bytes,
            });
        }

        if tx.send(line).await.is_err() {
            break;
        }
    }

    Ok(())
}

/// Sends SIGTERM, waits up to `GRACE_PERIOD`, then force-kills with SIGKILL.
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    let nix_pid = Pid::from_raw(pid.cast_signed());
    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| ClaudeError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        source: e,
    })?;

    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(status)) => Ok(status),
        Ok(Err(e)) => Err(ClaudeError::SpawnFailed {
            stage: "graceful_shutdown wait".to_string(),
            source: e,
        }),
        Err(_) => {
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

/// Collects any remaining buffered lines from a channel without blocking.
fn collect_remaining(rx: &mut mpsc::Receiver<String>) -> String {
    let mut lines = Vec::new();
    while let Ok(line) = rx.try_recv() {
        lines.push(line);
    }
    lines.join("\n")
}
