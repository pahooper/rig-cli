//! Demonstrates a simple one-shot prompt using the Claude Code CLI adapter.

use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use rig_cli_claude::{init, ClaudeCli};
use rig_cli_provider::ClaudeModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize the Claude adapter
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path, report.capabilities);
    let model = ClaudeModel { cli };

    // 2. Create an agent using the adapter model
    let agent = AgentBuilder::new(model).build();

    // 3. Simple one-shot prompt using Rig's prompt trait
    println!("Asking Claude Code to analyze the current workspace...");
    let response = agent
        .prompt("List the files in the current directory and explain the project structure.")
        .await?;

    println!("\nClaude Response:\n{response}");

    Ok(())
}
