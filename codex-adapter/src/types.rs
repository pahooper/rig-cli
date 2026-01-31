use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalPolicy {
    Untrusted,
    OnFailure,
    OnRequest,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    pub model: Option<String>,
    pub sandbox: Option<SandboxMode>,
    pub ask_for_approval: Option<ApprovalPolicy>,
    pub full_auto: bool,
    pub search: bool,
    pub cd: Option<PathBuf>,
    pub add_dirs: Vec<PathBuf>,
    pub overrides: Vec<(String, String)>,
    pub timeout: Duration,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            model: None,
            sandbox: None,
            ask_for_approval: None,
            full_auto: false,
            search: false,
            cd: None,
            add_dirs: Vec::new(),
            overrides: Vec::new(),
            timeout: Duration::from_secs(300),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Text { text: String },
    Error { message: String },
    Unknown(serde_json::Value),
}
