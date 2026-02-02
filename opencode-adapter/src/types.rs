//! Shared types for `OpenCode` adapter configuration and results.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for an `OpenCode` CLI invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    /// LLM model name override (e.g. `gpt-4o`).
    pub model: Option<String>,
    /// System prompt override.
    pub prompt: Option<String>,
    /// Whether to pass `--print-logs` to the CLI.
    pub print_logs: bool,
    /// Log verbosity level (e.g. `debug`, `info`).
    pub log_level: Option<String>,
    /// Optional port for the `OpenCode` server.
    pub port: Option<u16>,
    /// Optional hostname for the `OpenCode` server.
    pub hostname: Option<String>,
    /// Extra environment variables passed to the subprocess.
    pub env_vars: Vec<(String, String)>,
    /// Path to an MCP configuration JSON file (`OpenCode` format).
    pub mcp_config_path: Option<PathBuf>,
    /// Maximum wall-clock time before the process is killed.
    pub timeout: Duration,
    /// Working directory for the child process.
    pub cwd: Option<PathBuf>,
}

impl Default for OpenCodeConfig {
    fn default() -> Self {
        Self {
            model: None,
            prompt: None,
            print_logs: false,
            log_level: None,
            port: None,
            hostname: None,
            env_vars: Vec::new(),
            mcp_config_path: None,
            timeout: Duration::from_secs(300),
            cwd: None,
        }
    }
}

/// Captured result of a completed `OpenCode` run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Full stdout output.
    pub stdout: String,
    /// Full stderr output.
    pub stderr: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// Events streamed from the `OpenCode` CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// A chunk of text content.
    Text {
        /// The text content.
        text: String,
    },
    /// An error event.
    Error {
        /// The error message.
        message: String,
    },
    /// An unknown event type.
    Unknown(
        /// The raw JSON value.
        serde_json::Value,
    ),
}
