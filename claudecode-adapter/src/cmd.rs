//! Command-line argument builder for Claude CLI invocations.

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
            OutputFormat::StreamJson => args.push(OsString::from("stream-json")),
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
}
