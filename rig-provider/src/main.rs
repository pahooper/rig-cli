//! The Rig Provider binary serves as an MCP server bridging AI CLI adapters.

use clap::{Parser, Subcommand};
use rig::tool::ToolSet;
use rig_mcp_server::prelude::*;
use rig_provider::adapters::claude::ClaudeTool;
use rig_provider::adapters::codex::CodexTool;
use rig_provider::adapters::opencode::OpenCodeTool;
use rig_provider::setup::{run_setup, SetupConfig};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the MCP server (default)
    Serve,
    /// Automatically register this provider in Claude/Codex/OpenCode configs
    Setup {
        /// Show what would be done without modifying files
        #[arg(long)]
        dry_run: bool,
    },
}

/// Structured output from the provider containing the AI result and metadata.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct ProviderOutput {
    /// The final result of the AI's work
    pub result: String,
    /// Additional metadata or extracted fields
    pub metadata: std::collections::HashMap<String, String>,
}

use rig_provider::errors::ProviderError;

#[tokio::main]
async fn main() -> Result<(), ProviderError> {
    let cli = Cli::parse();

    // Initialize tracing aligned with Rig's style
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    match cli.command {
        Some(Commands::Setup { dry_run }) => {
            run_setup(&SetupConfig { dry_run })?;
        }
        Some(Commands::Serve) | None => {
            run_serve().await?;
        }
    }

    Ok(())
}

async fn run_serve() -> Result<(), ProviderError> {
    let mut toolset = ToolSet::default();

    // In a real scenario, these configs would be detected or passed via env
    let mcp_configs = vec!["~/.claude.json".to_string()];

    tracing::info!("Initializing Claude Code adapter...");
    let claude = ClaudeTool::new(mcp_configs).await?;
    toolset.add_tool(claude);

    tracing::info!("Initializing Codex adapter...");
    let codex = CodexTool::new().await?;
    toolset.add_tool(codex);

    tracing::info!("Initializing OpenCode adapter...");
    let opencode = OpenCodeTool::new().await?;
    toolset.add_tool(opencode);

    // 2. Add the 3 default MCP tools for seamless extraction
    let toolkit = JsonSchemaToolkit::<ProviderOutput>::builder()
        .on_success("Output successfully processed and extracted.")
        .build();

    let (submit, validate, example) = toolkit.build_tools();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);

    tracing::info!("Rig Provider MCP Server starting over stdio...");

    // Start the MCP Server
    toolset
        .run_stdio()
        .await
        .map_err(|e| ProviderError::Init(e.to_string()))?;

    Ok(())
}
