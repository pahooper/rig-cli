//! Simplified MCP extraction using [`McpToolAgent`].
//!
//! This example shows how `McpToolAgent` eliminates MCP boilerplate.
//! Compare with `mcp_extraction_e2e.rs` (~300 lines) for the manual approach.
//!
//! The binary runs in two modes via environment variable detection:
//! - **Server mode** (`RIG_MCP_SERVER=1`): Serves MCP tools over stdio
//! - **Client mode** (default): Uses `McpToolAgent` to orchestrate the CLI
//!
//! See also `payload_extraction_e2e.rs` for payload injection demo.
//!
//! Run with: `cargo run --example mcp_tool_agent_e2e -- [claude|codex|opencode]`

use rig::tool::ToolSet;
use rig_cli_mcp::prelude::ToolSetExt;
use rig_cli_mcp::tools::JsonSchemaToolkit;
use rig_cli_provider::{CliAdapter, McpToolAgent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A structured movie review for extraction.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MovieReview {
    /// Title of the movie.
    title: String,
    /// Rating from 1 to 10.
    rating: u8,
    /// Brief summary in 1-2 sentences.
    summary: String,
    /// List of genres.
    genres: Vec<String>,
}

fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<MovieReview>::builder()
        .example(MovieReview {
            title: "The Matrix".to_string(),
            rating: 9,
            summary: "A hacker discovers reality is a simulation.".to_string(),
            genres: vec!["Sci-Fi".to_string(), "Action".to_string()],
        })
        .build()
        .build_tools();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);
    toolset
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Server mode: MCP tools over stdio
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    // Parse adapter from CLI args (default: claude)
    let adapter = match std::env::args().nth(1).as_deref() {
        Some("codex") => CliAdapter::Codex,
        Some("opencode") => CliAdapter::OpenCode,
        Some("claude") | None => CliAdapter::ClaudeCode,
        Some(other) => {
            eprintln!("Unknown adapter: {other}. Use: claude, codex, or opencode");
            std::process::exit(1);
        }
    };

    println!("Using adapter: {adapter:?}");

    // Client mode: McpToolAgent handles everything
    let result = McpToolAgent::builder()
        .toolset(build_toolset())
        .prompt(
            "Create a movie review for Inception (2010) by Christopher Nolan. \
             Use the json_example tool to see the format, validate_json to check, \
             then submit your review.",
        )
        .adapter(adapter)
        .server_name("rig_extraction")
        .timeout(Duration::from_secs(120))
        .run()
        .await?;

    println!("Exit code: {}", result.exit_code);
    println!("Duration: {}ms", result.duration_ms);
    println!("Output:\n{}", result.stdout);
    if !result.stderr.is_empty() {
        eprintln!("Stderr:\n{}", result.stderr);
    }

    Ok(())
}
