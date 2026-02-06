//! # rig-cli
//!
//! Turn CLI-based AI agents into idiomatic Rig 0.29 providers.
//!
//! This crate provides a Rig-idiomatic facade over CLI-based AI agents (`Claude Code`, `Codex`, `OpenCode`),
//! allowing you to use them with the same patterns you use for cloud API providers like `OpenAI` or Anthropic.
//!
//! ## Quick Start
//!
//! ```no_run
//! use rig_cli::prelude::*;
//!
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
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `claude` | Yes | Enable Claude Code provider |
//! | `codex` | Yes | Enable Codex provider |
//! | `opencode` | Yes | Enable `OpenCode` provider |
//! | `debug-output` | No | Include raw CLI output in error messages |
//!
//! Enable specific providers:
//!
//! ```toml
//! [dependencies]
//! rig-cli = { version = "0.1", default-features = false, features = ["claude"] }
//! ```
//!
//! ## Module Overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`claude`] | Claude Code provider with `CompletionModel` |
//! | [`codex`] | Codex provider with `CompletionModel` |
//! | [`opencode`] | `OpenCode` provider with `CompletionModel` |
//! | [`extraction`] | MCP extraction types (re-exported from rig-mcp-server) |
//! | [`tools`] | MCP tool types (re-exported from rig-mcp-server) |
//! | [`prelude`] | Common imports for quick start |
//! | [`config`] | Shared client configuration |
//! | [`errors`] | Public error types |
//! | [`response`] | Shared response type |
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
//! **Decision tree:**
//!
//! ```text
//! Need structured output? ─── Yes ──> mcp_agent()
//!         │
//!         No
//!         │
//!         └─> agent()
//! ```
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
//! use rig_cli::tools::JsonSchemaToolkit;
//! use rig_cli::extraction::ExtractionOrchestrator;
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
//! ```
//!
//! ## Adapter Comparison
//!
//! | Feature | Claude Code | Codex | OpenCode |
//! |---------|-------------|-------|----------|
//! | MCP support | Yes | Yes | Yes |
//! | Streaming events | Full (ToolCall/ToolResult) | Text/Error only | Text/Error only |
//! | Sandbox | `--tools ""` | `--sandbox` | None |
//! | System prompt | `--system-prompt` | Prepend | Prepend |

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

/// Unified CLI binary discovery across all adapters.
pub mod discovery;

/// Public error types.
pub mod errors;

/// Shared response type.
pub mod response;

/// Commonly used types and traits.
pub mod prelude;

// Re-export the Rig crate so users can access Rig types via rig_cli::rig::...
pub use rig;

// MCP-enforced agent types (from rig-provider)
pub use rig_cli_provider::mcp_agent::{
    CliAdapter, CliAgent, CliAgentBuilder, McpStreamEvent, McpStreamHandle, McpToolAgent,
    McpToolAgentBuilder,
};

/// Re-export of MCP extraction types for structured data extraction workflows.
///
/// These types enable building MCP-enforced extraction pipelines that guarantee
/// schema-compliant output from CLI agents.
pub mod extraction {
    pub use rig_cli_mcp::extraction::{
        ExtractionConfig, ExtractionError, ExtractionMetrics, ExtractionOrchestrator,
    };
}

/// Re-export of MCP tool types for building tool-based extraction workflows.
///
/// These types provide the building blocks for creating JSON schema-based toolkits
/// and configuring MCP servers for structured agent execution.
pub mod tools {
    pub use rig_cli_mcp::server::{McpConfig, RigMcpHandler, ToolSetExt};
    pub use rig_cli_mcp::tools::{DynamicJsonSchemaToolkit, JsonSchemaToolkit};
}
