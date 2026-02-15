use crate::errors::ProviderError;
use crate::sessions::SessionManager;
use crate::utils::format_chat_history;
use futures::StreamExt;
use rig::completion::{
    message::AssistantContent, CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse, GetTokenUsage, ToolDefinition, Usage,
};
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};
use rig::tool::Tool;
use rig::OneOrMany;
use rig_cli_codex::{discover_codex, CodexCli, CodexConfig, RunResult, StreamEvent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;

/// Arguments for the Codex tool.
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct CodexArgs {
    /// The prompt or instruction for Codex
    pub prompt: String,
    /// Optional session ID for persistent sandboxed environment
    pub session_id: Option<String>,
}

/// A wrapper for `StreamEvent` to satisfy Rig traits.
#[derive(Clone, Serialize, Deserialize)]
pub struct CodexStreamEvent(pub StreamEvent);

impl GetTokenUsage for CodexStreamEvent {
    fn token_usage(&self) -> Option<Usage> {
        None
    }
}

/// The Rig `CompletionModel` implementation for Codex.
#[derive(Clone)]
pub struct CodexModel {
    /// The underlying CLI client.
    pub cli: CodexCli,
}

impl CompletionModel for CodexModel {
    type Response = RunResult;
    // We use () because we don't have a structured final response object during streaming that carries token usage yet
    type StreamingResponse = ();
    type Client = CodexCli;

    fn make(client: &Self::Client, _model: impl Into<String>) -> Self {
        Self {
            cli: client.clone(),
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let prompt_str = format_chat_history(&request);

        let config = CodexConfig::default();
        let result = self
            .cli
            .run(&prompt_str, &config)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        Ok(CompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(result.stdout.clone())),
            usage: Usage::default(),
            raw_response: result,
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let prompt_str = format_chat_history(&request);

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let cli = self.cli.clone();

        // Spawn the CLI process in the background
        let config = CodexConfig::default();

        tokio::spawn(async move {
            // Error from CLI stream is intentionally dropped here;
            // the receiver will see the channel close and handle accordingly
            let _ = cli.stream(&prompt_str, &config, tx).await;
        });

        // Convert the receiver into a stream
        let stream = ReceiverStream::new(rx).map(|event| match event {
            StreamEvent::Text { text } => Ok(RawStreamingChoice::Message(text)),
            StreamEvent::Error { message } => Err(CompletionError::ProviderError(message)),
            StreamEvent::Unknown(_) => Ok(RawStreamingChoice::Message(String::new())),
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}

/// A Rig Tool that exposes the Codex CLI.
pub struct CodexTool {
    /// The underlying CLI client.
    pub cli: CodexCli,
    /// The session manager.
    pub manager: SessionManager,
}

impl CodexTool {
    /// Creates a new `CodexTool`.
    ///
    /// # Errors
    /// Returns a `CodexError` if discovery or health check fails.
    pub async fn new() -> Result<Self, rig_cli_codex::CodexError> {
        let path = discover_codex(None)?;
        let cli = CodexCli::new(path);
        cli.check_health().await?;
        Ok(Self {
            cli,
            manager: SessionManager::new(),
        })
    }
}

impl Tool for CodexTool {
    const NAME: &'static str = "codex";
    type Error = ProviderError;
    type Args = CodexArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute a command using Codex CLI.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CodexArgs))
                .unwrap_or_else(|_| serde_json::json!({})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session_id = args.session_id.unwrap_or_else(|| "default".to_string());
        let cwd = self
            .manager
            .get_session_dir(&session_id)
            .await
            .map_err(|e| ProviderError::Session(e.to_string()))?;

        let config = CodexConfig {
            full_auto: true, // Default to high-automation for MCP
            cd: Some(cwd),
            ..Default::default()
        };
        let result = self.cli.run(&args.prompt, &config).await?;

        if result.exit_code != 0 {
            return Err(ProviderError::Codex(
                rig_cli_codex::CodexError::NonZeroExit {
                    exit_code: result.exit_code,
                    pid: 0,
                    elapsed: std::time::Duration::from_millis(result.duration_ms),
                    stdout: result.stdout,
                    stderr: result.stderr,
                },
            ));
        }

        Ok(result.stdout)
    }
}
