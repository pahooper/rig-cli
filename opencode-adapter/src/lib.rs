//! Adapter crate for running the `OpenCode` CLI as a subprocess.
//!
//! This crate provides a Rust interface for executing the `OpenCode` CLI tool,
//! with streaming output, timeout handling, and graceful shutdown support.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use opencode_adapter::{discover_opencode, OpenCodeCli, OpenCodeConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Discover and validate CLI
//!     let path = discover_opencode(None)?;
//!     let cli = OpenCodeCli::new(path);
//!     cli.check_health().await?;
//!
//!     // Configure and run
//!     let config = OpenCodeConfig {
//!         model: Some("opencode/big-pickle".to_string()),
//!         timeout: Duration::from_secs(120),
//!         ..OpenCodeConfig::default()
//!     };
//!
//!     let result = cli.run("What is 2 + 2?", &config).await?;
//!     println!("Output: {}", result.stdout);
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The adapter follows the same structure as `claudecode-adapter` and `codex-adapter`:
//!
//! - **Discovery** ([`discover_opencode`]): Locates the CLI binary via PATH, env var, or fallbacks
//! - **Configuration** ([`OpenCodeConfig`]): Typed config for model, timeout, working directory
//! - **Execution** ([`run_opencode`]): Spawns subprocess with bounded output and timeout
//! - **Streaming** ([`OpenCodeCli::stream`]): Real-time event streaming via channels
//! - **Errors** ([`OpenCodeError`]): Rich error types with context (PID, elapsed time, partial output)
//!
//! ## Containment
//!
//! Unlike Claude Code and Codex, `OpenCode` has no CLI flags for sandboxing or tool restriction.
//! Containment is achieved through:
//!
//! - **Working directory**: Set via [`OpenCodeConfig::cwd`], passed to `Command::current_dir()`
//! - **MCP configuration**: Set via [`OpenCodeConfig::mcp_config_path`], passed as `OPENCODE_CONFIG` env var
//! - **System prompt**: Set via [`OpenCodeConfig::prompt`], prepended to the user message
//!
//! ## Process Lifecycle
//!
//! The adapter implements production-grade subprocess management:
//!
//! 1. **Bounded channels**: 100-message capacity prevents memory exhaustion
//! 2. **Output limits**: 10MB cap with [`OpenCodeError::OutputTruncated`] on overflow
//! 3. **Graceful shutdown**: SIGTERM with 5-second grace period, then SIGKILL
//! 4. **Task cleanup**: `JoinSet` ensures all async tasks complete or abort
//!
//! ## Feature Parity
//!
//! This adapter is production-hardened to the same standards as `claudecode-adapter`
//! and `codex-adapter`. All three adapters share:
//!
//! - Identical error handling patterns
//! - Same timeout and shutdown behavior
//! - Equivalent test coverage (unit + E2E)
//! - Zero clippy pedantic warnings

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
    ///
    /// # Errors
    ///
    /// Returns `OpenCodeError::SpawnFailed` if the version check cannot be executed
    /// or if the process exits with non-zero status.
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
    ///
    /// # Errors
    ///
    /// Returns `OpenCodeError` if the `OpenCode` process fails to spawn, stream capture
    /// fails, or the process exits with non-zero status. See [`run_opencode`] for details.
    pub async fn run(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, None).await
    }

    /// Runs `OpenCode` while streaming events through `sender`.
    ///
    /// # Errors
    ///
    /// Returns `OpenCodeError` if the `OpenCode` process fails to spawn, stream capture
    /// fails, or the process exits with non-zero status. See [`run_opencode`] for details.
    pub async fn stream(
        &self,
        message: &str,
        config: &types::OpenCodeConfig,
        sender: tokio::sync::mpsc::Sender<types::StreamEvent>,
    ) -> Result<types::RunResult, OpenCodeError> {
        run_opencode(&self.path, message, config, Some(sender)).await
    }
}
