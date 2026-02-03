//! # rig-cli
//!
//! Turn CLI-based AI agents into idiomatic Rig 0.29 providers.
//!
//! This crate provides a Rig-idiomatic facade over CLI-based AI agents (Claude Code, Codex, OpenCode),
//! allowing you to use them with the same patterns you use for cloud API providers like OpenAI or Anthropic.
//!
//! ## Quick Start
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
//! ## Two Execution Paths
//!
//! rig-cli provides two ways to interact with CLI agents:
//!
//! | Method | When to Use |
//! |--------|-------------|
//! | `client.agent("model")` | Simple prompts, chat, streaming - direct CLI execution |
//! | `client.mcp_agent("model")` | Structured extraction - MCP-enforced tool use |
//!
//! For structured data extraction where the agent MUST respond via tool calls
//! (not freeform text), use `mcp_agent()`:
//!
//! ```ignore
//! let client = rig_cli::claude::Client::new().await?;
//! let agent = client.mcp_agent("sonnet")
//!     .toolset(my_tools)  // Your ToolSet here
//!     .build()?;
//! let result = agent.prompt("Extract data...").await?;
//! ```
//!
//! ## Structured Extraction with MCP
//!
//! For structured data extraction, use the MCP-enforced extraction pattern:
//!
//! ```ignore
//! use rig_cli::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Deserialize, Serialize)]
//! struct PersonInfo {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Create toolkit from your schema
//! let toolkit = JsonSchemaToolkit::from_type::<PersonInfo>()?;
//!
//! // Build MCP orchestrator
//! let orchestrator = ExtractionOrchestrator::builder()
//!     .with_toolkit(toolkit)
//!     .build();
//!
//! // Create client and extract
//! let client = rig_cli::claude::Client::new().await?;
//! let result = orchestrator
//!     .extract::<PersonInfo>(
//!         &client.agent("claude-sonnet-4").build(),
//!         "Extract person info: Alice is 30 years old"
//!     )
//!     .await?;
//!
//! println!("{:?}", result);
//! ```
//!
//! ## Feature Flags
//!
//! - `claude` (default): Enable Claude Code provider
//! - `codex` (default): Enable Codex provider
//! - `opencode` (default): Enable OpenCode provider
//! - `debug-output` (opt-in): Include raw CLI output in error messages for debugging

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

// Re-export the Rig crate so users can access Rig types via rig_cli::rig::...
pub use rig;

// MCP-enforced agent types (from rig-provider)
pub use rig_provider::mcp_agent::{CliAgent, CliAgentBuilder, CliAdapter, McpStreamEvent};

/// Re-export of MCP extraction types for structured data extraction workflows.
///
/// These types enable building MCP-enforced extraction pipelines that guarantee
/// schema-compliant output from CLI agents.
pub mod extraction {
    pub use rig_mcp_server::extraction::{
        ExtractionConfig, ExtractionError, ExtractionMetrics, ExtractionOrchestrator,
    };
}

/// Re-export of MCP tool types for building tool-based extraction workflows.
///
/// These types provide the building blocks for creating JSON schema-based toolkits
/// and configuring MCP servers for structured agent execution.
pub mod tools {
    pub use rig_mcp_server::server::{McpConfig, RigMcpHandler, ToolSetExt};
    pub use rig_mcp_server::tools::JsonSchemaToolkit;
}
