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
        let output = Command::new(&self.path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| OpenCodeError::SpawnFailed {
                stage: "health check".to_string(),
                source: e,
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(OpenCodeError::SpawnFailed {
                stage: "health check validation".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "OpenCode health check failed",
                ),
            })
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
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, Some(sender)).await
    }
}
