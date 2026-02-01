use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenCodeError {
    #[error("OpenCode executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Executable not found: {0}")]
    WhichError(#[from] which::Error),

    #[error("Failed to spawn process at stage '{stage}': {source}")]
    SpawnFailed {
        stage: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Process timed out after {elapsed:?} (PID: {pid})")]
    Timeout {
        elapsed: std::time::Duration,
        pid: u32,
        partial_stdout: String,
        partial_stderr: String,
    },

    #[error("Process exited with code {exit_code} (PID: {pid}, elapsed: {elapsed:?})\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
    NonZeroExit {
        exit_code: i32,
        pid: u32,
        elapsed: std::time::Duration,
        stdout: String,
        stderr: String,
    },

    #[error("Stream task failed at stage '{stage}': {source}")]
    StreamFailed {
        stage: String,
        #[source]
        source: tokio::task::JoinError,
    },

    #[error("Failed to send signal {signal} to PID {pid}: {source}")]
    SignalFailed {
        signal: String,
        pid: u32,
        #[source]
        source: nix::errno::Errno,
    },

    #[error("Child process stdout was not captured")]
    NoStdout,

    #[error("Child process stderr was not captured")]
    NoStderr,

    #[error("Could not get PID from child process")]
    NoPid,

    #[error("Output truncated: captured {captured_bytes} bytes (limit: {limit_bytes} bytes)")]
    OutputTruncated {
        captured_bytes: usize,
        limit_bytes: usize,
    },

    #[error("Channel closed unexpectedly at stage '{stage}'")]
    ChannelClosed { stage: String },
}

// Manual From implementation for io::Error
impl From<std::io::Error> for OpenCodeError {
    fn from(error: std::io::Error) -> Self {
        OpenCodeError::SpawnFailed {
            stage: "unknown".to_string(),
            source: error,
        }
    }
}
