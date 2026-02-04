//! Example: Agent with MCP and extra tools
//!
//! Demonstrates combining the standard 3-tool extraction pattern with
//! additional custom tools. The custom `DateExtractor` tool shows how
//! to define a full tool with input/output types implementing Rig's Tool trait.
//!
//! Run: `cargo run -p rig-cli --example agent_extra_tools`

use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolSet};
use rig_cli::prelude::*;
use rig_cli::tools::ToolSetExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================
// CUSTOM TOOL DEFINITION
// ============================================================

/// Input for the date extractor tool.
#[derive(Debug, Deserialize, JsonSchema)]
struct DateExtractorInput {
    /// Text to extract dates from.
    text: String,
}

/// Output from the date extractor tool.
#[derive(Debug, Serialize, JsonSchema)]
struct DateExtractorOutput {
    /// Extracted date-like strings.
    dates: Vec<String>,
}

/// Error type for the DateExtractor tool.
#[derive(Debug)]
struct DateExtractorError(String);

impl fmt::Display for DateExtractorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DateExtractorError {}

/// Custom tool that extracts date-like patterns from text.
///
/// This demonstrates how to implement the Rig `Tool` trait for
/// custom functionality alongside the standard extraction tools.
struct DateExtractor;

impl Tool for DateExtractor {
    const NAME: &'static str = "extract_dates";

    type Args = DateExtractorInput;
    type Output = DateExtractorOutput;
    type Error = DateExtractorError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Extracts date patterns from text (YYYY-MM-DD, Month Day Year, etc.)"
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to extract dates from"
                    }
                },
                "required": ["text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Simple extraction for demo purposes
        // In production, use a proper date parsing library
        let dates: Vec<String> = args
            .text
            .split_whitespace()
            .filter(|word| {
                word.contains('-')
                    || word.parse::<u32>().is_ok()
                    || ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"]
                        .iter()
                        .any(|m| word.to_lowercase().starts_with(m))
            })
            .map(String::from)
            .collect();

        Ok(DateExtractorOutput { dates })
    }
}

// ============================================================
// END CUSTOM TOOL DEFINITION
// ============================================================

/// Event data with dates to extract.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EventInfo {
    /// Name of the event.
    event_name: String,
    /// Dates mentioned in the source text.
    dates_mentioned: Vec<String>,
    /// Brief summary of the event.
    summary: String,
}

/// Builds a ToolSet with extraction tools plus the custom DateExtractor.
fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();

    // Standard 3-tool pattern
    let (submit, validate, example) = JsonSchemaToolkit::<EventInfo>::builder()
        .example(EventInfo {
            event_name: "Product Launch".to_string(),
            dates_mentioned: vec!["2026-03-15".to_string()],
            summary: "New product launching in March".to_string(),
        })
        .build()
        .build_tools();

    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);

    // --- KEY CODE: Add custom tool ---
    toolset.add_tool(DateExtractor);
    // --- END KEY CODE ---

    toolset
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // MCP server mode: serves tools over stdio when RIG_MCP_SERVER=1
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    let client = rig_cli::claude::Client::new().await?;

    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble(
            "You are an event information extractor. \
             Use extract_dates to find dates in text, then use the \
             extraction tools to submit structured event info.",
        )
        .build()?;

    let result = agent
        .prompt(
            "Extract event info: The conference runs from March 15-17, 2026 \
             in San Francisco. Early bird registration ends February 1st.",
        )
        .await?;

    println!("Extraction result:\n{result}");
    Ok(())
}
