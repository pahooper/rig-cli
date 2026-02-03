//! MCP tool agent builder for transparent CLI orchestration.
//!
//! Provides [`McpToolAgent`] and its builder, which handle MCP config generation,
//! CLI discovery, tool name computation, and execution across all three supported
//! CLI adapters (Claude Code, Codex, OpenCode).

use crate::errors::ProviderError;
use std::io::Write as _;
use std::time::Duration;

/// Version requirements for CLI adapters. Hardcoded per adapter, not configurable.
struct VersionRequirement {
    /// Minimum supported version (below this = unsupported, warn).
    min_version: semver::Version,
    /// Maximum tested version (above this = untested, warn with different message).
    max_tested: semver::Version,
    /// CLI name for log messages.
    cli_name: &'static str,
}

/// Version requirement for Claude Code CLI.
const fn claude_code_version_req() -> VersionRequirement {
    VersionRequirement {
        min_version: semver::Version::new(1, 0, 0),
        max_tested: semver::Version::new(1, 99, 0),
        cli_name: "Claude Code",
    }
}

/// Version requirement for Codex CLI.
const fn codex_version_req() -> VersionRequirement {
    VersionRequirement {
        min_version: semver::Version::new(0, 1, 0),
        max_tested: semver::Version::new(0, 99, 0),
        cli_name: "Codex",
    }
}

/// Version requirement for `OpenCode` CLI.
const fn opencode_version_req() -> VersionRequirement {
    VersionRequirement {
        min_version: semver::Version::new(0, 1, 0),
        max_tested: semver::Version::new(0, 99, 0),
        cli_name: "OpenCode",
    }
}

/// Detects CLI version and validates against requirements.
///
/// Runs `<binary> --version`, parses the version string with semver,
/// and emits structured tracing warnings for unsupported or untested versions.
/// Always returns Ok — version issues are warnings, never blockers.
async fn detect_and_validate_version(
    binary_path: &std::path::Path,
    requirement: &VersionRequirement,
) {
    let output = match tokio::process::Command::new(binary_path)
        .arg("--version")
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            tracing::warn!(
                event = "version_detection_failed",
                cli = requirement.cli_name,
                error = %e,
                "version_detection_failed"
            );
            return;
        }
    };

    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Extract version substring: strip common prefixes like "v", split on whitespace
    // to handle formats like "claude 1.2.3" or "codex v0.91.0"
    let cleaned = extract_version_string(&version_str);

    let version = match semver::Version::parse(&cleaned) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                event = "version_parse_failed",
                cli = requirement.cli_name,
                raw_version = %version_str,
                error = %e,
                "version_parse_failed"
            );
            return;
        }
    };

    tracing::debug!(
        event = "version_detected",
        cli = requirement.cli_name,
        version = %version,
        "version_detected"
    );

    if version < requirement.min_version {
        tracing::warn!(
            event = "version_unsupported",
            cli = requirement.cli_name,
            detected = %version,
            minimum = %requirement.min_version,
            "version_unsupported: {} {} is below minimum supported version {}",
            requirement.cli_name,
            version,
            requirement.min_version,
        );
    } else if version > requirement.max_tested {
        tracing::warn!(
            event = "version_untested",
            cli = requirement.cli_name,
            detected = %version,
            max_tested = %requirement.max_tested,
            "version_untested: {} {} is newer than maximum tested version {}",
            requirement.cli_name,
            version,
            requirement.max_tested,
        );
    }
}

/// Extracts a semver-parseable version string from CLI version output.
///
/// Handles common formats:
/// - "1.2.3" -> "1.2.3"
/// - "v1.2.3" -> "1.2.3"
/// - "claude 1.2.3" -> "1.2.3"
/// - "codex v0.91.0-beta" -> "0.91.0-beta"
fn extract_version_string(raw: &str) -> String {
    // Split on whitespace and find the token that looks like a version
    for token in raw.split_whitespace() {
        let stripped = token.strip_prefix('v').unwrap_or(token);
        if semver::Version::parse(stripped).is_ok() {
            return stripped.to_string();
        }
    }
    // Fallback: try stripping 'v' from the whole string
    raw.strip_prefix('v').unwrap_or(raw).to_string()
}

/// Default instruction template enforcing the three-tool workflow.
///
/// This template requires agents to follow the example -> validate -> submit
/// sequence and forbids freeform text responses. The `{allowed_tools}` placeholder
/// is replaced at runtime with the computed MCP tool names.
pub const DEFAULT_WORKFLOW_TEMPLATE: &str = r"You are a structured data extraction agent.

MANDATORY WORKFLOW:
1. Call the 'json_example' tool FIRST to see the expected output format
2. Draft your extraction based on the example and the provided context
3. Call 'validate_json' with your draft to check for errors
4. If validation fails, fix the errors and call 'validate_json' again
5. Once validation passes, call 'submit' with the validated data

RULES:
- You MUST complete all steps above in order
- Do NOT respond with freeform text as your final answer
- Do NOT output raw JSON in your response text
- ONLY the 'submit' tool call marks task completion
- The task is NOT complete until you call 'submit'";

/// Stream event from MCP-enforced CLI execution.
#[derive(Debug, Clone)]
pub enum McpStreamEvent {
    /// Text content from the agent.
    Text(String),
    /// Tool call initiated by the agent.
    ToolCall {
        /// The tool name.
        name: String,
        /// The tool input as a JSON string.
        input: String
    },
    /// Result from a tool execution.
    ToolResult {
        /// The tool use ID or name.
        tool_use_id: String,
        /// The tool output content.
        content: String
    },
    /// Error during execution.
    Error(String),
}

/// Which CLI adapter to use for MCP tool agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAdapter {
    /// Use the Claude Code CLI (`claude --print`).
    ClaudeCode,
    /// Use the Codex CLI (`codex exec`).
    Codex,
    /// Use the `OpenCode` CLI (`opencode run`).
    OpenCode,
}

/// Result of an [`McpToolAgent`] execution.
#[derive(Debug)]
pub struct McpToolAgentResult {
    /// The raw stdout output from the CLI.
    pub stdout: String,
    /// The raw stderr output from the CLI.
    pub stderr: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// MCP-backed CLI agent that transparently handles MCP config generation,
/// CLI discovery, and tool name computation.
pub struct McpToolAgent;

impl McpToolAgent {
    /// Returns a new builder for configuring the agent.
    #[must_use]
    pub fn builder() -> McpToolAgentBuilder {
        McpToolAgentBuilder::new()
    }
}

/// Builder for configuring and running an MCP-backed CLI agent.
///
/// Accepts a Rig [`ToolSet`](rig::tool::ToolSet), a prompt, and a CLI adapter
/// choice. Handles all MCP plumbing transparently: config generation, temp file
/// management, CLI discovery, tool name computation, and execution.
pub struct McpToolAgentBuilder {
    toolset: Option<rig::tool::ToolSet>,
    prompt: Option<String>,
    adapter: Option<CliAdapter>,
    server_name: String,
    system_prompt: Option<String>,
    timeout: Duration,
    payload: Option<String>,
    instruction_template: Option<String>,
    builtin_tools: Option<Vec<String>>,
    sandbox_mode: Option<codex_adapter::SandboxMode>,
    working_dir: Option<std::path::PathBuf>,
}

impl McpToolAgentBuilder {
    fn new() -> Self {
        Self {
            toolset: None,
            prompt: None,
            adapter: None,
            server_name: "rig_mcp".to_string(),
            system_prompt: None,
            timeout: Duration::from_secs(300),
            payload: None,
            instruction_template: None,
            builtin_tools: None,
            sandbox_mode: Some(codex_adapter::SandboxMode::ReadOnly),
            working_dir: None,
        }
    }

    /// Sets the Rig [`ToolSet`](rig::tool::ToolSet) that defines the MCP tools.
    #[must_use]
    pub fn toolset(mut self, toolset: rig::tool::ToolSet) -> Self {
        self.toolset = Some(toolset);
        self
    }

    /// Sets the user prompt to send to the CLI agent.
    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets which CLI adapter to use for execution.
    #[must_use]
    pub const fn adapter(mut self, adapter: CliAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Sets the MCP server name used in config and tool name prefixes.
    ///
    /// Defaults to `"rig_mcp"`.
    #[must_use]
    pub fn server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }

    /// Sets an optional system prompt to prepend to the MCP instructions.
    #[must_use]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the maximum wall-clock timeout for the CLI execution.
    ///
    /// Defaults to 300 seconds (5 minutes).
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets context data (file contents, text blobs) to inject into the prompt.
    ///
    /// When set, the user prompt is restructured into a 4-block XML format:
    /// `<instructions>`, `<context>`, `<task>`, `<output_format>`.
    /// The payload data is wrapped in `<context>` tags to clearly delimit it
    /// from instructions, preventing instruction/context confusion.
    ///
    /// When not set, the prompt is passed through unchanged (backward compatible).
    #[must_use]
    pub fn payload(mut self, data: impl Into<String>) -> Self {
        self.payload = Some(data.into());
        self
    }

    /// Sets a custom instruction template for tool workflow enforcement.
    ///
    /// If not set, [`DEFAULT_WORKFLOW_TEMPLATE`] is used, which enforces the
    /// example -> validate -> submit three-tool pattern.
    #[must_use]
    pub fn instruction_template(mut self, template: impl Into<String>) -> Self {
        self.instruction_template = Some(template.into());
        self
    }

    /// Opts in to specific builtin tools alongside MCP tools.
    ///
    /// By default, ALL builtin tools are disabled (CONT-01). Call this method
    /// to explicitly allow specific builtins like "Bash" or "Read".
    ///
    /// For Claude Code: maps to --tools flag with listed tools.
    /// For Codex: no direct equivalent (Codex builtins are controlled by sandbox mode).
    /// For `OpenCode`: documented as best-effort (no CLI flags for tool restriction).
    #[must_use]
    pub fn allow_builtins(mut self, tools: Vec<String>) -> Self {
        self.builtin_tools = Some(tools);
        self
    }

    /// Sets the Codex sandbox isolation level.
    ///
    /// Default: `SandboxMode::ReadOnly` (most restrictive).
    /// Only affects Codex adapter; Claude Code and `OpenCode` ignore this setting.
    #[must_use]
    pub const fn sandbox_mode(mut self, mode: codex_adapter::SandboxMode) -> Self {
        self.sandbox_mode = Some(mode);
        self
    }

    /// Overrides the default temp directory with a specific working directory.
    ///
    /// By default, agents execute in an auto-created temp directory (CONT-04).
    /// Call this to use a specific directory instead. The caller is responsible
    /// for the lifetime of the provided directory.
    #[must_use]
    pub fn working_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Computes the MCP tool names that will be passed to the CLI's allowed-tools list.
    ///
    /// Tool names follow the pattern `mcp__<server_name>__<tool_name>`.
    ///
    /// # Errors
    /// Returns an error if the toolset is not set or if fetching tool definitions fails.
    pub async fn compute_tool_names(&self) -> Result<Vec<String>, ProviderError> {
        let toolset = self
            .toolset
            .as_ref()
            .ok_or_else(|| ProviderError::McpToolAgent("toolset is required".to_string()))?;
        let definitions = toolset
            .get_tool_definitions()
            .await
            .map_err(|e| ProviderError::McpToolAgent(format!("Failed to get tool definitions: {e}")))?;
        Ok(definitions
            .iter()
            .map(|def| format!("mcp__{}__{}",  self.server_name, def.name))
            .collect())
    }

    /// Executes the MCP tool agent with streaming output.
    ///
    /// Similar to `run()`, but returns a receiver that yields `McpStreamEvent`
    /// as the CLI produces output. The agent spawns in a background task.
    ///
    /// # Errors
    /// Returns error if validation fails before spawning.
    pub async fn stream(self) -> Result<tokio::sync::mpsc::Receiver<McpStreamEvent>, ProviderError> {
        // 1. Validate required fields
        let toolset = self
            .toolset
            .ok_or_else(|| ProviderError::McpToolAgent("toolset is required".to_string()))?;
        let prompt = self
            .prompt
            .ok_or_else(|| ProviderError::McpToolAgent("prompt is required".to_string()))?;
        let adapter = self
            .adapter
            .ok_or_else(|| ProviderError::McpToolAgent("adapter is required".to_string()))?;
        let timeout = self.timeout;
        let payload = self.payload;
        let instruction_template = self.instruction_template;
        let system_prompt = self.system_prompt;
        let builtin_tools = self.builtin_tools;
        let sandbox_mode = self.sandbox_mode.unwrap_or(codex_adapter::SandboxMode::ReadOnly);
        let server_name = self.server_name.clone();

        // Create temp dir if working_dir not provided (CONT-04)
        let (_temp_dir, effective_cwd) = if let Some(dir) = self.working_dir { (None, dir) } else {
            let td = tempfile::TempDir::new()
                .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp dir: {e}")))?;
            let path = td.path().to_path_buf();
            (Some(td), path)
        };

        // 2. Get tool definitions
        let definitions = toolset
            .get_tool_definitions()
            .await
            .map_err(|e| ProviderError::McpToolAgent(format!("Failed to get tool definitions: {e}")))?;

        // 3. Build McpConfig
        let exe = std::env::current_exe()
            .map_err(|e| ProviderError::McpToolAgent(format!("Failed to get current exe: {e}")))?;
        let mcp_config = rig_mcp_server::server::McpConfig {
            name: server_name.clone(),
            command: exe.to_string_lossy().to_string(),
            args: vec![],
            env: {
                let mut env = std::collections::HashMap::new();
                env.insert("RIG_MCP_SERVER".to_string(), "1".to_string());
                env
            },
        };

        // 4. Compute allowed tool names
        let allowed_tools: Vec<String> = definitions
            .iter()
            .map(|def| format!("mcp__{}__{}",  server_name, def.name))
            .collect();

        // 5. Build workflow template (custom or default)
        let workflow_instructions = instruction_template.as_deref()
            .unwrap_or(DEFAULT_WORKFLOW_TEMPLATE);

        // 6. Build MCP instruction with workflow enforcement
        let mcp_instruction = format!(
            "{workflow_instructions}\n\nAvailable MCP tools: {}\n\n\
             You MUST use ONLY these MCP tools. Do NOT output raw JSON text as your response.",
            allowed_tools.join(", ")
        );

        // 7. Build full system prompt
        let full_system_prompt = match system_prompt {
            Some(sp) => format!("{sp}\n\n{mcp_instruction}"),
            None => mcp_instruction,
        };

        // 8. Build final user prompt (4-block XML if payload present)
        let final_prompt = if let Some(data) = payload {
            format!(
                r"<context>
{data}
</context>

<task>
{prompt}
</task>

<output_format>
Use ONLY the MCP tools listed in the system prompt. Final submission MUST be via the 'submit' tool.
</output_format>"
            )
        } else {
            prompt
        };

        // 9. Create channel for streaming events
        let (tx, rx) = tokio::sync::mpsc::channel::<McpStreamEvent>(100);

        // 10. Execute per adapter
        match adapter {
            CliAdapter::ClaudeCode => {
                run_claude_code_stream(&final_prompt, &mcp_config, &allowed_tools, &full_system_prompt, timeout, builtin_tools.as_ref(), &effective_cwd, tx)
                    .await?;
            }
            CliAdapter::Codex => {
                run_codex_stream(&final_prompt, &mcp_config, &full_system_prompt, timeout, &sandbox_mode, &effective_cwd, tx).await?;
            }
            CliAdapter::OpenCode => {
                run_opencode_stream(&final_prompt, &mcp_config, &full_system_prompt, timeout, &effective_cwd, tx).await?;
            }
        }

        Ok(rx)
    }

    /// Executes the MCP tool agent, returning the CLI output.
    ///
    /// This method:
    /// 1. Validates required fields (toolset, prompt, adapter)
    /// 2. Gets tool definitions from the toolset
    /// 3. Builds an [`McpConfig`](rig_mcp_server::server::McpConfig) for the target adapter
    /// 4. Computes allowed tool names as `mcp__<server>__<tool>`
    /// 5. Writes the config to a temp file in the adapter's format
    /// 6. Discovers and launches the CLI with correct flags
    /// 7. Returns the result; temp files are cleaned via RAII
    ///
    /// # Errors
    /// Returns [`ProviderError`] if any step fails (missing fields, CLI discovery,
    /// config generation, or CLI execution).
    pub async fn run(self) -> Result<McpToolAgentResult, ProviderError> {
        // 1. Validate required fields
        let toolset = self
            .toolset
            .ok_or_else(|| ProviderError::McpToolAgent("toolset is required".to_string()))?;
        let prompt = self
            .prompt
            .ok_or_else(|| ProviderError::McpToolAgent("prompt is required".to_string()))?;
        let adapter = self
            .adapter
            .ok_or_else(|| ProviderError::McpToolAgent("adapter is required".to_string()))?;
        let timeout = self.timeout;
        let payload = self.payload;
        let instruction_template = self.instruction_template;
        let system_prompt = self.system_prompt;
        let builtin_tools = self.builtin_tools;
        let sandbox_mode = self.sandbox_mode.unwrap_or(codex_adapter::SandboxMode::ReadOnly);

        // Create temp dir if working_dir not provided (CONT-04)
        let (_temp_dir, effective_cwd) = if let Some(dir) = self.working_dir { (None, dir) } else {
            let td = tempfile::TempDir::new()
                .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp dir: {e}")))?;
            let path = td.path().to_path_buf();
            (Some(td), path)
        };

        // 2. Get tool definitions
        let definitions = toolset
            .get_tool_definitions()
            .await
            .map_err(|e| ProviderError::McpToolAgent(format!("Failed to get tool definitions: {e}")))?;

        // 3. Build McpConfig
        let exe = std::env::current_exe()
            .map_err(|e| ProviderError::McpToolAgent(format!("Failed to get current exe: {e}")))?;
        let mcp_config = rig_mcp_server::server::McpConfig {
            name: self.server_name.clone(),
            // Path-to-string for JSON serialization; lossy is acceptable since CLI commands
            // must be valid UTF-8 in JSON config format.
            command: exe.to_string_lossy().to_string(),
            args: vec![],
            env: {
                let mut env = std::collections::HashMap::new();
                env.insert("RIG_MCP_SERVER".to_string(), "1".to_string());
                env
            },
        };

        // 4. Compute allowed tool names
        let allowed_tools: Vec<String> = definitions
            .iter()
            .map(|def| format!("mcp__{}__{}",  self.server_name, def.name))
            .collect();

        // 5. Build workflow template (custom or default)
        let workflow_instructions = instruction_template.as_deref()
            .unwrap_or(DEFAULT_WORKFLOW_TEMPLATE);

        // 6. Build MCP instruction with workflow enforcement
        let mcp_instruction = format!(
            "{workflow_instructions}\n\nAvailable MCP tools: {}\n\n\
             You MUST use ONLY these MCP tools. Do NOT output raw JSON text as your response.",
            allowed_tools.join(", ")
        );

        // 7. Build full system prompt
        let full_system_prompt = match system_prompt {
            Some(sp) => format!("{sp}\n\n{mcp_instruction}"),
            None => mcp_instruction,
        };

        // 8. Build final user prompt (4-block XML if payload present)
        let final_prompt = if let Some(data) = payload {
            format!(
                r"<context>
{data}
</context>

<task>
{prompt}
</task>

<output_format>
Use ONLY the MCP tools listed in the system prompt. Final submission MUST be via the 'submit' tool.
</output_format>"
            )
        } else {
            prompt
        };

        // 9. Execute per adapter
        match adapter {
            CliAdapter::ClaudeCode => {
                run_claude_code(&final_prompt, &mcp_config, &allowed_tools, &full_system_prompt, timeout, builtin_tools.as_ref(), &effective_cwd)
                    .await
            }
            CliAdapter::Codex => {
                run_codex(&final_prompt, &mcp_config, &full_system_prompt, timeout, &sandbox_mode, &effective_cwd).await
            }
            CliAdapter::OpenCode => {
                run_opencode(&final_prompt, &mcp_config, &full_system_prompt, timeout, &effective_cwd).await
            }
        }
    }
}

/// MCP-enforced CLI agent that implements Rig's Prompt and Chat traits.
///
/// Unlike `CompletionModel` (which receives `ToolDefinitions`), `CliAgent` holds
/// a concrete `ToolSet`, enabling true MCP enforcement where all agent
/// interactions must go through MCP tool calls.
///
/// # Example
///
/// ```no_run
/// use rig_provider::mcp_agent::{CliAgent, CliAdapter};
/// use rig::completion::Prompt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let my_toolset = rig::tool::ToolSet::builder().build();
/// let agent = CliAgent::builder()
///     .adapter(CliAdapter::ClaudeCode)
///     .toolset(my_toolset)
///     .preamble("You are a data extraction agent")
///     .build()?;
///
/// let response = agent.prompt("Extract the user data").await?;
/// # Ok(())
/// # }
/// ```
pub struct CliAgent {
    toolset: rig::tool::ToolSet,
    adapter: CliAdapter,
    preamble: Option<String>,
    timeout: Duration,
    payload: Option<String>,
    instruction_template: Option<String>,
    builtin_tools: Option<Vec<String>>,
    sandbox_mode: Option<codex_adapter::SandboxMode>,
    working_dir: Option<std::path::PathBuf>,
    server_name: String,
}

/// Builder for `CliAgent`.
pub struct CliAgentBuilder {
    toolset: Option<rig::tool::ToolSet>,
    adapter: Option<CliAdapter>,
    preamble: Option<String>,
    timeout: Duration,
    payload: Option<String>,
    instruction_template: Option<String>,
    builtin_tools: Option<Vec<String>>,
    sandbox_mode: Option<codex_adapter::SandboxMode>,
    working_dir: Option<std::path::PathBuf>,
    server_name: String,
}

impl CliAgentBuilder {
    fn new() -> Self {
        Self {
            toolset: None,
            adapter: None,
            preamble: None,
            timeout: Duration::from_secs(300),
            payload: None,
            instruction_template: None,
            builtin_tools: None,
            sandbox_mode: Some(codex_adapter::SandboxMode::ReadOnly),
            working_dir: None,
            server_name: "rig_mcp".to_string(),
        }
    }

    /// Sets the Rig [`ToolSet`](rig::tool::ToolSet) that defines the MCP tools.
    #[must_use]
    pub fn toolset(mut self, toolset: rig::tool::ToolSet) -> Self {
        self.toolset = Some(toolset);
        self
    }

    /// Sets which CLI adapter to use for execution.
    #[must_use]
    pub const fn adapter(mut self, adapter: CliAdapter) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Sets an optional preamble (system prompt) for the agent.
    #[must_use]
    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// Sets the maximum wall-clock timeout for CLI execution.
    ///
    /// Defaults to 300 seconds (5 minutes).
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets context data (file contents, text blobs) to inject into prompts.
    #[must_use]
    pub fn payload(mut self, data: impl Into<String>) -> Self {
        self.payload = Some(data.into());
        self
    }

    /// Sets a custom instruction template for tool workflow enforcement.
    #[must_use]
    pub fn instruction_template(mut self, template: impl Into<String>) -> Self {
        self.instruction_template = Some(template.into());
        self
    }

    /// Opts in to specific builtin tools alongside MCP tools.
    #[must_use]
    pub fn allow_builtins(mut self, tools: Vec<String>) -> Self {
        self.builtin_tools = Some(tools);
        self
    }

    /// Sets the Codex sandbox isolation level.
    ///
    /// Default: `SandboxMode::ReadOnly`. Only affects Codex adapter.
    #[must_use]
    pub const fn sandbox_mode(mut self, mode: codex_adapter::SandboxMode) -> Self {
        self.sandbox_mode = Some(mode);
        self
    }

    /// Overrides the default temp directory with a specific working directory.
    #[must_use]
    pub fn working_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Sets the MCP server name used in config and tool name prefixes.
    ///
    /// Defaults to `"rig_mcp"`.
    #[must_use]
    pub fn server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }

    /// Builds the `CliAgent`.
    ///
    /// # Errors
    /// Returns error if required fields (toolset, adapter) are not set.
    pub fn build(self) -> Result<CliAgent, ProviderError> {
        let toolset = self
            .toolset
            .ok_or_else(|| ProviderError::McpToolAgent("toolset is required".to_string()))?;
        let adapter = self
            .adapter
            .ok_or_else(|| ProviderError::McpToolAgent("adapter is required".to_string()))?;

        Ok(CliAgent {
            toolset,
            adapter,
            preamble: self.preamble,
            timeout: self.timeout,
            payload: self.payload,
            instruction_template: self.instruction_template,
            builtin_tools: self.builtin_tools,
            sandbox_mode: self.sandbox_mode,
            working_dir: self.working_dir,
            server_name: self.server_name,
        })
    }
}

impl CliAgent {
    /// Returns a new builder for configuring the agent.
    #[must_use]
    pub fn builder() -> CliAgentBuilder {
        CliAgentBuilder::new()
    }

    /// Executes a prompt and returns the agent's output.
    ///
    /// This method builds an [`McpToolAgent`] internally with the configured
    /// fields and executes it. Consumes the `CliAgent` since `ToolSet` cannot be cloned.
    ///
    /// # Errors
    /// Returns [`ProviderError`] if execution fails.
    pub async fn prompt(self, prompt: &str) -> Result<String, ProviderError> {
        let mut builder = McpToolAgent::builder()
            .toolset(self.toolset)
            .adapter(self.adapter)
            .prompt(prompt)
            .timeout(self.timeout)
            .server_name(&self.server_name);

        // Apply optional fields
        if let Some(ref preamble) = self.preamble {
            builder = builder.system_prompt(preamble);
        }
        if let Some(ref payload) = self.payload {
            builder = builder.payload(payload);
        }
        if let Some(ref template) = self.instruction_template {
            builder = builder.instruction_template(template);
        }
        if let Some(ref builtins) = self.builtin_tools {
            builder = builder.allow_builtins(builtins.clone());
        }
        if let Some(ref mode) = self.sandbox_mode {
            builder = builder.sandbox_mode(mode.clone());
        }
        if let Some(ref dir) = self.working_dir {
            builder = builder.working_dir(dir);
        }

        let result = builder.run().await?;

        Ok(result.stdout)
    }

    /// Executes a chat-style interaction with message history.
    ///
    /// Formats the chat history as a conversation and appends the current prompt.
    /// Consumes the `CliAgent` since `ToolSet` cannot be cloned.
    ///
    /// # Errors
    /// Returns [`ProviderError`] if execution fails.
    pub async fn chat(
        self,
        prompt: &str,
        chat_history: &[String],
    ) -> Result<String, ProviderError> {
        // Format chat history into prompt
        let full_prompt = if chat_history.is_empty() {
            prompt.to_string()
        } else {
            let history = chat_history.join("\n");
            format!("{history}\n\nuser: {prompt}")
        };

        self.prompt(&full_prompt).await
    }
}

async fn run_claude_code(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    allowed_tools: &[String],
    system_prompt: &str,
    timeout: Duration,
    builtin_tools: Option<&Vec<String>>,
    cwd: &std::path::Path,
) -> Result<McpToolAgentResult, ProviderError> {
    // Write Claude Code MCP config JSON to temp file
    let mut config_file = tempfile::NamedTempFile::new()
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp file: {e}")))?;
    let json = serde_json::to_string_pretty(&mcp_config.to_claude_json())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to serialize config: {e}")))?;
    config_file
        .write_all(json.as_bytes())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to write config: {e}")))?;
    let config_path = config_file.path().to_path_buf();
    let _config_guard = config_file.into_temp_path();

    let report = claudecode_adapter::init(None)
        .await
        .map_err(|e| ProviderError::McpToolAgent(format!("Claude init failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&report.claude_path, &claude_code_version_req()).await;

    let cli = claudecode_adapter::ClaudeCli::new(report.claude_path, report.capabilities);

    // Apply containment: disable all builtins by default, opt-in via builtin_tools
    let builtin_set = builtin_tools.map_or(
        claudecode_adapter::BuiltinToolSet::None,
        |tools| claudecode_adapter::BuiltinToolSet::Explicit(tools.clone()),
    );

    let config = claudecode_adapter::RunConfig {
        output_format: Some(claudecode_adapter::OutputFormat::Text),
        system_prompt: claudecode_adapter::SystemPromptMode::Append(system_prompt.to_string()),
        mcp: Some(claudecode_adapter::McpPolicy {
            // Temp file paths are always valid UTF-8 (created by tempfile crate).
            configs: vec![config_path.to_string_lossy().to_string()],
            strict: true,
        }),
        tools: claudecode_adapter::ToolPolicy {
            builtin: builtin_set,
            allowed: Some(allowed_tools.to_vec()),
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout,
        cwd: Some(cwd.to_path_buf()),
        ..claudecode_adapter::RunConfig::default()
    };

    let result = cli.run(prompt, &config).await.map_err(ProviderError::Claude)?;

    Ok(McpToolAgentResult {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
    })
}

async fn run_codex(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    system_prompt: &str,
    timeout: Duration,
    sandbox_mode: &codex_adapter::SandboxMode,
    cwd: &std::path::Path,
) -> Result<McpToolAgentResult, ProviderError> {
    let path = codex_adapter::discover_codex(None)
        .map_err(|e| ProviderError::McpToolAgent(format!("Codex discovery failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&path, &codex_version_req()).await;

    let cli = codex_adapter::CodexCli::new(path);

    // Codex reads MCP server config from its config.toml. Inject via -c overrides.
    let server_name = &mcp_config.name;
    let mut overrides = vec![
        (
            format!("mcp_servers.{server_name}.command"),
            format!("\"{}\"", mcp_config.command),
        ),
        (
            format!("mcp_servers.{server_name}.args"),
            format!("{:?}", mcp_config.args),
        ),
    ];
    for (k, v) in &mcp_config.env {
        overrides.push((
            format!("mcp_servers.{server_name}.env.{k}"),
            format!("\"{v}\""),
        ));
    }

    let config = codex_adapter::CodexConfig {
        full_auto: false,
        sandbox: Some(sandbox_mode.clone()),
        skip_git_repo_check: true,
        cd: Some(cwd.to_path_buf()),
        system_prompt: Some(system_prompt.to_string()),
        overrides,
        timeout,
        ..codex_adapter::CodexConfig::default()
    };

    let result = cli.run(prompt, &config).await.map_err(ProviderError::Codex)?;

    Ok(McpToolAgentResult {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
    })
}

async fn run_opencode(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    system_prompt: &str,
    timeout: Duration,
    cwd: &std::path::Path,
) -> Result<McpToolAgentResult, ProviderError> {
    let path = opencode_adapter::discover_opencode(None)
        .map_err(|e| ProviderError::McpToolAgent(format!("OpenCode discovery failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&path, &opencode_version_req()).await;

    let cli = opencode_adapter::OpenCodeCli::new(path);

    // OpenCode config format: {"mcp": {"name": {"type":"local","command":[...],"environment":{...}}}}
    let mut command = vec![mcp_config.command.clone()];
    command.extend(mcp_config.args.iter().cloned());

    let opencode_cfg = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            &mcp_config.name: {
                "type": "local",
                "command": command,
                "environment": &mcp_config.env,
            }
        }
    });

    let mut config_file = tempfile::NamedTempFile::new()
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp file: {e}")))?;
    let json = serde_json::to_string_pretty(&opencode_cfg)
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to serialize config: {e}")))?;
    config_file
        .write_all(json.as_bytes())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to write config: {e}")))?;

    let config_path = config_file.path().to_path_buf();
    let _config_guard = config_file.into_temp_path();

    let config = opencode_adapter::OpenCodeConfig {
        model: Some("opencode/big-pickle".to_string()),
        prompt: Some(system_prompt.to_string()),
        mcp_config_path: Some(config_path),
        cwd: Some(cwd.to_path_buf()),
        timeout,
        ..opencode_adapter::OpenCodeConfig::default()
    };

    let result = cli.run(prompt, &config).await.map_err(ProviderError::OpenCode)?;

    Ok(McpToolAgentResult {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
    })
}

async fn run_claude_code_stream(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    allowed_tools: &[String],
    system_prompt: &str,
    timeout: Duration,
    builtin_tools: Option<&Vec<String>>,
    cwd: &std::path::Path,
    tx: tokio::sync::mpsc::Sender<McpStreamEvent>,
) -> Result<(), ProviderError> {
    // Write Claude Code MCP config JSON to temp file
    let mut config_file = tempfile::NamedTempFile::new()
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp file: {e}")))?;
    let json = serde_json::to_string_pretty(&mcp_config.to_claude_json())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to serialize config: {e}")))?;
    config_file
        .write_all(json.as_bytes())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to write config: {e}")))?;
    let config_path = config_file.path().to_path_buf();
    let _config_guard = config_file.into_temp_path();

    let report = claudecode_adapter::init(None)
        .await
        .map_err(|e| ProviderError::McpToolAgent(format!("Claude init failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&report.claude_path, &claude_code_version_req()).await;

    let cli = claudecode_adapter::ClaudeCli::new(report.claude_path, report.capabilities);

    // Apply containment: disable all builtins by default, opt-in via builtin_tools
    let builtin_set = builtin_tools.map_or(
        claudecode_adapter::BuiltinToolSet::None,
        |tools| claudecode_adapter::BuiltinToolSet::Explicit(tools.clone()),
    );

    let config = claudecode_adapter::RunConfig {
        output_format: Some(claudecode_adapter::OutputFormat::StreamJson),
        system_prompt: claudecode_adapter::SystemPromptMode::Append(system_prompt.to_string()),
        mcp: Some(claudecode_adapter::McpPolicy {
            configs: vec![config_path.to_string_lossy().to_string()],
            strict: true,
        }),
        tools: claudecode_adapter::ToolPolicy {
            builtin: builtin_set,
            allowed: Some(allowed_tools.to_vec()),
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout,
        cwd: Some(cwd.to_path_buf()),
        ..claudecode_adapter::RunConfig::default()
    };

    // Create internal channel for adapter's native StreamEvent
    let (adapter_tx, mut adapter_rx) = tokio::sync::mpsc::channel::<claudecode_adapter::StreamEvent>(100);

    // Clone prompt for 'static lifetime in spawned task
    let prompt_owned = prompt.to_string();

    // Spawn task to run CLI and convert events
    tokio::spawn(async move {
        // Run the CLI with streaming
        let _result = cli.stream(&prompt_owned, &config, adapter_tx.clone()).await;

        // Convert adapter events to McpStreamEvent
        while let Some(event) = adapter_rx.recv().await {
            let mcp_event = match event {
                claudecode_adapter::StreamEvent::Text { text } => McpStreamEvent::Text(text),
                claudecode_adapter::StreamEvent::ToolCall { name, input } => {
                    McpStreamEvent::ToolCall {
                        name,
                        input: input.to_string()
                    }
                },
                claudecode_adapter::StreamEvent::ToolResult { name, output } => {
                    McpStreamEvent::ToolResult {
                        tool_use_id: name,
                        content: output
                    }
                },
                claudecode_adapter::StreamEvent::Error { message } => McpStreamEvent::Error(message),
                claudecode_adapter::StreamEvent::Unknown(_) => continue, // skip unknowns
            };

            // Send converted event (ignore if receiver dropped)
            let _ = tx.send(mcp_event).await;
        }
    });

    Ok(())
}

async fn run_codex_stream(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    system_prompt: &str,
    timeout: Duration,
    sandbox_mode: &codex_adapter::SandboxMode,
    cwd: &std::path::Path,
    tx: tokio::sync::mpsc::Sender<McpStreamEvent>,
) -> Result<(), ProviderError> {
    let path = codex_adapter::discover_codex(None)
        .map_err(|e| ProviderError::McpToolAgent(format!("Codex discovery failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&path, &codex_version_req()).await;

    let cli = codex_adapter::CodexCli::new(path);

    // Codex reads MCP server config from its config.toml. Inject via -c overrides.
    let server_name = &mcp_config.name;
    let mut overrides = vec![
        (
            format!("mcp_servers.{server_name}.command"),
            format!("\"{}\"", mcp_config.command),
        ),
        (
            format!("mcp_servers.{server_name}.args"),
            format!("{:?}", mcp_config.args),
        ),
    ];
    for (k, v) in &mcp_config.env {
        overrides.push((
            format!("mcp_servers.{server_name}.env.{k}"),
            format!("\"{v}\""),
        ));
    }

    let config = codex_adapter::CodexConfig {
        full_auto: false,
        sandbox: Some(sandbox_mode.clone()),
        skip_git_repo_check: true,
        cd: Some(cwd.to_path_buf()),
        system_prompt: Some(system_prompt.to_string()),
        overrides,
        timeout,
        ..codex_adapter::CodexConfig::default()
    };

    // Create internal channel for adapter's native StreamEvent
    let (adapter_tx, mut adapter_rx) = tokio::sync::mpsc::channel::<codex_adapter::StreamEvent>(100);

    // Clone prompt for 'static lifetime in spawned task
    let prompt_owned = prompt.to_string();

    // Spawn task to run CLI and convert events
    tokio::spawn(async move {
        // Run the CLI with streaming
        let _result = cli.stream(&prompt_owned, &config, adapter_tx.clone()).await;

        // Convert adapter events to McpStreamEvent
        while let Some(event) = adapter_rx.recv().await {
            let mcp_event = match event {
                codex_adapter::StreamEvent::Text { text } => McpStreamEvent::Text(text),
                codex_adapter::StreamEvent::Error { message } => McpStreamEvent::Error(message),
                codex_adapter::StreamEvent::Unknown(_) => continue, // skip unknowns
            };

            // Send converted event (ignore if receiver dropped)
            let _ = tx.send(mcp_event).await;
        }
    });

    Ok(())
}

async fn run_opencode_stream(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    system_prompt: &str,
    timeout: Duration,
    cwd: &std::path::Path,
    tx: tokio::sync::mpsc::Sender<McpStreamEvent>,
) -> Result<(), ProviderError> {
    let path = opencode_adapter::discover_opencode(None)
        .map_err(|e| ProviderError::McpToolAgent(format!("OpenCode discovery failed: {e}")))?;

    // Detect and validate CLI version
    detect_and_validate_version(&path, &opencode_version_req()).await;

    let cli = opencode_adapter::OpenCodeCli::new(path);

    // OpenCode config format: {"mcp": {"name": {"type":"local","command":[...],"environment":{...}}}}
    let mut command = vec![mcp_config.command.clone()];
    command.extend(mcp_config.args.iter().cloned());

    let opencode_cfg = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            &mcp_config.name: {
                "type": "local",
                "command": command,
                "environment": &mcp_config.env,
            }
        }
    });

    let mut config_file = tempfile::NamedTempFile::new()
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp file: {e}")))?;
    let json = serde_json::to_string_pretty(&opencode_cfg)
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to serialize config: {e}")))?;
    config_file
        .write_all(json.as_bytes())
        .map_err(|e| ProviderError::McpToolAgent(format!("Failed to write config: {e}")))?;

    let config_path = config_file.path().to_path_buf();
    let _config_guard = config_file.into_temp_path();

    let config = opencode_adapter::OpenCodeConfig {
        model: Some("opencode/big-pickle".to_string()),
        prompt: Some(system_prompt.to_string()),
        mcp_config_path: Some(config_path),
        cwd: Some(cwd.to_path_buf()),
        timeout,
        ..opencode_adapter::OpenCodeConfig::default()
    };

    // Create internal channel for adapter's native StreamEvent
    let (adapter_tx, mut adapter_rx) = tokio::sync::mpsc::channel::<opencode_adapter::StreamEvent>(100);

    // Clone prompt for 'static lifetime in spawned task
    let prompt_owned = prompt.to_string();

    // Spawn task to run CLI and convert events
    tokio::spawn(async move {
        // Run the CLI with streaming
        let _result = cli.stream(&prompt_owned, &config, adapter_tx.clone()).await;

        // Convert adapter events to McpStreamEvent
        while let Some(event) = adapter_rx.recv().await {
            let mcp_event = match event {
                opencode_adapter::StreamEvent::Text { text } => McpStreamEvent::Text(text),
                opencode_adapter::StreamEvent::Error { message } => McpStreamEvent::Error(message),
                opencode_adapter::StreamEvent::Unknown(_) => continue, // skip unknowns
            };

            // Send converted event (ignore if receiver dropped)
            let _ = tx.send(mcp_event).await;
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_string_simple() {
        assert_eq!(extract_version_string("1.2.3"), "1.2.3");
    }

    #[test]
    fn test_extract_version_string_with_v_prefix() {
        assert_eq!(extract_version_string("v1.2.3"), "1.2.3");
    }

    #[test]
    fn test_extract_version_string_with_cli_name() {
        assert_eq!(extract_version_string("claude 1.2.3"), "1.2.3");
        assert_eq!(extract_version_string("codex v0.91.0"), "0.91.0");
    }

    #[test]
    fn test_extract_version_string_with_prerelease() {
        assert_eq!(extract_version_string("v0.91.0-beta.1"), "0.91.0-beta.1");
    }

    #[test]
    fn test_extract_version_string_unparseable_fallback() {
        // Returns best-effort string even if not valid semver
        let result = extract_version_string("not-a-version");
        assert_eq!(result, "not-a-version");
    }

    #[test]
    fn test_version_requirement_constants() {
        let claude_req = claude_code_version_req();
        assert!(claude_req.min_version < claude_req.max_tested);
        assert_eq!(claude_req.cli_name, "Claude Code");

        let codex_req = codex_version_req();
        assert!(codex_req.min_version < codex_req.max_tested);
        assert_eq!(codex_req.cli_name, "Codex");

        let opencode_req = opencode_version_req();
        assert!(opencode_req.min_version < opencode_req.max_tested);
        assert_eq!(opencode_req.cli_name, "OpenCode");
    }

    #[test]
    fn test_version_comparison_logic() {
        let req = claude_code_version_req();
        let below_min = semver::Version::new(0, 0, 1);
        let in_range = semver::Version::new(1, 5, 0);
        let above_max = semver::Version::new(2, 0, 0);

        assert!(below_min < req.min_version);
        assert!(in_range >= req.min_version && in_range <= req.max_tested);
        assert!(above_max > req.max_tested);
    }
}
