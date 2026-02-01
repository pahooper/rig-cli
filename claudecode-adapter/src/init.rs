//! Initialization and capability probing of the Claude CLI.

use crate::discovery::discover_claude;
use crate::error::ClaudeError;
use crate::types::{Capabilities, Feature, InitReport};
use std::path::PathBuf;
use tokio::process::Command;

/// Discovers the Claude CLI, checks its health, and probes capabilities.
///
/// # Errors
///
/// Returns `ClaudeError` if the executable cannot be found, the version
/// command fails, or an I/O error occurs during probing.
pub async fn init(explicit_path: Option<PathBuf>) -> Result<InitReport, ClaudeError> {
    let path = discover_claude(explicit_path)?;

    let version_output = Command::new(&path).arg("--version").output().await?;
    let version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();

    // Run a lightweight health check to verify the CLI is functional.
    let health_check = Command::new(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let (doctor_ok, doctor_stdout, doctor_stderr) = if let Ok(mut child) = health_check {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(b"respond with: ok\n").await;
            drop(stdin);
        }

        let wait_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            child.wait_with_output(),
        )
        .await;

        match wait_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success =
                    output.status.success() || (!stdout.is_empty() && !stderr.contains("error"));
                (success, stdout, stderr)
            }
            Ok(Err(e)) => (
                false,
                String::new(),
                format!("Health check IO error: {e}"),
            ),
            Err(_) => (
                false,
                String::new(),
                "Health check timed out".to_string(),
            ),
        }
    } else {
        (
            false,
            String::new(),
            "Failed to spawn health check command".to_string(),
        )
    };

    let help_output = Command::new(&path).arg("--help").output().await?;
    let help_text = String::from_utf8_lossy(&help_output.stdout);

    let feature_checks: &[(Feature, &str)] = &[
        (Feature::StreamJson, "stream-json"),
        (Feature::JsonSchema, "--json-schema"),
        (Feature::SystemPrompt, "--system-prompt"),
        (Feature::AppendSystemPrompt, "--append-system-prompt"),
        (Feature::Mcp, "--mcp-config"),
        (Feature::StrictMcp, "--strict-mcp-config"),
        (Feature::ToolsFlag, "--tools"),
    ];

    let features = feature_checks
        .iter()
        .filter(|(_, pattern)| help_text.contains(pattern))
        .map(|(feature, _)| *feature)
        .collect();

    let capabilities = Capabilities { features };

    Ok(InitReport {
        claude_path: path,
        version,
        doctor_ok,
        doctor_stdout,
        doctor_stderr,
        capabilities,
    })
}
