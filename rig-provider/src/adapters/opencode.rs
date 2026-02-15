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
use rig_cli_opencode::{discover_opencode, OpenCodeCli, OpenCodeConfig, RunResult, StreamEvent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;

/// Arguments for the `OpenCode` tool.
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct OpenCodeArgs {
    /// The instruction for `OpenCode`
    pub instruction: String,
    /// Optional session ID for persistent sandboxed environment
    pub session_id: Option<String>,
}

/// A wrapper for `StreamEvent` to satisfy Rig traits.
#[derive(Clone, Serialize, Deserialize)]
pub struct OpenCodeStreamEvent(pub StreamEvent);

impl GetTokenUsage for OpenCodeStreamEvent {
    fn token_usage(&self) -> Option<Usage> {
        None
    }
}

/// The Rig `CompletionModel` implementation for `OpenCode`.
#[derive(Clone)]
pub struct OpenCodeModel {
    /// The underlying CLI client.
    pub cli: OpenCodeCli,
}

impl CompletionModel for OpenCodeModel {
    type Response = RunResult;
    type StreamingResponse = ();
    type Client = OpenCodeCli;

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

        let config = OpenCodeConfig::default();
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
        let config = OpenCodeConfig::default();

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

/// A Rig Tool that exposes the `OpenCode` CLI.
pub struct OpenCodeTool {
    /// The underlying CLI client.
    pub cli: OpenCodeCli,
    /// The session manager.
    pub manager: SessionManager,
}

impl OpenCodeTool {
    /// Creates a new `OpenCodeTool`.
    ///
    /// # Errors
    /// Returns an `OpenCodeError` if discovery or health check fails.
    pub async fn new() -> Result<Self, rig_cli_opencode::OpenCodeError> {
        let path = discover_opencode(None)?;
        let cli = OpenCodeCli::new(path);
        cli.check_health().await?;
        Ok(Self {
            cli,
            manager: SessionManager::new(),
        })
    }
}

impl Tool for OpenCodeTool {
    const NAME: &'static str = "opencode";
    type Error = ProviderError;
    type Args = OpenCodeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute a command using OpenCode CLI.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(OpenCodeArgs))
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

        let config = OpenCodeConfig {
            cwd: Some(cwd),
            ..Default::default()
        };
        let result = self.cli.run(&args.instruction, &config).await?;

        if result.exit_code != 0 {
            return Err(ProviderError::OpenCode(
                rig_cli_opencode::OpenCodeError::NonZeroExit {
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
