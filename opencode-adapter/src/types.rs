use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub print_logs: bool,
    pub log_level: Option<String>,
    pub port: Option<u16>,
    pub hostname: Option<String>,
    pub timeout: Duration,
    pub cwd: Option<PathBuf>,
}

impl Default for OpenCodeConfig {
    fn default() -> Self {
        Self {
            model: None,
            prompt: None,
            print_logs: false,
            log_level: None,
            port: None,
            hostname: None,
            timeout: Duration::from_secs(300),
            cwd: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Events streamed from the OpenCode CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// A chunk of text content.
    Text { 
        /// The text content.
        text: String 
    },
    /// An error event.
    Error { 
        /// The error message.
        message: String 
    },
    /// An unknown event type.
    Unknown(serde_json::Value),
}
