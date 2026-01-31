use crate::error::OpenCodeError;
use std::path::PathBuf;
use which::which;

pub fn discover_opencode() -> Result<PathBuf, OpenCodeError> {
    which("opencode").map_err(OpenCodeError::from)
}
