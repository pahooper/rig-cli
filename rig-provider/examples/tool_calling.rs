//! Demonstrates tool calling with the Claude Code CLI adapter.

use rig::agent::AgentBuilder;
use rig::completion::{Prompt, ToolDefinition};
use rig::tool::Tool;
use rig_cli_claude::{init, ClaudeCli};
use rig_cli_provider::ClaudeModel;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct CalculatorArgs {
    _operation: String,
    _x: f64,
    _y: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Calculator;

impl Tool for Calculator {
    const NAME: &'static str = "calculator";
    type Error = std::io::Error;
    type Args = CalculatorArgs;
    type Output = f64;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "calculator".to_string(),
            description: "Perform basic arithmetic operations".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "enum": ["add", "subtract", "multiply", "divide"] },
                    "x": { "type": "number" },
                    "y": { "type": "number" }
                },
                "required": ["operation", "x", "y"]
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // This function will NOT be called when using ClaudeModel,
        // because Claude Code CLI executes tools internally (MCP or built-in).
        // However, Rig requires this implementation to define the tool.
        // We print a message to prove this point if it IS called.
        println!(">>> RIG CALCULATOR CALLED! (Unexpected behavior for Adapter) <<<");
        Ok(0.0)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Note: For this example to work effectively, the `calculator` tool must be
    // available to the Claude Code CLI (e.g., via a configured MCP server).
    // The `ClaudeModel` will pass "calculator" to the `--allowed-tools` flag,
    // ensuring Claude determines it *can* use this tool.
    //
    // IMPORTANT: Rig tools defined here in Rust are NOT automatically executed by Claude Code.
    // Claude Code runs as a separate process and only executes tools it knows about (MCP or builtin).
    // This Rust definition serves to validate the request structure and permissions within Rig.

    println!("Initializing Claude adapter...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path, report.capabilities);
    let model = ClaudeModel { cli };

    let tool = Calculator;

    let agent = AgentBuilder::new(model)
        .max_tokens(1024)
        .tool(tool) // This defines the tool AND restricts Claude to use it
        .build();

    println!("Sending prompt with tool restriction...");
    let response = agent
        .prompt("Calculate 5 + 3 using the calculator tool.")
        .await?;

    println!("Response: {response}");

    Ok(())
}
