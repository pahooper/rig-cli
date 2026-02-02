//! MCP server implementation bridging Rig and RMCP.

use rig::completion::ToolDefinition;
use rig::tool::server::{ToolServerError, ToolServerHandle};
use rig::tool::{ToolError, ToolSet, ToolSetError};
use rmcp::RoleServer;
use rmcp::service::RequestContext;
use rmcp::{
    ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, ErrorData, JsonObject, ListToolsResult,
        PaginatedRequestParams, Tool as McpTool,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;

/// Internal abstraction for the source of tools.
enum ToolSource {
    Set(ToolSet),
    Server(ToolServerHandle),
}

/// MCP server handler that serves tools from a Rig `ToolSet` or `ToolServer`.
pub struct RigMcpHandler {
    source: ToolSource,
    /// The name of the server (e.g. "rig-mcp-server").
    pub name: String,
    /// Pre-computed tool definitions.
    pub tool_definitions: Vec<McpTool>,
}

impl RigMcpHandler {
    /// Returns a new builder for configuring the handler.
    #[must_use]
    pub fn builder() -> RigMcpHandlerBuilder {
        RigMcpHandlerBuilder::default()
    }

    /// Creates a new `RigMcpHandler` from a `ToolSet` by automatically extracting tool definitions.
    ///
    /// # Errors
    /// Returns `ToolSetError` if fetching tool definitions from the `ToolSet` fails.
    pub async fn from_toolset(toolset: ToolSet) -> Result<Self, ToolSetError> {
        Self::builder().toolset(toolset).build().await
    }

    /// Creates a new `RigMcpHandler` from a `ToolServerHandle` by automatically extracting tool definitions.
    ///
    /// # Errors
    /// Returns `ToolServerError` if fetching tool definitions from the server fails.
    pub async fn from_tool_server(handle: ToolServerHandle) -> Result<Self, ToolServerError> {
        Self::builder()
            .tool_server(handle)
            .build_from_server()
            .await
    }

    /// Converts a Rig `ToolDefinition` into an MCP tool definition.
    #[must_use]
    pub fn definition_to_mcp(definition: ToolDefinition) -> McpTool {
        let input_schema = if let Value::Object(map) = definition.parameters {
            Arc::new(map)
        } else {
            Arc::new(JsonObject::new())
        };

        McpTool {
            name: Cow::Owned(definition.name.clone()),
            title: Some(definition.name),
            description: Some(Cow::Owned(definition.description)),
            input_schema,
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    /// Returns the configuration details for this MCP server.
    ///
    /// This provides programmatic access to the executable path, server name, and arguments,
    /// allowing you to automate the creation or editing of configuration files (e.g. `~/.claude.json`).
    ///
    /// # Errors
    /// Returns an error if the current executable path cannot be determined.
    pub fn config(&self) -> Result<McpConfig, std::io::Error> {
        let exe = std::env::current_exe()?;
        Ok(McpConfig {
            name: self.name.clone(),
            command: exe.to_string_lossy().to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
        })
    }

    /// Starts the server over stdio and prints configuration details for popular MCP clients.
    ///
    /// This method outputs ready-to-use configuration snippets to `stderr` and then blocks
    /// while serving the protocol.
    ///
    /// # Errors
    /// Returns an error if the executable path cannot be determined or if the server fails to start.
    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config()?;

        eprintln!("\n\x1b[1;36m🚀 Rig MCP Server Starting...\x1b[0m");
        eprintln!(
            "\n\x1b[1mTo use this server with your favorite tools, add the following to your config:\x1b[0m"
        );

        // `Claude` format
        eprintln!("\n\x1b[32m--- Claude Code (~/.claude.json) ---\x1b[0m");
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&config.to_claude_json())?
        );

        // `Codex` format
        eprintln!("\n\x1b[32m--- Codex (~/.codex/config.toml) ---\x1b[0m");
        eprintln!("{}", config.to_codex_toml());

        // `OpenCode` format
        eprintln!("\n\x1b[32m--- OpenCode (opencode.json) ---\x1b[0m");
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&config.to_opencode_json())?
        );

        eprintln!("\n\x1b[1;34mStarting stdio bridge...\x1b[0m\n");

        self.serve_stdio().await?;

        Ok(())
    }

    /// Serves the MCP protocol over stdio without printing configuration details.
    ///
    /// This method is intended to be used when you want to run the server as part of a larger
    /// application, for example by spawning it in a background task.
    ///
    /// # Errors
    /// Returns an error if the server fails to initialize or if the connection is lost.
    pub async fn serve_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let service = rmcp::ServiceExt::serve(self, (stdin, stdout)).await?;
        service.waiting().await?;
        Ok(())
    }
}

/// Programmatic configuration details for an MCP server.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpConfig {
    /// The name of the server (e.g. "rig-mcp-server").
    pub name: String,
    /// The absolute path to the executable.
    pub command: String,
    /// Arguments to pass to the executable.
    pub args: Vec<String>,
    /// Environment variables for the server process.
    pub env: std::collections::HashMap<String, String>,
}

impl McpConfig {
    /// Returns the configuration in `Claude` Code JSON format.
    /// This typically goes into `~/.claude.json` or `.mcp.json`.
    #[must_use]
    pub fn to_claude_json(&self) -> serde_json::Value {
        serde_json::json!({
            "mcpServers": {
                &self.name: {
                    "command": &self.command,
                    "args": &self.args,
                    "env": &self.env
                }
            }
        })
    }

    /// Returns the configuration in `Codex` TOML format.
    /// This typically goes into `~/.codex/config.toml`.
    #[must_use]
    pub fn to_codex_toml(&self) -> String {
        let mut toml = format!(
            "[mcp_servers.{}]\ncommand = \"{}\"\nargs = {:?}",
            self.name, self.command, self.args
        );

        if !self.env.is_empty() {
            use std::fmt::Write;
            let _ = writeln!(toml, "\n[mcp_servers.{}.env]", self.name);
            for (k, v) in &self.env {
                let _ = writeln!(toml, "{k} = \"{v}\"");
            }
        }
        toml
    }

    /// Returns the configuration in `OpenCode` JSON format.
    /// This typically goes into `opencode.json`.
    #[must_use]
    pub fn to_opencode_json(&self) -> serde_json::Value {
        // OpenCode follows the standard mcpServers object in opencode.json
        serde_json::json!({
            "mcpServers": {
                &self.name: {
                    "command": &self.command,
                    "args": &self.args,
                    "env": &self.env
                }
            }
        })
    }
}

/// Builder for `RigMcpHandler`.
pub struct RigMcpHandlerBuilder {
    toolset: Option<ToolSet>,
    tool_server: Option<ToolServerHandle>,
    name: String,
}

impl Default for RigMcpHandlerBuilder {
    fn default() -> Self {
        Self {
            toolset: None,
            tool_server: None,
            name: "rig-mcp-server".to_string(),
        }
    }
}

impl RigMcpHandlerBuilder {
    /// Sets the `ToolSet` as the source of tools.
    #[must_use]
    pub fn toolset(mut self, toolset: ToolSet) -> Self {
        self.toolset = Some(toolset);
        self
    }

    /// Sets the `ToolServerHandle` as the source of tools.
    #[must_use]
    pub fn tool_server(mut self, handle: ToolServerHandle) -> Self {
        self.tool_server = Some(handle);
        self
    }

    /// Sets the name of the MCP server. Defaults to "rig-mcp-server".
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Builds the handler from a `ToolSet`.
    ///
    /// # Errors
    /// Returns `ToolSetError` if the toolset is missing or if fetching definitions fails.
    pub async fn build(self) -> Result<RigMcpHandler, ToolSetError> {
        let toolset = self.toolset.ok_or_else(|| {
            ToolSetError::ToolCallError(ToolError::ToolCallError(
                "ToolSet is required for build(); call .toolset() first".into(),
            ))
        })?;
        let definitions = toolset.get_tool_definitions().await?;
        let tool_definitions = definitions
            .into_iter()
            .map(RigMcpHandler::definition_to_mcp)
            .collect();
        Ok(RigMcpHandler {
            source: ToolSource::Set(toolset),
            name: self.name,
            tool_definitions,
        })
    }

    /// Builds the handler from a `ToolServerHandle`.
    ///
    /// # Errors
    /// Returns `ToolServerError` if the server handle is missing or if fetching definitions fails.
    pub async fn build_from_server(self) -> Result<RigMcpHandler, ToolServerError> {
        let handle = self.tool_server.ok_or_else(|| {
            ToolServerError::ToolsetError(ToolSetError::ToolCallError(ToolError::ToolCallError(
                "ToolServerHandle is required for build_from_server(); call .tool_server() first"
                    .into(),
            )))
        })?;
        let definitions = handle.get_tool_defs(None).await?;
        let tool_definitions = definitions
            .into_iter()
            .map(RigMcpHandler::definition_to_mcp)
            .collect();
        Ok(RigMcpHandler {
            source: ToolSource::Server(handle),
            name: self.name,
            tool_definitions,
        })
    }
}

/// Extension trait for `ToolSet` to provide MCP integration.
#[async_trait::async_trait]
pub trait ToolSetExt {
    /// Converts the `ToolSet` into a `RigMcpHandler`.
    async fn into_handler(self) -> Result<RigMcpHandler, ToolSetError>;

    /// Converts the `ToolSet` into a handler and starts it over stdio with auto-generated CLI configs.
    async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns the configuration details for this `ToolSet` when served as an MCP server.
    async fn config(&self) -> Result<McpConfig, std::io::Error>;
}

#[async_trait::async_trait]
impl ToolSetExt for ToolSet {
    async fn into_handler(self) -> Result<RigMcpHandler, ToolSetError> {
        RigMcpHandler::from_toolset(self).await
    }

    async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        self.into_handler().await?.run_stdio().await
    }

    async fn config(&self) -> Result<McpConfig, std::io::Error> {
        let exe = std::env::current_exe()?;
        Ok(McpConfig {
            name: "rig-mcp-server".to_string(),
            command: exe.to_string_lossy().to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
        })
    }
}

impl ServerHandler for RigMcpHandler {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: self.name.clone(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
            instructions: None,
        }
    }

    async fn initialize(
        &self,
        _request: rmcp::model::InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, ErrorData> {
        Ok(self.get_info())
    }

    #[tracing::instrument(skip(self, _request, _context), fields(rpc.method = "list_tools"))]
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: self.tool_definitions.clone(),
            next_cursor: None,
            meta: None,
        })
    }

    #[tracing::instrument(skip(self, request, _context), fields(rpc.method = "call_tool", tool.name = %request.name))]
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let args_str = request
            .arguments
            .as_ref()
            .map_or_else(String::new, |a| Value::Object(a.clone()).to_string());

        tracing::debug!(target: "rig", tool_name = %request.name, "Calling tool via MCP bridge");

        let result = match &self.source {
            ToolSource::Set(set) => set
                .call(&request.name, args_str)
                .await
                .map_err(|e| e.to_string()),
            ToolSource::Server(server) => server
                .call_tool(&request.name, &args_str)
                .await
                .map_err(|e| e.to_string()),
        };

        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(e) => {
                tracing::error!(target: "rig", tool_name = %request.name, error = %e, "Tool call failed");
                Ok(CallToolResult::error(vec![Content::text(e)]))
            }
        }
    }
}
