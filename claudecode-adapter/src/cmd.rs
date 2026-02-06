//! Command-line argument builder for Claude CLI invocations.
//!
//! ## Flag Reference
//!
//! ### Output Flags
//! - `--print`: Non-interactive mode, returns output directly
//! - `--output-format <format>`: text | json | stream-json
//! - `--model <model>`: Model selection (e.g., claude-sonnet-4)
//!
//! ### System Prompt Flags
//! - `--system-prompt <prompt>`: Replace default system prompt entirely
//! - `--append-system-prompt <prompt>`: Append to default system prompt
//!
//! ### Tool Control Flags (Containment)
//! - `--tools <list>`: Builtin tool set. `""` disables all, `"Bash,Read"` explicit list
//! - `--allowed-tools <list>`: Allowlist of tool names (MCP format: `mcp__server__tool`)
//! - `--disallowed-tools <list>`: Denylist of tool names
//! - `--disable-slash-commands`: Disable interactive slash commands
//!
//! ### MCP Configuration Flags
//! - `--mcp-config <path>`: Load MCP servers from JSON file
//! - `--strict-mcp-config`: Only use MCP servers from --mcp-config (see Known Limitations)
//!
//! ### JSON Schema Flags
//! - `--json-schema <schema>`: Force JSON output matching schema
//!
//! ## Flag Combinations and Compatibility
//!
//! ### Valid Containment Combinations
//! | Combination | Effect |
//! |-------------|--------|
//! | `--tools ""` | Disable all builtin tools |
//! | `--tools "" --allowed-tools mcp__x__y` | MCP-only mode |
//! | `--tools "" --allowed-tools x --strict-mcp-config` | Full containment |
//! | `--tools "" --disable-slash-commands` | No builtins, no slash commands |
//!
//! ### Invalid/Undefined Combinations
//! | Combination | Issue |
//! |-------------|-------|
//! | Multiple `--tools` flags | Last wins (undefined which) |
//! | `--allowed-tools` without MCP | Empty tool set (no tools available) |
//! | `--tools "X"` + `--tools ""` | Conflict, behavior undefined |
//!
//! ## Version Notes
//! - `--strict-mcp-config`: Added ~v0.45.0
//! - `--disable-slash-commands`: Available in current versions
//! - Flag availability varies by CLI version; check `claude --help`
//!
//! ## Known Limitations
//! - `--strict-mcp-config` does not override `disabledMcpServers` in ~/.claude.json
//!   ([GitHub #14490](https://github.com/anthropics/claude-code/issues/14490))
//! - No CLI flag for sandbox/filesystem restriction (containment is tool-based only)
//!
//! ## External References
//! - [Claude CLI Reference](https://docs.anthropic.com/en/docs/claude-code/cli-reference)

use crate::types::{BuiltinToolSet, JsonSchema, OutputFormat, RunConfig, SystemPromptMode};
use std::ffi::OsString;

/// Builds the argument list for a `claude --print` invocation from the given
/// prompt and configuration.
#[must_use]
pub fn build_args(prompt: &str, config: &RunConfig) -> Vec<OsString> {
    let mut args = Vec::new();

    args.push(OsString::from("--print"));

    if let Some(ref model) = config.model {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }

    if let Some(format) = config.output_format {
        args.push(OsString::from("--output-format"));
        match format {
            OutputFormat::Text => args.push(OsString::from("text")),
            OutputFormat::Json => args.push(OsString::from("json")),
            OutputFormat::StreamJson => {
                args.push(OsString::from("stream-json"));
                // Claude Code requires --verbose when using --print with stream-json
                args.push(OsString::from("--verbose"));
            }
        }
    }

    match &config.system_prompt {
        SystemPromptMode::None => {}
        SystemPromptMode::Append(p) => {
            args.push(OsString::from("--append-system-prompt"));
            args.push(OsString::from(p));
        }
        SystemPromptMode::Replace(p) => {
            args.push(OsString::from("--system-prompt"));
            args.push(OsString::from(p));
        }
    }

    if let Some(mcp) = &config.mcp {
        for cfg in &mcp.configs {
            args.push(OsString::from("--mcp-config"));
            args.push(OsString::from(cfg));
        }
        if mcp.strict {
            args.push(OsString::from("--strict-mcp-config"));
        }
    }

    match &config.tools.builtin {
        BuiltinToolSet::Default => {}
        BuiltinToolSet::None => {
            args.push(OsString::from("--tools"));
            args.push(OsString::from(""));
        }
        BuiltinToolSet::Explicit(tools) => {
            args.push(OsString::from("--tools"));
            args.push(OsString::from(tools.join(",")));
        }
    }

    if let Some(allowed) = &config.tools.allowed {
        args.push(OsString::from("--allowed-tools"));
        args.push(OsString::from(allowed.join(",")));
    }

    if let Some(disallowed) = &config.tools.disallowed {
        args.push(OsString::from("--disallowed-tools"));
        args.push(OsString::from(disallowed.join(",")));
    }

    if config.tools.disable_slash_commands {
        args.push(OsString::from("--disable-slash-commands"));
    }

    match &config.json_schema {
        JsonSchema::None => {}
        JsonSchema::JsonValue(val) => {
            args.push(OsString::from("--json-schema"));
            args.push(OsString::from(val.to_string()));
        }
        JsonSchema::Inline(s) => {
            args.push(OsString::from("--json-schema"));
            args.push(OsString::from(s));
        }
    }

    args.push(OsString::from(prompt));

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BuiltinToolSet, McpPolicy, ToolPolicy};

    #[test]
    fn test_builtin_none_generates_empty_tools_flag() {
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::None,
                allowed: None,
                disallowed: None,
                disable_slash_commands: false,
            },
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --tools "" is present (CONT-01)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == ""),
            "Expected '--tools \"\"' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_builtin_explicit_generates_tools_flag() {
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Explicit(vec!["Bash".to_string()]),
                allowed: None,
                disallowed: None,
                disable_slash_commands: false,
            },
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --tools "Bash" is present (CONT-02)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == "Bash"),
            "Expected '--tools \"Bash\"' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_disable_slash_commands_flag() {
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Default,
                allowed: None,
                disallowed: None,
                disable_slash_commands: true,
            },
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --disable-slash-commands is present
        assert!(
            args_str.contains(&"--disable-slash-commands"),
            "Expected '--disable-slash-commands' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_strict_mcp_config_flag() {
        let config = RunConfig {
            mcp: Some(McpPolicy {
                configs: vec!["test.json".to_string()],
                strict: true,
            }),
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --mcp-config and --strict-mcp-config are present
        assert!(
            args_str.windows(2).any(|w| w[0] == "--mcp-config" && w[1] == "test.json"),
            "Expected '--mcp-config test.json' but got: {:?}",
            args_str
        );
        assert!(
            args_str.contains(&"--strict-mcp-config"),
            "Expected '--strict-mcp-config' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_allowed_tools_flag() {
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Default,
                allowed: Some(vec!["mcp__rig__submit".to_string()]),
                disallowed: None,
                disable_slash_commands: false,
            },
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert --allowed-tools is present
        assert!(
            args_str.windows(2).any(|w| w[0] == "--allowed-tools" && w[1] == "mcp__rig__submit"),
            "Expected '--allowed-tools \"mcp__rig__submit\"' but got: {:?}",
            args_str
        );
    }

    #[test]
    fn test_full_containment_config() {
        use std::path::PathBuf;

        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::None,
                allowed: Some(vec!["mcp__rig__submit".to_string()]),
                disallowed: None,
                disable_slash_commands: true,
            },
            mcp: Some(McpPolicy {
                configs: vec!["mcp.json".to_string()],
                strict: true,
            }),
            cwd: Some(PathBuf::from("/tmp/sandbox")),
            ..RunConfig::default()
        };
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Assert all containment flags are present (CONT-03)
        assert!(
            args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == ""),
            "Expected '--tools \"\"' for builtin none"
        );
        assert!(
            args_str.windows(2).any(|w| w[0] == "--allowed-tools" && w[1] == "mcp__rig__submit"),
            "Expected '--allowed-tools mcp__rig__submit'"
        );
        assert!(
            args_str.contains(&"--disable-slash-commands"),
            "Expected '--disable-slash-commands'"
        );
        assert!(
            args_str.contains(&"--strict-mcp-config"),
            "Expected '--strict-mcp-config'"
        );
        assert!(
            args_str.windows(2).any(|w| w[0] == "--mcp-config" && w[1] == "mcp.json"),
            "Expected '--mcp-config mcp.json'"
        );
    }

    #[test]
    fn test_mcp_only_containment_combination() {
        // Most common containment pattern: MCP tools only, no builtins
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::None,
                allowed: Some(vec![
                    "mcp__rig__submit".to_string(),
                    "mcp__rig__validate".to_string(),
                ]),
                disallowed: None,
                disable_slash_commands: true,
            },
            mcp: Some(McpPolicy {
                configs: vec!["mcp.json".to_string()],
                strict: true,
            }),
            ..RunConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Verify all containment flags present in correct order
        assert!(args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == ""));
        assert!(args_str.windows(2).any(|w| w[0] == "--allowed-tools"
            && w[1] == "mcp__rig__submit,mcp__rig__validate"));
        assert!(args_str.contains(&"--disable-slash-commands"));
        assert!(args_str.contains(&"--strict-mcp-config"));
        assert!(args_str.windows(2).any(|w| w[0] == "--mcp-config" && w[1] == "mcp.json"));
    }

    #[test]
    fn test_explicit_builtin_with_mcp_combination() {
        // Hybrid mode: specific builtins + MCP tools
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Explicit(vec!["Read".to_string()]),
                allowed: Some(vec!["mcp__rig__submit".to_string()]),
                disallowed: None,
                disable_slash_commands: false,
            },
            mcp: Some(McpPolicy {
                configs: vec!["mcp.json".to_string()],
                strict: false,
            }),
            ..RunConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Verify hybrid configuration
        assert!(args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == "Read"));
        assert!(args_str.windows(2).any(|w| w[0] == "--allowed-tools" && w[1] == "mcp__rig__submit"));
        assert!(!args_str.contains(&"--disable-slash-commands")); // Not set
        assert!(!args_str.contains(&"--strict-mcp-config")); // Not strict
    }

    #[test]
    fn test_system_prompt_modes_exclusive() {
        // Replace mode
        let config_replace = RunConfig {
            system_prompt: SystemPromptMode::Replace("Custom system".to_string()),
            ..RunConfig::default()
        };
        let args_replace = build_args("test", &config_replace);
        let args_str: Vec<&str> = args_replace.iter().filter_map(|s| s.to_str()).collect();
        assert!(args_str.windows(2).any(|w| w[0] == "--system-prompt" && w[1] == "Custom system"));
        assert!(!args_str.contains(&"--append-system-prompt"));

        // Append mode
        let config_append = RunConfig {
            system_prompt: SystemPromptMode::Append("Extra context".to_string()),
            ..RunConfig::default()
        };
        let args_append = build_args("test", &config_append);
        let args_str: Vec<&str> = args_append.iter().filter_map(|s| s.to_str()).collect();
        assert!(args_str.windows(2).any(|w| w[0] == "--append-system-prompt" && w[1] == "Extra context"));
        assert!(!args_str.contains(&"--system-prompt"));
    }

    #[test]
    fn test_multiple_mcp_configs() {
        // Multiple MCP config files
        let config = RunConfig {
            mcp: Some(McpPolicy {
                configs: vec!["primary.json".to_string(), "secondary.json".to_string()],
                strict: true,
            }),
            ..RunConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        // Both configs should be present with separate --mcp-config flags
        let mcp_configs: Vec<_> = args_str.windows(2)
            .filter(|w| w[0] == "--mcp-config")
            .map(|w| w[1])
            .collect();
        assert_eq!(mcp_configs.len(), 2);
        assert!(mcp_configs.contains(&"primary.json"));
        assert!(mcp_configs.contains(&"secondary.json"));
    }

    #[test]
    fn test_json_schema_inline() {
        let config = RunConfig {
            json_schema: JsonSchema::Inline(r#"{"type":"object"}"#.to_string()),
            ..RunConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(args_str.windows(2).any(|w| w[0] == "--json-schema" && w[1] == r#"{"type":"object"}"#));
    }

    #[test]
    fn test_disallowed_tools_flag() {
        let config = RunConfig {
            tools: ToolPolicy {
                builtin: BuiltinToolSet::Default,
                allowed: None,
                disallowed: Some(vec!["Bash".to_string(), "Write".to_string()]),
                disable_slash_commands: false,
            },
            ..RunConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(args_str.windows(2).any(|w| w[0] == "--disallowed-tools" && w[1] == "Bash,Write"));
    }

    #[test]
    fn test_default_config_minimal_args() {
        // Default config should generate minimal args (--print, --output-format, and prompt)
        let config = RunConfig::default();
        let args = build_args("test prompt", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert_eq!(args_str.len(), 4); // --print, --output-format, text, prompt
        assert_eq!(args_str[0], "--print");
        assert_eq!(args_str[1], "--output-format");
        assert_eq!(args_str[2], "text");
        assert_eq!(args_str[3], "test prompt");
    }
}
