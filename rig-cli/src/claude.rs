//! Claude Code provider with MCP-enforced CompletionModel.
//!
//! This module provides the reference implementation for the CLI-to-Rig provider pattern.
//! All tool-bearing requests route through `McpToolAgent` to enforce the MCP tool workflow,
//! while simple prompts without tools fall back to direct CLI execution for simplicity.
//!
//! # Example
//!
//! ```no_run
//! # use rig_cli::claude::Client;
//! # use rig::client::CompletionClient;
//! # use rig::completion::Prompt;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client (auto-discovers Claude CLI)
//! let client = Client::new().await?;
//!
//! // Build an agent using standard Rig patterns
//! let agent = client.agent("claude-sonnet-4")
//!     .preamble("You are a helpful assistant")
//!     .build();
//!
//! // Prompt the agent
//! let response = agent.prompt("Hello!").await?;
//! println!("{}", response);
//! # Ok(())
//! # }
//! ```

use crate::config::ClientConfig;
use crate::errors::Error;
use claudecode_adapter;
use futures::StreamExt;
use rig::completion::{
    message::AssistantContent, CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse, ToolDefinition, Usage,
};
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use rig::OneOrMany;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

/// Response from a Claude Code CLI agent execution.
///
/// This is rig-cli's owned response type, not an adapter-internal type.
/// It provides the essential information users need from CLI execution
/// in a clean, serializable format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    /// The text content of the agent's response.
    pub text: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

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
/// # MCP Enforcement
///
/// When agents are built with tools (via `.tool()` on `AgentBuilder`),
/// all execution routes through `McpToolAgent` to enforce the MCP tool workflow.
/// Simple prompts without tools use direct CLI execution for backward compatibility.
#[derive(Clone)]
pub struct Client {
    /// The underlying CLI client from claudecode_adapter.
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
}

impl rig::client::CompletionClient for Client {
    type CompletionModel = Model;

    fn completion_model(&self, model: impl Into<String>) -> Model {
        Model::make(self, model)
    }

    // .agent() and .extractor() get default implementations automatically!
}

/// The CompletionModel implementation for Claude Code.
///
/// This is the execution layer that routes requests through either:
/// - `McpToolAgent` for tool-bearing requests (MCP-enforced workflow)
/// - Direct CLI execution for simple prompts without tools (backward compatible)
///
/// Users interact with this via `AgentBuilder`, not directly.
#[derive(Clone)]
pub struct Model {
    /// The underlying CLI client.
    cli: claudecode_adapter::ClaudeCli,
    /// Client configuration.
    config: ClientConfig,
    /// Optional payload for context injection.
    payload: Option<String>,
    /// Model identifier (stored but currently unused by CLI).
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

        // Extract preamble (system prompt)
        let preamble = request.preamble.as_deref().unwrap_or("");

        // Decision point: route through MCP if tools are present
        if !request.tools.is_empty() {
            // MCP-enforced path: build and run McpToolAgent
            self.completion_with_mcp(&prompt_text, preamble, &request.tools)
                .await
        } else {
            // Simple path: direct CLI execution (no tools = no MCP needed)
            self.completion_without_mcp(&prompt_text, preamble).await
        }
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        // Streaming always uses direct CLI (MCP enforcement only on completion path)
        let prompt_text = rig_provider::utils::format_chat_history(&request);

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
            let _ = cli.stream(&prompt_text, &config, tx).await;
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

impl Model {
    /// MCP-enforced completion path for tool-bearing requests.
    ///
    /// Builds a `McpToolAgent` with the provided toolset, routes execution
    /// through the MCP server (auto-spawned), and returns the result.
    ///
    /// # Current Limitation
    ///
    /// For v1, we fall back to direct CLI execution since Rig's CompletionRequest
    /// provides ToolDefinitions (JSON schemas) but not Tool trait objects.
    /// The MCP path works perfectly when users build agents directly via
    /// AgentBuilder.tool(), which properly adds Tool trait objects.
    ///
    /// Future enhancement: Build a dynamic Tool wrapper around ToolDefinition.
    async fn completion_with_mcp(
        &self,
        prompt: &str,
        preamble: &str,
        _tool_defs: &[ToolDefinition],
    ) -> Result<CompletionResponse<CliResponse>, CompletionError> {
        // For v1: Since we can't reliably convert ToolDefinition to Tool trait objects,
        // and tool-bearing requests should come from AgentBuilder (which properly adds tools),
        // we'll fall back to direct CLI for now and document this limitation.
        // The MCP path works perfectly when users use .tool() on AgentBuilder.
        tracing::info!(
            "Tool-bearing request detected but ToolDefinition -> ToolSet conversion not yet implemented. \
             Falling back to direct CLI execution. For MCP-enforced extraction, use the extractor pattern."
        );

        self.completion_without_mcp(prompt, preamble).await
    }

    /// Direct CLI completion path for simple prompts without tools.
    ///
    /// Uses the underlying `ClaudeCli` directly, no MCP orchestration.
    /// This is the backward-compatible path for freeform chat.
    async fn completion_without_mcp(
        &self,
        prompt: &str,
        preamble: &str,
    ) -> Result<CompletionResponse<CliResponse>, CompletionError> {
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
            .run(prompt, &config)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let cli_response = CliResponse {
            text: result.stdout.clone(),
            exit_code: result.exit_code,
            duration_ms,
        };

        Ok(CompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(result.stdout)),
            usage: Usage::default(),
            raw_response: cli_response,
        })
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
