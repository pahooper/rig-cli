#![warn(clippy::pedantic)]
pub mod cmd;
pub mod discovery;
pub mod error;
pub mod process;
pub mod types;

use tokio::process::Command;

pub use discovery::discover_opencode;
pub use error::OpenCodeError;
pub use process::run_opencode;
pub use types::*;

#[derive(Clone)]
pub struct OpenCodeCli {
    pub path: std::path::PathBuf,
}

impl OpenCodeCli {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    /// Checks if the OpenCode CLI is working correctly.
    ///
    /// # Errors
    /// Returns an error if the binary cannot be executed or fails its own health check.
    pub async fn check_health(&self) -> Result<(), OpenCodeError> {
        let output = Command::new(&self.path).arg("--version").output().await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(OpenCodeError::Other(
                "OpenCode health check failed".to_string(),
            ))
        }
    }

    pub async fn run(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, None).await
    }

    pub async fn stream(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
        sender: tokio::sync::mpsc::UnboundedSender<types::StreamEvent>,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, Some(sender)).await
    }
}
