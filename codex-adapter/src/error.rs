//! Error types for the Codex adapter.

use thiserror::Error;

/// Errors that can occur when interacting with the Codex CLI.
#[derive(Debug, Error)]
pub enum CodexError {
    /// The Codex executable was not found at the expected path.
    #[error("Codex executable not found: {0}")]
    ExecutableNotFound(String),

    /// Path lookup via `which` failed.
    #[error("Executable not found via which: {0}")]
    WhichError(#[from] which::Error),

    /// A subprocess I/O operation failed.
    #[error("Failed to spawn process at stage {stage}: {source}")]
    SpawnFailed {
        /// The lifecycle stage where the failure occurred.
        stage: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The subprocess exceeded its configured timeout.
    #[error("Process timed out after {elapsed:?} (PID: {pid})\nPartial STDOUT: {partial_stdout}\nPartial STDERR: {partial_stderr}")]
    Timeout {
        /// How long the process ran before being killed.
        elapsed: std::time::Duration,
        /// OS process identifier.
        pid: u32,
        /// Stdout captured before the timeout.
        partial_stdout: String,
        /// Stderr captured before the timeout.
        partial_stderr: String,
    },

    /// The subprocess exited with a non-zero status.
    #[error("Process exited with non-zero status: {exit_code} (PID: {pid}, elapsed: {elapsed:?})\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
    NonZeroExit {
        /// The non-zero exit code.
        exit_code: i32,
        /// OS process identifier.
        pid: u32,
        /// How long the process ran.
        elapsed: std::time::Duration,
        /// Captured standard output.
        stdout: String,
        /// Captured standard error.
        stderr: String,
    },

    /// A reader task failed to join.
    #[error("Stream processing failed at stage {stage}: {source}")]
    StreamFailed {
        /// The lifecycle stage where the failure occurred.
        stage: String,
        /// The underlying join error.
        #[source]
        source: tokio::task::JoinError,
    },

    /// Sending a signal to the subprocess failed.
    #[error("Failed to send signal {signal} to PID {pid}: {reason}")]
    SignalFailed {
        /// The signal name (e.g. `"SIGTERM"`).
        signal: String,
        /// OS process identifier.
        pid: u32,
        /// Platform-specific error description.
        reason: String,
    },

    /// Subprocess stdout handle was `None`.
    #[error("Subprocess stdout was None")]
    NoStdout,

    /// Subprocess stderr handle was `None`.
    #[error("Subprocess stderr was None")]
    NoStderr,

    /// Subprocess PID was `None`.
    #[error("Subprocess PID was None")]
    NoPid,

    /// Output exceeded the capture limit and was truncated.
    #[error("Output truncated: captured {captured_bytes} bytes, limit {limit_bytes} bytes")]
    OutputTruncated {
        /// Bytes captured before truncation.
        captured_bytes: usize,
        /// Maximum allowed bytes.
        limit_bytes: usize,
    },

    /// An internal channel was closed unexpectedly.
    #[error("Channel closed at stage {stage}")]
    ChannelClosed {
        /// The lifecycle stage where the channel closed.
        stage: String,
    },
}

impl From<std::io::Error> for CodexError {
    fn from(err: std::io::Error) -> Self {
        Self::SpawnFailed {
            stage: "io".to_string(),
            source: err,
        }
    }
}
