#![warn(clippy::pedantic)]
pub mod cmd;
pub mod discovery;
pub mod error;
pub mod init;
pub mod process;
pub mod types;

pub use discovery::{discover_claude, CC_BIN_ENV_VAR};
pub use error::ClaudeError;
pub use init::init;
pub use process::run_claude;
pub use types::*;

#[derive(Clone)]
pub struct ClaudeCli {
    pub path: std::path::PathBuf,
    pub capabilities: types::Capabilities,
}

impl ClaudeCli {
    pub fn new(path: std::path::PathBuf, capabilities: types::Capabilities) -> Self {
        Self { path, capabilities }
    }

    pub async fn run(
        &self,
        prompt: &str,
        config: &types::RunConfig,
    ) -> Result<types::RunResult, ClaudeError> {
        run_claude(&self.path, prompt, config, None).await
    }

    pub async fn stream(
        &self,
        prompt: &str,
        config: &types::RunConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, ClaudeError> {
        run_claude(&self.path, prompt, config, Some(sender)).await
    }
}
