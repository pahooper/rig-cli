use crate::error::CodexError;
use crate::types::{CodexConfig, RunResult};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

const CHANNEL_CAPACITY: usize = 100;
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn run_codex(
    path: &std::path::Path,
    prompt: &str,
    config: &CodexConfig,
    sender: Option<tokio::sync::mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, CodexError> {
    let args = crate::cmd::build_args(prompt, config);
    let start_time = Instant::now();

    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| CodexError::SpawnFailed {
            stage: "spawn".to_string(),
            source: e,
        })?;

    let stdout = child.stdout.take().ok_or(CodexError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(CodexError::NoStderr)?;
    let pid = child.id().ok_or(CodexError::NoPid)?;

    // Create bounded internal channels for stdout and stderr lines
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);

    // Spawn reader tasks in JoinSet
    let mut tasks = JoinSet::new();

    // Stdout reader task
    let sender_clone = sender.clone();
    tasks.spawn(async move {
        drain_stream_bounded(stdout, stdout_tx, sender_clone, "stdout").await
    });

    // Stderr reader task
    tasks.spawn(async move {
        drain_stream_bounded(stderr, stderr_tx, None, "stderr").await
    });

    // Accumulate output lines from both channels
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut stdout_bytes = 0;
    let mut stderr_bytes = 0;
    let mut stdout_truncated = false;
    let mut stderr_truncated = false;

    // Main processing loop with timeout wrapper
    let process_result = timeout(config.timeout, async {
        loop {
            tokio::select! {
                Some(line) = stdout_rx.recv() => {
                    let line_bytes = line.len();
                    if stdout_bytes + line_bytes <= MAX_OUTPUT_BYTES {
                        stdout_lines.push(line);
                        stdout_bytes += line_bytes;
                    } else {
                        stdout_truncated = true;
                    }
                }
                Some(line) = stderr_rx.recv() => {
                    let line_bytes = line.len();
                    if stderr_bytes + line_bytes <= MAX_OUTPUT_BYTES {
                        stderr_lines.push(line);
                        stderr_bytes += line_bytes;
                    } else {
                        stderr_truncated = true;
                    }
                }
                else => break,
            }
        }

        // Wait for child process
        let status = child.wait().await.map_err(|e| CodexError::SpawnFailed {
            stage: "wait".to_string(),
            source: e,
        })?;

        Ok::<_, CodexError>(status)
    })
    .await;

    let duration = start_time.elapsed();

    match process_result {
        Ok(Ok(status)) => {
            // Process completed successfully (may have non-zero exit)
            // Wait for reader tasks to complete
            while let Some(result) = tasks.join_next().await {
                result.map_err(|e| CodexError::StreamFailed {
                    stage: "join".to_string(),
                    source: e,
                })?;
            }

            let final_stdout = stdout_lines.join("\n");
            let final_stderr = stderr_lines.join("\n");

            if stdout_truncated {
                return Err(CodexError::OutputTruncated {
                    captured_bytes: stdout_bytes,
                    limit_bytes: MAX_OUTPUT_BYTES,
                });
            }
            if stderr_truncated {
                return Err(CodexError::OutputTruncated {
                    captured_bytes: stderr_bytes,
                    limit_bytes: MAX_OUTPUT_BYTES,
                });
            }

            Ok(RunResult {
                stdout: final_stdout,
                stderr: final_stderr,
                exit_code: status.code().unwrap_or(-1),
                duration_ms: duration.as_millis() as u64,
            })
        }
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Timeout occurred - graceful shutdown
            graceful_shutdown(pid, &mut tasks).await?;

            let partial_stdout = stdout_lines.join("\n");
            let partial_stderr = stderr_lines.join("\n");

            Err(CodexError::Timeout {
                elapsed: duration,
                pid,
                partial_stdout,
                partial_stderr,
            })
        }
    }
}

/// Drain a stream with bounded accumulation and optional JSONL parsing for StreamEvents
async fn drain_stream_bounded(
    stream: impl tokio::io::AsyncRead + Unpin,
    line_tx: mpsc::Sender<String>,
    event_tx: Option<mpsc::Sender<crate::types::StreamEvent>>,
    _stage: &str,
) -> Result<(), CodexError> {
    let mut reader = BufReader::new(stream).lines();

    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|e| CodexError::SpawnFailed {
            stage: "read_line".to_string(),
            source: e,
        })?
    {
        // Forward to event sender if configured
        if let Some(ref tx) = event_tx {
            // Try to parse as JSON for StreamEvent
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val) {
                    let _ = tx.send(event).await;
                }
            } else {
                // Raw text line
                let _ = tx
                    .send(crate::types::StreamEvent::Text {
                        text: line.clone() + "\n",
                    })
                    .await;
            }
        }

        // Send to line accumulator
        if line_tx.send(line).await.is_err() {
            break; // Channel closed, stop reading
        }
    }

    Ok(())
}

/// Gracefully shutdown subprocess: SIGTERM -> wait -> SIGKILL
async fn graceful_shutdown(pid: u32, tasks: &mut JoinSet<Result<(), CodexError>>) -> Result<(), CodexError> {
    let nix_pid = Pid::from_raw(pid as i32);

    // Send SIGTERM
    if let Err(e) = signal::kill(nix_pid, Signal::SIGTERM) {
        return Err(CodexError::SignalFailed {
            signal: "SIGTERM".to_string(),
            pid,
            source: e,
        });
    }

    // Wait for grace period
    sleep(GRACE_PERIOD).await;

    // Send SIGKILL
    if let Err(e) = signal::kill(nix_pid, Signal::SIGKILL) {
        // If SIGKILL fails, process may have already exited (ESRCH is OK)
        if e != nix::errno::Errno::ESRCH {
            return Err(CodexError::SignalFailed {
                signal: "SIGKILL".to_string(),
                pid,
                source: e,
            });
        }
    }

    // Abort all reader tasks
    tasks.abort_all();

    Ok(())
}
