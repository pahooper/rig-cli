//! Error types for extraction operations with attempt history tracking.

use std::time::Duration;
use thiserror::Error;

use super::metrics::ExtractionMetrics;

/// Record of a single extraction attempt including submission and validation errors.
#[derive(Debug, Clone)]
pub struct AttemptRecord {
    /// The attempt number (1-indexed).
    pub attempt_number: usize,
    /// The JSON submitted during this attempt.
    pub submitted_json: serde_json::Value,
    /// Validation error messages from this attempt.
    pub validation_errors: Vec<String>,
    /// Raw agent output text (not just parsed JSON).
    pub raw_agent_output: String,
    /// Elapsed time at this attempt.
    pub elapsed: Duration,
}

/// Errors that can occur during extraction operations.
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// Maximum retry attempts exceeded - extraction failed.
    #[error("Extraction failed after {attempts} attempts (max: {max_attempts})")]
    MaxRetriesExceeded {
        /// Number of attempts made.
        attempts: usize,
        /// Maximum attempts allowed.
        max_attempts: usize,
        /// History of all attempts with their validation errors.
        history: Vec<AttemptRecord>,
        /// Raw agent output text for debugging.
        raw_output: String,
        /// Metrics tracked across all attempts.
        metrics: ExtractionMetrics,
    },

    /// JSON parsing failed - agent output was not valid JSON.
    #[error("JSON parsing failed at attempt {attempt}: {message}")]
    ParseError {
        /// Parse error message.
        message: String,
        /// Raw text that failed to parse.
        raw_text: String,
        /// Attempt number where parse failure occurred.
        attempt: usize,
    },

    /// Schema compilation or validation setup failed.
    #[error("Schema error: {0}")]
    SchemaError(String),

    /// Agent execution itself failed (CLI error, timeout, etc.).
    #[error("Agent execution failed: {0}")]
    AgentError(String),

    /// Callback rejection - schema-valid JSON was rejected by business logic.
    #[error("Callback rejected submission at attempt {attempt}: {reason}")]
    CallbackRejection {
        /// Reason for rejection.
        reason: String,
        /// Attempt number where rejection occurred.
        attempt: usize,
    },
}
