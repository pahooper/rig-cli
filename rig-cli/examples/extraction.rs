//! Example: Extraction agent with MCP
//!
//! Demonstrates structured data extraction from unstructured text.
//! The agent is forced to submit via MCP tools, ensuring schema compliance.
//!
//! This is the primary use case for rig-cli: reliable structured extraction.
//!
//! Run: `cargo run -p rig-cli --example extraction`

use rig::tool::ToolSet;
use rig_cli::tools::{JsonSchemaToolkit, ToolSetExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Person information to extract
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct PersonInfo {
    /// Full name of the person
    name: String,
    /// Age in years
    age: u32,
    /// Email address if mentioned
    email: Option<String>,
    /// List of skills or expertise
    skills: Vec<String>,
}

fn build_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    let (submit, validate, example) = JsonSchemaToolkit::<PersonInfo>::builder()
        .example(PersonInfo {
            name: "Jane Doe".to_string(),
            age: 28,
            email: Some("jane@example.com".to_string()),
            skills: vec!["Python".to_string(), "Machine Learning".to_string()],
        })
        .on_success("Person info extracted successfully!")
        .build()
        .build_tools();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);
    toolset
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RIG_MCP_SERVER").is_ok() {
        return Ok(build_toolset().into_handler().await?.serve_stdio().await?);
    }

    let client = rig_cli::claude::Client::new().await?;

    // --- KEY CODE: Extraction workflow ---
    let agent = client
        .mcp_agent("sonnet")
        .toolset(build_toolset())
        .preamble(
            "You are a data extraction agent. \
             Extract person information from the provided text. \
             Use json_example to see the format, validate_json to check, \
             then submit.",
        )
        .build()?;

    let unstructured_text = r#"
        Meet Sarah Chen, a 32-year-old software engineer from Seattle.
        She specializes in Rust, distributed systems, and cloud architecture.
        You can reach her at sarah.chen@techcorp.io for consulting work.
    "#;

    let result = agent
        .prompt(&format!(
            "Extract person information from this text:\n{}",
            unstructured_text
        ))
        .await?;
    // --- END KEY CODE ---

    println!("Extracted:\n{}", result);

    // Optionally parse the result
    if let Ok(person) = serde_json::from_str::<PersonInfo>(&result) {
        println!("\nParsed PersonInfo:");
        println!("  Name: {}", person.name);
        println!("  Age: {}", person.age);
        println!("  Email: {:?}", person.email);
        println!("  Skills: {:?}", person.skills);
    }

    Ok(())
}
