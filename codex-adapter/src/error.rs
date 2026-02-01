use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodexError {
    #[error("Codex executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Executable not found via which: {0}")]
    WhichError(#[from] which::Error),

    #[error("Failed to spawn process at stage {stage}: {source}")]
    SpawnFailed {
        stage: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Process timed out after {elapsed:?} (PID: {pid})\nPartial STDOUT: {partial_stdout}\nPartial STDERR: {partial_stderr}")]
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

    #[error("Stream processing failed at stage {stage}: {source}")]
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

    #[error("Subprocess stdout was None")]
    NoStdout,

    #[error("Subprocess stderr was None")]
    NoStderr,

    #[error("Subprocess PID was None")]
    NoPid,

    #[error("Output truncated: captured {captured_bytes} bytes, limit {limit_bytes} bytes")]
    OutputTruncated {
        captured_bytes: usize,
        limit_bytes: usize,
    },

    #[error("Channel closed at stage {stage}")]
    ChannelClosed { stage: String },
}

impl From<std::io::Error> for CodexError {
    fn from(err: std::io::Error) -> Self {
        CodexError::SpawnFailed {
            stage: "io".to_string(),
            source: err,
        }
    }
}
