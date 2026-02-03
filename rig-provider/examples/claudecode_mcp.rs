use clap::Parser;
use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use rig::tool::ToolSet;
use rig_mcp_server::prelude::ToolSetExt;
use rig_provider::ClaudeModel;
use claudecode_adapter::{init, ClaudeCli};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Example tool error")]
struct ExampleError;

#[derive(Parser)]
struct Args {
    /// Run as an MCP server
    #[clap(long)]
    server: bool,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ExampleArgs {
    name: String,
    data: serde_json::Value,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct VerifyArgs {
    id: String,
    schema_version: u32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SubmitArgs {
    id: String,
    final_payload: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
struct ExampleTool;
impl rig::tool::Tool for ExampleTool {
    const NAME: &'static str = "example";
    type Error = ExampleError; // Use a type that impls std::error::Error
    type Args = ExampleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "example".to_string(),
            description: "Create an example data object".to_string(),
            parameters: json!(schemars::schema_for!(ExampleArgs)),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Created example for {}", args.name))
    }
}

#[derive(Deserialize, Serialize)]
struct VerifyTool;
impl rig::tool::Tool for VerifyTool {
    const NAME: &'static str = "verify";
    type Error = ExampleError;
    type Args = VerifyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "verify".to_string(),
            description: "Verify the schema of a data object by ID".to_string(),
            parameters: json!(schemars::schema_for!(VerifyArgs)),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Verified ID {} version {}", args.id, args.schema_version))
    }
}

#[derive(Deserialize, Serialize)]
struct SubmitTool;
impl rig::tool::Tool for SubmitTool {
    const NAME: &'static str = "submit";
    type Error = ExampleError;
    type Args = SubmitArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "submit".to_string(),
            description: "Submit a verified data object".to_string(),
            parameters: json!(schemars::schema_for!(SubmitArgs)),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Submitted payload for ID {}", args.id))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut toolset = ToolSet::default();
    toolset.add_tool(ExampleTool);
    toolset.add_tool(VerifyTool);
    toolset.add_tool(SubmitTool);

    if args.server {
        eprintln!("Starting OpenCode Example MCP Server...");
        toolset.run_stdio().await?;
        return Ok(());
    }

    // --- CLIENT MODE ---

    // 1. Register this binary as an MCP server for OpenCode
    register_self_as_mcp()?;

    // 2. Initialize Claude Code
    println!("Initializing Claude Code adapter...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path, report.capabilities);
    let model = ClaudeModel { cli: cli.clone() };

    // 3. Construct System Prompt (We still guide the model, but tools are provided via MCP)
    let system_prompt = 
        "You are a helpful assistant. You have access to tools via MCP.\n\
        RULES:\n\
        1. You MUST use the tools in the following order: example -> verify -> submit.\n\
        2. You MUST output ONLY valid JSONL (JSON Lines).\n\
        3. Each line must be a JSON object like: {{ \"tool\": \"tool_name\", \"args\": {{ ... }} }}\n\
        4. Do not output any markdown or explanation, JUST the JSONL lines.";

    // 4. Create Agent and Run
    // Note: We do NOT add tools to the agent here. The OpenCode CLI has them via MCP.
    let agent = AgentBuilder::new(model)
        .preamble(system_prompt)
        .build();

    println!("Sending prompt to Claude Code...");
    let prompt = "Please generate a user profile example for 'Alice', verify it, and then submit it.";
    
    println!("--- DEBUG: Running via agent ---");
    let response = agent.prompt(prompt).await?;
    
    println!("\n=== Claude Code Response ===");
    println!("{}", response);
    println!("=== End Response ===\n");
    
    println!("SUCCESS: Claude Code MCP example completed.");
    println!("MCP server registered at ~/.claude.json as 'claudecode_mcp_example'");
    
    Ok(())
}

fn register_self_as_mcp() -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().to_string();
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let path = home.join(".claude.json");

    println!("Registering example MCP server at: {}", path.display());

    let mut data = if path.exists() {
        let content = fs::read_to_string(&path)?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or(json!({"mcpServers": {}}))
    } else {
        json!({"mcpServers": {}})
    };

    if data.get("mcpServers").is_none() {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("mcpServers".to_string(), json!({}));
        }
    }

    let servers = data.get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("Invalid config format")?;
    
    servers.insert("claudecode_mcp_example".to_string(), json!({
        "command": exe_str,
        "args": ["--server"],
        "env": {}
    }));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&data)?)?;
    println!("Registered 'claudecode_mcp_example' in MCP config.");
    Ok(())
}
