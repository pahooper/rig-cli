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
