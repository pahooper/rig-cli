//! Example: Error Handling Patterns
//!
//! Demonstrates error handling patterns for CLI-based agents including
//! timeout handling, CLI not found handling, and graceful error recovery
//! with fallback values. Shows retry exhaustion scenarios and parse failures.
//!
//! Run: `cargo run -p rig-cli --example error_handling`
//!
//! This example demonstrates three key error handling patterns:
//! 1. Timeout handling with short timeout configuration
//! 2. CLI not found handling with invalid binary path
//! 3. Graceful error recovery with fallback values
//!
//! In production, these patterns help build robust applications that handle
//! edge cases gracefully rather than crashing on first error.

use rig_cli::config::ClientConfig;
use rig_cli::errors::Error;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP server mode: not needed for this example (no extraction)
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        println!("This example demonstrates error handling, not MCP tools.");
        return Ok(());
    }

    println!("=== Error Handling Patterns ===\n");

    // --- KEY CODE: Pattern 1 - Timeout Handling ---
    println!("Pattern 1: Timeout Handling");
    println!("{}", "-".repeat(40));

    // Configure a very short timeout to demonstrate timeout handling
    let short_timeout_config = ClientConfig {
        timeout: Duration::from_millis(100), // Intentionally too short
        ..Default::default()
    };

    match rig_cli::claude::Client::from_config(short_timeout_config).await {
        Ok(client) => {
            // Client created successfully, but operations may timeout
            println!("  Client created with 100ms timeout");
            println!("  Note: Most operations would timeout with this setting");

            // Demonstrate how to check timeout configuration
            let config = client.config();
            println!("  Configured timeout: {:?}", config.timeout);
        }
        Err(e) => {
            println!("  Error creating client: {e}");
            println!("  This is expected if CLI discovery fails quickly");
        }
    }
    println!();
    // --- END KEY CODE ---

    // --- KEY CODE: Pattern 2 - CLI Not Found Handling ---
    println!("Pattern 2: CLI Not Found Handling");
    println!("{}", "-".repeat(40));

    // Configure with an invalid binary path to demonstrate not-found handling
    let invalid_path_config = ClientConfig {
        binary_path: Some(PathBuf::from("/nonexistent/path/to/claude")),
        ..Default::default()
    };

    match rig_cli::claude::Client::from_config(invalid_path_config).await {
        Ok(_) => {
            println!("  Unexpected: Client created with invalid path");
        }
        Err(Error::ClaudeNotFound) => {
            // This is the expected error variant
            println!("  Caught Error::ClaudeNotFound");
            println!("  Tip: Install Claude CLI with: npm i -g @anthropic-ai/claude-code");
        }
        Err(e) => {
            println!("  Caught other error: {e}");
        }
    }
    println!();
    // --- END KEY CODE ---

    // --- KEY CODE: Pattern 3 - Graceful Error Recovery ---
    println!("Pattern 3: Graceful Error Recovery");
    println!("{}", "-".repeat(40));

    // Demonstrate fallback pattern when CLI operations fail
    let result = try_get_response().await;

    match result {
        Ok(response) => {
            println!("  Got response: {response}");
        }
        Err(e) => {
            // Fallback to default value instead of propagating error
            let fallback = "Default response (CLI unavailable)";
            println!("  Operation failed: {e}");
            println!("  Using fallback: {fallback}");
        }
    }
    println!();
    // --- END KEY CODE ---

    println!("=== Error Handling Summary ===");
    println!("1. Use short timeouts cautiously - most AI operations need time");
    println!("2. Match on specific error variants for targeted recovery");
    println!("3. Provide fallback values for non-critical operations");
    println!("4. Log errors for debugging while keeping user experience smooth");

    Ok(())
}

/// Attempts to get a response from the CLI, demonstrating error propagation.
///
/// Uses the `?` operator for clean error handling while allowing the caller
/// to decide how to handle failures.
async fn try_get_response() -> Result<String, Error> {
    // Attempt to create client with default discovery
    // This will fail if Claude CLI is not installed
    let client = rig_cli::claude::Client::new().await?;

    // If we get here, client is ready
    // In a real app, you'd make actual requests
    let config = client.config();
    Ok(format!(
        "Client ready with {}s timeout",
        config.timeout.as_secs()
    ))
}
