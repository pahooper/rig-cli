//! Subprocess lifecycle management for `OpenCode` invocations.

use crate::error::OpenCodeError;
use crate::types::{OpenCodeConfig, RunResult};
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

/// Mutable state shared across the output-accumulation helpers.
struct OutputState {
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    stdout_lines: Vec<String>,
    stderr_lines: Vec<String>,
    stdout_bytes: usize,
    stderr_bytes: usize,
    join_set: JoinSet<()>,
}

/// Runs `OpenCode` as a child process, optionally streaming events.
///
/// If `sender` is provided, parsed events are forwarded in real time.
/// Output is bounded to 10MB per stream to prevent memory exhaustion.
///
/// # Errors
///
/// Returns `OpenCodeError` if:
/// - The `OpenCode` process fails to spawn (`SpawnFailed`)
/// - Stdout or stderr handles cannot be captured (`NoStdout`, `NoStderr`)
/// - The process exits with non-zero status (`NonZeroExit`)
pub async fn run_opencode(
    path: &std::path::Path,
    message: &str,
    config: &OpenCodeConfig,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, OpenCodeError> {
    let start_time = Instant::now();
    let (mut child, pid) = spawn_child(path, message, config)?;

    let stdout = child.stdout.take().ok_or(OpenCodeError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(OpenCodeError::NoStderr)?;

    let (stdout_tx, stdout_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
    let (stderr_tx, stderr_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);

    let mut state = OutputState {
        stdout_rx,
        stderr_rx,
        stdout_lines: Vec::new(),
        stderr_lines: Vec::new(),
        stdout_bytes: 0,
        stderr_bytes: 0,
        join_set: JoinSet::new(),
    };

    spawn_readers(
        &mut state.join_set,
        stdout,
        stderr,
        stdout_tx,
        stderr_tx,
        sender,
    );

    let execution_result = timeout(
        config.timeout,
        accumulate_output(&mut child, &mut state, start_time, pid),
    )
    .await;

    match execution_result {
        Ok(result) => result,
        Err(_timeout_elapsed) => handle_timeout(&mut child, pid, &mut state, start_time).await,
    }
}

/// Spawns the `OpenCode` child process and returns it with its PID.
fn spawn_child(
    path: &std::path::Path,
    message: &str,
    config: &OpenCodeConfig,
) -> Result<(tokio::process::Child, u32), OpenCodeError> {
    let args = crate::cmd::build_args(message, config);
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env_vars {
        cmd.env(k, v);
    }

    if let Some(ref mcp_path) = config.mcp_config_path {
        cmd.env("OPENCODE_CONFIG", mcp_path);
    }

    let child = cmd.spawn().map_err(|e| OpenCodeError::SpawnFailed {
        stage: "spawn subprocess".to_string(),
        source: e,
    })?;

    let pid = child.id().ok_or(OpenCodeError::NoPid)?;
    Ok((child, pid))
}

/// Spawns async reader tasks for stdout and stderr into the `JoinSet`.
fn spawn_readers(
    join_set: &mut JoinSet<()>,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    stdout_tx: mpsc::Sender<String>,
    stderr_tx: mpsc::Sender<String>,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) {
    join_set.spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(tx) = &sender {
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
            if stdout_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    join_set.spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if stderr_tx.send(line).await.is_err() {
                break;
            }
        }
    });
}

/// Main select loop that accumulates stdout/stderr and waits for exit.
async fn accumulate_output(
    child: &mut tokio::process::Child,
    state: &mut OutputState,
    start_time: Instant,
    pid: u32,
) -> Result<RunResult, OpenCodeError> {
    loop {
        tokio::select! {
            Some(line) = state.stdout_rx.recv() => {
                push_bounded(line, &mut state.stdout_lines, &mut state.stdout_bytes)?;
            }
            Some(line) = state.stderr_rx.recv() => {
                push_bounded(line, &mut state.stderr_lines, &mut state.stderr_bytes)?;
            }
            status = child.wait() => {
                drain_remaining(state)?;

                state.join_set.abort_all();
                while state.join_set.join_next().await.is_some() {}

                let status = status.map_err(|e| OpenCodeError::SpawnFailed {
                    stage: "wait for child".to_string(),
                    source: e,
                })?;

                let duration = start_time.elapsed();
                let exit_code = status.code().unwrap_or(-1);

                let stdout = state.stdout_lines.join("\n");
                let stderr = state.stderr_lines.join("\n");

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
                    duration_ms: duration_to_millis(duration),
                });
            }
        }
    }
}

/// Pushes a line into the accumulator, enforcing the byte limit.
fn push_bounded(
    line: String,
    lines: &mut Vec<String>,
    bytes: &mut usize,
) -> Result<(), OpenCodeError> {
    *bytes += line.len();
    if *bytes > MAX_OUTPUT_BYTES {
        return Err(OpenCodeError::OutputTruncated {
            captured_bytes: *bytes,
            limit_bytes: MAX_OUTPUT_BYTES,
        });
    }
    lines.push(line);
    Ok(())
}

/// Handles the timeout path: graceful shutdown, drain, and error.
async fn handle_timeout(
    child: &mut tokio::process::Child,
    pid: u32,
    state: &mut OutputState,
    start_time: Instant,
) -> Result<RunResult, OpenCodeError> {
    let elapsed = start_time.elapsed();
    let _ = graceful_shutdown(child, pid).await;

    drain_remaining(state)?;

    state.join_set.abort_all();
    while state.join_set.join_next().await.is_some() {}

    Err(OpenCodeError::Timeout {
        elapsed,
        pid,
        partial_stdout: state.stdout_lines.join("\n"),
        partial_stderr: state.stderr_lines.join("\n"),
    })
}

/// Drains remaining buffered lines from both channels synchronously.
fn drain_remaining(state: &mut OutputState) -> Result<(), OpenCodeError> {
    drain_channel(
        &mut state.stdout_rx,
        &mut state.stdout_lines,
        &mut state.stdout_bytes,
    )?;
    drain_channel(
        &mut state.stderr_rx,
        &mut state.stderr_lines,
        &mut state.stderr_bytes,
    )?;
    Ok(())
}

/// Drains a single channel with bounded memory enforcement.
fn drain_channel(
    rx: &mut mpsc::Receiver<String>,
    lines: &mut Vec<String>,
    bytes: &mut usize,
) -> Result<(), OpenCodeError> {
    while let Ok(line) = rx.try_recv() {
        *bytes += line.len();
        if *bytes > MAX_OUTPUT_BYTES {
            return Err(OpenCodeError::OutputTruncated {
                captured_bytes: *bytes,
                limit_bytes: MAX_OUTPUT_BYTES,
            });
        }
        lines.push(line);
    }
    Ok(())
}

/// Graceful shutdown: `SIGTERM`, wait grace period, then `SIGKILL`.
#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<(), OpenCodeError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let raw = i32::try_from(pid).map_err(|_| OpenCodeError::SpawnFailed {
        stage: "PID conversion overflow".to_string(),
        source: std::io::Error::other("PID value exceeds i32::MAX"),
    })?;
    let nix_pid = Pid::from_raw(raw);

    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| OpenCodeError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: e.to_string(),
    })?;

    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(_status)) => Ok(()),
        Ok(Err(e)) => Err(OpenCodeError::SpawnFailed {
            stage: "graceful_shutdown wait".to_string(),
            source: e,
        }),
        Err(_) => {
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

/// Windows: immediate termination, no graceful shutdown for console processes.
#[cfg(windows)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    _pid: u32,
) -> Result<(), OpenCodeError> {
    child.kill().await.map_err(|e| OpenCodeError::SpawnFailed {
        stage: "TerminateProcess".to_string(),
        source: e,
    })?;
    child.wait().await.map_err(|e| OpenCodeError::SpawnFailed {
        stage: "post-kill wait".to_string(),
        source: e,
    })?;
    Ok(())
}

/// Converts a `Duration` to milliseconds as `u64`, saturating on overflow.
fn duration_to_millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
