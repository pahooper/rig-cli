//! MCP tool agent builder for transparent CLI orchestration.
//!
//! Provides [`McpToolAgent`] and its builder, which handle MCP config generation,
//! CLI discovery, tool name computation, and execution across all three supported
//! CLI adapters (Claude Code, Codex, OpenCode).

use crate::errors::ProviderError;
use std::io::Write as _;
use std::time::Duration;

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

        // 5. Build system prompt
        let mcp_instruction = format!(
            "You MUST use the MCP tools to complete this task. \
             Available tools: {}. \
             Do NOT output raw JSON text as your response -- use the tools.",
            allowed_tools.join(", ")
        );
        let full_system_prompt = match self.system_prompt {
            Some(sp) => format!("{sp}\n\n{mcp_instruction}"),
            None => mcp_instruction,
        };

        // 7. Execute per adapter
        match adapter {
            CliAdapter::ClaudeCode => {
                run_claude_code(&prompt, &mcp_config, &allowed_tools, &full_system_prompt, timeout)
                    .await
            }
            CliAdapter::Codex => {
                run_codex(&prompt, &mcp_config, &full_system_prompt, timeout).await
            }
            CliAdapter::OpenCode => {
                run_opencode(&prompt, &mcp_config, &full_system_prompt, timeout).await
            }
        }
    }
}

async fn run_claude_code(
    prompt: &str,
    mcp_config: &rig_mcp_server::server::McpConfig,
    allowed_tools: &[String],
    system_prompt: &str,
    timeout: Duration,
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
    let cli = claudecode_adapter::ClaudeCli::new(report.claude_path, report.capabilities);

    let config = claudecode_adapter::RunConfig {
        output_format: Some(claudecode_adapter::OutputFormat::Text),
        system_prompt: claudecode_adapter::SystemPromptMode::Append(system_prompt.to_string()),
        mcp: Some(claudecode_adapter::McpPolicy {
            configs: vec![config_path.to_string_lossy().to_string()],
            strict: false,
        }),
        tools: claudecode_adapter::ToolPolicy {
            builtin: claudecode_adapter::BuiltinToolSet::Default,
            allowed: Some(allowed_tools.to_vec()),
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout,
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
) -> Result<McpToolAgentResult, ProviderError> {
    let path = codex_adapter::discover_codex()
        .map_err(|e| ProviderError::McpToolAgent(format!("Codex discovery failed: {e}")))?;
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
        full_auto: true,
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
) -> Result<McpToolAgentResult, ProviderError> {
    let path = opencode_adapter::discover_opencode()
        .map_err(|e| ProviderError::McpToolAgent(format!("OpenCode discovery failed: {e}")))?;
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
