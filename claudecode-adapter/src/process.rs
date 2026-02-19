//! Subprocess execution with streaming, timeouts, and signal handling.

use crate::error::ClaudeError;
use crate::types::{OutputFormat, RunConfig, RunResult, SystemPromptMode};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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

/// Byte threshold above which prompts and system prompts are offloaded from
/// CLI positional arguments.  Windows limits total arg length to ~32 KB; we
/// use 30 KB as a safe cross-platform threshold.
const ARG_THRESHOLD: usize = 30_000;

/// Executes the Claude CLI as a subprocess, optionally streaming events.
///
/// When the prompt exceeds [`ARG_THRESHOLD`] bytes it is piped via **stdin**
/// instead of being passed as a positional CLI argument.  This avoids the
/// Windows ~32 KB argument-length limit while keeping the prompt transparent
/// to the agent (no Read tool required).
///
/// If the stdin approach produces an empty result (the Bug #7263 signature —
/// exit 0, empty stdout), the function automatically retries using a **temp
/// file** fallback: the prompt is written to a temp file and the agent is
/// instructed to read it, with the `Read` builtin tool temporarily granted
/// if the config uses `BuiltinToolSet::None`.
///
/// System prompts exceeding the threshold are always handled via the
/// official `--append-system-prompt-file` / `--system-prompt-file` flags.
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
    let use_stdin = prompt.len() > ARG_THRESHOLD;

    // --- System prompt: temp file if large, inline otherwise ---------------
    let sys_prompt_text = match &config.system_prompt {
        SystemPromptMode::Append(p) | SystemPromptMode::Replace(p) => Some(p.as_str()),
        SystemPromptMode::None => None,
    };
    let needs_sys_file = sys_prompt_text.map_or(false, |t| t.len() > ARG_THRESHOLD);

    let _sys_prompt_file: Option<NamedTempFile> = if needs_sys_file {
        Some(write_temp_file("rig_sysprompt_", sys_prompt_text.unwrap_or_default())?)
    } else {
        None
    };

    // --- Primary path: stdin for large prompts, positional arg otherwise ---
    let effective_prompt = if use_stdin { "" } else { prompt };

    let args = crate::cmd::build_args(
        effective_prompt,
        config,
        _sys_prompt_file.as_ref().map(|f| f.path()),
    );

    let result = execute_once(path, &args, config, use_stdin, prompt, sender.clone()).await?;

    // --- Bug #7263 regression guard: empty stdout with stdin mode ----------
    // The bug signature is exit 0 + empty stdout when prompt was piped via
    // stdin.  On detection we retry with a temp-file fallback.
    if use_stdin && result.exit_code == 0 && result.stdout.trim().is_empty() {
        tracing::warn!(
            prompt_bytes = prompt.len(),
            "Empty stdout with stdin mode — possible Bug #7263 regression, retrying with temp file"
        );
        return run_with_tempfile_fallback(path, prompt, config, &_sys_prompt_file, sender).await;
    }

    Ok(result)
}

/// Fallback path: write prompt to temp file, grant Read tool if needed.
async fn run_with_tempfile_fallback(
    path: &std::path::Path,
    prompt: &str,
    config: &RunConfig,
    sys_prompt_file: &Option<NamedTempFile>,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, ClaudeError> {
    let _prompt_file = write_temp_file("rig_prompt_", prompt)?;
    let instruction = format!(
        "Read the file at {} and follow the instructions within.",
        _prompt_file.path().display()
    );

    // When the agent has BuiltinToolSet::None we need to grant Read so it
    // can access the prompt file.
    let config_override;
    let effective_config =
        if matches!(config.tools.builtin, crate::types::BuiltinToolSet::None) {
            config_override = RunConfig {
                tools: crate::types::ToolPolicy {
                    builtin: crate::types::BuiltinToolSet::Explicit(vec!["Read".to_string()]),
                    ..config.tools.clone()
                },
                ..config.clone()
            };
            &config_override
        } else {
            config
        };

    let args = crate::cmd::build_args(
        &instruction,
        effective_config,
        sys_prompt_file.as_ref().map(|f| f.path()),
    );

    execute_once(path, &args, effective_config, false, "", sender).await
}

/// Creates a named temp file with the given prefix and content.
fn write_temp_file(prefix: &str, content: &str) -> Result<NamedTempFile, ClaudeError> {
    let f = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".txt")
        .tempfile()
        .map_err(|e| ClaudeError::SpawnFailed {
            stage: format!("{prefix} temp file creation"),
            source: e,
        })?;
    std::fs::write(f.path(), content).map_err(|e| ClaudeError::SpawnFailed {
        stage: format!("{prefix} temp file write"),
        source: e,
    })?;
    Ok(f)
}

/// Spawns a single Claude CLI subprocess, optionally piping the prompt via
/// stdin, collects output, and returns the result.
async fn execute_once(
    path: &std::path::Path,
    args: &[std::ffi::OsString],
    config: &RunConfig,
    pipe_stdin: bool,
    stdin_content: &str,
    sender: Option<mpsc::Sender<crate::types::StreamEvent>>,
) -> Result<RunResult, ClaudeError> {
    let start_time = Instant::now();

    let mut child = spawn_child(path, args, config, pipe_stdin)?;

    // Write prompt to stdin and close the pipe so Claude sees EOF.
    if pipe_stdin {
        if let Some(mut stdin_pipe) = child.stdin.take() {
            stdin_pipe
                .write_all(stdin_content.as_bytes())
                .await
                .map_err(|e| ClaudeError::SpawnFailed {
                    stage: "stdin write".to_string(),
                    source: e,
                })?;
            drop(stdin_pipe);
        }
    }

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
        handle_timeout(
            &mut child,
            pid,
            &mut stdout_rx,
            &mut stderr_rx,
            &mut tasks,
            config,
        )
        .await
    }
}

/// Spawns the Claude CLI subprocess with the given arguments and config.
fn spawn_child(
    path: &std::path::Path,
    args: &[std::ffi::OsString],
    config: &RunConfig,
    pipe_stdin: bool,
) -> Result<tokio::process::Child, ClaudeError> {
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    // On Windows, prevent a visible console window from flashing when spawning
    // the CLI subprocess from a GUI application (windows_subsystem = "windows").
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if pipe_stdin {
        cmd.stdin(Stdio::piped());
    }

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

    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|e| ClaudeError::SpawnFailed {
            stage: "stdout read".to_string(),
            source: e,
        })?
    {
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
                    // Try v1.x flat format first, fall back to v2.x envelope format
                    match serde_json::from_value::<crate::types::StreamEvent>(val.clone()) {
                        Ok(event) => {
                            let _ = stream_tx.send(event).await;
                        }
                        Err(_) => {
                            for event in crate::types::extract_v2_events(&val) {
                                let _ = stream_tx.send(event).await;
                            }
                        }
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

    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|e| ClaudeError::SpawnFailed {
            stage: "stderr read".to_string(),
            source: e,
        })?
    {
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
#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let nix_pid = Pid::from_raw(pid.cast_signed());
    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| ClaudeError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: e.to_string(),
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

/// Windows: no graceful shutdown mechanism for console processes.
/// Uses immediate TerminateProcess via Child::kill().
#[cfg(windows)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    _pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    child.kill().await.map_err(|e| ClaudeError::SpawnFailed {
        stage: "TerminateProcess".to_string(),
        source: e,
    })?;
    child.wait().await.map_err(|e| ClaudeError::SpawnFailed {
        stage: "post-kill wait".to_string(),
        source: e,
    })
}

/// Collects any remaining buffered lines from a channel without blocking.
fn collect_remaining(rx: &mut mpsc::Receiver<String>) -> String {
    let mut lines = Vec::new();
    while let Ok(line) = rx.try_recv() {
        lines.push(line);
    }
    lines.join("\n")
}
