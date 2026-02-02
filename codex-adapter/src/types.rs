//! Shared configuration, result, and streaming types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Sandbox isolation level for the Codex subprocess.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxMode {
    /// Read-only filesystem access.
    ReadOnly,
    /// Write access limited to the workspace directory.
    WorkspaceWrite,
    /// Unrestricted filesystem access (use with caution).
    DangerFullAccess,
}

/// Configuration for a Codex CLI invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    /// Model identifier to use (e.g. `"o4-mini"`).
    pub model: Option<String>,
    /// Sandbox isolation level.
    pub sandbox: Option<SandboxMode>,
    /// Enable full-auto mode (no approval prompts).
    pub full_auto: bool,
    /// Enable web search capability.
    pub search: bool,
    /// Working directory for the subprocess.
    pub cd: Option<PathBuf>,
    /// Skip git repository check (needed when running in temp dirs).
    pub skip_git_repo_check: bool,
    /// Additional directories to expose to the subprocess.
    pub add_dirs: Vec<PathBuf>,
    /// Key-value config overrides passed via `--config`.
    pub overrides: Vec<(String, String)>,
    /// System prompt to append to the agent's instructions.
    pub system_prompt: Option<String>,
    /// Extra environment variables passed to the subprocess.
    pub env_vars: Vec<(String, String)>,
    /// Path to an MCP configuration file (TOML format for Codex).
    pub mcp_config_path: Option<std::path::PathBuf>,
    /// Maximum wall-clock time before the subprocess is killed.
    pub timeout: Duration,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            model: None,
            sandbox: None,
            full_auto: false,
            search: false,
            skip_git_repo_check: false,
            cd: None,
            add_dirs: Vec::new(),
            overrides: Vec::new(),
            system_prompt: None,
            env_vars: Vec::new(),
            mcp_config_path: None,
            timeout: Duration::from_secs(300),
        }
    }
}

/// Outcome of a completed Codex CLI run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Process exit code (`-1` if unavailable).
    pub exit_code: i32,
    /// Wall-clock duration of the run in milliseconds.
    pub duration_ms: u64,
}

/// Incremental event emitted while streaming Codex output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// A chunk of text output.
    Text {
        /// The text content.
        text: String,
    },
    /// An error message from the subprocess.
    Error {
        /// The error description.
        message: String,
    },
    /// An unrecognised JSON value.
    Unknown(serde_json::Value),
}
