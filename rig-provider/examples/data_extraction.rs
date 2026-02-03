use opencode_adapter::{discover_opencode, OpenCodeCli};
use rig::agent::AgentBuilder;
use rig::completion::Prompt;
use rig_provider::OpenCodeModel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize OpenCode adapter
    let path = discover_opencode(None)?;
    let cli = OpenCodeCli::new(path);
    let model = OpenCodeModel { cli };

    // 2. Create an agent
    let agent = AgentBuilder::new(model).build();

    // 3. Perform structured data extraction task
    println!("Extracting dependency information using OpenCode...");

    let prompt = "Extract the list of internal dependencies (starting with ../) from rig-provider/Cargo.toml and return them as a JSON list.";

    let response = agent.prompt(prompt).await?;

    println!("\nExtracted Data:\n{}", response);

    Ok(())
}
