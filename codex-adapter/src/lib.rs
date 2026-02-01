//! Rust adapter for `OpenAI` Codex CLI subprocess execution.
//!
//! Provides types, discovery, and process management for invoking the
//! Codex CLI as a child process with streaming and timeout support.

/// Command-line argument building utilities.
pub mod cmd;
/// Codex binary discovery on the host system.
pub mod discovery;
/// Error types for the adapter.
pub mod error;
/// Subprocess execution and lifecycle management.
pub mod process;
/// Shared configuration and result types.
pub mod types;

use tokio::process::Command;

pub use discovery::discover_codex;
pub use error::CodexError;
pub use process::run_codex;
pub use types::*;

/// High-level handle for the Codex CLI binary.
#[derive(Clone)]
pub struct CodexCli {
    /// Filesystem path to the Codex executable.
    pub path: std::path::PathBuf,
}

impl CodexCli {
    /// Creates a new `CodexCli` pointing at the given binary path.
    #[must_use]
    pub const fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    /// Checks if the Codex CLI is working correctly.
    ///
    /// # Errors
    /// Returns an error if the binary cannot be executed or fails its own health check.
    pub async fn check_health(&self) -> Result<(), CodexError> {
        let output = Command::new(&self.path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| CodexError::SpawnFailed {
                stage: "health check".to_string(),
                source: e,
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(CodexError::ExecutableNotFound(
                "Codex health check failed".to_string(),
            ))
        }
    }

    /// Runs the Codex CLI to completion and returns the result.
    ///
    /// # Errors
    /// Returns an error if the process fails to spawn, times out, or produces truncated output.
    pub async fn run(
        &self,
        prompt: &str,
        config: &types::CodexConfig,
    ) -> Result<types::RunResult, CodexError> {
        run_codex(&self.path, prompt, config, None).await
    }

    /// Runs the Codex CLI, streaming events through `sender` as they arrive.
    ///
    /// # Errors
    /// Returns an error if the process fails to spawn, times out, or produces truncated output.
    pub async fn stream(
        &self,
        prompt: &str,
        config: &types::CodexConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, CodexError> {
        run_codex(&self.path, prompt, config, Some(sender)).await
    }
}
