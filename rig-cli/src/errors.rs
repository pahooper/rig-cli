//! Public error types for rig-cli.

use thiserror::Error;

/// Errors that can occur when using CLI-based providers.
///
/// This enum provides user-friendly error messages with actionable guidance
/// for common failure modes (CLI not found, execution failures, configuration errors).
/// Internal provider errors are wrapped with full error chain preserved.
#[derive(Debug, Error)]
pub enum Error {
    /// Claude Code CLI not found on the system.
    #[error("Claude Code CLI not found. Install: npm i -g @anthropic-ai/claude-code")]
    ClaudeNotFound,

    /// Codex CLI not found on the system.
    #[error("Codex CLI not found. Install: npm i -g @openai/codex")]
    CodexNotFound,

    /// `OpenCode` CLI not found on the system.
    #[error("OpenCode CLI not found. Install: go install github.com/nicholasgasior/opencode@latest")]
    OpenCodeNotFound,

    /// CLI execution failed (non-zero exit, timeout, or process error).
    #[error("CLI execution failed: {0}")]
    ExecutionFailed(String),

    /// Error from the internal provider implementation.
    ///
    /// This wraps errors from the rig-provider crate, preserving the full
    /// error chain for debugging while providing actionable Display messages.
    #[error("{0}")]
    Provider(#[from] rig_cli_provider::errors::ProviderError),

    /// Error from Rig's completion system.
    ///
    /// This wraps errors from Rig's core completion machinery, allowing
    /// them to pass through unchanged to maintain compatibility with
    /// Rig's error handling patterns.
    #[error("Rig completion error: {0}")]
    Completion(#[from] rig::completion::CompletionError),

    /// Configuration error (invalid settings or options).
    #[error("Configuration error: {0}")]
    Config(String),
}
