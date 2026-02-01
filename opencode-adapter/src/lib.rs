//! Adapter crate for running the `OpenCode` CLI as a subprocess.
//!
//! Provides discovery, configuration, health checks, and streaming
//! execution of the `OpenCode` binary from Rust.

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

/// High-level handle for the `OpenCode` CLI binary.
#[derive(Clone)]
pub struct OpenCodeCli {
    /// Filesystem path to the `OpenCode` executable.
    pub path: std::path::PathBuf,
}

impl OpenCodeCli {
    /// Creates a new handle pointing at the given binary path.
    #[must_use]
    pub const fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    /// Runs `--version` to verify the binary is functional.
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
                source: std::io::Error::other(
                    "OpenCode health check failed",
                ),
            })
        }
    }

    /// Runs `OpenCode` to completion and returns the full result.
    pub async fn run(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, None).await
    }

    /// Runs `OpenCode` while streaming events through `sender`.
    pub async fn stream(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, Some(sender)).await
    }
}
