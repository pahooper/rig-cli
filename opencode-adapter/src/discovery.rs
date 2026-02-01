//! Locates the `OpenCode` binary on the system `PATH`.

use crate::error::OpenCodeError;
use std::path::PathBuf;
use which::which;

/// Searches `PATH` for an `opencode` executable and returns its location.
pub fn discover_opencode() -> Result<PathBuf, OpenCodeError> {
    which("opencode").map_err(OpenCodeError::from)
}
