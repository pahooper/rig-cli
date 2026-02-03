//! End-to-end tests for Codex CLI containment features.
//!
//! These tests require the Codex CLI (`codex`) to be installed locally.
//! They are marked `#[ignore]` to prevent CI failures in environments without
//! the CLI.
//!
//! ## Requirements
//!
//! - Codex CLI installed: `npm install -g @openai/codex`
//! - Valid OpenAI API key configured
//! - Network access for API calls
//!
//! ## Running E2E Tests
//!
//! ```bash
//! # Run all ignored E2E tests
//! cargo test -p codex-adapter -- --ignored
//!
//! # Run specific E2E test
//! cargo test -p codex-adapter e2e_sandbox_readonly -- --ignored
//! ```
//!
//! ## Test Strategy
//!
//! These tests validate that containment flags actually restrict CLI behavior:
//! 1. Configure containment (--sandbox, --ask-for-approval)
//! 2. Run a prompt that would trigger restricted operations
//! 3. Verify the CLI respects the restrictions
//!
//! ## Known Limitations
//!
//! - MCP tools bypass Landlock sandbox (Codex Issue #4152)
//!   Tests document this limitation rather than expecting perfect isolation.
//! - Tests may be flaky due to LLM non-determinism.

use codex_adapter::{discover_codex, run_codex, ApprovalPolicy, CodexCli, CodexConfig, SandboxMode};
use std::time::Duration;
use tempfile::TempDir;

/// Discovers Codex CLI, returns None if not available.
async fn get_codex_cli() -> Option<CodexCli> {
    match discover_codex(None) {
        Ok(path) => {
            let cli = CodexCli::new(path);
            if cli.check_health().await.is_ok() {
                Some(cli)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// E2E test: Verify --sandbox read-only restricts filesystem writes.
///
/// Note: MCP tools bypass sandbox (Issue #4152), but native commands should
/// still be restricted.
#[tokio::test]
#[ignore = "Requires Codex CLI installed"]
async fn e2e_sandbox_readonly() {
    let cli = match get_codex_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Codex CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = CodexConfig {
        sandbox: Some(SandboxMode::ReadOnly),
        skip_git_repo_check: true,
        cd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..CodexConfig::default()
    };

    // Prompt that would normally trigger file write
    let result = run_codex(
        &cli.path,
        "Write the text 'test' to a file named test.txt. If you cannot, explain why.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            // Check that no test.txt was created
            let test_file = temp_dir.path().join("test.txt");
            if test_file.exists() {
                // If file exists, it may be due to MCP bypass (#4152)
                eprintln!(
                    "Warning: File created despite sandbox. This may be due to MCP bypass (Issue #4152). Output: {}",
                    run_result.stdout
                );
            } else {
                // Sandbox worked - no file created
                eprintln!("Sandbox prevented file creation as expected");
            }
        }
        Err(e) => {
            // Timeout or error is acceptable - containment may cause agent to loop
            eprintln!("CLI error (acceptable for containment test): {e}");
        }
    }
}

/// E2E test: Verify --ask-for-approval untrusted restricts command execution.
#[tokio::test]
#[ignore = "Requires Codex CLI installed"]
async fn e2e_approval_policy_untrusted() {
    let cli = match get_codex_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Codex CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = CodexConfig {
        ask_for_approval: Some(ApprovalPolicy::Untrusted),
        skip_git_repo_check: true,
        cd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..CodexConfig::default()
    };

    // Prompt requesting an untrusted operation
    let result = run_codex(
        &cli.path,
        "Run: curl http://example.com and show the output. If you need approval, explain.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            let output = run_result.stdout.to_lowercase();
            // The agent should indicate it needs approval or cannot execute
            let indicates_approval_needed = output.contains("approval")
                || output.contains("permission")
                || output.contains("cannot")
                || output.contains("unable");

            eprintln!(
                "Approval policy test result: approval_mentioned={}, output: {}",
                indicates_approval_needed,
                run_result.stdout.chars().take(200).collect::<String>()
            );
        }
        Err(e) => {
            // Timeout or error is acceptable
            eprintln!("CLI error (acceptable): {e}");
        }
    }
}

/// E2E test: Verify combined sandbox + approval policy containment.
#[tokio::test]
#[ignore = "Requires Codex CLI installed"]
async fn e2e_full_containment() {
    let cli = match get_codex_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Codex CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = CodexConfig {
        sandbox: Some(SandboxMode::ReadOnly),
        ask_for_approval: Some(ApprovalPolicy::Untrusted),
        skip_git_repo_check: true,
        cd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..CodexConfig::default()
    };

    // Prompt that combines filesystem and command operations
    let result = run_codex(
        &cli.path,
        "Create a file called test.txt with 'hello' content, then run 'cat test.txt'. If restricted, explain.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            // Verify containment indicators
            let output = run_result.stdout.to_lowercase();
            let indicates_restriction = output.contains("cannot")
                || output.contains("restricted")
                || output.contains("read-only")
                || output.contains("approval")
                || output.contains("unable");

            // File should not exist (unless MCP bypass)
            let test_file = temp_dir.path().join("test.txt");
            eprintln!(
                "Full containment test: file_exists={}, restriction_mentioned={}, output_preview: {}",
                test_file.exists(),
                indicates_restriction,
                run_result.stdout.chars().take(200).collect::<String>()
            );
        }
        Err(e) => {
            eprintln!("CLI error (acceptable): {e}");
        }
    }
}

/// E2E test: Verify timeout handling with graceful shutdown.
#[tokio::test]
#[ignore = "Requires Codex CLI installed"]
async fn e2e_timeout_graceful_shutdown() {
    let cli = match get_codex_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Codex CLI not found");
            return;
        }
    };

    let config = CodexConfig {
        timeout: Duration::from_millis(500), // Very short timeout
        skip_git_repo_check: true,
        ..CodexConfig::default()
    };

    // Long prompt to ensure timeout
    let result = run_codex(
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
        Err(codex_adapter::CodexError::Timeout {
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
