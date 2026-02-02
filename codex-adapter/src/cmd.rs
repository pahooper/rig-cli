//! Command-line argument construction for the Codex CLI.

use crate::types::{ApprovalPolicy, CodexConfig, SandboxMode};
use std::ffi::OsString;

/// Builds the argument list for a Codex CLI invocation.
#[must_use]
pub fn build_args(prompt: &str, config: &CodexConfig) -> Vec<OsString> {
    let mut args = Vec::new();

    args.push(OsString::from("exec"));

    if let Some(ref model) = config.model {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }

    if let Some(ref sandbox) = config.sandbox {
        args.push(OsString::from("--sandbox"));
        match sandbox {
            SandboxMode::ReadOnly => args.push(OsString::from("read-only")),
            SandboxMode::WorkspaceWrite => args.push(OsString::from("workspace-write")),
            SandboxMode::DangerFullAccess => args.push(OsString::from("danger-full-access")),
        }
    }

    if let Some(ref approval) = config.ask_for_approval {
        args.push(OsString::from("--ask-for-approval"));
        match approval {
            ApprovalPolicy::Untrusted => args.push(OsString::from("untrusted")),
            ApprovalPolicy::OnFailure => args.push(OsString::from("on-failure")),
            ApprovalPolicy::OnRequest => args.push(OsString::from("on-request")),
            ApprovalPolicy::Never => args.push(OsString::from("never")),
        }
    }

    if config.full_auto {
        args.push(OsString::from("--full-auto"));
    }

    if config.search {
        args.push(OsString::from("--search"));
    }

    if let Some(ref cd) = config.cd {
        args.push(OsString::from("--cd"));
        args.push(OsString::from(cd));
    }

    for dir in &config.add_dirs {
        args.push(OsString::from("--add-dir"));
        args.push(OsString::from(dir));
    }

    for (k, v) in &config.overrides {
        args.push(OsString::from("--config"));
        args.push(OsString::from(format!("{k}={v}")));
    }

    // Codex has no --system-prompt flag; prepend to the user prompt.
    let effective_prompt = config
        .system_prompt
        .as_ref()
        .map_or_else(|| prompt.to_string(), |sp| format!("{sp}\n\n{prompt}"));
    args.push(OsString::from(effective_prompt));

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ApprovalPolicy, CodexConfig, SandboxMode};
    use std::path::PathBuf;

    // NOTE: Codex Issue #4152 -- MCP tools bypass sandbox restrictions.
    // This means our MCP tools (submit/validate/example) are NOT subject to
    // Codex's Landlock sandbox enforcement. For strong isolation, use
    // Docker Sandboxes externally. This is a known Codex bug, not a rig-cli issue.

    #[test]
    fn test_sandbox_readonly_flag() {
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --sandbox read-only is present (CONT-04)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_sandbox_workspace_write_flag() {
        let config = CodexConfig {
            sandbox: Some(SandboxMode::WorkspaceWrite),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --sandbox workspace-write is present
        assert!(
            args_str.windows(2).any(|w| w[0] == "--sandbox" && w[1] == "workspace-write"),
            "Expected '--sandbox workspace-write' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_approval_never_flag() {
        let config = CodexConfig {
            ask_for_approval: Some(ApprovalPolicy::Never),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --ask-for-approval never is present (non-interactive)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--ask-for-approval" && w[1] == "never"),
            "Expected '--ask-for-approval never' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_cd_flag() {
        let config = CodexConfig {
            cd: Some(PathBuf::from("/tmp/sandbox")),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --cd is present (working directory isolation)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--cd" && w[1] == "/tmp/sandbox"),
            "Expected '--cd /tmp/sandbox' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_full_auto_not_set_by_default() {
        let config = CodexConfig::default();
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --full-auto is NOT present (CONT-03 audit: full_auto bypasses containment)
        assert!(
            !args_str.contains(&"--full-auto"),
            "Expected --full-auto to be absent by default, but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_full_containment_config() {
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ask_for_approval: Some(ApprovalPolicy::Never),
            cd: Some(PathBuf::from("/tmp/isolated")),
            full_auto: false, // explicit false to document containment posture
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert all containment flags present and --full-auto absent
        assert!(
            args_str.windows(2).any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only'"
        );
        assert!(
            args_str.windows(2).any(|w| w[0] == "--ask-for-approval" && w[1] == "never"),
            "Expected '--ask-for-approval never'"
        );
        assert!(
            args_str.windows(2).any(|w| w[0] == "--cd" && w[1] == "/tmp/isolated"),
            "Expected '--cd /tmp/isolated'"
        );
        assert!(
            !args_str.contains(&"--full-auto"),
            "Expected --full-auto to be absent"
        );
    }
}
