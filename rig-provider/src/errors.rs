use thiserror::Error;

/// Errors relating to the Rig Provider.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Error from the Claude Code adapter.
    #[error("Claude adapter error: {0}")]
    Claude(#[from] rig_cli_claude::ClaudeError),

    /// Error from the Codex adapter.
    #[error("Codex adapter error: {0}")]
    Codex(#[from] rig_cli_codex::CodexError),

    /// Error from the `OpenCode` adapter.
    #[error("OpenCode adapter error: {0}")]
    OpenCode(#[from] rig_cli_opencode::OpenCodeError),

    /// Session management error.
    #[error("Session management error: {0}")]
    Session(String),

    /// Initialization error.
    #[error("Initialization error: {0}")]
    Init(String),

    /// Discovery error.
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Anyhow error.
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// Error from the MCP tool agent builder.
    #[error("MCP tool agent error: {0}")]
    McpToolAgent(String),
}
