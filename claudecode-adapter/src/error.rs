use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClaudeError {
    // Existing non-subprocess variants
    #[error("Claude executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Failed to check version: {0}")]
    VersionCheckFailed(String),

    #[error("Claude doctor failed: {stdout}")]
    DoctorFailed {
        stdout: String,
        stderr: String,
        status: ExitStatus,
    },

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    // Rich subprocess-specific variants
    #[error("Failed to spawn process at stage '{stage}': {source}")]
    SpawnFailed {
        stage: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Process timed out after {elapsed:?} (PID: {pid})\nPartial stdout: {partial_stdout}\nPartial stderr: {partial_stderr}")]
    Timeout {
        elapsed: std::time::Duration,
        pid: u32,
        partial_stdout: String,
        partial_stderr: String,
    },

    #[error("Process exited with non-zero status: {exit_code} (PID: {pid}, elapsed: {elapsed:?})\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
    NonZeroExit {
        exit_code: i32,
        pid: u32,
        elapsed: std::time::Duration,
        stdout: String,
        stderr: String,
    },

    #[error("Stream reader task failed at stage '{stage}': {source}")]
    StreamFailed {
        stage: String,
        #[source]
        source: tokio::task::JoinError,
    },

    #[error("Failed to send {signal} signal to PID {pid}: {source}")]
    SignalFailed {
        signal: String,
        pid: u32,
        #[source]
        source: nix::errno::Errno,
    },

    #[error("Subprocess stdout pipe is None (child.stdout.take() returned None)")]
    NoStdout,

    #[error("Subprocess stderr pipe is None (child.stderr.take() returned None)")]
    NoStderr,

    #[error("Subprocess PID is None (child.id() returned None)")]
    NoPid,

    #[error("Output truncated: captured {captured_bytes} bytes, limit {limit_bytes} bytes")]
    OutputTruncated {
        captured_bytes: usize,
        limit_bytes: usize,
    },

    #[error("Internal channel closed unexpectedly at stage '{stage}'")]
    ChannelClosed { stage: String },
}

// Provide default conversion from std::io::Error
impl From<std::io::Error> for ClaudeError {
    fn from(source: std::io::Error) -> Self {
        ClaudeError::SpawnFailed {
            stage: "io".to_string(),
            source,
        }
    }
}
