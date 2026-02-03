//! Locates the Codex CLI binary on the host system.

use crate::error::CodexError;
use std::path::PathBuf;
use which::which;

/// Environment variable that overrides the default Codex CLI binary path.
pub const CODEX_BIN_ENV_VAR: &str = "CODEX_ADAPTER_BIN";

/// Locates the Codex CLI executable.
///
/// Resolution order:
/// 1. `explicit_path` if provided and the file exists.
/// 2. The path in the `CODEX_ADAPTER_BIN` environment variable.
/// 3. `codex` resolved via `$PATH`.
/// 4. Common install location fallbacks (platform-specific).
/// 5. Helpful error with install instructions.
///
/// # Errors
///
/// Returns `CodexError::ExecutableNotFound` when no valid executable can be
/// located.
pub fn discover_codex(explicit_path: Option<PathBuf>) -> Result<PathBuf, CodexError> {
    // 1. Explicit path
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(CodexError::ExecutableNotFound(format!(
            "Explicit path does not exist: {}",
            path.display()
        )));
    }

    // 2. Environment variable
    if let Ok(path_str) = std::env::var(CODEX_BIN_ENV_VAR) {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    // 3. PATH lookup
    if let Ok(path) = which("codex") {
        return Ok(path);
    }

    // 4. Common install locations
    for location in fallback_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 5. Helpful error
    Err(CodexError::ExecutableNotFound(
        "codex not found. Install: npm install -g @openai/codex\n\
         Searched: PATH, common npm install locations.".to_string()
    ))
}

#[cfg(unix)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        locations.push(home.join(".npm/bin/codex"));
        locations.push(home.join(".local/bin/codex"));
    }
    locations.push(PathBuf::from("/usr/local/bin/codex"));
    locations
}

#[cfg(windows)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        locations.push(home.join("AppData/Roaming/npm/codex.cmd"));
    }
    locations.push(PathBuf::from(r"C:\Program Files\nodejs\codex.cmd"));
    locations
}
