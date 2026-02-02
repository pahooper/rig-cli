//! End-to-end test of the ExtractionOrchestrator using a real Claude Code CLI.
//!
//! This example verifies the full extraction pipeline:
//! 1. Discovers and initializes Claude Code CLI
//! 2. Creates an ExtractionOrchestrator with a JSON schema
//! 3. Wraps the Claude CLI as the agent_fn closure
//! 4. Extracts structured data and validates the result
//!
//! Run with: `cargo run --example extraction_e2e`

use claudecode_adapter::{init, ClaudeCli, OutputFormat, RunConfig};
use rig_mcp_server::extraction::{ExtractionConfig, ExtractionOrchestrator};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// The target struct we want to extract from the LLM.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ProjectSummary {
    /// Name of the project.
    name: String,
    /// Primary programming language used.
    language: String,
    /// List of key dependencies (at least one).
    dependencies: Vec<String>,
    /// Whether the project has a test suite.
    has_tests: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Claude Code CLI
    println!("[1/4] Discovering Claude Code CLI...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path.clone(), report.capabilities.clone());
    println!("  Found: {}", report.claude_path.display());

    // 2. Build schema from the target type using schemars
    let schema = json!(schemars::schema_for!(ProjectSummary));
    println!("[2/4] Schema generated for ProjectSummary");

    // 3. Configure the orchestrator
    let config = ExtractionConfig::default()
        .with_max_attempts(3)
        .with_schema_in_feedback(true);

    let orchestrator = ExtractionOrchestrator::with_config(schema, config);
    println!("[3/4] Orchestrator configured (max_attempts: 3)");

    // 4. Run extraction with the Claude CLI as the agent function
    println!("[4/4] Running extraction...");
    println!("  Sending prompt to Claude Code CLI (this may take a moment)...\n");

    let prompt = "You are a JSON extraction assistant. You MUST respond with ONLY a valid JSON object, no markdown, no explanation, no code fences.\n\nExtract information about the rig-cli project and return it as JSON matching this schema:\n- name: string (project name)\n- language: string (primary language)\n- dependencies: array of strings (key dependencies, at least one)\n- has_tests: boolean (whether the project has tests)\n\nRespond with ONLY the JSON object. Example:\n{\"name\": \"example\", \"language\": \"Rust\", \"dependencies\": [\"tokio\", \"serde\"], \"has_tests\": true}";

    let cli_clone = cli.clone();
    let result = orchestrator
        .extract(
            |prompt_text| {
                let cli = cli_clone.clone();
                async move {
                    let config = RunConfig {
                        output_format: Some(OutputFormat::Text),
                        ..RunConfig::default()
                    };
                    cli.run(&prompt_text, &config)
                        .await
                        .map(|r| r.stdout)
                        .map_err(|e| e.to_string())
                }
            },
            prompt.to_string(),
        )
        .await;

    match result {
        Ok((value, metrics)) => {
            println!("=== EXTRACTION SUCCEEDED ===\n");
            println!(
                "Extracted JSON:\n{}",
                serde_json::to_string_pretty(&value)?
            );
            println!("\nMetrics:");
            println!("  Attempts: {}", metrics.total_attempts);
            println!("  Wall time: {:?}", metrics.wall_time);
            println!("  Est. input tokens: {}", metrics.estimated_input_tokens);
            println!("  Est. output tokens: {}", metrics.estimated_output_tokens);

            // Deserialize to the typed struct to verify
            let summary: ProjectSummary = serde_json::from_value(value)?;
            println!("\nDeserialized ProjectSummary:");
            println!("  name: {}", summary.name);
            println!("  language: {}", summary.language);
            println!("  dependencies: {:?}", summary.dependencies);
            println!("  has_tests: {}", summary.has_tests);

            // Basic assertions
            assert!(!summary.name.is_empty(), "name should not be empty");
            assert!(!summary.language.is_empty(), "language should not be empty");
            assert!(
                !summary.dependencies.is_empty(),
                "should have at least one dependency"
            );
            println!("\nAll assertions passed.");
        }
        Err(e) => {
            println!("=== EXTRACTION FAILED ===\n");
            println!("Error: {e}");
            match &e {
                rig_mcp_server::extraction::ExtractionError::MaxRetriesExceeded {
                    history,
                    metrics,
                    ..
                } => {
                    println!("\nAttempt history:");
                    for record in history {
                        println!(
                            "  Attempt {}: {} errors",
                            record.attempt_number,
                            record.validation_errors.len()
                        );
                        for err in &record.validation_errors {
                            println!("    - {err}");
                        }
                        println!(
                            "    Raw output (first 200 chars): {}",
                            &record.raw_agent_output[..record.raw_agent_output.len().min(200)]
                        );
                    }
                    println!("\nMetrics:");
                    println!("  Attempts: {}", metrics.total_attempts);
                    println!("  Wall time: {:?}", metrics.wall_time);
                }
                _ => {}
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
