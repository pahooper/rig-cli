# rig-cli Crate Overview

This document describes each crate published to the kellnr registry under the `rig-cli` namespace.

## Crate Hierarchy

```
rig-cli (facade)
├── rig-cli-provider (core implementation)
│   ├── rig-cli-mcp (MCP server/tools)
│   ├── rig-cli-claude (Claude Code adapter)
│   ├── rig-cli-codex (Codex adapter)
│   └── rig-cli-opencode (OpenCode adapter)
```

---

## rig-cli

**Registry:** `kellnr`
**Version:** `0.1.0`

The main facade crate that most users should depend on. Provides a unified API for interacting with CLI-based AI agents through the Rig framework.

**Key exports:**
- `rig_cli::claude::Client` - Claude Code client
- `rig_cli::codex::Client` - Codex client
- `rig_cli::opencode::Client` - OpenCode client
- `rig_cli::prelude::*` - Common imports
- `rig_cli::mcp::*` - MCP extraction tools

**Usage:**
```toml
[dependencies]
rig-cli = { version = "0.1.0", registry = "kellnr" }
```

```rust
use rig_cli::prelude::*;

let client = rig_cli::claude::Client::new().await?;
let agent = client.agent("claude-sonnet-4").build();
let response = agent.prompt("Hello").await?;
```

---

## rig-cli-provider

**Registry:** `kellnr`
**Version:** `0.1.0`

Core implementation crate containing the `CompletionModel` and `CliAdapter` implementations. Bridges CLI agents to Rig's trait system.

**Key components:**
- `CliAgent` - Wraps CLI adapters as Rig completion models
- `CliAgentBuilder` - Fluent builder for agent configuration
- `CliAdapter` trait - Abstraction over different CLI tools
- `McpToolAgent` - Agent with MCP tool integration
- Adapter implementations for Claude, Codex, OpenCode

**When to use directly:**
- Building custom adapters
- Extending the provider system
- Low-level CLI agent control

---

## rig-cli-mcp

**Registry:** `kellnr`
**Version:** `0.1.0`

MCP (Model Context Protocol) server implementation for structured extraction. Forces agents to return schema-validated JSON through tool constraints.

**Key components:**
- `JsonSchemaToolkit` - Generates MCP tools from Rust types
- `ExtractionOrchestrator` - Manages extraction with retry logic
- `RigMcpHandler` - MCP server request handler
- Validation feedback system for self-correction

**The 3-tool pattern:**
1. `show_example` - Shows valid JSON example
2. `validate_json` - Validates without submitting
3. `submit` - Final submission with schema enforcement

**When to use directly:**
- Custom MCP server configurations
- Advanced extraction pipelines
- Tool generation from schemas

---

## rig-cli-claude

**Registry:** `kellnr`
**Version:** `0.1.0`

Adapter for [Claude Code](https://claude.com/claude-code) CLI. Handles process spawning, streaming events, and MCP configuration.

**Key types:**
- `ClaudeCli` - Main CLI wrapper
- `RunConfig` - Execution configuration
- `StreamEvent` - Streaming event types (text, tool calls, results)
- `McpPolicy` - MCP server access control
- `ToolPolicy` - Built-in tool permissions

**Features:**
- Full streaming support with tool call/result events
- MCP config file generation
- Tool-based containment (`--tools ""`, `--allowed-tools`)
- System prompt and working directory control

---

## rig-cli-codex

**Registry:** `kellnr`
**Version:** `0.1.0`

Adapter for [OpenAI Codex CLI](https://github.com/openai/codex). Handles process spawning, sandbox configuration, and approval policies.

**Key types:**
- `CodexCli` - Main CLI wrapper
- `CodexConfig` - Execution configuration
- `SandboxMode` - Landlock sandbox settings
- `ApprovalPolicy` - Tool approval behavior
- `StreamEvent` - Text and error events

**Features:**
- Landlock sandbox for filesystem isolation (Linux)
- Configurable approval policies
- MCP config overrides via `-c` flag
- Working directory control

---

## rig-cli-opencode

**Registry:** `kellnr`
**Version:** `0.1.0`

Adapter for [OpenCode](https://opencode.ai) CLI. Handles process spawning and configuration through environment variables.

**Key types:**
- `OpenCodeCli` - Main CLI wrapper
- `OpenCodeConfig` - Execution configuration
- `StreamEvent` - Text and error events

**Features:**
- Configuration via `OPENCODE_CONFIG` environment variable
- Working directory isolation
- JSONL output parsing
- Best-effort containment through cwd restriction

---

## Installation

Add to your `~/.cargo/config.toml`:

```toml
[registries.kellnr]
index = "sparse+http://192.168.1.79:8000/api/v1/crates/"
```

Then in your project:

```toml
[dependencies]
rig-cli = { version = "0.1.0", registry = "kellnr" }
```

## Feature Flags

The `rig-cli` crate supports feature flags to control which adapters are included:

```toml
[dependencies]
# All adapters (default)
rig-cli = { version = "0.1.0", registry = "kellnr" }

# Only Claude
rig-cli = { version = "0.1.0", registry = "kellnr", default-features = false, features = ["claude"] }

# Claude + Codex
rig-cli = { version = "0.1.0", registry = "kellnr", default-features = false, features = ["claude", "codex"] }
```

Available features: `claude`, `codex`, `opencode`, `debug-output`
