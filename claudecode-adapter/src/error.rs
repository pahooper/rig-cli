use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClaudeError {
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

    #[error("Failed to spawn process: {0}")]
    SpawnFailed(#[from] std::io::Error),

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

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Other error: {0}")]
    Other(String),
}
