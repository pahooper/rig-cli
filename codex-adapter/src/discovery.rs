use crate::error::CodexError;
use std::path::PathBuf;
use which::which;

pub fn discover_codex() -> Result<PathBuf, CodexError> {
    which("codex").map_err(CodexError::from)
}
