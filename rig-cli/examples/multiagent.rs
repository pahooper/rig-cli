//! Example: Multiagent with extra tools and MCP
//!
//! Demonstrates multiple agents working together:
//! 1. Researcher agent extracts information
//! 2. Summarizer agent condenses the extraction
//!
//! Each agent uses MCP tools for structured output.
//!
//! Run: `cargo run -p rig-cli --example multiagent`

use rig::tool::ToolSet;
use rig_cli::tools::{JsonSchemaToolkit, ToolSetExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Research findings from first agent
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct ResearchFindings {
    topic: String,
    key_points: Vec<String>,
    sources_count: u32,
}

/// Summary from second agent
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Summary {
    one_liner: String,
    confidence: f32, // 0.0 to 1.0
}

fn research_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<ResearchFindings>::builder()
        .example(ResearchFindings {
            topic: "Rust async".to_string(),
            key_points: vec!["Zero-cost abstractions".to_string()],
            sources_count: 5,
        })
        .build()
        .build_tools();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);
    toolset
}

fn summary_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<Summary>::builder()
        .example(Summary {
            one_liner: "Topic summary here".to_string(),
            confidence: 0.85,
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
    // Dual-mode: check which toolset based on env
    let mode = std::env::var("RIG_MCP_MODE").ok();
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return match mode.as_deref() {
            Some("summary") => Ok(summary_toolset()
                .into_handler()
                .await?
                .serve_stdio()
                .await?),
            _ => Ok(research_toolset()
                .into_handler()
                .await?
                .serve_stdio()
                .await?),
        };
    }

    let client = rig_cli::claude::Client::new().await?;

    // --- KEY CODE: Agent 1 - Researcher ---
    println!("=== Agent 1: Researcher ===");
    let researcher = client
        .mcp_agent("sonnet")
        .toolset(research_toolset())
        .preamble("You are a research assistant. Extract key findings.")
        .build()?;

    let research_output = researcher
        .prompt("Research: What are the benefits of Rust's ownership system?")
        .await?;
    println!("Research output:\n{research_output}\n");
    // --- END KEY CODE ---

    // --- KEY CODE: Agent 2 - Summarizer ---
    println!("=== Agent 2: Summarizer ===");
    let summarizer = client
        .mcp_agent("sonnet")
        .toolset(summary_toolset())
        .preamble("You are a summarizer. Create a one-line summary.")
        .build()?;

    // Pass research output to summarizer
    let summary_output = summarizer
        .prompt(&format!(
            "Summarize this research into one line:\n{research_output}",
        ))
        .await?;
    println!("Summary:\n{summary_output}");
    // --- END KEY CODE ---

    Ok(())
}
