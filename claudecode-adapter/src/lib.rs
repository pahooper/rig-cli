//! Rust adapter for driving the Claude Code CLI as a subprocess.
//!
//! This crate provides discovery, initialization, and execution of the
//! `claude` command-line tool, with support for streaming, structured
//! output, and graceful timeout handling.

/// Command-line argument construction for Claude CLI invocations.
pub mod cmd;
/// Discovery and resolution of the Claude CLI executable path.
pub mod discovery;
/// Error types returned by adapter operations.
pub mod error;
/// Initialization and capability probing of the Claude CLI.
pub mod init;
/// Subprocess execution with streaming, timeouts, and signal handling.
pub mod process;
/// Shared data types for configuration, results, and stream events.
pub mod types;

pub use discovery::{discover_claude, CC_BIN_ENV_VAR};
pub use error::ClaudeError;
pub use init::init;
pub use process::run_claude;
pub use types::*;

/// High-level client for the Claude Code CLI.
#[derive(Clone)]
pub struct ClaudeCli {
    /// Filesystem path to the `claude` executable.
    pub path: std::path::PathBuf,
    /// Feature capabilities detected during initialization.
    pub capabilities: types::Capabilities,
}

impl ClaudeCli {
    /// Creates a new `ClaudeCli` from a resolved path and detected capabilities.
    #[must_use]
    pub const fn new(path: std::path::PathBuf, capabilities: types::Capabilities) -> Self {
        Self { path, capabilities }
    }

    /// Runs a prompt through the Claude CLI and returns the complete result.
    ///
    /// # Errors
    ///
    /// Returns `ClaudeError` if the subprocess fails to spawn, times out,
    /// or exits with an I/O error.
    pub async fn run(
        &self,
        prompt: &str,
        config: &types::RunConfig,
    ) -> Result<types::RunResult, ClaudeError> {
        run_claude(&self.path, prompt, config, None).await
    }

    /// Runs a prompt with real-time streaming of events through the provided channel.
    ///
    /// # Errors
    ///
    /// Returns `ClaudeError` if the subprocess fails to spawn, times out,
    /// or exits with an I/O error.
    pub async fn stream(
        &self,
        prompt: &str,
        config: &types::RunConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, ClaudeError> {
        run_claude(&self.path, prompt, config, Some(sender)).await
    }
}
