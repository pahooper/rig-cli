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
/// 4. Common install location fallbacks (platform-specific).
/// 5. Helpful error with install instructions.
///
/// # Errors
///
/// Returns `ClaudeError::ExecutableNotFound` when no valid executable can be
/// located.
pub fn discover_claude(explicit_path: Option<PathBuf>) -> Result<PathBuf, ClaudeError> {
    // 1. Explicit path
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(ClaudeError::ExecutableNotFound(format!(
            "Explicit path does not exist: {}",
            path.display()
        )));
    }

    // 2. Environment variable
    if let Ok(path_str) = std::env::var(CC_BIN_ENV_VAR) {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    // 3. PATH lookup (which handles .exe/.cmd on Windows automatically)
    if let Ok(path) = which("claude") {
        return Ok(path);
    }

    // 4. Common install locations
    for location in fallback_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 5. Helpful error with install instructions
    Err(ClaudeError::ExecutableNotFound(
        "claude not found. Install: npm install -g @anthropic-ai/claude-code\n\
         Searched: PATH, common npm install locations."
            .to_string(),
    ))
}

#[cfg(unix)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // npm global install locations on Unix
        locations.push(home.join(".npm/bin/claude"));
        locations.push(home.join(".local/bin/claude"));
    }
    // System-wide npm
    locations.push(PathBuf::from("/usr/local/bin/claude"));
    locations
}

#[cfg(windows)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Native installer location (preferred — actual .exe, not .cmd wrapper)
        locations.push(home.join(".local/bin/claude.exe"));
        // npm global install on Windows
        locations.push(home.join("AppData/Roaming/npm/claude.cmd"));
    }
    // Program Files npm
    locations.push(PathBuf::from(r"C:\Program Files\nodejs\claude.cmd"));
    locations
}
