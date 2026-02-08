//! Example: MCP Agent + deterministic tool
//!
//! Demonstrates combining MCP extraction with a deterministic (non-LLM) tool.
//! The `CurrentDateTool` returns the actual system date, showing how to mix
//! AI and deterministic operations.
//!
//! This example shows FULL tool definition, not assuming the tool exists.
//!
//! Run: `cargo run -p rig-cli --example mcp_deterministic`

use chrono::{DateTime, Utc};
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolSet};
use rig_cli::tools::{JsonSchemaToolkit, ToolSetExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// --- DETERMINISTIC TOOL: Current Date ---

/// Error type for the date tool
#[derive(Debug, Error)]
pub enum DateToolError {
    /// Never actually used, but required for the trait
    #[error("Date tool error: {0}")]
    Internal(String),
}

/// Input for the current date tool (no parameters needed)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CurrentDateInput {}

/// Output from the current date tool
#[derive(Debug, Serialize, JsonSchema)]
struct CurrentDateOutput {
    /// ISO 8601 formatted date-time
    iso8601: String,
    /// Human-readable date
    human_readable: String,
    /// Unix timestamp
    unix_timestamp: i64,
}

/// Deterministic tool that returns the current system date/time
struct CurrentDateTool;

impl Tool for CurrentDateTool {
    const NAME: &'static str = "get_current_date";

    type Args = CurrentDateInput;
    type Output = CurrentDateOutput;
    type Error = DateToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Returns the current system date and time. Use this for any date-related queries.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _input: Self::Args) -> Result<Self::Output, Self::Error> {
        let now: DateTime<Utc> = Utc::now();
        Ok(CurrentDateOutput {
            iso8601: now.to_rfc3339(),
            human_readable: now.format("%B %d, %Y at %H:%M UTC").to_string(),
            unix_timestamp: now.timestamp(),
        })
    }
}

// --- END DETERMINISTIC TOOL ---

/// Scheduled event with computed dates
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ScheduledEvent {
    event_name: String,
    scheduled_date: String,
    days_until: i32,
    is_past: bool,
}

fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();

    // Standard 3-tool pattern
    let (submit, validate, example) = JsonSchemaToolkit::<ScheduledEvent>::builder()
        .example(ScheduledEvent {
            event_name: "Team Meeting".to_string(),
            scheduled_date: "2026-02-10".to_string(),
            days_until: 7,
            is_past: false,
        })
        .build()
        .build_tools();

    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);

    // --- KEY CODE: Add deterministic date tool ---
    toolset.add_tool(CurrentDateTool);
    // --- END KEY CODE ---

    toolset
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        build_toolset().into_handler().await?.serve_stdio().await?;
        return Ok(());
    }

    let client = rig_cli::claude::Client::new().await?;

    // --- KEY CODE: Agent with deterministic + extraction tools ---
    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble(
            "You are an event scheduler. \
             Use get_current_date to get today's date, then calculate \
             days until the scheduled event.",
        )
        .build()?;

    let result = agent
        .prompt("How many days until the product launch on March 15, 2026?")
        .await?;
    // --- END KEY CODE ---

    println!("Scheduled event info:\n{result}");

    Ok(())
}
