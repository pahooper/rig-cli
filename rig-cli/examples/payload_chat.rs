//! Example: Chat about file via payload
//!
//! Demonstrates payload injection for file content analysis.
//! Shows both single Q&A and multi-turn patterns for payload-based chat.
//!
//! The payload is wrapped in XML <context> tags, separating data from instructions.
//!
//! Run: `cargo run -p rig-cli --example payload_chat`

use rig::tool::ToolSet;
use rig_cli::tools::{JsonSchemaToolkit, ToolSetExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Analysis result for the file content
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct FileAnalysis {
    file_type: String,
    summary: String,
    key_findings: Vec<String>,
}

fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<FileAnalysis>::builder()
        .example(FileAnalysis {
            file_type: "configuration".to_string(),
            summary: "Database configuration file".to_string(),
            key_findings: vec!["Uses PostgreSQL".to_string()],
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
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return build_toolset().into_handler().await?.serve_stdio().await;
    }

    // Simulated file content (in practice, use std::fs::read_to_string)
    let file_content = r#"
[database]
host = "localhost"
port = 5432
name = "myapp_production"
max_connections = 100

[cache]
redis_url = "redis://localhost:6379"
ttl_seconds = 3600

[logging]
level = "info"
format = "json"
    "#;

    // --- KEY CODE: Single Q&A with payload ---
    println!("=== Single Q&A Pattern ===");
    let client = rig_cli::claude::Client::new()
        .await?
        .with_payload(file_content); // Inject file content

    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble("Analyze the configuration file in <context>")
        .build()?;

    let analysis = agent.prompt("What is this configuration file for?").await?;
    println!("Analysis:\n{analysis}\n");
    // --- END KEY CODE ---

    // --- KEY CODE: Multi-turn pattern with payload ---
    println!("=== Multi-turn Pattern ===");

    // Follow-up question (new agent with same payload)
    let client2 = rig_cli::claude::Client::new()
        .await?
        .with_payload(file_content);

    let agent2 = client2
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble("Analyze the configuration file in <context>")
        .build()?;

    let followup = agent2
        .prompt("Are there any security concerns with this configuration?")
        .await?;
    println!("Follow-up:\n{followup}");
    // --- END KEY CODE ---

    Ok(())
}
