//! Shared response type for CLI agent execution.

use serde::{Deserialize, Serialize};

/// Response from a CLI agent execution.
///
/// This is rig-cli's response type, not an adapter internal.
/// Access raw adapter output via `.raw_output` for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    /// The text content of the agent's response.
    pub text: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl CliResponse {
    /// Creates a new `CliResponse` from adapter-internal types.
    #[must_use]
    pub fn from_run_result(stdout: String, exit_code: i32, duration_ms: u64) -> Self {
        Self {
            text: stdout,
            exit_code,
            duration_ms,
        }
    }
}
