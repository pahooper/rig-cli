//! Example: Agent with MCP
//!
//! Demonstrates the standard 3-tool pattern for structured extraction:
//! 1. `json_example` - Shows expected output format
//! 2. `validate_json` - Validates submission before final
//! 3. `submit` - Submits the final validated result
//!
//! This is the recommended pattern for reliable structured extraction.
//!
//! Run: `cargo run -p rig-cli --example agent_mcp`

use rig::tool::ToolSet;
use rig_cli::prelude::*;
use rig_cli::tools::ToolSetExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Movie review structure for extraction.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MovieReview {
    /// Title of the movie.
    title: String,
    /// Rating from 1 to 10.
    rating: u8,
    /// Brief summary in 1-2 sentences.
    summary: String,
    /// List of genres (e.g., "Sci-Fi", "Action").
    genres: Vec<String>,
}

/// Builds a ToolSet with the 3-tool extraction pattern.
fn build_toolset() -> ToolSet {
    // --- KEY CODE: 3-tool pattern setup ---
    let mut toolset = ToolSet::default();

    // JsonSchemaToolkit creates all three tools from a single type
    let (submit, validate, example) = JsonSchemaToolkit::<MovieReview>::builder()
        .example(MovieReview {
            title: "The Matrix".to_string(),
            rating: 9,
            summary: "A hacker discovers reality is a simulation.".to_string(),
            genres: vec!["Sci-Fi".to_string(), "Action".to_string()],
        })
        .on_success("Review submitted successfully!")
        .build()
        .build_tools();

    // Add all three tools to the toolset
    toolset.add_tool(submit);   // Final submission
    toolset.add_tool(validate); // Pre-submission validation
    toolset.add_tool(example);  // Format reference
    toolset
    // --- END KEY CODE ---
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP server mode: serves tools over stdio when RIG_MCP_SERVER=1
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    let client = rig_cli::claude::Client::new().await?;

    // --- KEY CODE: Agent with workflow guidance ---
    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble(
            "You are a movie review extractor. \
             Use json_example to see the format, validate_json to check, \
             then submit your final review.",
        )
        .build()?;

    let result = agent
        .prompt("Create a review for Inception (2010) by Christopher Nolan")
        .await?;
    // --- END KEY CODE ---

    println!("Extraction result:\n{result}");
    Ok(())
}
