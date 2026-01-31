# rig-mcp-server

A "Zero-Config" bridge between the [Rig](https://github.com/0xPlaygrounds/rig) toolset and the [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

## Features

- **Native Rig Integration**: Built specifically for the Rig ecosystem.
- **Zero-Config Metadata**: Automatically extracts tool definitions from any Rig `ToolSet`.
- **Declarative Toolkit**: Unified schema, examples, and validation logic via `JsonSchemaToolkit<T>`.
- **Future-Proof**: Supports both static `ToolSet` and dynamic, RAG-enabled `ToolServer`.
- **Observability**: Specialized `tracing` instrumentation aligned with Rig's internal tracing style.
- **High Quality**: Adheres to `clippy::pedantic` with clean, type-safe Rust patterns.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rig-mcp-server = { path = "../path/to/rig-mcp-server" } # Or from registry when available
rig = { package = "rig-core", version = "0.29.0" }
```

## Quick Start (Pre-configured Tools)

The library provides a declarative way to create a trio of tools (`submit`, `validate`, `example`) that share a single source of truth:

```rust
use rig_mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use rig::tool::ToolSet;

// 1. Define your domain model
#[derive(JsonSchema, Serialize, Deserialize)]
struct UserSubmission {
    user_id: String,
    action: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. Create the declarative toolkit with a type-safe callback
    let (submit, validate, example) = JsonSchemaToolkit::<UserSubmission>::builder()
        .example(UserSubmission { user_id: "USR-001".to_string(), action: "login".to_string() })
        .on_submit(|data| {
            format!("Success! Action {} for user {} stored.", data.action, data.user_id)
        })
        .build()
        .build_tools();

    // 3. Add to a Rig ToolSet
    let mut toolset = ToolSet::default();
    toolset.add_tool(submit);
    toolset.add_tool(validate);
    toolset.add_tool(example);

    // 4. Ergonomically start the MCP Server
    // This will print the needed config for Claude/Codex/OpenCode to stderr!
    toolset.run_stdio().await?;
    
    Ok(())
}
```

## Advanced Features

### Custom Server Naming
You can customize the server name (e.g., for white-labeling) using the builder:

```rust
let handler = RigMcpHandler::builder()
    .name("my-custom-server")
    .toolset(tools)
    .build()
    .await?;
```

### Non-blocking Execution
Use `serve_stdio` to run the server in the background (e.g., within a `tokio::spawn`) without printing configuration snippets:

```rust
tokio::spawn(async move {
    handler.serve_stdio().await.expect("Server failed");
});
```

## Programmatic Configuration

You can access the MCP configuration details directly to automate file generation (e.g., writing a `.mcp.json` file for your team):

```rust
let config = toolset.config().await?;

// Ready-to-use formats for different CLIs:
let claude_json = config.to_claude_json();   // serde_json::Value
let codex_toml = config.to_codex_toml();     // String (TOML)
let opencode_json = config.to_opencode_json(); // serde_json::Value
```

## Library Components

### `RigMcpHandler`
The core adapter that implements `rmcp::ServerHandler`. It can be constructed from a `ToolSet` or a `ToolServerHandle` (for dynamic tools).

### `JsonSchemaToolkit<T>`
A factory for creating consistent validation and example tools derived directly from your Rust types using `schemars`.

### `ToolSetExt`
An extension trait that adds `.into_handler()` to Rig's native `ToolSet` for seamless conversion.

## License

This project is licensed under the MIT License.
