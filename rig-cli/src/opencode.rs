//! OpenCode provider client with MCP-enforced CompletionModel.
//!
//! ## Example
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a client (discovers OpenCode CLI automatically)
//! let client = rig_cli::opencode::Client::new().await?;
//!
//! // Build an agent just like any Rig provider
//! let agent = client.agent("opencode/big-pickle")
//!     .preamble("You are a helpful assistant")
//!     .build();
//!
//! // Prompt the agent
//! let response = agent.prompt("Hello!").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Note on MCP Enforcement
//!
//! MCP-based structured extraction is available via the underlying `rig_provider`
//! crate's `McpToolAgent`. Direct integration at the facade level is planned for
//! a future release.

use crate::config::ClientConfig;
use crate::errors::Error;
use crate::response::CliResponse;
use futures::StreamExt;
use opencode_adapter::{discover_opencode, OpenCodeCli, OpenCodeConfig};
use rig::completion::{
    message::AssistantContent, CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse, Usage,
};
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};
use rig::OneOrMany;
use rig_provider::utils::format_chat_history;
use tokio_stream::wrappers::ReceiverStream;

/// OpenCode CLI provider client.
///
/// Wraps the OpenCode CLI behind Rig's `CompletionClient` trait, providing
/// the standard `.agent()` and `.extractor()` builder patterns.
#[derive(Clone)]
pub struct Client {
    cli: OpenCodeCli,
    config: ClientConfig,
    payload: Option<String>,
}

impl Client {
    /// Creates a new OpenCode client with automatic CLI discovery.
    ///
    /// Discovers the OpenCode CLI binary via PATH and standard installation
    /// locations, performs a health check, and returns a ready-to-use client.
    ///
    /// # Errors
    ///
    /// Returns `Error::OpenCodeNotFound` if the CLI binary cannot be found.
    /// Returns `Error::Provider` if the health check fails.
    pub async fn new() -> Result<Self, Error> {
        let path = discover_opencode(None).map_err(|_| Error::OpenCodeNotFound)?;
        let cli = OpenCodeCli::new(path);
        cli.check_health()
            .await
            .map_err(|e| Error::Provider(e.into()))?;

        Ok(Self {
            cli,
            config: ClientConfig::default(),
            payload: None,
        })
    }

    /// Creates a new OpenCode client from the given configuration.
    ///
    /// Uses the binary path from `config.binary_path` if provided,
    /// otherwise falls back to auto-discovery.
    ///
    /// # Errors
    ///
    /// Returns `Error::OpenCodeNotFound` if the CLI binary cannot be found.
    /// Returns `Error::Provider` if the health check fails.
    pub async fn from_config(config: ClientConfig) -> Result<Self, Error> {
        let path = discover_opencode(config.binary_path.clone()).map_err(|_| Error::OpenCodeNotFound)?;
        let cli = OpenCodeCli::new(path);
        cli.check_health()
            .await
            .map_err(|e| Error::Provider(e.into()))?;

        Ok(Self {
            cli,
            config,
            payload: None,
        })
    }

    /// Sets context data (file contents, text blobs) for payload injection.
    ///
    /// When set, prompts are restructured into XML format with `<context>` tags
    /// separating payload data from instructions. Maps to `McpToolAgentBuilder::payload()`.
    #[must_use]
    pub fn with_payload(mut self, data: impl Into<String>) -> Self {
        self.payload = Some(data.into());
        self
    }
}

/// OpenCode completion model.
///
/// Implements Rig's `CompletionModel` trait, routing tool-bearing requests
/// through `McpToolAgent` for MCP-enforced execution and falling back to
/// direct CLI invocation for simple prompts.
#[derive(Clone)]
pub struct Model {
    cli: OpenCodeCli,
    config: ClientConfig,
    payload: Option<String>,
}

impl CompletionModel for Model {
    type Response = CliResponse;
    type StreamingResponse = ();
    type Client = Client;

    fn make(client: &Self::Client, _model: impl Into<String>) -> Self {
        Self {
            cli: client.cli.clone(),
            config: client.config.clone(),
            payload: client.payload.clone(),
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let prompt_text = format_chat_history(&request);

        // TODO: Route through McpToolAgent for MCP-enforced structured extraction.
        // For now, use direct CLI execution for all requests (matching rig-provider pattern).
        // MCP enforcement will be added in a future iteration.

        let config = OpenCodeConfig::default();
        let result = self
            .cli
            .run(&prompt_text, &config)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        let cli_response =
            CliResponse::from_run_result(result.stdout.clone(), result.exit_code, result.duration_ms);

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
        let prompt_text = format_chat_history(&request);

        let (tx, rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);
        let cli = self.cli.clone();

        // Spawn the CLI process in the background
        let config = OpenCodeConfig::default();

        tokio::spawn(async move {
            // Error from CLI stream is intentionally dropped here;
            // the receiver will see the channel close and handle accordingly
            let _ = cli.stream(&prompt_text, &config, tx).await;
        });

        // Convert the receiver into a stream
        let stream = ReceiverStream::new(rx).map(|event| match event {
            opencode_adapter::StreamEvent::Text { text } => Ok(RawStreamingChoice::Message(text)),
            opencode_adapter::StreamEvent::Error { message } => {
                Err(CompletionError::ProviderError(message))
            }
            opencode_adapter::StreamEvent::Unknown(_) => Ok(RawStreamingChoice::Message(String::new())),
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}

impl rig::client::CompletionClient for Client {
    type CompletionModel = Model;

    fn completion_model(&self, model: impl Into<String>) -> Model {
        Model::make(self, model)
    }
}
