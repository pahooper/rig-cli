//! Shared data types for Claude CLI adapter configuration and results.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

/// Output format requested from the Claude CLI.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text output.
    Text,
    /// Single JSON object output.
    Json,
    /// Newline-delimited JSON stream.
    StreamJson,
}

/// How the system prompt should be supplied to the CLI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemPromptMode {
    /// No system prompt override.
    None,
    /// Append to the default system prompt.
    Append(String),
    /// Replace the default system prompt entirely.
    Replace(String),
}

/// MCP server configuration policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicy {
    /// Paths to MCP configuration files.
    pub configs: Vec<String>,
    /// Whether to enforce strict MCP configuration.
    pub strict: bool,
}

/// Which built-in tool set the CLI should use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuiltinToolSet {
    /// Use the CLI default tool set.
    Default,
    /// Disable all built-in tools.
    None,
    /// Use only the explicitly listed tools.
    Explicit(Vec<String>),
}

/// Tool access control policy for a CLI run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    /// Which built-in tools are enabled.
    pub builtin: BuiltinToolSet,
    /// Explicitly allowed tool names (if any).
    pub allowed: Option<Vec<String>>,
    /// Explicitly disallowed tool names (if any).
    pub disallowed: Option<Vec<String>>,
    /// Whether to disable interactive slash-commands.
    pub disable_slash_commands: bool,
}

/// Schema constraint for JSON-structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JsonSchema {
    /// No schema constraint.
    None,
    /// Schema provided as a parsed JSON value.
    JsonValue(serde_json::Value),
    /// Schema provided as a raw JSON string.
    Inline(String),
}

/// Individual feature that the Claude CLI may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Feature {
    /// The `stream-json` output format.
    StreamJson,
    /// The `--json-schema` flag.
    JsonSchema,
    /// The `--system-prompt` flag.
    SystemPrompt,
    /// The `--append-system-prompt` flag.
    AppendSystemPrompt,
    /// The `--mcp-config` flag.
    Mcp,
    /// The `--strict-mcp-config` flag.
    StrictMcp,
    /// The `--tools` flag.
    ToolsFlag,
}

/// Set of features detected from the Claude CLI help text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Features present in this CLI build.
    pub features: BTreeSet<Feature>,
}

impl Capabilities {
    /// Returns `true` if the given feature is supported.
    #[must_use]
    pub fn supports(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }
}

/// Report produced by the initialization / health-check sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitReport {
    /// Resolved path to the Claude CLI executable.
    pub claude_path: PathBuf,
    /// Version string reported by `claude --version`.
    pub version: String,
    /// Whether the health check passed.
    pub doctor_ok: bool,
    /// Captured stdout from the health check.
    pub doctor_stdout: String,
    /// Captured stderr from the health check.
    pub doctor_stderr: String,
    /// Detected CLI capabilities.
    pub capabilities: Capabilities,
}

/// Configuration for a single Claude CLI invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Model name override (e.g. `"claude-3-opus"`).
    pub model: Option<String>,
    /// Desired output format.
    pub output_format: Option<OutputFormat>,
    /// System prompt mode.
    pub system_prompt: SystemPromptMode,
    /// Optional MCP server policy.
    pub mcp: Option<McpPolicy>,
    /// Tool access control policy.
    pub tools: ToolPolicy,
    /// Optional JSON schema constraint for structured output.
    pub json_schema: JsonSchema,
    /// Whether to include partial / in-progress messages in stream output.
    pub include_partial_messages: bool,
    /// Maximum wall-clock duration before the process is killed.
    pub timeout: Duration,
    /// Working directory for the subprocess.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables passed to the subprocess.
    pub env: Vec<(String, String)>,
    /// Disable session persistence to avoid version-lock conflicts.
    ///
    /// When `true`, adds `--no-session-persistence` to the CLI invocation.
    /// This prevents hanging when another Claude Code session is already
    /// running (which holds a version lock file).
    pub no_session_persistence: bool,
    /// Override which setting sources the CLI loads (CLAUDE.md, hooks, MCP).
    ///
    /// When `Some("")`, adds `--setting-sources ""` which skips all user
    /// configuration — CLAUDE.md files, hooks, and MCP servers defined in
    /// `~/.claude/settings.json`. Useful for programmatic invocations that
    /// need isolation from user-level config.
    ///
    /// When `None` (default), the flag is omitted and the CLI uses its
    /// normal setting source resolution.
    pub setting_sources: Option<String>,
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
            no_session_persistence: false,
            setting_sources: None,
        }
    }
}

/// Result of a completed Claude CLI invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Process exit code (`-1` if unavailable).
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Parsed JSON output (when `OutputFormat::Json` was requested).
    pub json: Option<serde_json::Value>,
    /// Parsed JSONL stream events (when `OutputFormat::StreamJson` was used).
    pub stream_events: Vec<serde_json::Value>,
    /// Optional structured output parsed against a JSON schema.
    pub structured_output: Option<serde_json::Value>,
}

/// A typed event received during a streaming Claude CLI run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// A chunk of assistant text output.
    Text {
        /// The text content.
        text: String,
    },
    /// The assistant invoked a tool.
    ToolCall {
        /// Name of the tool being called.
        name: String,
        /// JSON input payload for the tool.
        input: serde_json::Value,
    },
    /// A tool produced a result.
    ToolResult {
        /// Name of the tool that produced the result.
        name: String,
        /// Textual output from the tool.
        output: String,
    },
    /// An error event emitted by the CLI.
    Error {
        /// Human-readable error message.
        message: String,
    },
    /// An unrecognized event type.
    Unknown(serde_json::Value),
}

/// Extracts [`StreamEvent`]s from Claude Code v2.x stream-json envelope format.
///
/// Claude Code v2.x wraps content in message envelopes:
/// - `{"type":"assistant","message":{"content":[{"type":"text",...},{"type":"tool_use",...}]}}`
/// - `{"type":"result","result":"...","is_error":false}`
/// - `{"type":"system",...}` (informational, skipped)
///
/// This function unwraps those envelopes into the flat [`StreamEvent`] variants
/// that downstream consumers expect, providing backward compatibility.
pub fn extract_v2_events(val: &serde_json::Value) -> Vec<StreamEvent> {
    let mut events = Vec::new();

    match val.get("type").and_then(serde_json::Value::as_str) {
        Some("assistant") => {
            if let Some(content) = val
                .pointer("/message/content")
                .and_then(serde_json::Value::as_array)
            {
                for block in content {
                    match block.get("type").and_then(serde_json::Value::as_str) {
                        Some("text") => {
                            if let Some(text) =
                                block.get("text").and_then(serde_json::Value::as_str)
                            {
                                events.push(StreamEvent::Text {
                                    text: text.to_string(),
                                });
                            }
                        }
                        Some("tool_use") => {
                            if let (Some(name), Some(input)) = (
                                block.get("name").and_then(serde_json::Value::as_str),
                                block.get("input"),
                            ) {
                                events.push(StreamEvent::ToolCall {
                                    name: name.to_string(),
                                    input: input.clone(),
                                });
                            }
                        }
                        Some("tool_result") => {
                            if let Some(tool_use_id) =
                                block.get("tool_use_id").and_then(serde_json::Value::as_str)
                            {
                                let output = block
                                    .get("content")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                events.push(StreamEvent::ToolResult {
                                    name: tool_use_id.to_string(),
                                    output,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Some("result") => {
            if val.get("is_error") == Some(&serde_json::Value::Bool(true)) {
                let msg = val
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| val.get("result").and_then(serde_json::Value::as_str))
                    .unwrap_or("Unknown error")
                    .to_string();
                events.push(StreamEvent::Error { message: msg });
            }
        }
        // System events (hooks, init) and other unknown types are informational — skip.
        _ => {}
    }

    events
}
