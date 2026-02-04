//! E2E test of the retry/validation feedback loop with a strict schema.
//!
//! This example uses a more constrained schema to exercise the retry path:
//! - Required fields with specific formats
//! - Enum-like string constraints
//! - Numeric ranges
//! - Array minimum length
//!
//! Run with: `cargo run --example extraction_retry_e2e`

use rig_cli_claude::{init, ClaudeCli, OutputFormat, RunConfig};
use rig_cli_mcp::extraction::{ExtractionConfig, ExtractionOrchestrator};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Strict schema with constraints that may trigger validation failures.
#[derive(Debug, Deserialize, Serialize)]
struct CodeReview {
    file_path: String,
    severity: String,
    line_start: u32,
    line_end: u32,
    category: String,
    message: String,
    suggestion: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize
    println!("[1/4] Discovering Claude Code CLI...");
    let report = init(None).await?;
    let cli = ClaudeCli::new(report.claude_path.clone(), report.capabilities.clone());
    println!("  Found: {}", report.claude_path.display());

    // 2. Build a strict schema with constraints (not just type checking)
    let schema = json!({
        "type": "object",
        "properties": {
            "file_path": {
                "type": "string",
                "minLength": 1,
                "description": "Relative file path (e.g., src/main.rs)"
            },
            "severity": {
                "type": "string",
                "enum": ["low", "medium", "high", "critical"],
                "description": "Must be exactly one of: low, medium, high, critical"
            },
            "line_start": {
                "type": "integer",
                "minimum": 1,
                "description": "Starting line number (must be >= 1)"
            },
            "line_end": {
                "type": "integer",
                "minimum": 1,
                "description": "Ending line number (must be >= line_start)"
            },
            "category": {
                "type": "string",
                "enum": ["bug", "performance", "security", "style", "documentation"],
                "description": "Must be exactly one of: bug, performance, security, style, documentation"
            },
            "message": {
                "type": "string",
                "minLength": 10,
                "description": "Description of the issue (at least 10 characters)"
            },
            "suggestion": {
                "type": "string",
                "minLength": 10,
                "description": "Suggested fix (at least 10 characters)"
            }
        },
        "required": ["file_path", "severity", "line_start", "line_end", "category", "message", "suggestion"],
        "additionalProperties": false
    });
    println!("[2/4] Strict schema created (7 required fields, enums, ranges, no additionalProperties)");

    // 3. Configure with 3 attempts so we can observe retry behavior
    let config = ExtractionConfig::default()
        .with_max_attempts(3)
        .with_schema_in_feedback(true);

    let orchestrator = ExtractionOrchestrator::with_config(schema, config);
    println!("[3/4] Orchestrator configured (max_attempts: 3)");

    // 4. Run extraction
    println!("[4/4] Running extraction...\n");

    let prompt = "You are a JSON extraction assistant. Respond with ONLY a valid JSON object, no markdown, no explanation, no code fences.\n\nAnalyze the file `mcp/src/extraction/orchestrator.rs` in this project and produce a code review finding as JSON.\n\nThe JSON MUST have exactly these fields (no extra fields allowed):\n- file_path: string (relative path, e.g. \"mcp/src/extraction/orchestrator.rs\")\n- severity: one of \"low\", \"medium\", \"high\", \"critical\" (lowercase only)\n- line_start: integer >= 1\n- line_end: integer >= 1\n- category: one of \"bug\", \"performance\", \"security\", \"style\", \"documentation\" (lowercase only)\n- message: string (at least 10 characters describing the issue)\n- suggestion: string (at least 10 characters describing the fix)\n\nRespond with ONLY the JSON object.";

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

            // Deserialize to struct
            let review: CodeReview = serde_json::from_value(value)?;
            println!("\nDeserialized CodeReview:");
            println!("  file_path: {}", review.file_path);
            println!("  severity: {}", review.severity);
            println!("  line_start: {}", review.line_start);
            println!("  line_end: {}", review.line_end);
            println!("  category: {}", review.category);
            println!("  message: {}", review.message);
            println!("  suggestion: {}", review.suggestion);

            // Verify constraints
            assert!(
                ["low", "medium", "high", "critical"].contains(&review.severity.as_str()),
                "severity must be a valid enum value"
            );
            assert!(
                ["bug", "performance", "security", "style", "documentation"]
                    .contains(&review.category.as_str()),
                "category must be a valid enum value"
            );
            assert!(review.line_start >= 1, "line_start must be >= 1");
            assert!(review.line_end >= 1, "line_end must be >= 1");
            assert!(
                review.message.len() >= 10,
                "message must be at least 10 chars"
            );
            assert!(
                review.suggestion.len() >= 10,
                "suggestion must be at least 10 chars"
            );

            if metrics.total_attempts > 1 {
                println!(
                    "\nRetry loop exercised: took {} attempts to get valid output.",
                    metrics.total_attempts
                );
            } else {
                println!("\nFirst-attempt success (retry path not exercised).");
            }

            println!("\nAll assertions passed.");
        }
        Err(e) => {
            println!("=== EXTRACTION FAILED ===\n");
            println!("Error: {e}");
            if let rig_cli_mcp::extraction::ExtractionError::MaxRetriesExceeded {
                history,
                metrics,
                ..
            } = &e
            {
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
                    let preview_len = record.raw_agent_output.len().min(200);
                    println!(
                        "    Raw output (first 200 chars): {}",
                        &record.raw_agent_output[..preview_len]
                    );
                }
                println!("\nMetrics:");
                println!("  Attempts: {}", metrics.total_attempts);
                println!("  Wall time: {:?}", metrics.wall_time);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
