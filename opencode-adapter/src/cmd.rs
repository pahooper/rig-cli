//! Command-line argument builder for OpenCode CLI invocations.
//!
//! ## Flag Reference
//!
//! ### Model and Output Flags
//! - `--model <model>`: Model selection (e.g., opencode/big-pickle)
//! - `--print-logs`: Enable debug log output
//! - `--log-level <level>`: Log verbosity (debug, info, warn, error)
//!
//! ### Server Flags (for OpenCode server mode)
//! - `--port <port>`: Server port override
//! - `--hostname <host>`: Server hostname override
//!
//! ## Containment Strategy
//!
//! **IMPORTANT:** OpenCode has no CLI flags for sandbox, approval policy, or tool
//! restriction (unlike Claude Code and Codex). Containment is achieved through:
//!
//! 1. **Working Directory Isolation**: `Command::current_dir()` sets cwd, not a CLI arg
//! 2. **MCP Config via Environment**: `OPENCODE_CONFIG` env var points to config file
//! 3. **System Prompt Prepending**: No `--system-prompt` flag; prompt is prepended to message
//!
//! ### Containment Comparison
//! | Feature | Claude Code | Codex | OpenCode |
//! |---------|-------------|-------|----------|
//! | Sandbox | --tools "" | --sandbox | (none) |
//! | Tool restriction | --allowed-tools | (none) | (none) |
//! | Working dir | --cwd | --cd | Command::current_dir() |
//! | MCP config | --mcp-config | -c overrides | OPENCODE_CONFIG env |
//! | System prompt | --system-prompt | (prepend) | (prepend) |
//!
//! ## Version Notes
//! - `run` subcommand: Standard execution mode
//! - `--model`: Supports opencode/big-pickle and other available models
//! - No version-specific flags known; OpenCode CLI has minimal flag surface
//!
//! ## Known Limitations
//! - No filesystem sandbox mechanism (containment relies on process isolation)
//! - No tool restriction flags (all configured tools are available)
//! - System prompt must be prepended to user message (no dedicated flag)
//!
//! ## External References
//! - [OpenCode Documentation](https://opencode.ai/docs/)
//! - [OpenCode MCP Servers](https://opencode.ai/docs/mcp-servers/)

use crate::types::OpenCodeConfig;
use std::ffi::OsString;

/// Builds the argument list for an `OpenCode` subprocess invocation.
#[must_use]
pub fn build_args(message: &str, config: &OpenCodeConfig) -> Vec<OsString> {
    let mut args = Vec::new();

    args.push(OsString::from("run"));

    if let Some(ref model) = config.model {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }

    if config.print_logs {
        args.push(OsString::from("--print-logs"));
    }

    if let Some(ref level) = config.log_level {
        args.push(OsString::from("--log-level"));
        args.push(OsString::from(level));
    }

    if let Some(port) = config.port {
        args.push(OsString::from("--port"));
        args.push(OsString::from(port.to_string()));
    }

    if let Some(ref host) = config.hostname {
        args.push(OsString::from("--hostname"));
        args.push(OsString::from(host));
    }

    // OpenCode has no --system-prompt flag; prepend to the user message.
    let effective_message = config
        .prompt
        .as_ref()
        .map_or_else(|| message.to_string(), |sp| format!("{sp}\n\n{message}"));
    args.push(OsString::from(effective_message));

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OpenCodeConfig;

    // NOTE: OpenCode has no sandbox, builtin-tool-disable, or strict-MCP flags.
    // Containment is best-effort through system prompt enforcement and
    // working directory isolation (set via Command::current_dir, not CLI args).
    // MCP config is delivered via OPENCODE_CONFIG env var, not CLI args.

    #[test]
    fn test_default_config_generates_run_subcommand() {
        let config = OpenCodeConfig::default();
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert_eq!(args_str[0], "run", "First arg must be 'run' subcommand");
        assert_eq!(
            args_str.last().unwrap(),
            &"test prompt",
            "Last arg must be the prompt"
        );
        // Default config should only produce: run <prompt>
        assert_eq!(args_str.len(), 2, "Default config should produce exactly 2 args: {:?}", args_str);
    }

    #[test]
    fn test_model_flag() {
        let config = OpenCodeConfig {
            model: Some("opencode/big-pickle".to_string()),
            ..OpenCodeConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str.windows(2).any(|w| w[0] == "--model" && w[1] == "opencode/big-pickle"),
            "Expected '--model opencode/big-pickle' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_system_prompt_prepended_to_message() {
        let config = OpenCodeConfig {
            prompt: Some("You are a data extractor.".to_string()),
            ..OpenCodeConfig::default()
        };
        let args = build_args("Extract this data.", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        let last = args_str.last().unwrap();
        assert!(
            last.starts_with("You are a data extractor."),
            "System prompt must be prepended: {:?}",
            last
        );
        assert!(
            last.ends_with("Extract this data."),
            "User message must follow system prompt: {:?}",
            last
        );
    }

    #[test]
    fn test_print_logs_flag() {
        let config = OpenCodeConfig {
            print_logs: true,
            ..OpenCodeConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str.contains(&"--print-logs"),
            "Expected '--print-logs' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_log_level_flag() {
        let config = OpenCodeConfig {
            log_level: Some("DEBUG".to_string()),
            ..OpenCodeConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str.windows(2).any(|w| w[0] == "--log-level" && w[1] == "DEBUG"),
            "Expected '--log-level DEBUG' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_containment_is_prompt_and_process_only() {
        // OpenCode containment relies on:
        // 1. System prompt enforcement (no --tools or --strict-mcp-config flags)
        // 2. Working directory via Command::current_dir() (not a CLI arg)
        // 3. MCP config via OPENCODE_CONFIG env var (not a CLI arg)
        //
        // Verify that cwd and mcp_config_path do NOT appear in CLI args:
        let config = OpenCodeConfig {
            cwd: Some(std::path::PathBuf::from("/tmp/isolated")),
            mcp_config_path: Some(std::path::PathBuf::from("/tmp/mcp.json")),
            ..OpenCodeConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            !args_str.contains(&"--cd"),
            "cwd should be set via Command::current_dir, not --cd"
        );
        assert!(
            !args_str.iter().any(|a| a.contains("/tmp/isolated")),
            "cwd path should not appear in args"
        );
        assert!(
            !args_str.iter().any(|a| a.contains("mcp.json")),
            "MCP config path should be set via OPENCODE_CONFIG env var, not args"
        );
    }
}
