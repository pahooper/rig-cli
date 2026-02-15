//! Common imports for rig-cli usage.
//!
//! ```
//! use rig_cli::prelude::*;
//! ```
//!
//! This module re-exports the most commonly used types and traits for typical
//! rig-cli usage patterns, including client types, error handling, Rig traits,
//! and MCP types for structured extraction workflows.

// Client types (feature-gated)
#[cfg(feature = "claude")]
pub use crate::claude::Client as ClaudeClient;
#[cfg(feature = "codex")]
pub use crate::codex::Client as CodexClient;
#[cfg(feature = "opencode")]
pub use crate::opencode::Client as OpenCodeClient;

// Error type (always available)
pub use crate::errors::Error;

// Re-export key Rig traits so users don't need separate rig import
pub use rig::client::CompletionClient;
pub use rig::completion::Chat;
pub use rig::completion::Prompt;

// Re-export key MCP types for structured extraction workflows
// These are the types users need to build ToolSets for extraction
pub use rig_cli_mcp::extraction::{ExtractionConfig, ExtractionOrchestrator};
pub use rig_cli_mcp::tools::{DynamicJsonSchemaToolkit, JsonSchemaToolkit};
