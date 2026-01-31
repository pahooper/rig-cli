use crate::errors::ProviderError;
use crate::sessions::SessionManager;
use crate::utils::format_chat_history;
use claudecode_adapter::{init, ClaudeCli, RunConfig, RunResult, StreamEvent};
use futures::StreamExt;
use rig::completion::{
    message::AssistantContent, CompletionError, CompletionModel, CompletionRequest,
    CompletionResponse, ToolDefinition,
};
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use rig::tool::Tool;
use rig::OneOrMany;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

/// Arguments for the Claude Code tool.
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ClaudeArgs {
    /// The prompt or instruction for Claude Code
    pub prompt: String,
    /// Optional session ID for persistent sandboxed environment
    pub session_id: Option<String>,
}

/// The Rig `CompletionModel` implementation for Claude Code.
#[derive(Clone)]
pub struct ClaudeModel {
    /// The underlying CLI client.
    pub cli: ClaudeCli,
}

impl CompletionModel for ClaudeModel {
    type Response = RunResult;
    // We use () because we don't have a structured final response object during streaming that carries token usage yet
    type StreamingResponse = ();
    type Client = ClaudeCli;

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

        let mut config = RunConfig::default();
        if !request.tools.is_empty() {
            let allowed_tools: Vec<String> = request.tools.iter().map(|t| t.name.clone()).collect();
            config.tools.allowed = Some(allowed_tools);
            config.tools.builtin = claudecode_adapter::BuiltinToolSet::Default; // or implicit?
        }
        let result = self
            .cli
            .run(&prompt_str, &config)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        Ok(CompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(result.stdout.clone())),
            usage: Default::default(),
            raw_response: result,
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let prompt_str = format_chat_history(&request);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let cli = self.cli.clone();

        // Spawn the CLI process in the background
        let mut config = RunConfig::default();
        config.output_format = Some(claudecode_adapter::OutputFormat::StreamJson);

        if !request.tools.is_empty() {
            let allowed_tools: Vec<String> = request.tools.iter().map(|t| t.name.clone()).collect();
            config.tools.allowed = Some(allowed_tools);
        }

        tokio::spawn(async move {
            let _ = cli.stream(&prompt_str, &config, tx).await;
        });

        // Convert the receiver into a stream
        let stream = UnboundedReceiverStream::new(rx).map(|event| {
            match event {
                StreamEvent::Text { text } => Ok(RawStreamingChoice::Message(text)),
                StreamEvent::ToolCall { name, input } => {
                    let id = Uuid::new_v4().to_string();
                    let tool_call = RawStreamingToolCall::new(id, name, input);
                    Ok(RawStreamingChoice::ToolCall(tool_call))
                }
                StreamEvent::ToolResult { .. } => {
                    // For now we ignore tool results in the assistant output stream
                    // They are usually input for the next turn
                    // Empty message acts as a no-op heartbeat
                    Ok(RawStreamingChoice::Message(String::new()))
                }
                StreamEvent::Error { message } => Err(CompletionError::ProviderError(message)),
                StreamEvent::Unknown(_) => Ok(RawStreamingChoice::Message(String::new())),
            }
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}

/// A Rig Tool that exposes the Claude Code CLI.
pub struct ClaudeTool {
    /// The underlying CLI client.
    pub cli: ClaudeCli,
    /// List of auto-approved MCP servers.
    pub mcp_configs: Vec<String>,
    /// The session manager.
    pub manager: SessionManager,
}

impl ClaudeTool {
    /// Creates a new `ClaudeTool` with the given MCP configurations.
    ///
    /// # Errors
    /// Returns a `ClaudeError` if initialization or discovery fails.
    pub async fn new(mcp_configs: Vec<String>) -> Result<Self, claudecode_adapter::ClaudeError> {
        let report = init(None).await?;
        Ok(Self {
            cli: ClaudeCli::new(report.claude_path, report.capabilities),
            mcp_configs,
            manager: SessionManager::new(),
        })
    }
}

impl Tool for ClaudeTool {
    const NAME: &'static str = "claude_code";
    type Error = ProviderError;
    type Args = ClaudeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute a command using Claude Code CLI with mandatory MCP guardrails."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ClaudeArgs))
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

        let config = RunConfig {
            mcp: Some(claudecode_adapter::McpPolicy {
                configs: self.mcp_configs.clone(),
                strict: true,
            }),
            cwd: Some(cwd),
            ..Default::default()
        };

        let result = self.cli.run(&args.prompt, &config).await?;

        if result.exit_code != 0 {
            return Err(ProviderError::Claude(
                claudecode_adapter::ClaudeError::NonZeroExit {
                    exit_code: result.exit_code,
                    stdout: result.stdout,
                    stderr: result.stderr,
                },
            ));
        }

        Ok(result.stdout)
    }
}
