//! E2E test: agent calls MCP tools to produce structured JSON output.
//!
//! This binary runs in two modes:
//!
//! **Server mode** (`--server`): Serves `submit`, `validate_json`, and `json_example`
//! tools over stdio MCP protocol. Claude Code spawns this process as an MCP server.
//!
//! **Client mode** (default): Writes an MCP config file pointing to this binary,
//! launches Claude Code CLI with `--mcp-config`, `--tools ""` (disable builtins),
//! and `--allowed-tools` (whitelist MCP tools). The agent discovers and calls the
//! tools autonomously. The `on_submit` callback writes the result to a temp file
//! so the client can verify it.
//!
//! This tests the core value of rig-cli: the agent is forced through MCP tool
//! constraints to submit conforming JSON rather than freeform text.
//!
//! Run with: `cargo run --example mcp_extraction_e2e`

use clap::Parser;
use rig::tool::ToolSet;
use rig_cli_mcp::prelude::ToolSetExt;
use rig_cli_mcp::tools::JsonSchemaToolkit;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// MCP server name used in config JSON and --allowed-tools prefix.
const MCP_SERVER_NAME: &str = "rig_extraction";

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
/// stdout is the MCP transport — do NOT print anything to stdout.
async fn run_server(output_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let output_path =
        output_path.unwrap_or_else(|| PathBuf::from("/tmp/mcp_extraction_e2e_result.json"));

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
            // Write submitted data to output file for client verification
            let _ = fs::write(&output_path, &json);
            format!(
                "Summary submitted: \"{}\" ({} key points, sentiment: {})",
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

    // serve_stdio() — no banners to stdout, clean MCP protocol only
    toolset.into_handler().await?.serve_stdio().await?;

    Ok(())
}

/// Verify the submitted result file contains valid structured data.
fn verify_result(result_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("[5/5] Verifying submitted result...\n");

    let submitted_json = fs::read_to_string(result_path)?;

    if submitted_json.is_empty() {
        println!("FAIL: No data was submitted.");
        println!("The agent did not call the submit tool.");
        println!("Check that:");
        println!("  - MCP server started successfully (check stderr above)");
        println!("  - Tool names match: mcp__{MCP_SERVER_NAME}__submit");
        println!("  - --allowed-tools includes all three MCP tools");
        std::process::exit(1);
    }

    println!("Submitted JSON:\n{submitted_json}\n");

    let summary: TextSummary = serde_json::from_str(&submitted_json)
        .map_err(|e| format!("Failed to deserialize submitted JSON: {e}\nRaw: {submitted_json}"))?;

    println!("=== VERIFIED TextSummary ===");
    println!("  title:      {}", summary.title);
    println!("  key_points:");
    for (i, point) in summary.key_points.iter().enumerate() {
        println!("    {}: {point}", i + 1);
    }
    println!("  sentiment:  {}", summary.sentiment);
    println!("  word_count: {}", summary.word_count);
    println!("  entities:   {:?}", summary.entities);

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
    assert!(
        !summary.entities.is_empty(),
        "should have at least 1 entity"
    );

    println!("\n=== ALL ASSERTIONS PASSED ===");
    println!("The agent called MCP tools and submitted valid structured data.");
    println!("Core value verified: structured output via MCP tool constraints.");

    Ok(())
}

/// Client mode: write MCP config, launch Claude Code, verify result.
async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    use rig_cli_claude::{
        init, BuiltinToolSet, ClaudeCli, McpPolicy, OutputFormat, RunConfig, SystemPromptMode,
    };
    use std::time::Duration;

    // ── 1. Discover Claude Code CLI ────────────────────────────────────
    println!("[1/5] Discovering Claude Code CLI...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path.clone(), report.capabilities.clone());
    println!("  Found: {}", report.claude_path.display());

    // ── 2. Write MCP config pointing to this binary in --server mode ──
    println!("[2/5] Writing MCP config...");
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().to_string();

    // Temp files for result and MCP config
    let result_tmp = tempfile::NamedTempFile::new()?;
    let result_path = result_tmp.path().to_path_buf();
    let _result_handle = result_tmp.into_temp_path(); // keep alive

    let config_tmp = tempfile::NamedTempFile::new()?;
    let config_path = config_tmp.path().to_path_buf();

    let mcp_config = json!({
        "mcpServers": {
            MCP_SERVER_NAME: {
                "command": exe_str,
                "args": [
                    "--server",
                    "--output-path",
                    result_path.to_string_lossy()
                ]
            }
        }
    });

    fs::write(&config_path, serde_json::to_string_pretty(&mcp_config)?)?;
    let _config_handle = config_tmp.into_temp_path(); // keep alive

    println!("  MCP config: {}", config_path.display());
    println!("  Result file: {}", result_path.display());
    println!("  Server binary: {exe_str}");

    // ── 3. Build prompt ────────────────────────────────────────────────
    println!("[3/5] Building prompt...");

    let tool_prefix = format!("mcp__{MCP_SERVER_NAME}__");
    let allowed_tools = vec![
        format!("{tool_prefix}submit"),
        format!("{tool_prefix}validate_json"),
        format!("{tool_prefix}json_example"),
    ];

    println!("  Allowed tools: {}", allowed_tools.join(", "));

    let prompt = format!(
        "You have access to three MCP tools. Complete this task:\n\n\
        1. Call json_example to see the expected JSON format\n\
        2. Read the passage below and create a structured summary matching that format\n\
        3. Call validate_json with your JSON to verify it\n\
        4. If valid, call submit with the final JSON\n\
        5. If invalid, fix the errors and validate again\n\n\
        PASSAGE:\n{SOURCE_TEXT}"
    );

    // ── 4. Run Claude Code with MCP tools ──────────────────────────────
    println!("[4/5] Running Claude Code with MCP tools...");
    println!("  Builtins: default (--allowed-tools whitelists MCP tools only)");
    println!("  Timeout: 600s\n");

    let config = RunConfig {
        output_format: Some(OutputFormat::Text),
        system_prompt: SystemPromptMode::Append(
            "You MUST use the MCP tools to complete this task. \
             Call json_example first to see the format, then validate_json to check your work, \
             then submit with the final result. Do NOT output raw JSON text as your response."
                .to_string(),
        ),
        mcp: Some(McpPolicy {
            configs: vec![config_path.to_string_lossy().to_string()],
            strict: false,
        }),
        tools: rig_cli_claude::ToolPolicy {
            builtin: BuiltinToolSet::Default,
            allowed: Some(allowed_tools),
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout: Duration::from_secs(600),
        ..RunConfig::default()
    };

    let result = cli.run(&prompt, &config).await;

    match result {
        Ok(run_result) => {
            println!("=== CLI OUTPUT ===");
            println!("{}", run_result.stdout);
            if !run_result.stderr.is_empty() {
                println!("--- stderr ---");
                println!("{}", run_result.stderr);
            }
            println!("=== END CLI OUTPUT ===\n");
        }
        Err(e) => {
            println!("CLI execution failed: {e}");
            println!("This may indicate the MCP server failed to start or handshake.");
            std::process::exit(1);
        }
    }

    verify_result(&result_path)
}
