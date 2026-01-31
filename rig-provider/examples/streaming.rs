use claudecode_adapter::{init as init_claude, ClaudeCli};
use codex_adapter::{discover_codex, CodexCli};
use rig::agent::{stream_to_stdout, AgentBuilder};
use rig::streaming::StreamingPrompt;
use rig_provider::{ClaudeModel, CodexModel, OpenCodeModel};
use opencode_adapter::{discover_opencode, OpenCodeCli};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let model_type = args.get(1).map(|s| s.as_str()).unwrap_or("claude");

    println!("Starting streaming request with model: {}", model_type);

    // Create an agent using the selected model
    // We have to use dynamic dispatch or conditional compilation if we want to share the AgentBuilder line
    // but Rig Agents are strongly typed to the model.
    // So we'll just branch here.

    if model_type == "codex" {
        println!("Initializing Codex adapter...");
        let path = discover_codex()?;
        let cli = CodexCli::new(path);
        let model = CodexModel { cli };
        let agent = AgentBuilder::new(model).build();

        let mut stream = agent
            .stream_prompt("Count from 1 to 5 slowly, explaining each number.")
            .await;
        stream_to_stdout(&mut stream).await?;
    } else if model_type == "opencode" {
        println!("Initializing OpenCode adapter...");
        let path = discover_opencode()?;
        let cli = OpenCodeCli::new(path);
        let model = OpenCodeModel { cli };
        let agent = AgentBuilder::new(model).build();
        
        let mut stream = agent
            .stream_prompt("Count from 1 to 5 slowly, explaining each number.")
            .await;
        stream_to_stdout(&mut stream).await?;
    } else {
        println!("Initializing Claude adapter...");
        let report = init_claude(None).await?;
        let cli = ClaudeCli::new(report.claude_path, report.capabilities);
        let model = ClaudeModel { cli };
        let agent = AgentBuilder::new(model).build();

        let mut stream = agent
            .stream_prompt("Count from 1 to 5 slowly, explaining each number.")
            .await;
        stream_to_stdout(&mut stream).await?;
    }

    println!("\nStream complete.");
    Ok(())
}
