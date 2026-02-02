//! E2E test: the agent calls MCP tools to summarize text into structured JSON.
//!
//! This binary runs in two modes:
//!
//! **Server mode** (`--server`): Serves `submit`, `validate_json`, and `json_example`
//! tools over stdio MCP. Claude Code spawns this process and calls these tools.
//!
//! **Client mode** (default): Writes an MCP config file pointing to this binary,
//! launches Claude Code CLI with that config, and tells the agent to use the tools
//! to produce a structured summary. The `on_submit` callback writes the result to
//! a temp file so the client can verify it.
//!
//! Run with: `cargo run --example extraction_summarize_e2e`

use clap::Parser;
use rig::tool::ToolSet;
use rig_mcp_server::prelude::ToolSetExt;
use rig_mcp_server::tools::JsonSchemaToolkit;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    /// Run as an MCP server (invoked by Claude Code, not by the user).
    #[clap(long)]
    server: bool,

    /// Path to write submitted JSON to (passed by client mode to server mode).
    #[clap(long)]
    output_path: Option<PathBuf>,
}

/// Target extraction type: a structured summary of a text passage.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct TextSummary {
    /// One-sentence summary of the passage.
    title: String,
    /// 2-5 key points extracted from the passage.
    key_points: Vec<String>,
    /// Overall sentiment: "positive", "negative", or "neutral".
    sentiment: String,
    /// Estimated word count of the original passage.
    word_count: u32,
    /// Named entities (people, places, organizations) mentioned.
    entities: Vec<String>,
}

/// The paragraph we want the model to summarize.
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.server {
        return run_server(args.output_path).await;
    }

    run_client().await
}

/// MCP server mode: serve the toolkit tools over stdio.
async fn run_server(output_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output_path
        .unwrap_or_else(|| PathBuf::from("/tmp/extraction_summarize_e2e_result.json"));

    let mut toolset = ToolSet::default();

    let (submit, validate, example) = JsonSchemaToolkit::<TextSummary>::builder()
        .example(TextSummary {
            title: "Rust adoption growing rapidly across industry".to_string(),
            key_points: vec![
                "Developer count increased 40%".to_string(),
                "Major companies investing heavily".to_string(),
                "Linux kernel now accepts Rust".to_string(),
            ],
            sentiment: "positive".to_string(),
            word_count: 150,
            entities: vec![
                "Rust Foundation".to_string(),
                "Microsoft".to_string(),
                "Google".to_string(),
            ],
        })
        .on_submit(move |data: TextSummary| {
            let json = serde_json::to_string_pretty(&data).unwrap_or_default();
            // Write the submitted data to the output file for the client to verify
            let _ = fs::write(&output_path, &json);
            format!(
                "Summary received: \"{}\" ({} key points, sentiment: {})",
                data.title,
                data.key_points.len(),
                data.sentiment
            )
        })
        .build()
        .build_tools();

    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);

    // Serve over stdio — Claude Code will spawn this process
    toolset
        .into_handler()
        .await?
        .serve_stdio()
        .await?;

    Ok(())
}

/// Client mode: register MCP server, launch Claude Code, verify result.
async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    use claudecode_adapter::{init, ClaudeCli, McpPolicy, OutputFormat, RunConfig};

    // ── 1. Initialize Claude Code CLI ──────────────────────────────────
    println!("[1/5] Discovering Claude Code CLI...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path.clone(), report.capabilities.clone());
    println!("  Found: {}", report.claude_path.display());

    // ── 2. Write MCP config pointing to this binary in --server mode ──
    println!("[2/5] Writing MCP config...");
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().to_string();

    let result_file = tempfile::NamedTempFile::new()?;
    let result_path = result_file.path().to_path_buf();
    // Keep the file alive but allow the server to write to it
    let _result_file = result_file.into_temp_path();

    let mcp_config_file = tempfile::NamedTempFile::new()?;
    let mcp_config_path = mcp_config_file.path().to_path_buf();

    let mcp_config = json!({
        "mcpServers": {
            "summarize-tools": {
                "command": exe_str,
                "args": [
                    "--server",
                    "--output-path",
                    result_path.to_string_lossy()
                ]
            }
        }
    });

    fs::write(&mcp_config_path, serde_json::to_string_pretty(&mcp_config)?)?;
    println!("  MCP config: {}", mcp_config_path.display());
    println!("  Result file: {}", result_path.display());

    // ── 3. Build the prompt ────────────────────────────────────────────
    println!("[3/5] Building prompt...");

    let prompt = format!(
        "You have access to three MCP tools: json_example, validate_json, and submit.\n\n\
        Your task:\n\
        1. Call the json_example tool to see the expected output format\n\
        2. Summarize the following passage into structured JSON matching that format\n\
        3. Call validate_json with your JSON to check it\n\
        4. If valid, call submit with the final JSON. If invalid, fix the errors and validate again.\n\n\
        IMPORTANT: You MUST call the submit tool with the final structured data. \
        Do not just output text. Use the tools.\n\n\
        PASSAGE:\n{SOURCE_TEXT}\n\n\
        Begin by calling json_example to see the expected format."
    );

    // ── 4. Run Claude Code with MCP tools ──────────────────────────────
    println!("[4/5] Running Claude Code with MCP tools...\n");

    let config = RunConfig {
        output_format: Some(OutputFormat::Text),
        mcp: Some(McpPolicy {
            configs: vec![mcp_config_path.to_string_lossy().to_string()],
            strict: false,
        }),
        ..RunConfig::default()
    };

    let result = cli.run(&prompt, &config).await?;

    println!("=== CLI OUTPUT ===");
    println!("{}", result.stdout);
    println!("=== END CLI OUTPUT ===\n");

    // ── 5. Verify the submitted result ─────────────────────────────────
    println!("[5/5] Verifying submitted result...\n");

    let submitted_json = fs::read_to_string(&result_path)?;

    if submitted_json.is_empty() {
        println!("FAIL: No data was submitted. The agent did not call the submit tool.");
        println!("\nCLI stdout:\n{}", result.stdout);
        println!("CLI stderr:\n{}", result.stderr);
        std::process::exit(1);
    }

    println!("Submitted JSON:\n{submitted_json}\n");

    let summary: TextSummary = serde_json::from_str(&submitted_json)?;

    println!("Deserialized TextSummary:");
    println!("  title:      {}", summary.title);
    println!("  key_points:");
    for (i, point) in summary.key_points.iter().enumerate() {
        println!("    {}: {point}", i + 1);
    }
    println!("  sentiment:  {}", summary.sentiment);
    println!("  word_count: {}", summary.word_count);
    println!("  entities:   {:?}", summary.entities);

    // Assertions
    assert!(!summary.title.is_empty(), "title should not be empty");
    assert!(
        summary.key_points.len() >= 2,
        "should have at least 2 key points, got {}",
        summary.key_points.len()
    );
    assert!(
        ["positive", "negative", "neutral"].contains(&summary.sentiment.as_str()),
        "sentiment must be positive/negative/neutral, got: {}",
        summary.sentiment
    );
    assert!(summary.word_count >= 1, "word_count must be >= 1");
    assert!(!summary.entities.is_empty(), "should have at least 1 entity");

    println!("\nAll assertions passed.");
    println!("The agent called the MCP tools and submitted valid structured data.");

    Ok(())
}
