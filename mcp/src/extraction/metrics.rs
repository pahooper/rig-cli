//! Metrics tracking and token estimation for extraction operations.

use std::time::Duration;

/// Metrics collected during an extraction operation.
#[derive(Debug, Clone, Default)]
pub struct ExtractionMetrics {
    /// Total number of attempts made.
    pub total_attempts: usize,
    /// Wall-clock time elapsed during extraction.
    pub wall_time: Duration,
    /// Estimated input tokens sent to agent.
    pub estimated_input_tokens: usize,
    /// Estimated output tokens received from agent.
    pub estimated_output_tokens: usize,
}

/// Estimate token count from text using the standard 4-chars-per-token heuristic.
///
/// Uses `chars().count()` to handle UTF-8 correctly (not `len()` which counts bytes).
/// Returns ceiling division to avoid underestimation.
///
/// # Examples
///
/// ```
/// use rig_mcp_server::extraction::estimate_tokens;
///
/// assert_eq!(estimate_tokens("hello"), 2);  // 5 chars / 4 = 1.25 -> 2
/// assert_eq!(estimate_tokens("hello world"), 3);  // 11 chars / 4 = 2.75 -> 3
/// ```
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    // Ceiling division using div_ceil
    text.chars().count().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars
    }

    #[test]
    fn test_estimate_tokens_utf8() {
        // UTF-8 characters: "你好" is 2 chars but 6 bytes
        assert_eq!(estimate_tokens("你好"), 1); // 2 chars / 4 = 0.5 -> 1
        assert_eq!(estimate_tokens("hello 世界"), 2); // 8 chars / 4 = 2
    }
}
