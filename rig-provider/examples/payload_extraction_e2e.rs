//! Payload-driven extraction using [`McpToolAgent`] with `.payload()`.
//!
//! This example demonstrates Phase 3's payload injection feature: passing file
//! contents or text blobs to the agent via `.payload()`. The agent receives the
//! data in structured XML format (`<instructions>`, `<context>`, `<task>`,
//! `<output_format>`), preventing instruction/context confusion.
//!
//! The binary runs in two modes via environment variable detection:
//! - **Server mode** (`RIG_MCP_SERVER=1`): Serves MCP tools over stdio
//! - **Client mode** (default): Uses `McpToolAgent` to orchestrate the CLI
//!
//! Run with: `cargo run --example payload_extraction_e2e -- [claude|codex|opencode]`
//!
//! See also `mcp_tool_agent_e2e.rs` for basic usage without payload injection.

use rig::tool::ToolSet;
use rig_cli_mcp::prelude::ToolSetExt;
use rig_cli_mcp::tools::JsonSchemaToolkit;
use rig_cli_provider::{CliAdapter, McpToolAgent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A structured document analysis for extraction.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct DocumentAnalysis {
    /// Title or subject of the document.
    title: String,
    /// Author or source attribution.
    author: String,
    /// Key topics covered (2-5 items).
    key_topics: Vec<String>,
    /// Overall sentiment: "positive", "negative", or "neutral".
    sentiment: String,
    /// Brief summary in 1-3 sentences.
    summary: String,
    /// Approximate word count of the original document.
    word_count: u32,
}

/// The source document we want the agent to analyze.
const SOURCE_TEXT: &str = "\
The Rust programming language continues to gain mass adoption across the software industry. \
In its annual survey, the Rust Foundation reported that over 2.8 million developers now use \
Rust regularly, a 40% increase from the previous year. Major technology companies including \
Microsoft, Google, and Amazon have expanded their Rust investments significantly. Microsoft \
is rewriting core Windows components in Rust for memory safety, while Google has adopted Rust \
for new Android kernel modules. The Linux kernel project, led by Linus Torvalds, now accepts \
Rust as a second implementation language alongside C. Critics note that Rust's steep learning \
curve and long compile times remain barriers to adoption, but proponents argue that the \
language's safety guarantees and performance characteristics make it worth the investment. \
The Rust compiler team recently released version 1.78, which includes improved async support \
and faster incremental compilation times.";

fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<DocumentAnalysis>::builder()
        .example(DocumentAnalysis {
            title: "Rust Adoption in Enterprise".to_string(),
            author: "Tech Industry Report".to_string(),
            key_topics: vec![
                "Memory Safety".to_string(),
                "Compiler Performance".to_string(),
                "Enterprise Adoption".to_string(),
            ],
            sentiment: "positive".to_string(),
            summary: "Analysis of Rust's growing adoption in the software industry.".to_string(),
            word_count: 150,
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
        return build_toolset().into_handler().await?.serve_stdio().await;
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

    // Client mode: McpToolAgent with payload injection
    let result = McpToolAgent::builder()
        .toolset(build_toolset())
        .prompt("Analyze the provided document and extract structured metadata.")
        .payload(SOURCE_TEXT) // Phase 3 feature: inject context data
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
