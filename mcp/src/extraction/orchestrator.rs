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

            // Call agent with current prompt
            let agent_output = agent_fn(current_prompt.clone())
                .await
                .map_err(ExtractionError::AgentError)?;

            // Track output chars for this attempt
            total_output_chars += agent_output.chars().count();

            // Try to parse JSON from agent output
            let parsed = match serde_json::from_str::<Value>(&agent_output) {
                Ok(value) => value,
                Err(e) => {
                    // Parse failure - record attempt with empty submitted_json
                    let error_msg = e.to_string();
                    attempt_history.push(AttemptRecord {
                        attempt_number: attempt,
                        submitted_json: Value::Null,
                        validation_errors: vec![format!("JSON parse error: {error_msg}")],
                        raw_agent_output: agent_output.clone(),
                        elapsed: start.elapsed(),
                    });

                    // If not the last attempt, build parse error feedback and continue
                    if attempt < self.config.max_attempts {
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
                // SUCCESS - build metrics and return
                let metrics = ExtractionMetrics {
                    total_attempts: attempt,
                    wall_time: start.elapsed(),
                    estimated_input_tokens: estimate_tokens(&current_prompt),
                    estimated_output_tokens: estimate_tokens(&agent_output),
                };
                return Ok((parsed, metrics));
            }

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
