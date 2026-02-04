//! Example: One-shot with MCP
//!
//! Demonstrates single prompt that returns structured data via MCP.
//! This is the simplest MCP pattern - one question, one structured answer.
//!
//! Run: `cargo run -p rig-cli --example one_shot_mcp`
//!
//! Use this pattern when you need a single structured response without
//! conversation history or multiple turns.

use rig::tool::ToolSet;
use rig_cli::prelude::*;
use rig_cli::tools::ToolSetExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Weather information structure for extraction.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct WeatherInfo {
    /// City and state/country.
    location: String,
    /// Temperature in Fahrenheit.
    temperature_f: i32,
    /// Weather conditions (e.g., "Partly cloudy", "Sunny").
    conditions: String,
}

/// Builds a ToolSet with the 3-tool extraction pattern.
fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<WeatherInfo>::builder()
        .example(WeatherInfo {
            location: "San Francisco, CA".to_string(),
            temperature_f: 65,
            conditions: "Partly cloudy".to_string(),
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
    // MCP server mode: serves tools over stdio when RIG_MCP_SERVER=1
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    // --- KEY CODE: One-shot MCP extraction ---
    let client = rig_cli::claude::Client::new().await?;

    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .build()?;

    let result = agent
        .prompt("What's the weather like in Seattle today? Make up realistic data.")
        .await?;

    println!("Extracted weather info:\n{result}");
    // --- END KEY CODE ---

    Ok(())
}
