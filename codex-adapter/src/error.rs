use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodexError {
    #[error("Codex executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Failed to spawn process: {0}")]
    SpawnFailed(#[from] std::io::Error),

    #[error("Executable not found: {0}")]
    WhichError(#[from] which::Error),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("Process timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error(
        "Process exited with non-zero status: {exit_code}\nSTDOUT: {stdout}\nSTDERR: {stderr}"
    )]
    NonZeroExit {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },

    #[error("Other error: {0}")]
    Other(String),
}
