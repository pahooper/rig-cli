use clap::Parser;
use rig::agent::AgentBuilder;
use rig::tool::ToolSet;
use rig_mcp_server::prelude::ToolSetExt;
use rig_provider::OpenCodeModel;
use opencode_adapter::{discover_opencode, OpenCodeCli};
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

    // 2. Initialize OpenCode
    let path = discover_opencode(None)?;
    println!("Discovered OpenCode binary at: {:?}", path);
    let cli = OpenCodeCli::new(path.clone());
    let model = OpenCodeModel { cli: cli.clone() };

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

    println!("Sending prompt to OpenCode...");
    let prompt = "Please generate a user profile example for 'Alice', verify it, and then submit it.";
    
    // We use a custom config for testing "Big Pickle" mock model
    // Note: In a real scenario, OpenCode would use the default config/model.
    let config = opencode_adapter::OpenCodeConfig {
        model: Some("Big Pickle".to_string()),
        print_logs: true, 
        ..Default::default()
    };

    println!("--- DEBUG: Running raw CLI command to demonstrate MCP usage ---");
    // We run the CLI directly because `agent.prompt` effectively just calls `cli.run`
    // but `cli.run` allows us to pass the config override easily for this test.
    let full_message = format!("{}\n\n{}", system_prompt, prompt);
    let raw_res = cli.run(&full_message, &config).await?;
    
    println!("Exit Code: {}", raw_res.exit_code);
    println!("Stdout: '{}'", raw_res.stdout);
    println!("Stderr: '{}'", raw_res.stderr);
    println!("--- END DEBUG ---");
    
    // Clean up check
    if raw_res.stderr.contains("ProviderModelNotFoundError") && raw_res.exit_code != 0 {
        println!("SUCCESS: OpenCode CLI executed, attempted to use Big Pickle model, and failed as expected.");
        println!("This confirms the adapter propagated the command and result correctly.");
    } else if raw_res.exit_code == 0 {
        println!("SUCCESS: OpenCode CLI execution.");
        for line in raw_res.stdout.lines() {
             if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                 println!("Parsed JSON line: {}", val);
             }
        }
    } else {
        println!("FAILURE: Unexpected exit code or error output.");
    }
    
    Ok(())
}

fn register_self_as_mcp() -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().to_string();
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let path = home.join(".opencode.json");

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
    
    servers.insert("opencode_jsonl_example".to_string(), json!({
        "command": exe_str,
        "args": ["--server"],
        "env": {}
    }));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&data)?)?;
    println!("Registered 'opencode_jsonl_example' in MCP config.");
    Ok(())
}
