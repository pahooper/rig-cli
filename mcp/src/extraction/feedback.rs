//! Validation feedback builders for agent retry loops.

use serde_json::Value;

/// Build validation feedback message for the agent with complete error context.
///
/// Includes:
/// - Attempt counter (e.g., "Attempt 2/3")
/// - All validation errors with instance paths
/// - Full expected schema (for agent reference)
/// - Echoed submission (so agent can compare)
/// - Instruction to fix and resubmit
///
/// # Examples
///
/// ```
/// use rig_mcp_server::extraction::build_validation_feedback;
/// use serde_json::json;
///
/// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
/// let instance = json!({"name": 123});
/// let errors = vec!["At path '/name': 123 is not of type 'string'".to_string()];
///
/// let feedback = build_validation_feedback(&schema, &instance, &errors, 1, 3);
/// assert!(feedback.contains("Attempt 1/3"));
/// assert!(feedback.contains("JSON validation failed"));
/// ```
#[must_use]
pub fn build_validation_feedback(
    schema: &Value,
    instance: &Value,
    errors: &[String],
    attempt: usize,
    max_attempts: usize,
) -> String {
    let mut feedback = format!("Attempt {attempt}/{max_attempts}: JSON validation failed.\n\n");

    // Add all validation errors
    feedback.push_str("Errors:\n");
    for error in errors {
        feedback.push_str("  - ");
        feedback.push_str(error);
        feedback.push('\n');
    }

    // Add expected schema
    feedback.push_str("\nExpected schema:\n");
    let schema_str = serde_json::to_string_pretty(schema)
        .unwrap_or_else(|_| schema.to_string());
    feedback.push_str(&schema_str);

    // Echo back the invalid submission
    feedback.push_str("\n\nYour submission:\n");
    let instance_str = serde_json::to_string_pretty(instance)
        .unwrap_or_else(|_| instance.to_string());
    feedback.push_str(&instance_str);

    feedback.push_str("\n\nPlease fix all errors and resubmit.");

    feedback
}

/// Collect all validation errors from jsonschema validation.
///
/// Returns a vector of formatted error strings with instance paths.
/// Uses `iter_errors()` to collect ALL validation failures, not just the first.
///
/// # Examples
///
/// ```
/// use rig_mcp_server::extraction::feedback::collect_validation_errors;
/// use serde_json::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "name": {"type": "string"},
///         "age": {"type": "integer", "minimum": 0}
///     },
///     "required": ["name", "age"]
/// });
///
/// let instance = json!({"age": -5});
/// let errors = collect_validation_errors(&schema, &instance);
/// assert!(!errors.is_empty());
/// ```
#[must_use]
pub fn collect_validation_errors(schema: &Value, instance: &Value) -> Vec<String> {
    match jsonschema::Validator::new(schema) {
        Ok(validator) => {
            validator
                .iter_errors(instance)
                .map(|error| format!("At path '{}': {}", error.instance_path, error))
                .collect()
        }
        Err(e) => {
            // Schema compilation failed
            vec![format!("Schema compilation error: {}", e)]
        }
    }
}

/// Build parse error feedback for when agent output is not valid JSON.
///
/// Includes:
/// - Attempt counter
/// - Parse error message
/// - Truncated raw output (first 500 chars)
/// - Expected schema (for reference)
/// - Instruction to respond with valid JSON
///
/// # Examples
///
/// ```
/// use rig_mcp_server::extraction::feedback::build_parse_error_feedback;
/// use serde_json::json;
///
/// let schema = json!({"type": "object"});
/// let raw_text = "This is not JSON at all!";
/// let parse_error = "expected value at line 1 column 1";
///
/// let feedback = build_parse_error_feedback(raw_text, parse_error, 1, 3, &schema);
/// assert!(feedback.contains("Attempt 1/3"));
/// assert!(feedback.contains("Could not parse"));
/// ```
#[must_use]
pub fn build_parse_error_feedback(
    raw_text: &str,
    parse_error: &str,
    attempt: usize,
    max_attempts: usize,
    schema: &Value,
) -> String {
    let mut feedback = format!(
        "Attempt {attempt}/{max_attempts}: Could not parse your response as JSON.\n\n"
    );

    // Add parse error
    feedback.push_str("Parse error: ");
    feedback.push_str(parse_error);
    feedback.push_str("\n\n");

    // Add truncated raw response
    feedback.push_str("Your response (first 500 chars):\n");
    let truncated = if raw_text.len() > 500 {
        format!("{}...", &raw_text[..500])
    } else {
        raw_text.to_string()
    };
    feedback.push_str(&truncated);

    // Add expected schema
    feedback.push_str("\n\nExpected schema:\n");
    let schema_str = serde_json::to_string_pretty(schema)
        .unwrap_or_else(|_| schema.to_string());
    feedback.push_str(&schema_str);

    feedback.push_str("\n\nPlease respond with valid JSON matching the schema above.");

    feedback
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_validation_feedback() {
        let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
        let instance = json!({"name": 123});
        let errors = vec!["At path '/name': 123 is not of type 'string'".to_string()];

        let feedback = build_validation_feedback(&schema, &instance, &errors, 1, 3);

        assert!(feedback.contains("Attempt 1/3"));
        assert!(feedback.contains("JSON validation failed"));
        assert!(feedback.contains("Errors:"));
        assert!(feedback.contains("Expected schema:"));
        assert!(feedback.contains("Your submission:"));
        assert!(feedback.contains("Please fix all errors"));
    }

    #[test]
    fn test_collect_validation_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name", "age"]
        });

        let instance = json!({"age": -5});
        let errors = collect_validation_errors(&schema, &instance);

        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_build_parse_error_feedback() {
        let schema = json!({"type": "object"});
        let raw_text = "This is not JSON!";
        let parse_error = "expected value";

        let feedback = build_parse_error_feedback(raw_text, parse_error, 2, 3, &schema);

        assert!(feedback.contains("Attempt 2/3"));
        assert!(feedback.contains("Could not parse"));
        assert!(feedback.contains("Parse error:"));
        assert!(feedback.contains("This is not JSON!"));
        assert!(feedback.contains("Expected schema:"));
    }

    #[test]
    fn test_build_parse_error_feedback_truncates_long_text() {
        let schema = json!({"type": "object"});
        let raw_text = "x".repeat(1000);
        let parse_error = "error";

        let feedback = build_parse_error_feedback(&raw_text, parse_error, 1, 3, &schema);

        // Should truncate to 500 chars + "..."
        assert!(feedback.contains("..."));
        let response_section = feedback.split("Your response").nth(1).unwrap();
        let text_part = response_section.split("Expected schema").next().unwrap();
        assert!(text_part.len() < 600); // 500 chars + some formatting
    }
}
