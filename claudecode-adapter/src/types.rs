use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemPromptMode {
    None,
    Append(String),
    Replace(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicy {
    pub configs: Vec<String>,
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuiltinToolSet {
    Default,
    None,
    Explicit(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub builtin: BuiltinToolSet,
    pub allowed: Option<Vec<String>>,
    pub disallowed: Option<Vec<String>>,
    pub disable_slash_commands: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JsonSchema {
    None,
    JsonValue(serde_json::Value),
    Inline(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub supports_stream_json: bool,
    pub supports_json_schema: bool,
    pub supports_system_prompt: bool,
    pub supports_append_system_prompt: bool,
    pub supports_mcp: bool,
    pub supports_strict_mcp: bool,
    pub supports_tools_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitReport {
    pub claude_path: PathBuf,
    pub version: String,
    pub doctor_ok: bool,
    pub doctor_stdout: String,
    pub doctor_stderr: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub model: Option<String>,
    pub output_format: Option<OutputFormat>,
    pub system_prompt: SystemPromptMode,
    pub mcp: Option<McpPolicy>,
    pub tools: ToolPolicy,
    pub json_schema: JsonSchema,
    pub include_partial_messages: bool,
    pub timeout: Duration,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            model: None,
            output_format: Some(OutputFormat::Text),
            system_prompt: SystemPromptMode::None,
            mcp: None,
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Default,
                allowed: None,
                disallowed: None,
                disable_slash_commands: false,
            },
            json_schema: JsonSchema::None,
            include_partial_messages: false,
            timeout: Duration::from_secs(300),
            cwd: None,
            env: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub json: Option<serde_json::Value>,
    pub stream_events: Vec<serde_json::Value>,
    pub structured_output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Text {
        text: String,
    },
    ToolCall {
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        name: String,
        output: String,
    },
    Error {
        message: String,
    },
    Unknown(serde_json::Value),
}
