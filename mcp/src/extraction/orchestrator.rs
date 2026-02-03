//! Orchestration layer for retry/validation feedback loops in structured extraction.

use serde_json::Value;
use tokio::time::Instant;

use super::config::ExtractionConfig;
use super::error::{AttemptRecord, ExtractionError};
use super::feedback::{build_parse_error_feedback, build_validation_feedback, collect_validation_errors};
use super::metrics::{estimate_tokens, ExtractionMetrics};

/// Orchestrator for running bounded retry loops with validation feedback.
///
/// The orchestrator validates agent output against a JSON schema, feeds back
/// validation errors with full context, and tracks metrics across attempts.
pub struct ExtractionOrchestrator {
    schema: Value,
    config: ExtractionConfig,
}

impl ExtractionOrchestrator {
    /// Creates a new orchestrator with the given schema and default configuration.
    #[must_use]
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            config: ExtractionConfig::default(),
        }
    }

    /// Creates a new orchestrator with the given schema and configuration.
    #[must_use]
    pub const fn with_config(schema: Value, config: ExtractionConfig) -> Self {
        Self { schema, config }
    }

    /// Sets the maximum number of retry attempts (fluent builder pattern).
    #[must_use]
    pub const fn max_attempts(mut self, max: usize) -> Self {
        self.config.max_attempts = max;
        self
    }

    /// Runs the extraction retry loop with the given agent function.
    ///
    /// The agent function receives a prompt string and returns the agent's text output
    /// (or error string). This abstraction allows any adapter to be used.
    ///
    /// Returns the validated JSON and metrics on success, or a typed error on failure.
    ///
    /// # Errors
    ///
    /// Returns `ExtractionError::MaxRetriesExceeded` if all retry attempts are exhausted.
    /// Returns `ExtractionError::SchemaError` if the schema is invalid.
    /// Returns `ExtractionError::AgentError` if the agent function returns an error.
    // Extraction retry loop is inherently complex with 5 stages (prompt, call, parse, validate, retry).
    // Splitting would fragment the state machine and reduce readability.
    #[allow(clippy::too_many_lines)]
    #[tracing::instrument(
        name = "extraction_orchestrator_extract",
        skip_all,
        fields(max_attempts = self.config.max_attempts)
    )]
    pub async fn extract<F, Fut>(
        &self,
        agent_fn: F,
        initial_prompt: String,
    ) -> Result<(Value, ExtractionMetrics), ExtractionError>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        let start = Instant::now();
        let mut attempt_history: Vec<AttemptRecord> = Vec::new();
        let mut total_input_chars: usize = 0;
        let mut total_output_chars: usize = 0;
        let mut current_prompt = initial_prompt.clone();

        // Validate schema compiles (early error if schema is invalid)
        let _validator = jsonschema::Validator::new(&self.schema)
            .map_err(|e| ExtractionError::SchemaError(e.to_string()))?;

        for attempt in 1..=self.config.max_attempts {
            // Track input chars for this attempt
            total_input_chars += current_prompt.chars().count();

            // Event 1: prompt_sent_to_agent
            tracing::debug!(
                event = "prompt_sent_to_agent",
                attempt = attempt,
                prompt_chars = current_prompt.chars().count(),
                "prompt_sent_to_agent"
            );

            // Call agent with current prompt
            let agent_output = match agent_fn(current_prompt.clone()).await {
                Ok(output) => output,
                Err(e) => {
                    tracing::warn!(
                        event = "extraction_outcome",
                        success = false,
                        total_attempts = attempt,
                        total_duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                        error_kind = "agent_error",
                        "extraction_outcome"
                    );
                    return Err(ExtractionError::AgentError(e));
                }
            };

            // Track output chars for this attempt
            total_output_chars += agent_output.chars().count();

            // Event 2: agent_response_received
            tracing::debug!(
                event = "agent_response_received",
                attempt = attempt,
                output_chars = agent_output.chars().count(),
                "agent_response_received"
            );

            // Try to parse JSON from agent output
            let parsed = match serde_json::from_str::<Value>(&agent_output) {
                Ok(value) => value,
                Err(e) => {
                    // Parse failure - record attempt with empty submitted_json
                    let error_msg = e.to_string();

                    // Event 3: validation_result (parse failure)
                    tracing::debug!(
                        event = "validation_result",
                        attempt = attempt,
                        valid = false,
                        parse_failure = true,
                        error_count = 1,
                        "validation_result"
                    );

                    attempt_history.push(AttemptRecord {
                        attempt_number: attempt,
                        submitted_json: Value::Null,
                        validation_errors: vec![format!("JSON parse error: {error_msg}")],
                        raw_agent_output: agent_output.clone(),
                        elapsed: start.elapsed(),
                    });

                    // If not the last attempt, build parse error feedback and continue
                    if attempt < self.config.max_attempts {
                        // Event 4: retry_decision
                        tracing::debug!(
                            event = "retry_decision",
                            attempt = attempt,
                            remaining_attempts = self.config.max_attempts - attempt,
                            will_retry = true,
                            "retry_decision"
                        );

                        let feedback = build_parse_error_feedback(
                            &agent_output,
                            &error_msg,
                            attempt,
                            self.config.max_attempts,
                            &self.schema,
                        );
                        current_prompt = format!("{current_prompt}\n\n{feedback}");
                        continue;
                    }

                    // Last attempt - fall through to MaxRetriesExceeded
                    break;
                }
            };

            // Validate parsed JSON against schema
            let errors = collect_validation_errors(&self.schema, &parsed);

            if errors.is_empty() {
                // Event 3: validation_result (success)
                tracing::debug!(
                    event = "validation_result",
                    attempt = attempt,
                    valid = true,
                    error_count = 0,
                    "validation_result"
                );

                // Event 5: extraction_outcome (success)
                tracing::info!(
                    event = "extraction_outcome",
                    success = true,
                    total_attempts = attempt,
                    total_duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "extraction_outcome"
                );

                // SUCCESS - build metrics and return
                let metrics = ExtractionMetrics {
                    total_attempts: attempt,
                    wall_time: start.elapsed(),
                    estimated_input_tokens: estimate_tokens(&current_prompt),
                    estimated_output_tokens: estimate_tokens(&agent_output),
                };
                return Ok((parsed, metrics));
            }

            // Event 3: validation_result (failure)
            tracing::debug!(
                event = "validation_result",
                attempt = attempt,
                valid = false,
                error_count = errors.len(),
                "validation_result"
            );

            // Validation failed - record attempt
            attempt_history.push(AttemptRecord {
                attempt_number: attempt,
                submitted_json: parsed.clone(),
                validation_errors: errors.clone(),
                raw_agent_output: agent_output.clone(),
                elapsed: start.elapsed(),
            });

            // If not the last attempt, build validation feedback and continue
            if attempt < self.config.max_attempts {
                // Event 4: retry_decision
                tracing::debug!(
                    event = "retry_decision",
                    attempt = attempt,
                    remaining_attempts = self.config.max_attempts - attempt,
                    will_retry = true,
                    "retry_decision"
                );

                let feedback = build_validation_feedback(
                    &self.schema,
                    &parsed,
                    &errors,
                    attempt,
                    self.config.max_attempts,
                );
                // Conversation continuation strategy
                current_prompt = format!("{current_prompt}\n\n{feedback}");
            }
        }

        // Event 5: extraction_outcome (max retries exceeded)
        tracing::warn!(
            event = "extraction_outcome",
            success = false,
            total_attempts = self.config.max_attempts,
            total_duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            "extraction_outcome"
        );

        // Max attempts exhausted - build final metrics and return error
        let metrics = ExtractionMetrics {
            total_attempts: self.config.max_attempts,
            wall_time: start.elapsed(),
            estimated_input_tokens: total_input_chars.saturating_div(4),
            estimated_output_tokens: total_output_chars.saturating_div(4),
        };

        Err(ExtractionError::MaxRetriesExceeded {
            attempts: self.config.max_attempts,
            max_attempts: self.config.max_attempts,
            history: attempt_history,
            raw_output: current_prompt,
            metrics,
        })
    }

    /// Convenience method that extracts and deserializes to a typed value.
    ///
    /// This calls `extract()` and then deserializes the resulting `Value` to `T`.
    ///
    /// # Errors
    ///
    /// Returns `ExtractionError::ParseError` if deserialization to `T` fails after
    /// successful schema validation.
    #[tracing::instrument(
        name = "extraction_orchestrator_extract_typed",
        skip_all,
        fields(max_attempts = self.config.max_attempts)
    )]
    pub async fn extract_typed<T, F, Fut>(
        &self,
        agent_fn: F,
        initial_prompt: String,
    ) -> Result<(T, ExtractionMetrics), ExtractionError>
    where
        T: serde::de::DeserializeOwned,
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        let (value, metrics) = self.extract(agent_fn, initial_prompt).await?;

        let typed = serde_json::from_value(value.clone()).map_err(|e| {
            ExtractionError::ParseError {
                message: format!("Deserialization to target type failed: {e}"),
                raw_text: serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
                attempt: metrics.total_attempts,
            }
        })?;

        Ok((typed, metrics))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_extract_emits_tracing_events() {
        // Test that tracing instrumentation doesn't break the happy path
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let orchestrator = ExtractionOrchestrator::new(schema).max_attempts(1);

        let agent_fn = |_prompt: String| async {
            Ok(r#"{"name": "test"}"#.to_string())
        };

        let result = orchestrator.extract(agent_fn, "initial prompt".to_string()).await;
        assert!(result.is_ok());
        let (parsed, metrics) = result.unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(metrics.total_attempts, 1);
    }

    #[tokio::test]
    async fn test_extract_retry_emits_tracing_events() {
        // Test that retry path with tracing works correctly
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let orchestrator = ExtractionOrchestrator::new(schema).max_attempts(2);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let agent_fn = move |_prompt: String| {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // First attempt: return invalid JSON (wrong type)
                    Ok(r#"{"name": 123}"#.to_string())
                } else {
                    // Second attempt: return valid JSON
                    Ok(r#"{"name": "fixed"}"#.to_string())
                }
            }
        };

        let result = orchestrator.extract(agent_fn, "initial prompt".to_string()).await;
        assert!(result.is_ok());
        let (parsed, metrics) = result.unwrap();
        assert_eq!(parsed["name"], "fixed");
        assert_eq!(metrics.total_attempts, 2);
    }

    #[tokio::test]
    async fn test_extract_agent_error_emits_tracing() {
        // Test that agent error path emits tracing correctly
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let orchestrator = ExtractionOrchestrator::new(schema).max_attempts(1);

        let agent_fn = |_prompt: String| async {
            Err("agent failed".to_string())
        };

        let result = orchestrator.extract(agent_fn, "initial prompt".to_string()).await;
        assert!(result.is_err());
        match result {
            Err(ExtractionError::AgentError(msg)) => {
                assert_eq!(msg, "agent failed");
            }
            _ => panic!("Expected AgentError"),
        }
    }

    #[tokio::test]
    async fn test_extraction_max_retries_complete_history() {
        // Test that MaxRetriesExceeded contains complete attempt history
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name", "age"]
        });

        let orchestrator = ExtractionOrchestrator::new(schema).max_attempts(3);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let agent_fn = move |_prompt: String| {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                // Return different invalid JSONs each attempt for history verification
                match count {
                    0 => Ok(r#"{"name": "test"}"#.to_string()), // Missing age
                    1 => Ok(r#"{"name": 123, "age": 25}"#.to_string()), // Wrong name type
                    _ => Ok(r#"{"name": "test", "age": -5}"#.to_string()), // Negative age
                }
            }
        };

        let result = orchestrator.extract(agent_fn, "initial".to_string()).await;

        match result {
            Err(ExtractionError::MaxRetriesExceeded {
                attempts,
                max_attempts,
                history,
                metrics,
                ..
            }) => {
                assert_eq!(attempts, 3, "Should report 3 attempts");
                assert_eq!(max_attempts, 3, "Should report max_attempts=3");
                assert_eq!(history.len(), 3, "History should contain all 3 attempts");

                // Verify each attempt record
                for (i, record) in history.iter().enumerate() {
                    assert_eq!(record.attempt_number, i + 1, "Attempt number mismatch");
                    assert!(!record.validation_errors.is_empty(), "Should have validation errors");
                    assert!(!record.raw_agent_output.is_empty(), "Should capture raw output");
                }

                // Verify metrics
                assert_eq!(metrics.total_attempts, 3);
                assert!(metrics.wall_time.as_millis() > 0);
            }
            Ok(_) => panic!("Expected MaxRetriesExceeded error"),
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_extraction_parse_failure_counts_against_budget() {
        // Test that JSON parse failures count against retry budget
        let schema = json!({"type": "object", "properties": {"x": {"type": "number"}}});

        let orchestrator = ExtractionOrchestrator::new(schema).max_attempts(2);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let agent_fn = move |_prompt: String| {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                match count {
                    0 => Ok("not valid json at all".to_string()), // Parse failure
                    _ => Ok(r#"{"x": "string"}"#.to_string()), // Valid JSON but wrong type
                }
            }
        };

        let result = orchestrator.extract(agent_fn, "initial".to_string()).await;

        match result {
            Err(ExtractionError::MaxRetriesExceeded { history, .. }) => {
                assert_eq!(history.len(), 2, "Both attempts should be recorded");

                // First attempt: parse failure
                assert!(
                    history[0].validation_errors[0].contains("JSON parse error"),
                    "First attempt should be parse error: {:?}",
                    history[0].validation_errors
                );
                assert_eq!(history[0].submitted_json, serde_json::Value::Null);

                // Second attempt: validation failure
                assert!(
                    !history[1].validation_errors[0].contains("JSON parse error"),
                    "Second attempt should be validation error"
                );
            }
            Ok(_) => panic!("Expected MaxRetriesExceeded"),
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }
}
