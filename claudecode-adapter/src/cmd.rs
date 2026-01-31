use crate::types::{BuiltinToolSet, JsonSchema, OutputFormat, RunConfig, SystemPromptMode};
use std::ffi::OsString;

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
