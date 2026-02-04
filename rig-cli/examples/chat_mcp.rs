//! Example: Chat with MCP and sessions
//!
//! Demonstrates multi-turn conversation where the agent uses MCP tools
//! to respond, maintaining structured output across turns.
//!
//! Run: `cargo run -p rig-cli --example chat_mcp`
//!
//! This example shows how to use `mcp_agent()` for multiple conversation
//! turns. Each turn creates a new agent instance (since ToolSet lacks Clone),
//! but demonstrates the pattern for structured chat interactions.

use rig::tool::ToolSet;
use rig_cli::prelude::*;
use rig_cli::tools::ToolSetExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Structured chat response with sentiment analysis.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ChatResponse {
    /// The assistant's message.
    message: String,
    /// Detected sentiment: "positive", "neutral", or "negative".
    sentiment: String,
}

/// Builds a ToolSet with the 3-tool extraction pattern.
fn build_toolset() -> ToolSet {
    // --- KEY CODE: Build extraction toolkit ---
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<ChatResponse>::builder()
        .example(ChatResponse {
            message: "Hello! How can I help you today?".to_string(),
            sentiment: "positive".to_string(),
        })
        .build()
        .build_tools();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);
    toolset
    // --- END KEY CODE ---
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP server mode: serves tools over stdio when RIG_MCP_SERVER=1
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    // --- KEY CODE: Multi-turn chat with MCP ---
    let client = rig_cli::claude::Client::new().await?;

    // First turn: greeting with positive sentiment
    let agent1 = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble("Respond with structured sentiment analysis. Analyze the user's message and respond with your message and detected sentiment.")
        .build()?;

    println!("Turn 1: Sending greeting...");
    let response1 = agent1.prompt("Hello, I'm excited to try this new library!").await?;
    println!("Response: {response1}");

    // Second turn: new agent instance (ToolSet can't be cloned)
    let agent2 = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble("Respond with structured sentiment analysis. Analyze the user's message and respond with your message and detected sentiment.")
        .build()?;

    println!("\nTurn 2: Sending complaint...");
    let response2 = agent2.prompt("This is frustrating, the documentation is confusing.").await?;
    println!("Response: {response2}");
    // --- END KEY CODE ---

    Ok(())
}
