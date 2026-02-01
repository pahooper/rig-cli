//! Configuration for extraction retry behavior.

/// Configuration for extraction retry behavior.
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Maximum number of attempts before giving up (default: 3).
    pub max_attempts: usize,
    /// Whether to include the full schema in validation feedback (default: true).
    pub include_schema_in_feedback: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            include_schema_in_feedback: true,
        }
    }
}

impl ExtractionConfig {
    /// Set the maximum number of retry attempts.
    #[must_use]
    pub const fn with_max_attempts(mut self, max: usize) -> Self {
        self.max_attempts = max;
        self
    }

    /// Set whether to include the schema in validation feedback.
    #[must_use]
    pub const fn with_schema_in_feedback(mut self, include: bool) -> Self {
        self.include_schema_in_feedback = include;
        self
    }
}
