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
/// 3. (Windows only) Native `.exe` installer locations — avoids `.cmd` shim
///    which forces a `cmd.exe` intermediary that flashes a console window.
/// 4. `claude` resolved via `$PATH`.
/// 5. Common install location fallbacks (platform-specific).
/// 6. Helpful error with install instructions.
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

    // 3. Native installer locations (checked BEFORE PATH on Windows)
    //
    // On Windows, `which("claude")` finds `claude.cmd` (the npm shim) via
    // PATHEXT, which forces a `cmd.exe` intermediary — a console-subsystem
    // binary that can flash a visible window even with CREATE_NO_WINDOW.
    // Checking native `.exe` locations first avoids this entirely.
    #[cfg(windows)]
    for location in native_exe_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 4. PATH lookup (which handles .exe/.cmd on Windows automatically)
    if let Ok(path) = which("claude") {
        return Ok(path);
    }

    // 5. Remaining fallback install locations
    for location in fallback_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 6. Helpful error with install instructions
    Err(ClaudeError::ExecutableNotFound(
        "claude not found.\n\
         Install (native): curl -fsSL https://claude.ai/install.sh | sh\n\
         Install (npm):    npm install -g @anthropic-ai/claude-code\n\
         Searched: PATH, native installer locations, npm install locations."
            .to_string(),
    ))
}

#[cfg(unix)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Native installer location (preferred)
        locations.push(home.join(".local/bin/claude"));
        // npm global install fallbacks
        locations.push(home.join(".npm/bin/claude"));
    }
    // System-wide locations (native or npm)
    locations.push(PathBuf::from("/usr/local/bin/claude"));
    locations
}

/// Native `.exe` locations checked BEFORE PATH on Windows.
///
/// These avoid the `cmd.exe` intermediary that `.cmd` shims require,
/// which reduces console window flash (upstream Claude Code issue #28138).
#[cfg(windows)]
fn native_exe_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Native installer location
        locations.push(home.join(".local/bin/claude.exe"));
    }
    // Native installer (alternate location)
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        locations.push(PathBuf::from(&local).join("Programs/claude-code/claude.exe"));
    }
    locations
}

#[cfg(windows)]
fn fallback_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // npm global install on Windows
        locations.push(home.join("AppData/Roaming/npm/claude.cmd"));
    }
    // Program Files npm
    locations.push(PathBuf::from(r"C:\Program Files\nodejs\claude.cmd"));
    locations
}
