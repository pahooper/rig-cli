//! Error types for the Claude Code adapter.

use std::process::ExitStatus;
use thiserror::Error;

/// All errors that can occur during Claude CLI adapter operations.
#[derive(Debug, Error)]
pub enum ClaudeError {
    /// The Claude executable was not found at the expected location.
    #[error("Claude executable not found: {0}")]
    ExecutableNotFound(String),

    /// Running `claude --version` failed.
    #[error("Failed to check version: {0}")]
    VersionCheckFailed(String),

    /// The `claude doctor` health check reported a failure.
    #[error("Claude doctor failed: {stdout}")]
    DoctorFailed {
        /// Captured stdout from the doctor command.
        stdout: String,
        /// Captured stderr from the doctor command.
        stderr: String,
        /// Exit status of the doctor process.
        status: ExitStatus,
    },

    /// JSON output from the CLI could not be parsed.
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(String),

    /// The supplied configuration is invalid.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// The subprocess could not be spawned or waited on.
    #[error("Failed to spawn process at stage '{stage}': {source}")]
    SpawnFailed {
        /// Lifecycle stage where the I/O error occurred.
        stage: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The subprocess exceeded its configured timeout.
    #[error("Process timed out after {elapsed:?} (PID: {pid})\nPartial stdout: {partial_stdout}\nPartial stderr: {partial_stderr}")]
    Timeout {
        /// How long the process ran before being killed.
        elapsed: std::time::Duration,
        /// Operating-system PID of the timed-out process.
        pid: u32,
        /// Any stdout captured before the timeout.
        partial_stdout: String,
        /// Any stderr captured before the timeout.
        partial_stderr: String,
    },

    /// The subprocess exited with a non-zero status code.
    #[error("Process exited with non-zero status: {exit_code} (PID: {pid}, elapsed: {elapsed:?})\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
    NonZeroExit {
        /// The non-zero exit code.
        exit_code: i32,
        /// Operating-system PID.
        pid: u32,
        /// Wall-clock duration of the process.
        elapsed: std::time::Duration,
        /// Captured stdout.
        stdout: String,
        /// Captured stderr.
        stderr: String,
    },

    /// A background reader task failed to join.
    #[error("Stream reader task failed at stage '{stage}': {source}")]
    StreamFailed {
        /// Lifecycle stage where the join error occurred.
        stage: String,
        /// Underlying join error.
        #[source]
        source: tokio::task::JoinError,
    },

    /// Sending a POSIX signal to the subprocess failed.
    #[error("Failed to send {signal} signal to PID {pid}: {source}")]
    SignalFailed {
        /// Signal name (e.g. `SIGTERM`).
        signal: String,
        /// Target PID.
        pid: u32,
        /// Underlying errno.
        #[source]
        source: nix::errno::Errno,
    },

    /// The subprocess stdout pipe was unexpectedly `None`.
    #[error("Subprocess stdout pipe is None (child.stdout.take() returned None)")]
    NoStdout,

    /// The subprocess stderr pipe was unexpectedly `None`.
    #[error("Subprocess stderr pipe is None (child.stderr.take() returned None)")]
    NoStderr,

    /// The subprocess PID was unavailable.
    #[error("Subprocess PID is None (child.id() returned None)")]
    NoPid,

    /// Output exceeded the configured size limit.
    #[error("Output truncated: captured {captured_bytes} bytes, limit {limit_bytes} bytes")]
    OutputTruncated {
        /// Number of bytes captured before truncation.
        captured_bytes: usize,
        /// Configured maximum byte limit.
        limit_bytes: usize,
    },

    /// An internal channel closed before the operation completed.
    #[error("Internal channel closed unexpectedly at stage '{stage}'")]
    ChannelClosed {
        /// Lifecycle stage where the channel closed.
        stage: String,
    },
}

impl From<std::io::Error> for ClaudeError {
    fn from(source: std::io::Error) -> Self {
        Self::SpawnFailed {
            stage: "io".to_string(),
            source,
        }
    }
}
