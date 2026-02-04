# rig-cli

Turn CLI-based AI agents into idiomatic Rig providers with MCP-enforced structured extraction.

## What are CLI Agents?

CLI agents like [Claude Code](https://claude.com/claude-code), [Codex](https://github.com/openai/codex), and [OpenCode](https://opencode.ai) are local AI assistants that run on your machine. Unlike cloud APIs, they:

- Execute tools locally (file I/O, shell commands, git operations)
- Maintain persistent context across interactions
- Operate within your development environment

**Why MCP (Model Context Protocol)?**

When you need structured data from an agent (not freeform text), you face a problem: how do you guarantee the agent returns valid JSON matching your schema?

MCP solves this by exposing a `submit` tool with your schema. The agent *must* call this tool to respond, and MCP validates the payload against your schema. Invalid submissions get rejected with helpful errors, forcing the agent to retry until it succeeds.

**When to use CLI agents vs direct API calls:**

| Scenario | Use |
|----------|-----|
| Need local tool execution (file I/O, git, shell) | CLI agent |
| Need persistent workspace context | CLI agent |
| Simple text generation, no local tools | Direct API |
| High-throughput, stateless requests | Direct API |

## Quick Start

```bash
cargo add rig-cli
```

```rust
use rig_cli::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client (auto-discovers CLI binary)
    let client = rig_cli::claude::Client::new().await?;

    // Build an agent
    let agent = client.agent("claude-sonnet-4")
        .preamble("You are a helpful assistant")
        .build();

    // Prompt and get response
    let response = agent.prompt("What is 2 + 2?").await?;
    println!("{}", response);

    Ok(())
}
```

## Features

| Feature | Description |
|---------|-------------|
| Direct CLI execution | Simple prompts via `client.agent()` |
| MCP-enforced extraction | Structured output via `client.mcp_agent()` |
| Streaming support | Real-time event streaming |
| Payload injection | Context data attachment via `Payload` |
| Multi-adapter support | Claude Code, Codex, OpenCode |
| Auto-discovery | Finds CLI binaries automatically |

## Two Execution Paths

rig-cli provides two ways to interact with CLI agents:

| Method | Use Case |
|--------|----------|
| `client.agent("model")` | Simple prompts, chat, streaming |
| `client.mcp_agent("model")` | Structured extraction with schema enforcement |

**Decision tree:**

- Need the agent to return data matching a specific schema? Use `mcp_agent()`
- Just want text responses or chat? Use `agent()`

**MCP extraction example:**

```rust,ignore
use rig_cli::prelude::*;
use rig_cli::tools::JsonSchemaToolkit;
use rig_cli::extraction::ExtractionOrchestrator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct PersonInfo {
    name: String,
    age: u32,
}

// Create toolkit from your schema
let toolkit = JsonSchemaToolkit::from_type::<PersonInfo>()?;

// Build MCP orchestrator
let orchestrator = ExtractionOrchestrator::builder()
    .with_toolkit(toolkit)
    .build();

// Extract structured data
let client = rig_cli::claude::Client::new().await?;
let result = orchestrator
    .extract::<PersonInfo>(
        &client.agent("claude-sonnet-4").build(),
        "Extract person info: Alice is 30 years old"
    )
    .await?;

println!("{:?}", result); // PersonInfo { name: "Alice", age: 30 }
```

## Adapter Comparison

| Feature | Claude Code | Codex | OpenCode |
|---------|-------------|-------|----------|
| MCP support | Yes | Yes | Yes |
| Streaming events | Full (ToolCall/ToolResult) | Text/Error only | Text/Error only |
| Sandbox | `--tools ""` (disable builtins) | `--sandbox` (Landlock) | None (cwd only) |
| System prompt | `--system-prompt` | Prepend to prompt | Prepend to prompt |
| Working directory | `--cwd` | `--cd` | `Command::current_dir()` |
| MCP config | `--mcp-config` file | `-c` overrides | `OPENCODE_CONFIG` env |

**Containment notes:**

- **Claude Code**: Tool-based containment via `--tools ""` and `--allowed-tools`
- **Codex**: Landlock sandbox for filesystem isolation (Linux only)
- **OpenCode**: Best-effort via working directory isolation

## Examples

Examples will be available in [`rig-cli/examples/`](./rig-cli/examples/):

| Example | Description |
|---------|-------------|
| `simple_prompt.rs` | Basic prompt/response |
| `streaming.rs` | Real-time event streaming |
| `extraction.rs` | MCP-enforced structured extraction |
| `chat.rs` | Multi-turn conversation |
| `payload.rs` | Context injection with Payload |

*Note: Examples are being added in documentation phase.*

## Documentation

- [API Reference (docs.rs)](https://docs.rs/rig-cli) - Full rustdoc documentation
- [Rig Documentation](https://docs.rs/rig-core) - Core Rig framework docs

## License

MIT
