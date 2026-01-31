use claudecode_adapter::{init, ClaudeCli};
use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use rig_provider::ClaudeModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize the Claude adapter
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path, report.capabilities);
    let model = ClaudeModel { cli };

    // 2. Build a high-level agent where the adapter is the brain
    // No OpenAI/external LLM needed!
    let agent = AgentBuilder::new(model)
        .preamble("You are a senior Rust architect. Analyze the project structure and suggest improvements.")
        .build();

    // 3. Run a complex workflow
    println!("Agent is starting the project analysis...");
    let result = agent
        .prompt(
            "Analyze the claudecode-adapter crate and suggested a better error handling strategy.",
        )
        .await?;

    println!("\nFinal Agent Advice:\n{}", result);

    Ok(())
}
