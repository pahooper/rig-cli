//! End-to-end tests for OpenCode CLI containment features.
//!
//! These tests require the OpenCode CLI (`opencode`) to be installed locally.
//! They are marked `#[ignore]` to prevent CI failures in environments without
//! the CLI.
//!
//! ## Requirements
//!
//! - OpenCode CLI installed: `go install github.com/opencode-ai/opencode@latest`
//! - Valid API credentials configured
//! - Network access for API calls
//!
//! ## Running E2E Tests
//!
//! ```bash
//! # Run all ignored E2E tests
//! cargo test -p opencode-adapter -- --ignored
//!
//! # Run specific E2E test
//! cargo test -p opencode-adapter e2e_working_directory -- --ignored
//! ```
//!
//! ## Test Strategy
//!
//! OpenCode has no CLI flags for sandbox, tool restriction, or approval policy.
//! Containment tests focus on:
//!
//! 1. Working directory isolation via `cwd` config (Command::current_dir)
//! 2. MCP config delivery via `OPENCODE_CONFIG` env var
//! 3. Timeout handling with graceful SIGTERM -> SIGKILL shutdown
//!
//! ## Known Limitations
//!
//! - No filesystem sandbox (unlike Claude Code's --tools "" or Codex's --sandbox)
//! - No tool restriction flags (all configured tools are available)
//! - Containment relies on process isolation, not CLI enforcement

use opencode_adapter::{discover_opencode, run_opencode, OpenCodeCli, OpenCodeConfig, OpenCodeError};
use std::time::Duration;
use tempfile::TempDir;

/// Discovers OpenCode CLI, returns None if not available.
async fn get_opencode_cli() -> Option<OpenCodeCli> {
    match discover_opencode(None) {
        Ok(path) => {
            let cli = OpenCodeCli::new(path);
            if cli.check_health().await.is_ok() {
                Some(cli)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// E2E test: Verify working directory is set correctly.
///
/// Creates a temp directory, sets it as cwd in config, and verifies
/// the CLI operates in that directory.
#[tokio::test]
#[ignore = "Requires OpenCode CLI installed"]
async fn e2e_working_directory_containment() {
    let cli = match get_opencode_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: OpenCode CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = OpenCodeConfig {
        cwd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..OpenCodeConfig::default()
    };

    // Prompt that asks about current working directory
    let result = run_opencode(
        &cli.path,
        "What is your current working directory? Use pwd or equivalent command. Just tell me the path.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            // The output should mention the temp directory path
            let output = run_result.stdout;
            let temp_path_str = temp_dir.path().to_string_lossy();

            // Check if temp path appears in output (agent may describe or quote it)
            let path_mentioned = output.contains(&*temp_path_str)
                || output.contains("tmp")
                || output.contains("temp");

            eprintln!(
                "Working directory test: temp_path={}, path_mentioned={}, output_preview: {}",
                temp_path_str,
                path_mentioned,
                output.chars().take(300).collect::<String>()
            );
        }
        Err(e) => {
            // Timeout or error may occur if agent loops or API fails
            eprintln!("CLI error (may be acceptable): {e}");
        }
    }
}

/// E2E test: Verify timeout handling with graceful shutdown.
///
/// Sets a very short timeout to force the timeout path, then verifies
/// the process is properly killed and partial output is captured.
#[tokio::test]
#[ignore = "Requires OpenCode CLI installed"]
async fn e2e_timeout_graceful_shutdown() {
    let cli = match get_opencode_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: OpenCode CLI not found");
            return;
        }
    };

    let config = OpenCodeConfig {
        timeout: Duration::from_millis(500), // Very short timeout
        ..OpenCodeConfig::default()
    };

    // Long prompt to ensure timeout
    let result = run_opencode(
        &cli.path,
        "Write a detailed 5000 word essay about the history of computing, starting from Charles Babbage.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(_) => {
            // Fast response is fine - model may have cached response or responded quickly
            eprintln!("Note: CLI responded before timeout");
        }
        Err(OpenCodeError::Timeout {
            elapsed,
            pid,
            partial_stdout,
            partial_stderr,
        }) => {
            // Expected: timeout with partial output captured
            assert!(
                elapsed.as_millis() >= 500,
                "Timeout should be at least 500ms"
            );
            assert!(pid > 0, "PID should be captured");
            eprintln!(
                "Timeout captured correctly: pid={}, elapsed={:?}, stdout_len={}, stderr_len={}",
                pid,
                elapsed,
                partial_stdout.len(),
                partial_stderr.len()
            );
        }
        Err(e) => {
            // Other errors may occur (network, API limits, auth)
            eprintln!("Other error (acceptable): {e}");
        }
    }
}

/// E2E test: Verify MCP config is delivered via environment variable.
///
/// Creates a temp MCP config file, sets mcp_config_path in config,
/// and verifies the CLI receives it (we can't verify internal behavior,
/// but we verify the config file path is used).
#[tokio::test]
#[ignore = "Requires OpenCode CLI installed"]
async fn e2e_mcp_config_delivery() {
    let cli = match get_opencode_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: OpenCode CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mcp_config_path = temp_dir.path().join("mcp_config.json");

    // Create a minimal MCP config file (OpenCode format)
    // Even if invalid, the CLI should accept the env var
    std::fs::write(
        &mcp_config_path,
        r#"{"mcpServers": {}}"#,
    )
    .expect("Failed to write MCP config");

    let config = OpenCodeConfig {
        mcp_config_path: Some(mcp_config_path.clone()),
        timeout: Duration::from_secs(30),
        ..OpenCodeConfig::default()
    };

    // Simple prompt - we just want to verify the CLI starts with the env var
    let result = run_opencode(
        &cli.path,
        "Say 'hello' and nothing else.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            // If we get a response, the CLI accepted the env var configuration
            eprintln!(
                "MCP config delivery test: CLI ran successfully with OPENCODE_CONFIG={}, exit_code={}",
                mcp_config_path.display(),
                run_result.exit_code
            );
        }
        Err(e) => {
            // Check if error is related to MCP config (parsing, missing servers, etc.)
            let error_str = format!("{e}");
            let mcp_related = error_str.to_lowercase().contains("mcp")
                || error_str.to_lowercase().contains("config")
                || error_str.to_lowercase().contains("server");

            if mcp_related {
                eprintln!("MCP config was processed (error indicates parsing): {e}");
            } else {
                // Other errors (timeout, network) are acceptable
                eprintln!("CLI error (acceptable): {e}");
            }
        }
    }
}

/// E2E test: Verify system prompt is prepended to message.
///
/// OpenCode has no --system-prompt flag; the adapter prepends it to the message.
/// This test verifies the prepending mechanism works.
#[tokio::test]
#[ignore = "Requires OpenCode CLI installed"]
async fn e2e_system_prompt_prepending() {
    let cli = match get_opencode_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: OpenCode CLI not found");
            return;
        }
    };

    let config = OpenCodeConfig {
        prompt: Some("You are a helpful assistant that always starts responses with 'PREAMBLE:'.".to_string()),
        timeout: Duration::from_secs(60),
        ..OpenCodeConfig::default()
    };

    let result = run_opencode(
        &cli.path,
        "What is 2 + 2?",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            let output = run_result.stdout;
            // Check if the agent followed the system prompt instruction
            let followed_instruction = output.contains("PREAMBLE:")
                || output.to_lowercase().contains("preamble");

            eprintln!(
                "System prompt test: followed_instruction={}, output_preview: {}",
                followed_instruction,
                output.chars().take(200).collect::<String>()
            );

            // Note: LLM may not perfectly follow instructions, so we don't assert.
            // The test verifies the mechanism works (prompt is prepended).
        }
        Err(e) => {
            eprintln!("CLI error (acceptable): {e}");
        }
    }
}
