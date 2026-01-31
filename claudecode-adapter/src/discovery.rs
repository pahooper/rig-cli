use crate::error::ClaudeError;
use std::path::PathBuf;
use which::which;

pub const CC_BIN_ENV_VAR: &str = "CC_ADAPTER_CLAUDE_BIN";

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
