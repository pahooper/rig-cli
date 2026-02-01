//! Locates the Codex CLI binary on the host system.

use crate::error::CodexError;
use std::path::PathBuf;
use which::which;

/// Searches `PATH` for the `codex` executable and returns its location.
///
/// # Errors
/// Returns [`CodexError::WhichError`] if the binary cannot be found.
pub fn discover_codex() -> Result<PathBuf, CodexError> {
    which("codex").map_err(CodexError::from)
}
