//! # rig-cli
//!
//! Turn CLI-based AI agents into idiomatic Rig 0.29 providers.
//!
//! This crate provides a Rig-idiomatic facade over CLI-based AI agents (Claude Code, Codex, OpenCode),
//! allowing you to use them with the same patterns you use for cloud API providers like OpenAI or Anthropic.
//!
//! ## Example
//!
//! ```no_run
//! # use rig_cli::prelude::*;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client (discovers CLI automatically)
//! let client = rig_cli::claude::Client::new().await?;
//!
//! // Build an agent just like any Rig provider
//! let agent = client.agent("claude-sonnet-4")
//!     .preamble("You are a helpful assistant")
//!     .build();
//!
//! // Prompt the agent
//! let response = agent.prompt("Hello!").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `claude` (default): Enable Claude Code provider
//! - `codex` (default): Enable Codex provider
//! - `opencode` (default): Enable OpenCode provider

#![deny(missing_docs)]

/// Claude Code provider implementation.
#[cfg(feature = "claude")]
pub mod claude;

/// Codex provider implementation.
#[cfg(feature = "codex")]
pub mod codex;

/// OpenCode provider implementation.
#[cfg(feature = "opencode")]
pub mod opencode;

/// Shared client configuration.
pub mod config;

/// Public error types.
pub mod errors;

/// Shared response type.
pub mod response;

/// Commonly used types and traits.
pub mod prelude;
