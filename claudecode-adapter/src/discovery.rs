//! Discovery and resolution of the Claude CLI executable path.

use crate::error::ClaudeError;
use std::path::PathBuf;
use which::which;

/// Environment variable that overrides the default Claude CLI binary path.
pub const CC_BIN_ENV_VAR: &str = "CC_ADAPTER_CLAUDE_BIN";

/// Locates the Claude CLI executable.
///
/// Resolution order:
/// 1. `explicit_path` if provided and the file exists.
/// 2. The path in the `CC_ADAPTER_CLAUDE_BIN` environment variable.
/// 3. `claude` resolved via `$PATH`.
///
/// # Errors
///
/// Returns `ClaudeError::ExecutableNotFound` when no valid executable can be
/// located.
pub fn discover_claude(explicit_path: Option<PathBuf>) -> Result<PathBuf, ClaudeError> {
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(ClaudeError::ExecutableNotFound(format!(
            "Explicit path does not exist: {}",
            path.display()
        )));
    }

    if let Ok(path_str) = std::env::var(CC_BIN_ENV_VAR) {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    which("claude").map_err(|e| ClaudeError::ExecutableNotFound(e.to_string()))
}
