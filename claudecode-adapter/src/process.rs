use crate::error::ClaudeError;
use crate::types::{OutputFormat, RunConfig, RunResult};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

pub async fn run_claude(
    path: &std::path::Path,
    prompt: &str,
    config: &RunConfig,
    sender: Option<tokio::sync::mpsc::UnboundedSender<crate::types::StreamEvent>>,
) -> Result<RunResult, ClaudeError> {
    let args = crate::cmd::build_args(prompt, config);
    let start_time = Instant::now();

    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let stderr = child.stderr.take().expect("Failed to open stderr");

    let captured_stdout = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    let captured_stderr = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    let stream_events = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let stdout_cap = captured_stdout.clone();
    let stream_events_cap = stream_events.clone();
    let format = config.output_format;

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let mut s = stdout_cap.lock().await;
            if format == Some(OutputFormat::StreamJson) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                    stream_events_cap.lock().await.push(val.clone());
                    // Try to parse as a specific StreamEvent
                    if let Some(tx) = &sender {
                        if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val)
                        {
                            let _ = tx.send(event);
                        }
                    }
                }
            }
            s.push_str(&line);
            s.push('\n');
        }
    });

    let stderr_cap = captured_stderr.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let mut s = stderr_cap.lock().await;
            s.push_str(&line);
            s.push('\n');
        }
    });

    let wait_task = async {
        let status = child.wait().await?;
        let _ = stdout_task.await;
        let _ = stderr_task.await;
        Ok::<_, ClaudeError>(status)
    };

    match timeout(config.timeout, wait_task).await {
        Ok(res) => {
            let status = res?;
            let duration = start_time.elapsed();
            let final_stdout = captured_stdout.lock().await.clone();
            let final_stderr = captured_stderr.lock().await.clone();
            let final_events = stream_events.lock().await.clone();

            let json = if config.output_format == Some(OutputFormat::Json) {
                serde_json::from_str(&final_stdout).ok()
            } else {
                None
            };

            Ok(RunResult {
                stdout: final_stdout,
                stderr: final_stderr,
                exit_code: status.code().unwrap_or(-1),
                duration_ms: duration.as_millis() as u64,
                json,
                stream_events: final_events,
                structured_output: None, // Placeholder for future logic
            })
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(ClaudeError::Timeout(config.timeout))
        }
    }
}
