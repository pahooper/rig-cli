use crate::error::OpenCodeError;
use crate::types::{OpenCodeConfig, RunResult};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

pub async fn run_opencode(
    path: &std::path::Path,
    message: &str,
    config: &OpenCodeConfig,
    sender: Option<tokio::sync::mpsc::UnboundedSender<crate::types::StreamEvent>>,
) -> Result<RunResult, OpenCodeError> {
    let args = crate::cmd::build_args(message, config);
    let start_time = Instant::now();

    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let stderr = child.stderr.take().expect("Failed to open stderr");

    let captured_stdout = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    let captured_stderr = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));

    let stdout_cap = captured_stdout.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let mut s = stdout_cap.lock().await;

            if let Some(tx) = &sender {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                     if let Ok(event) = serde_json::from_value::<crate::types::StreamEvent>(val) {
                         let _ = tx.send(event);
                     } else {
                         // Fallback
                     }
                } else {
                    // Raw text line
                    let _ = tx.send(crate::types::StreamEvent::Text { text: line.clone() + "\n" });
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
        Ok::<_, OpenCodeError>(status)
    };

    match timeout(config.timeout, wait_task).await {
        Ok(res) => {
            let status = res?;
            let duration = start_time.elapsed();
            let final_stdout = captured_stdout.lock().await.clone();
            let final_stderr = captured_stderr.lock().await.clone();

            Ok(RunResult {
                stdout: final_stdout,
                stderr: final_stderr,
                exit_code: status.code().unwrap_or(-1),
                duration_ms: duration.as_millis() as u64,
            })
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(OpenCodeError::Timeout(config.timeout))
        }
    }
}
