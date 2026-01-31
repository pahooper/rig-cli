use crate::discovery::discover_claude;
use crate::error::ClaudeError;
use crate::types::{Capabilities, InitReport};
use std::path::PathBuf;
use tokio::process::Command;

pub async fn init(explicit_path: Option<PathBuf>) -> Result<InitReport, ClaudeError> {
    let path = discover_claude(explicit_path)?;

    let version_output = Command::new(&path).arg("--version").output().await?;
    let version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();

    // Proper health check: try to run a simple command to verify Claude is authenticated and working
    let health_check = Command::new(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    
    let (doctor_ok, doctor_stdout, doctor_stderr) = if let Ok(mut child) = health_check {
        // Write a simple test prompt
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(b"respond with: ok\n").await;
            drop(stdin);
        }
        
        // Give it 5 seconds to respond
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            child.wait_with_output()
        ).await;
        
        match timeout {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success() || (!stdout.is_empty() && !stderr.contains("error"));
                (success, stdout, stderr)
            }
            Ok(Err(e)) => {
                // IO error during execution
                (false, String::new(), format!("Health check IO error: {}", e))
            }
            Err(_) => {
                // Timeout - process will be killed when dropped
                (false, String::new(), "Health check timed out".to_string())
            }
        }
    } else {
        (false, String::new(), "Failed to spawn health check command".to_string())
    };

    let help_output = Command::new(&path).arg("--help").output().await?;
    let help_text = String::from_utf8_lossy(&help_output.stdout);

    let capabilities = Capabilities {
        supports_stream_json: help_text.contains("stream-json"),
        supports_json_schema: help_text.contains("--json-schema"),
        supports_system_prompt: help_text.contains("--system-prompt"),
        supports_append_system_prompt: help_text.contains("--append-system-prompt"),
        supports_mcp: help_text.contains("--mcp-config"),
        supports_strict_mcp: help_text.contains("--strict-mcp-config"),
        supports_tools_flag: help_text.contains("--tools"),
    };

    Ok(InitReport {
        claude_path: path,
        version,
        doctor_ok,
        doctor_stdout,
        doctor_stderr,
        capabilities,
    })
}
