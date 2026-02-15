//! Command-line argument builder for Codex CLI invocations.
//!
//! ## Flag Reference
//!
//! ### Containment Flags
//! - `-s, --sandbox <mode>`: Filesystem isolation (read-only | workspace-write | danger-full-access)
//! - `-a, --ask-for-approval <policy>`: Approval gating (untrusted | on-failure | on-request | never)
//! - `--full-auto`: Convenience alias (-a on-request, --sandbox workspace-write)
//! - `--dangerously-bypass-approvals-and-sandbox`: Disables ALL containment (extremely dangerous)
//!
//! ### Working Directory Flags
//! - `-C, --cd <dir>`: Set working directory for the agent
//! - `--add-dir <dir>`: Additional writable directories (repeatable)
//! - `--skip-git-repo-check`: Allow non-git directories (needed for temp dir containment)
//!
//! ### Model and Output Flags
//! - `-m, --model <model>`: Model to use (e.g., o4-mini)
//! - `--search`: Enable live web search capability
//! - `-c, --config <key=value>`: Override config values
//!
//! ## Flag Combinations and Compatibility
//!
//! ### Valid Containment Combinations
//! | Combination | Effect |
//! |-------------|--------|
//! | `--sandbox read-only` | Landlock enforces read-only filesystem access |
//! | `--ask-for-approval untrusted` | Only known-safe commands auto-run |
//! | `--sandbox read-only -a untrusted` | Maximum containment (both layers) |
//! | `--sandbox read-only --skip-git-repo-check` | Containment in temp directories |
//!
//! ### Invalid/Conflict Combinations
//! | Combination | Issue |
//! |-------------|-------|
//! | `--full-auto` + `--sandbox read-only` | full-auto overrides to workspace-write |
//! | `--full-auto` + `-a untrusted` | full-auto overrides to on-request |
//! | `--sandbox X` + `--dangerously-bypass...` | Bypass disables sandbox entirely |
//!
//! ## Version Notes
//! - `--ask-for-approval`: Available in Codex CLI 0.92.0+
//! - `--sandbox`: Uses Linux Landlock (may have reduced effect on other platforms)
//! - `--full-auto`: Convenience flag since Codex 0.90.0
//!
//! ## Known Limitations
//! - MCP tools bypass Landlock sandbox restrictions (Codex Issue #4152)
//!   ([GitHub #4152](https://github.com/openai/codex/issues/4152))
//!   For strong isolation, use external Docker sandboxes.
//! - `--dangerously-bypass-approvals-and-sandbox` disables ALL containment
//!
//! ## External References
//! - [Codex CLI Reference](https://developers.openai.com/codex/cli/reference/)

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

    if let Some(ref policy) = config.ask_for_approval {
        args.push(OsString::from("--ask-for-approval"));
        match policy {
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

    if config.skip_git_repo_check {
        args.push(OsString::from("--skip-git-repo-check"));
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
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only' but got: {args_str:?}",
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
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "workspace-write"),
            "Expected '--sandbox workspace-write' but got: {args_str:?}",
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
            args_str
                .windows(2)
                .any(|w| w[0] == "--cd" && w[1] == "/tmp/sandbox"),
            "Expected '--cd /tmp/sandbox' but got: {args_str:?}",
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
            "Expected --full-auto to be absent by default, but got: {args_str:?}",
        );
    }

    #[test]
    fn test_skip_git_repo_check_flag() {
        let config = CodexConfig {
            skip_git_repo_check: true,
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --skip-git-repo-check present (needed for temp dir containment)
        assert!(
            args_str.contains(&"--skip-git-repo-check"),
            "Expected '--skip-git-repo-check' but got: {args_str:?}",
        );
    }

    #[test]
    fn test_full_containment_config() {
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ask_for_approval: Some(ApprovalPolicy::Untrusted),
            skip_git_repo_check: true,
            cd: Some(PathBuf::from("/tmp/isolated")),
            full_auto: false, // explicit false to document containment posture
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert all containment flags present and --full-auto absent
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only'"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
            "Expected '--ask-for-approval untrusted'"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--cd" && w[1] == "/tmp/isolated"),
            "Expected '--cd /tmp/isolated'"
        );
        assert!(
            args_str.contains(&"--skip-git-repo-check"),
            "Expected '--skip-git-repo-check'"
        );
        assert!(
            !args_str.contains(&"--full-auto"),
            "Expected --full-auto to be absent"
        );
    }

    // ==================== Approval Policy Tests ====================

    #[test]
    fn test_approval_policy_untrusted_flag() {
        let config = CodexConfig {
            ask_for_approval: Some(ApprovalPolicy::Untrusted),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
            "Expected '--ask-for-approval untrusted' but got: {args_str:?}",
        );
    }

    #[test]
    fn test_approval_policy_never_flag() {
        let config = CodexConfig {
            ask_for_approval: Some(ApprovalPolicy::Never),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "never"),
            "Expected '--ask-for-approval never' but got: {args_str:?}",
        );
    }

    #[test]
    fn test_sandbox_with_approval_combination() {
        // Both containment layers together
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ask_for_approval: Some(ApprovalPolicy::Untrusted),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only'"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
            "Expected '--ask-for-approval untrusted'"
        );
    }

    #[test]
    fn test_full_containment_with_approval() {
        // Maximum containment: sandbox + approval + skip_git_repo_check + cd
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ask_for_approval: Some(ApprovalPolicy::Untrusted),
            skip_git_repo_check: true,
            cd: Some(PathBuf::from("/tmp/contained")),
            full_auto: false,
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Verify complete containment posture
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only'"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
            "Expected '--ask-for-approval untrusted'"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--cd" && w[1] == "/tmp/contained"),
            "Expected '--cd /tmp/contained'"
        );
        assert!(
            args_str.contains(&"--skip-git-repo-check"),
            "Expected '--skip-git-repo-check'"
        );
        assert!(
            !args_str.contains(&"--full-auto"),
            "Expected --full-auto to be absent for containment"
        );
    }

    #[test]
    fn test_full_auto_excludes_manual_containment() {
        // Document that full_auto overrides manual sandbox/approval settings.
        // This test documents the CONFLICT -- if both are set, full_auto wins at CLI level.
        // The Codex CLI will use on-request + workspace-write regardless of our flags.
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ask_for_approval: Some(ApprovalPolicy::Untrusted),
            full_auto: true, // This overrides the above at CLI level!
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // We still generate all flags (the API doesn't prevent conflicts)
        // but document that --full-auto takes precedence in the Codex CLI
        assert!(
            args_str.contains(&"--full-auto"),
            "Expected --full-auto present"
        );
        // These are still generated but will be overridden by --full-auto
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Sandbox flag still generated (but overridden by full-auto)"
        );
        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
            "Approval flag still generated (but overridden by full-auto)"
        );
    }

    #[test]
    fn test_approval_policy_on_failure_flag() {
        let config = CodexConfig {
            ask_for_approval: Some(ApprovalPolicy::OnFailure),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "on-failure"),
            "Expected '--ask-for-approval on-failure' but got: {args_str:?}",
        );
    }

    #[test]
    fn test_approval_policy_on_request_flag() {
        let config = CodexConfig {
            ask_for_approval: Some(ApprovalPolicy::OnRequest),
            ..CodexConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str
                .windows(2)
                .any(|w| w[0] == "--ask-for-approval" && w[1] == "on-request"),
            "Expected '--ask-for-approval on-request' but got: {args_str:?}",
        );
    }

    #[test]
    fn test_approval_policy_default_is_untrusted() {
        // Verify ApprovalPolicy::default() returns Untrusted (locked decision)
        assert_eq!(ApprovalPolicy::default(), ApprovalPolicy::Untrusted);
    }
}
