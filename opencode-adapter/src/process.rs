use crate::error::OpenCodeError;
use crate::types::{OpenCodeConfig, RunResult};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::timeout;

const CHANNEL_CAPACITY: usize = 100;
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn run_opencode(
    path: &std::path::Path,
    message: &str,
    config: &OpenCodeConfig,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, OpenCodeError> {
    let args = crate::cmd::build_args(message, config);
    let start_time = Instant::now();

    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| OpenCodeError::SpawnFailed {
            stage: "spawn subprocess".to_string(),
            source: e,
        })?;

    let stdout = child.stdout.take().ok_or(OpenCodeError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(OpenCodeError::NoStderr)?;
    let pid = child.id().ok_or(OpenCodeError::NoPid)?;

    // Bounded internal channels
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);

    // JoinSet for tracking reader tasks
    let mut join_set = JoinSet::new();

    // Stdout reader task
    let sender_clone = sender.clone();
    join_set.spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Try to parse as StreamEvent for forwarding
            if let Some(tx) = &sender_clone {
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

            // Send to internal channel for accumulation (bounded backpressure)
            if stdout_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    // Stderr reader task
    join_set.spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if stderr_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    // Main accumulation loop with bounded memory
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut stdout_bytes = 0;
    let mut stderr_bytes = 0;

    // Timeout wrapper for main execution
    let execution_result = timeout(config.timeout, async {
        loop {
            tokio::select! {
                Some(line) = stdout_rx.recv() => {
                    stdout_bytes += line.len();
                    if stdout_bytes > MAX_OUTPUT_BYTES {
                        return Err(OpenCodeError::OutputTruncated {
                            captured_bytes: stdout_bytes,
                            limit_bytes: MAX_OUTPUT_BYTES,
                        });
                    }
                    stdout_lines.push(line);
                }
                Some(line) = stderr_rx.recv() => {
                    stderr_bytes += line.len();
                    if stderr_bytes > MAX_OUTPUT_BYTES {
                        return Err(OpenCodeError::OutputTruncated {
                            captured_bytes: stderr_bytes,
                            limit_bytes: MAX_OUTPUT_BYTES,
                        });
                    }
                    stderr_lines.push(line);
                }
                status = child.wait() => {
                    // Process exited, drain remaining buffered output
                    drain_stream_bounded(&mut stdout_rx, &mut stdout_lines, &mut stdout_bytes, MAX_OUTPUT_BYTES).await?;
                    drain_stream_bounded(&mut stderr_rx, &mut stderr_lines, &mut stderr_bytes, MAX_OUTPUT_BYTES).await?;

                    // Abort reader tasks and wait for cleanup
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}

                    let status = status.map_err(|e| OpenCodeError::SpawnFailed {
                        stage: "wait for child".to_string(),
                        source: e,
                    })?;

                    let duration = start_time.elapsed();
                    let exit_code = status.code().unwrap_or(-1);

                    let stdout = stdout_lines.join("\n");
                    let stderr = stderr_lines.join("\n");

                    if exit_code != 0 {
                        return Err(OpenCodeError::NonZeroExit {
                            exit_code,
                            pid,
                            elapsed: duration,
                            stdout,
                            stderr,
                        });
                    }

                    return Ok(RunResult {
                        stdout,
                        stderr,
                        exit_code,
                        duration_ms: duration.as_millis() as u64,
                    });
                }
            }
        }
    })
    .await;

    match execution_result {
        Ok(result) => result,
        Err(_timeout_elapsed) => {
            // Timeout - graceful shutdown with SIGTERM -> SIGKILL
            let elapsed = start_time.elapsed();
            let _ = graceful_shutdown(&mut child, pid).await;

            // Drain remaining output after shutdown
            drain_stream_bounded(
                &mut stdout_rx,
                &mut stdout_lines,
                &mut stdout_bytes,
                MAX_OUTPUT_BYTES,
            )
            .await?;
            drain_stream_bounded(
                &mut stderr_rx,
                &mut stderr_lines,
                &mut stderr_bytes,
                MAX_OUTPUT_BYTES,
            )
            .await?;

            join_set.abort_all();
            while join_set.join_next().await.is_some() {}

            Err(OpenCodeError::Timeout {
                elapsed,
                pid,
                partial_stdout: stdout_lines.join("\n"),
                partial_stderr: stderr_lines.join("\n"),
            })
        }
    }
}

/// Drain remaining lines from channel with bounded memory enforcement
async fn drain_stream_bounded(
    rx: &mut mpsc::Receiver<String>,
    lines: &mut Vec<String>,
    bytes: &mut usize,
    max_bytes: usize,
) -> Result<(), OpenCodeError> {
    while let Ok(line) = rx.try_recv() {
        *bytes += line.len();
        if *bytes > max_bytes {
            return Err(OpenCodeError::OutputTruncated {
                captured_bytes: *bytes,
                limit_bytes: max_bytes,
            });
        }
        lines.push(line);
    }
    Ok(())
}

/// Graceful shutdown: SIGTERM, wait grace period, then SIGKILL
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<(), OpenCodeError> {
    let nix_pid = Pid::from_raw(pid as i32);

    // Send SIGTERM for graceful shutdown
    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| OpenCodeError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        source: e,
    })?;

    // Wait for process to exit within grace period
    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(_status)) => Ok(()),
        Ok(Err(e)) => Err(OpenCodeError::SpawnFailed {
            stage: "graceful_shutdown wait".to_string(),
            source: e,
        }),
        Err(_) => {
            // Grace period expired - force kill
            child.kill().await.map_err(|e| OpenCodeError::SpawnFailed {
                stage: "SIGKILL".to_string(),
                source: e,
            })?;
            child.wait().await.map_err(|e| OpenCodeError::SpawnFailed {
                stage: "post-SIGKILL wait".to_string(),
                source: e,
            })?;
            Ok(())
        }
    }
}
