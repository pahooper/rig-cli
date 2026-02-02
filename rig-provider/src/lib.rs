#![deny(missing_docs)]
//! The Rig Provider crate acts as the central integration point for AI CLI adapters.
//! 
//! It implements the MCP (Model Context Protocol) server and bridges
//! adapters like Claude Code, Codex, and `OpenCode` into the Rig ecosystem.

/// Adapter implementations for various AI providers.
pub mod adapters;
/// Error types for the provider.
pub mod errors;
/// Session management for isolated execution environments.
pub mod sessions;
/// Setup and configuration logic.
pub mod setup;

// Re-export specific adapters for easier access
pub use adapters::claude::ClaudeModel;
pub use adapters::codex::CodexModel;
pub use adapters::opencode::OpenCodeModel;
/// Utility functions.
pub mod utils;
/// MCP tool agent builder for transparent CLI orchestration.
pub mod mcp_agent;

pub use mcp_agent::{CliAdapter, McpToolAgent, McpToolAgentBuilder, McpToolAgentResult, DEFAULT_WORKFLOW_TEMPLATE};
