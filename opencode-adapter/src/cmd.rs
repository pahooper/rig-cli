//! Command-line argument construction for the `OpenCode` binary.

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

    args.push(OsString::from(message));

    args
}
