//! Error types for the `OpenCode` adapter.

use thiserror::Error;

/// Errors that can occur when running or managing the `OpenCode` CLI.
#[derive(Debug, Error)]
pub enum OpenCodeError {
    /// The `OpenCode` executable was not found at the expected location.
    #[error("OpenCode executable not found: {0}")]
    ExecutableNotFound(
        /// Path or description of where the binary was expected.
        String,
    ),

    /// The `which` lookup for the binary failed.
    #[error("Executable not found: {0}")]
    WhichError(#[from] which::Error),

    /// Failed to spawn or wait on the child process.
    #[error("Failed to spawn process at stage '{stage}': {source}")]
    SpawnFailed {
        /// Human-readable label for the lifecycle stage that failed.
        stage: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The child process exceeded the configured timeout.
    #[error("Process timed out after {elapsed:?} (PID: {pid})")]
    Timeout {
        /// Wall-clock time elapsed before the timeout fired.
        elapsed: std::time::Duration,
        /// OS process identifier of the timed-out child.
        pid: u32,
        /// Stdout captured before the timeout.
        partial_stdout: String,
        /// Stderr captured before the timeout.
        partial_stderr: String,
    },

    /// The child process exited with a non-zero status code.
    #[error("Process exited with code {exit_code} (PID: {pid}, elapsed: {elapsed:?})\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
    NonZeroExit {
        /// The non-zero exit code.
        exit_code: i32,
        /// OS process identifier.
        pid: u32,
        /// Wall-clock time the process ran.
        elapsed: std::time::Duration,
        /// Full captured stdout.
        stdout: String,
        /// Full captured stderr.
        stderr: String,
    },

    /// A background stream-reader task failed.
    #[error("Stream task failed at stage '{stage}': {source}")]
    StreamFailed {
        /// Lifecycle stage label.
        stage: String,
        /// The join error from the failed task.
        #[source]
        source: tokio::task::JoinError,
    },

    /// Sending a signal to the child process failed.
    #[error("Failed to send signal {signal} to PID {pid}: {source}")]
    SignalFailed {
        /// Signal name (e.g. `SIGTERM`).
        signal: String,
        /// OS process identifier.
        pid: u32,
        /// The errno returned by the signal call.
        #[source]
        source: nix::errno::Errno,
    },

    /// Child stdout pipe was not captured.
    #[error("Child process stdout was not captured")]
    NoStdout,

    /// Child stderr pipe was not captured.
    #[error("Child process stderr was not captured")]
    NoStderr,

    /// Could not retrieve the PID from the spawned child.
    #[error("Could not get PID from child process")]
    NoPid,

    /// Output exceeded the in-memory size limit.
    #[error("Output truncated: captured {captured_bytes} bytes (limit: {limit_bytes} bytes)")]
    OutputTruncated {
        /// Number of bytes captured so far.
        captured_bytes: usize,
        /// Maximum allowed bytes.
        limit_bytes: usize,
    },

    /// An internal channel was closed before the operation finished.
    #[error("Channel closed unexpectedly at stage '{stage}'")]
    ChannelClosed {
        /// Lifecycle stage where the channel closed.
        stage: String,
    },
}

// Manual `From` implementation for `io::Error`.
impl From<std::io::Error> for OpenCodeError {
    fn from(error: std::io::Error) -> Self {
        Self::SpawnFailed {
            stage: "unknown".to_string(),
            source: error,
        }
    }
}
