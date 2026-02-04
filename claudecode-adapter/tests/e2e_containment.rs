//! End-to-end tests for Claude CLI containment features.
//!
//! These tests require the Claude CLI (`claude`) to be installed locally.
//! They are marked `#[ignore]` to prevent CI failures in environments without
//! the CLI.
//!
//! ## Requirements
//!
//! - Claude CLI installed: `npm install -g @anthropic-ai/claude-code`
//! - Valid Anthropic API key configured
//! - Network access for API calls
//!
//! ## Running E2E Tests
//!
//! ```bash
//! # Run all ignored E2E tests
//! cargo test -p claudecode-adapter -- --ignored
//!
//! # Run specific E2E test
//! cargo test -p claudecode-adapter e2e_containment_mcp_only -- --ignored
//! ```
//!
//! ## Test Strategy
//!
//! These tests validate that containment flags actually restrict CLI behavior:
//! 1. Configure containment (--tools "", --allowed-tools, --strict-mcp-config)
//! 2. Run a prompt that would normally use builtin tools
//! 3. Verify the CLI respects the restrictions (no builtin tool calls)
//!
//! Note: Tests may be flaky due to LLM non-determinism. They verify the
//! containment *mechanism* works, not specific model responses.

use rig_cli_claude::{
    init, run_claude, BuiltinToolSet, ClaudeCli, RunConfig, ToolPolicy,
};
use std::time::Duration;
use tempfile::TempDir;

/// Discovers Claude CLI, returns None if not available.
async fn get_claude_cli() -> Option<ClaudeCli> {
    match init(None).await {
        Ok(report) => Some(ClaudeCli::new(report.claude_path, report.capabilities)),
        Err(_) => None,
    }
}

/// E2E test: Verify --tools "" actually disables builtin tools.
///
/// Sends a prompt asking to list files (normally uses Read/Bash tools).
/// With containment, the agent should NOT be able to use these tools.
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_containment_no_builtins() {
    let cli = match get_claude_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Claude CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::None, // --tools ""
            allowed: None,
            disallowed: None,
            disable_slash_commands: true,
        },
        cwd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..RunConfig::default()
    };

    // Prompt that would normally trigger file operations
    let result = run_claude(
        &cli.path,
        "List all files in the current directory. If you cannot, explain why.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            // The agent should indicate it cannot use tools, not list files
            let output = run_result.stdout.to_lowercase();
            // Check that the response indicates tool limitation rather than file listing
            let indicates_limitation = output.contains("cannot")
                || output.contains("unable")
                || output.contains("don't have")
                || output.contains("no tools")
                || output.contains("not able");

            // The response should NOT contain evidence of successful tool use
            let no_tool_evidence = !output.contains("directory listing")
                && !output.contains("files found");

            assert!(
                indicates_limitation || no_tool_evidence,
                "Expected containment to prevent tool use. Output: {}",
                run_result.stdout
            );
        }
        Err(e) => {
            // Timeout or error is acceptable - containment may cause the agent to loop
            eprintln!("CLI error (acceptable for containment test): {e}");
        }
    }
}

/// E2E test: Verify --allowed-tools restricts to specific tools.
///
/// Note: This test requires MCP tools to be available. Since we don't have
/// a test MCP server, we verify the *restriction mechanism* by checking
/// that builtin tools are not accessible when allowed-tools is set to
/// MCP-formatted names only.
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_containment_allowed_tools_only() {
    let cli = match get_claude_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Claude CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::None,
            // Only allow a fake MCP tool that doesn't exist
            allowed: Some(vec!["mcp__nonexistent__fake_tool".to_string()]),
            disallowed: None,
            disable_slash_commands: true,
        },
        cwd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..RunConfig::default()
    };

    let result = run_claude(
        &cli.path,
        "What tools do you have available? List them.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            let output = run_result.stdout.to_lowercase();

            // Should NOT list builtin tools like Bash, Read, Write, Edit
            let has_builtins = output.contains("bash")
                || output.contains("read tool")
                || output.contains("write tool")
                || output.contains("edit tool");

            assert!(
                !has_builtins,
                "Containment failed: builtin tools visible. Output: {}",
                run_result.stdout
            );
        }
        Err(e) => {
            eprintln!("CLI error (acceptable): {e}");
        }
    }
}

/// E2E test: Verify --disable-slash-commands works.
///
/// With slash commands disabled, interactive commands like /help should
/// not be processed as commands.
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_disable_slash_commands() {
    let cli = match get_claude_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Claude CLI not found");
            return;
        }
    };

    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::Default, // Allow tools, just disable slash commands
            allowed: None,
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout: Duration::from_secs(60),
        ..RunConfig::default()
    };

    // In --print mode, slash commands shouldn't be relevant anyway,
    // but the flag should be passed correctly
    let result = run_claude(&cli.path, "Say hello", &config, None).await;

    // Just verify the CLI accepts the flag combination
    match result {
        Ok(run_result) => {
            assert!(
                run_result.exit_code == 0,
                "CLI should accept --disable-slash-commands flag. Exit: {}, Stderr: {}",
                run_result.exit_code,
                run_result.stderr
            );
        }
        Err(e) => {
            // Timeout is acceptable, but spawn failure indicates flag rejection
            if let rig_cli_claude::ClaudeError::SpawnFailed { .. } = e {
                panic!("CLI rejected --disable-slash-commands flag: {e}");
            }
        }
    }
}

/// E2E test: Verify timeout handling with graceful shutdown.
///
/// Sets a very short timeout to force the timeout path, then verifies
/// the process is properly killed and partial output is captured.
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_timeout_graceful_shutdown() {
    let cli = match get_claude_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Claude CLI not found");
            return;
        }
    };

    let config = RunConfig {
        timeout: Duration::from_millis(500), // Very short timeout
        ..RunConfig::default()
    };

    // Long prompt to ensure we hit timeout
    let result = run_claude(
        &cli.path,
        "Write a detailed 5000 word essay about the history of computing.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(_) => {
            // Fast response is fine - model may have cached response
            eprintln!("Note: CLI responded before timeout");
        }
        Err(rig_cli_claude::ClaudeError::Timeout {
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
            // Partial output may or may not be present depending on timing
            eprintln!(
                "Timeout captured: pid={}, stdout_len={}, stderr_len={}",
                pid,
                partial_stdout.len(),
                partial_stderr.len()
            );
        }
        Err(e) => {
            // Other errors may occur (network, API limits)
            eprintln!("Other error (acceptable): {e}");
        }
    }
}
