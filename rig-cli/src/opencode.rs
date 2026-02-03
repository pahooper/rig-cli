//! OpenCode provider client with CompletionModel and MCP-enforced CliAgent.
//!
//! This module provides two execution paths:
//!
//! | Method | Execution | Use Case |
//! |--------|-----------|----------|
//! | `client.agent("model")` | Direct CLI | Simple prompts, chat, streaming |
//! | `client.mcp_agent("model")` | MCP Server | Structured extraction, forced tool use |
//!
//! ## Example
//!
//! ```no_run
//! # use rig::client::CompletionClient;
//! # use rig::completion::Prompt;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = rig_cli::opencode::Client::new().await?;
//! let agent = client.agent("opencode/big-pickle").preamble("You are helpful").build();
//! let response = agent.prompt("Hello!").await?;
//! # Ok(())
//! # }
//! ```

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
use rig_provider::mcp_agent::{CliAdapter, CliAgentBuilder};
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

    /// Access the underlying CLI handle for advanced use cases.
    ///
    /// This is an escape hatch for developers who need access to adapter-specific
    /// functionality not exposed through the standard Rig provider interface.
    #[must_use]
    pub fn cli(&self) -> &OpenCodeCli {
        &self.cli
    }

    /// Access the client configuration.
    #[must_use]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Creates an MCP-enforced agent builder for structured extraction.
    ///
    /// See module docs for the difference between `agent()` and `mcp_agent()`.
    #[must_use]
    pub fn mcp_agent(&self, _model: impl Into<String>) -> CliAgentBuilder {
        let mut builder = rig_provider::mcp_agent::CliAgent::builder()
            .adapter(CliAdapter::OpenCode)
            .timeout(self.config.timeout);

        if let Some(ref payload) = self.payload {
            builder = builder.payload(payload.clone());
        }

        builder
    }
}

/// OpenCode completion model.
///
/// Provides direct CLI execution for prompts and streaming. For MCP-enforced
/// structured extraction, use `Client::mcp_agent()` instead.
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

        // If payload is set, wrap prompt in XML context structure
        let final_prompt = if let Some(ref payload) = self.payload {
            format!(
                r#"<context>
{payload}
</context>

<task>
{prompt_text}
</task>"#
            )
        } else {
            prompt_text
        };

        let mut config = OpenCodeConfig {
            timeout: self.config.timeout,
            ..OpenCodeConfig::default()
        };

        // Wire preamble into prompt if present (OpenCode uses 'prompt' field for system prompt)
        if let Some(ref preamble) = request.preamble {
            config.prompt = Some(preamble.clone());
        }

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

        // If payload is set, wrap prompt in XML context structure
        let final_prompt = if let Some(ref payload) = self.payload {
            format!(
                r#"<context>
{payload}
</context>

<task>
{prompt_text}
</task>"#
            )
        } else {
            prompt_text
        };

        let (tx, rx) = tokio::sync::mpsc::channel(self.config.channel_capacity);
        let cli = self.cli.clone();

        // Spawn the CLI process in the background
        let mut config = OpenCodeConfig {
            timeout: self.config.timeout,
            ..OpenCodeConfig::default()
        };

        // Wire preamble into prompt if present (OpenCode uses 'prompt' field for system prompt)
        if let Some(preamble) = request.preamble {
            config.prompt = Some(preamble);
        }

        tokio::spawn(async move {
            // Error from CLI stream is intentionally dropped here;
            // the receiver will see the channel close and handle accordingly
            let _ = cli.stream(&final_prompt, &config, tx).await;
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
