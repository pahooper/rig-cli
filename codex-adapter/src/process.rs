//! Subprocess execution and lifecycle management for the Codex CLI.

use crate::error::CodexError;
use crate::types::{CodexConfig, RunResult};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::timeout;

const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Collected output lines, total byte count, and whether truncation occurred.
type StreamOutput = (Vec<String>, usize, bool);

/// Result of collecting all subprocess output: stdout lines, stderr lines, and exit status.
type CollectedOutput = (Vec<String>, Vec<String>, std::process::ExitStatus);

/// Timeout-wrapped result of the full collection phase.
type TimedCollectionResult =
    Result<Result<CollectedOutput, CodexError>, tokio::time::error::Elapsed>;

/// Spawns the Codex CLI and collects its output, optionally streaming events.
///
/// # Errors
/// Returns a [`CodexError`] if the process cannot be spawned, times out,
/// produces truncated output, or encounters an I/O failure.
pub async fn run_codex(
    path: &std::path::Path,
    prompt: &str,
    config: &CodexConfig,
    sender: Option<tokio::sync::mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, CodexError> {
    let args = crate::cmd::build_args(prompt, config);
    let start_time = Instant::now();

    let mut child = spawn_child(path, &args, config)?;

    let stdout = child.stdout.take().ok_or(CodexError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(CodexError::NoStderr)?;
    let pid = child.id().ok_or(CodexError::NoPid)?;

    let mut tasks = JoinSet::new();

    // Stdout reader task
    tasks.spawn(async move {
        drain_stream_bounded(stdout, sender, "stdout").await
    });

    // Stderr reader task
    tasks.spawn(async move {
        drain_stream_bounded(stderr, None, "stderr").await
    });

    let process_result = timeout(config.timeout, collect_output(&mut child, &mut tasks)).await;
    let duration = start_time.elapsed();

    build_run_result(process_result, &mut child, pid, &mut tasks, duration).await
}

/// Spawns the Codex child process with piped stdout/stderr.
fn spawn_child(
    path: &std::path::Path,
    args: &[std::ffi::OsString],
    config: &CodexConfig,
) -> Result<tokio::process::Child, CodexError> {
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(ref dir) = config.cd {
        cmd.current_dir(dir);
    }

    for (k, v) in &config.env_vars {
        cmd.env(k, v);
    }

    cmd.spawn().map_err(|e| CodexError::SpawnFailed {
        stage: "spawn".to_string(),
        source: e,
    })
}

/// Collects stdout and stderr output from reader tasks and waits for the child.
async fn collect_output(
    child: &mut tokio::process::Child,
    tasks: &mut JoinSet<StreamOutput>,
) -> Result<CollectedOutput, CodexError> {
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut results_received = 0u8;

    while let Some(result) = tasks.join_next().await {
        let (lines, _bytes, truncated) = result.map_err(|e| CodexError::StreamFailed {
            stage: "join".to_string(),
            source: e,
        })?;

        if truncated {
            let captured: usize = lines.iter().map(String::len).sum();
            return Err(CodexError::OutputTruncated {
                captured_bytes: captured,
                limit_bytes: MAX_OUTPUT_BYTES,
            });
        }

        // First result is stdout (spawned first), second is stderr.
        if results_received == 0 {
            stdout_lines = lines;
        } else {
            stderr_lines = lines;
        }
        results_received += 1;
    }

    let status = child.wait().await.map_err(|e| CodexError::SpawnFailed {
        stage: "wait".to_string(),
        source: e,
    })?;

    Ok((stdout_lines, stderr_lines, status))
}

/// Converts the raw process outcome into a [`RunResult`] or an appropriate error.
async fn build_run_result(
    process_result: TimedCollectionResult,
    child: &mut tokio::process::Child,
    pid: u32,
    tasks: &mut JoinSet<StreamOutput>,
    duration: Duration,
) -> Result<RunResult, CodexError> {
    let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);

    match process_result {
        Ok(Ok((stdout_lines, stderr_lines, status))) => Ok(RunResult {
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
            exit_code: status.code().unwrap_or(-1),
            duration_ms,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Timeout occurred -- graceful shutdown
            let _ = graceful_shutdown(child, pid, tasks).await;

            Err(CodexError::Timeout {
                elapsed: duration,
                pid,
                partial_stdout: String::new(),
                partial_stderr: String::new(),
            })
        }
    }
}

/// Drains a stream with bounded accumulation and optional JSONL parsing for `StreamEvent`.
async fn drain_stream_bounded(
    stream: impl tokio::io::AsyncRead + Unpin,
    event_tx: Option<mpsc::Sender<crate::types::StreamEvent>>,
    _stage: &str,
) -> StreamOutput {
    let mut reader = BufReader::new(stream).lines();
    let mut lines = Vec::new();
    let mut total_bytes = 0usize;
    let mut truncated = false;

    loop {
        let Ok(Some(line)) = reader.next_line().await else {
            break;
        };

        // Forward to event sender if configured.
        if let Some(ref tx) = event_tx {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val) {
                    let _ = tx.send(event).await;
                }
            } else {
                let _ = tx
                    .send(crate::types::StreamEvent::Text {
                        text: line.clone() + "\n",
                    })
                    .await;
            }
        }

        let line_bytes = line.len();
        if total_bytes + line_bytes <= MAX_OUTPUT_BYTES {
            lines.push(line);
            total_bytes += line_bytes;
        } else {
            truncated = true;
        }
    }

    (lines, total_bytes, truncated)
}

/// Graceful shutdown: `SIGTERM`, wait grace period, then `SIGKILL`.
#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
    tasks: &mut JoinSet<StreamOutput>,
) -> Result<(), CodexError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let raw_pid = i32::try_from(pid).map_err(|_| CodexError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: "PID value exceeds i32::MAX".to_string(),
    })?;
    let nix_pid = Pid::from_raw(raw_pid);

    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| CodexError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: e.to_string(),
    })?;

    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(_status)) => {}
        Ok(Err(e)) => {
            return Err(CodexError::SpawnFailed {
                stage: "graceful_shutdown wait".to_string(),
                source: e,
            });
        }
        Err(_) => {
            child.kill().await.map_err(|e| CodexError::SpawnFailed {
                stage: "SIGKILL".to_string(),
                source: e,
            })?;
            child.wait().await.map_err(|e| CodexError::SpawnFailed {
                stage: "post-SIGKILL wait".to_string(),
                source: e,
            })?;
        }
    }

    tasks.abort_all();
    Ok(())
}

/// Windows: immediate termination, no graceful shutdown for console processes.
#[cfg(windows)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    _pid: u32,
    tasks: &mut JoinSet<StreamOutput>,
) -> Result<(), CodexError> {
    child.kill().await.map_err(|e| CodexError::SpawnFailed {
        stage: "TerminateProcess".to_string(),
        source: e,
    })?;
    child.wait().await.map_err(|e| CodexError::SpawnFailed {
        stage: "post-kill wait".to_string(),
        source: e,
    })?;
    tasks.abort_all();
    Ok(())
}
