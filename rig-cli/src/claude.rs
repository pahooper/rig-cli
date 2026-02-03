//! Claude Code provider with CompletionModel and MCP-enforced CliAgent.
//!
//! This module provides two execution paths for the Claude Code CLI:
//!
//! | Method | Execution | Use Case |
//! |--------|-----------|----------|
//! | `client.agent("model")` | Direct CLI | Simple prompts, chat, streaming |
//! | `client.mcp_agent("model")` | MCP Server | Structured extraction, forced tool use |
//!
//! # Simple Prompts (Direct CLI)
//!
//! ```no_run
//! # use rig_cli::claude::Client;
//! # use rig::client::CompletionClient;
//! # use rig::completion::Prompt;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new().await?;
//! let agent = client.agent("claude-sonnet-4")
//!     .preamble("You are a helpful assistant")
//!     .build();
//! let response = agent.prompt("Hello!").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Structured Extraction (MCP-Enforced)
//!
//! ```ignore
//! use rig_cli::claude::Client;
//! use rig::completion::Prompt;
//!
//! let client = Client::new().await?;
//! let agent = client.mcp_agent("sonnet")
//!     .toolset(extraction_tools)  // Your ToolSet here
//!     .preamble("Extract structured data")
//!     .build()?;
//! let result = agent.prompt("Extract from: ...").await?;
//! ```
//!
//! For MCP enforcement, the agent is constrained to submit responses ONLY via
//! MCP tool calls, preventing freeform text responses and ensuring schema compliance.

use crate::config::ClientConfig;
use crate::errors::Error;
use crate::response::CliResponse;
use claudecode_adapter;
use futures::StreamExt;
use rig::completion::{
    message::AssistantContent, CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse, Usage,
};
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use rig::OneOrMany;
use rig_provider::mcp_agent::{CliAdapter, CliAgentBuilder};
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

/// Claude Code provider client.
///
/// Wraps the Claude Code CLI and provides Rig's `CompletionClient` trait,
/// enabling the standard `.agent()` and `.extractor()` builder patterns.
///
/// # CLI Discovery
///
/// `Client::new()` auto-discovers the Claude CLI binary via:
/// 1. `$CLAUDE_BIN` environment variable
/// 2. `claude` in PATH
/// 3. Standard installation locations
///
/// Returns `Error::ClaudeNotFound` if the binary cannot be found.
///
/// # Execution Paths
///
/// - `client.agent("model")` - Returns Rig's `AgentBuilder` for direct CLI execution.
///   Use for simple prompts and chat where structured output is not required.
///
/// - `client.mcp_agent("model")` - Returns `CliAgentBuilder` for MCP-enforced execution.
///   Use for structured extraction where the agent MUST respond via tool calls.
#[derive(Clone)]
pub struct Client {
    /// The underlying CLI client from `claudecode_adapter`.
    cli: claudecode_adapter::ClaudeCli,
    /// Client configuration (timeout, binary path override, etc.).
    config: ClientConfig,
    /// Optional payload data for context injection.
    payload: Option<String>,
}

impl Client {
    /// Creates a new Claude Code client with auto-discovery.
    ///
    /// Discovers the Claude CLI binary and validates it's executable.
    /// Uses default configuration (300s timeout, 100 message channel capacity).
    ///
    /// # Errors
    ///
    /// Returns `Error::ClaudeNotFound` if the CLI binary cannot be found,
    /// or `Error::Provider` if initialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rig_cli::claude::Client;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new() -> Result<Self, Error> {
        Self::from_config(ClientConfig::default()).await
    }

    /// Creates a new Claude Code client with custom configuration.
    ///
    /// Allows overriding the binary path, timeout, and channel capacity.
    /// If `config.binary_path` is `Some(path)`, uses that path instead of discovery.
    ///
    /// # Errors
    ///
    /// Returns `Error::ClaudeNotFound` if the CLI binary cannot be found,
    /// or `Error::Provider` if initialization fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rig_cli::claude::Client;
    /// # use rig_cli::config::ClientConfig;
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ClientConfig {
    ///     timeout: Duration::from_secs(600),
    ///     ..Default::default()
    /// };
    /// let client = Client::from_config(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_config(config: ClientConfig) -> Result<Self, Error> {
        let binary_path = config.binary_path.clone();
        let report = claudecode_adapter::init(binary_path)
            .await
            .map_err(|_| Error::ClaudeNotFound)?;

        let cli = claudecode_adapter::ClaudeCli::new(report.claude_path, report.capabilities);

        Ok(Self {
            cli,
            config,
            payload: None,
        })
    }

    /// Sets context data (file contents, text blobs) for payload injection.
    ///
    /// When set, prompts are restructured into XML format with `<context>` tags
    /// separating payload data from instructions. This prevents instruction/context
    /// confusion and enables clean data extraction workflows.
    ///
    /// The payload is passed to `McpToolAgentBuilder::payload()` during MCP execution.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rig_cli::claude::Client;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let file_contents = std::fs::read_to_string("data.json")?;
    /// let client = Client::new().await?
    ///     .with_payload(file_contents);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_payload(mut self, data: impl Into<String>) -> Self {
        self.payload = Some(data.into());
        self
    }

    /// Access the underlying CLI handle for advanced use cases.
    ///
    /// This is an escape hatch for developers who need access to adapter-specific
    /// functionality not exposed through the standard Rig provider interface.
    #[must_use]
    pub const fn cli(&self) -> &claudecode_adapter::ClaudeCli {
        &self.cli
    }

    /// Access the client configuration.
    #[must_use]
    pub const fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Creates an MCP-enforced agent builder for structured extraction.
    ///
    /// Unlike [`agent()`](rig::client::CompletionClient::agent) which uses direct CLI execution,
    /// `mcp_agent()` routes ALL interactions through the MCP server, ensuring the agent
    /// can ONLY respond via MCP tool calls. This enforces structured output extraction.
    ///
    /// # Two Execution Paths
    ///
    /// | Method | Path | Use Case |
    /// |--------|------|----------|
    /// | `client.agent("model")` | Direct CLI | Simple prompts, chat, streaming |
    /// | `client.mcp_agent("model")` | MCP Server | Structured extraction, forced tool use |
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rig_cli::claude::Client;
    /// use rig::completion::Prompt;
    ///
    /// let client = Client::new().await?;
    ///
    /// // MCP-enforced extraction
    /// let agent = client.mcp_agent("sonnet")
    ///     .toolset(extraction_tools)  // Your ToolSet here
    ///     .preamble("You are a data extraction agent")
    ///     .build()?;
    ///
    /// let result = agent.prompt("Extract user data from: ...").await?;
    /// ```
    #[must_use]
    pub fn mcp_agent(&self, _model: impl Into<String>) -> CliAgentBuilder {
        let mut builder = rig_provider::mcp_agent::CliAgent::builder()
            .adapter(CliAdapter::ClaudeCode)
            .timeout(self.config.timeout);

        // Transfer payload if set on client
        if let Some(ref payload) = self.payload {
            builder = builder.payload(payload.clone());
        }

        builder
    }
}

impl rig::client::CompletionClient for Client {
    type CompletionModel = Model;

    fn completion_model(&self, model: impl Into<String>) -> Model {
        Model::make(self, model)
    }

    // .agent() and .extractor() get default implementations automatically!
}

/// The `CompletionModel` implementation for Claude Code.
///
/// Provides direct CLI execution for prompts and streaming. For MCP-enforced
/// structured extraction, use `Client::mcp_agent()` instead.
///
/// Users interact with this via `AgentBuilder` (from `client.agent()`), not directly.
#[derive(Clone)]
pub struct Model {
    /// The underlying CLI client.
    cli: claudecode_adapter::ClaudeCli,
    /// Client configuration.
    config: ClientConfig,
    /// Optional payload for context injection.
    payload: Option<String>,
    /// Model identifier (stored for API consistency, CLI agents don't use per-request model selection).
    // Matches Rig's CompletionModel pattern where Model has a model identifier field
    #[allow(dead_code, clippy::struct_field_names)]
    model_name: String,
}

impl CompletionModel for Model {
    type Response = CliResponse;
    type StreamingResponse = ();
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self {
            cli: client.cli.clone(),
            config: client.config.clone(),
            payload: client.payload.clone(),
            model_name: model.into(),
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        // Extract prompt from chat history using the utility function
        let prompt_text = rig_provider::utils::format_chat_history(&request);

        // If payload is set, wrap prompt in XML context structure
        let final_prompt = if let Some(ref payload) = self.payload {
            format!(
                r"<context>
{payload}
</context>

<task>
{prompt_text}
</task>"
            )
        } else {
            prompt_text
        };

        // Extract preamble (system prompt)
        let preamble = request.preamble.as_deref().unwrap_or("");

        // Direct CLI execution path
        let start = Instant::now();

        let mut config = claudecode_adapter::RunConfig {
            timeout: self.config.timeout,
            ..claudecode_adapter::RunConfig::default()
        };

        // If preamble present, append to system prompt
        if !preamble.is_empty() {
            config.system_prompt = claudecode_adapter::SystemPromptMode::Append(preamble.to_string());
        }

        // Run the CLI
        let result = self
            .cli
            .run(&final_prompt, &config)
            .await
            .map_err(|e| {
                #[cfg(feature = "debug-output")]
                {
                    CompletionError::ProviderError(format!("{e}\n--- raw debug output ---\nError occurred during CLI execution. Enable tracing for detailed output."))
                }
                #[cfg(not(feature = "debug-output"))]
                {
                    CompletionError::ProviderError(e.to_string())
                }
            })?;

        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        let cli_response = CliResponse::from_run_result(
            result.stdout.clone(),
            result.exit_code,
            duration_ms,
        );

        Ok(CompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(result.stdout)),
            usage: Usage::default(),
            raw_response: cli_response,
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        // Streaming always uses direct CLI (MCP enforcement only on completion path)
        let prompt_text = rig_provider::utils::format_chat_history(&request);

        // If payload is set, wrap prompt in XML context structure
        let final_prompt = if let Some(ref payload) = self.payload {
            format!(
                r"<context>
{payload}
</context>

<task>
{prompt_text}
</task>"
            )
        } else {
            prompt_text
        };

        let (tx, rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);
        let cli = self.cli.clone();
        let timeout = self.config.timeout;

        let mut config = claudecode_adapter::RunConfig {
            output_format: Some(claudecode_adapter::OutputFormat::StreamJson),
            timeout,
            ..claudecode_adapter::RunConfig::default()
        };

        // If preamble present, append to system prompt
        if let Some(preamble) = &request.preamble {
            config.system_prompt =
                claudecode_adapter::SystemPromptMode::Append(preamble.clone());
        }

        // If tools provided, wire allowed tools
        if !request.tools.is_empty() {
            let allowed_tools: Vec<String> = request.tools.iter().map(|t| t.name.clone()).collect();
            config.tools.allowed = Some(allowed_tools);
        }

        tokio::spawn(async move {
            // Error from CLI stream is intentionally dropped;
            // the receiver will see the channel close and handle accordingly
            let _ = cli.stream(&final_prompt, &config, tx).await;
        });

        // Convert the receiver into a stream
        let stream = ReceiverStream::new(rx).map(|event| {
            match event {
                claudecode_adapter::StreamEvent::Text { text } => {
                    Ok(RawStreamingChoice::Message(text))
                }
                claudecode_adapter::StreamEvent::ToolCall { name, input } => {
                    let id = Uuid::new_v4().to_string();
                    let tool_call = RawStreamingToolCall::new(id, name, input);
                    Ok(RawStreamingChoice::ToolCall(tool_call))
                }
                claudecode_adapter::StreamEvent::ToolResult { .. } => {
                    // For now we ignore tool results in the assistant output stream
                    // They are usually input for the next turn
                    // Empty message acts as a no-op heartbeat
                    Ok(RawStreamingChoice::Message(String::new()))
                }
                claudecode_adapter::StreamEvent::Error { message } => {
                    Err(CompletionError::ProviderError(message))
                }
                claudecode_adapter::StreamEvent::Unknown(_) => {
                    Ok(RawStreamingChoice::Message(String::new()))
                }
            }
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_response_serialization() {
        let response = CliResponse {
            text: "Hello, world!".to_string(),
            exit_code: 0,
            duration_ms: 1234,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CliResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, "Hello, world!");
        assert_eq!(deserialized.exit_code, 0);
        assert_eq!(deserialized.duration_ms, 1234);
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert!(config.binary_path.is_none());
        assert_eq!(config.timeout.as_secs(), 300);
        assert_eq!(config.channel_capacity, 100);
    }
}
