use codex_adapter::{discover_codex, CodexCli};
use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use rig_provider::CodexModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Codex adapter
    let path = discover_codex()?;
    let cli = CodexCli::new(path);
    let model = CodexModel { cli };

    // 2. Create an agent
    let agent = AgentBuilder::new(model).build();

    // 3. Start a persistent session
    println!("Starting a session with Codex...");

    let response1 = agent
        .prompt("Create a new file called 'test.txt' with the content 'Hello from Rig!'")
        .await?;
    println!("Step 1: {}", response1);

    let response2 = agent
        .prompt("Read the content of 'test.txt' and verify it works.")
        .await?;
    println!("Step 2: {}", response2);

    println!("\nSession isolation verified. The second call found the file created by the first.");

    Ok(())
}
