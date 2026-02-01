#![warn(clippy::pedantic)]
pub mod cmd;
pub mod discovery;
pub mod error;
pub mod process;
pub mod types;

use tokio::process::Command;

pub use discovery::discover_codex;
pub use error::CodexError;
pub use process::run_codex;
pub use types::*;

#[derive(Clone)]
pub struct CodexCli {
    pub path: std::path::PathBuf,
}

impl CodexCli {
    pub fn new(path: std::path::PathBuf) -> Self {
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

    pub async fn run(
        &self,
        prompt: &str,
        config: &types::CodexConfig,
    ) -> Result<types::RunResult, CodexError> {
        run_codex(&self.path, prompt, config, None).await
    }

    pub async fn stream(
        &self,
        prompt: &str,
        config: &types::CodexConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, CodexError> {
        run_codex(&self.path, prompt, config, Some(sender)).await
    }
}
