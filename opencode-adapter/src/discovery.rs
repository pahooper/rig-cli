//! Locates the `OpenCode` binary on the system.

use crate::error::OpenCodeError;
use std::path::PathBuf;
use which::which;

/// Environment variable that overrides the default OpenCode CLI binary path.
pub const OPENCODE_BIN_ENV_VAR: &str = "OPENCODE_ADAPTER_BIN";

/// Locates the OpenCode CLI executable.
///
/// Resolution order:
/// 1. `explicit_path` if provided and the file exists.
/// 2. The path in the `OPENCODE_ADAPTER_BIN` environment variable.
/// 3. `opencode` resolved via `$PATH`.
/// 4. Common install location fallbacks (platform-specific).
/// 5. Helpful error with install instructions.
///
/// # Errors
///
/// Returns `OpenCodeError::ExecutableNotFound` when no valid executable can be
/// located.
pub fn discover_opencode(explicit_path: Option<PathBuf>) -> Result<PathBuf, OpenCodeError> {
    // 1. Explicit path
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(OpenCodeError::ExecutableNotFound(format!(
            "Explicit path does not exist: {}",
            path.display()
        )));
    }

    // 2. Environment variable
    if let Ok(path_str) = std::env::var(OPENCODE_BIN_ENV_VAR) {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    // 3. PATH lookup
    if let Ok(path) = which("opencode") {
        return Ok(path);
    }

    // 4. Common install locations
    for location in fallback_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 5. Helpful error
    Err(OpenCodeError::ExecutableNotFound(
        "opencode not found. Install: go install github.com/opencode-ai/opencode@latest\n\
         Searched: PATH, common install locations.".to_string()
    ))
}

#[cfg(unix)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Go binary install location
        locations.push(home.join("go/bin/opencode"));
        locations.push(home.join(".local/bin/opencode"));
    }
    locations.push(PathBuf::from("/usr/local/bin/opencode"));
    locations
}

#[cfg(windows)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Go binary install location on Windows
        locations.push(home.join("go/bin/opencode.exe"));
    }
    locations.push(PathBuf::from(r"C:\Program Files\Go\bin\opencode.exe"));
    locations
}
