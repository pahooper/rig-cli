# Architecture

**Analysis Date:** 2026-02-01

## Pattern Overview

**Overall:** Layered MCP (Model Context Protocol) server with adapter pattern for multi-CLI integration

**Key Characteristics:**
- Workspace composed of 5 interdependent Rust crates
- Adapter pattern bridges CLI executables to Rig framework via MCP protocol
- Session-based sandboxing using persistent temporary directories
- Generic tool exposure through Rig's ToolSet abstraction

## Layers

**CLI Adapters Layer:**
- Purpose: Wrap external AI CLI tools (Claude Code, Codex, OpenCode) as executable processes
- Location: `claudecode-adapter/`, `codex-adapter/`, `opencode-adapter/`
- Contains: Process spawning, argument construction, output parsing, error handling, executable discovery
- Depends on: Tokio for async process management, Serde for serialization, thiserror for error types
- Used by: `rig-provider` adapters, integration examples

**Provider Adapter Layer:**
- Purpose: Convert CLI adapters into Rig-compatible `CompletionModel` implementations
- Location: `rig-provider/src/adapters/` (`claude.rs`, `codex.rs`, `opencode.rs`)
- Contains: Rig trait implementations, tool definitions, session ID handling, chat history formatting
- Depends on: CLI adapter crates, Rig framework, session manager
- Used by: `run_serve()` in main entry point

**MCP Server Layer:**
- Purpose: Expose Rig tools via Model Context Protocol over stdio
- Location: `mcp/src/` (`server.rs`, `tools.rs`)
- Contains: MCP ServerHandler implementation, tool definition translation (Rig → RMCP), JSON schema validation
- Depends on: RMCP library, Rig framework, Serde for JSON handling
- Used by: Provider CLI to expose all toolsets

**Core Provider Layer:**
- Purpose: Orchestrates adapters, manages sessions, handles setup registration
- Location: `rig-provider/src/` (`main.rs`, `lib.rs`, `sessions.rs`, `setup.rs`)
- Contains: Initialization logic, session lifecycle, configuration discovery, MCP server startup
- Depends on: All adapter crates, MCP server, Clap for CLI parsing
- Used by: Executable entry point `rig-provider`

## Data Flow

**Tool Execution Flow:**

1. Client (Claude Code, Codex, OpenCode) connects to MCP server via stdio
2. MCP server routes `list_tools` request → `RigMcpHandler` fetches ToolSet definitions → Returns MCP tool definitions
3. Client calls tool → MCP server routes to `ToolSet::call_tool()`
4. `ToolSet` dispatches to appropriate adapter (ClaudeTool, CodexTool, OpenCodeTool)
5. Adapter constructs `CompletionRequest` with chat history and tools
6. `CompletionModel.completion()` format chat history → spawn CLI subprocess → parse stdout
7. Result returned through MCP protocol back to client

**Session State Flow:**

1. Client includes optional `session_id` in tool arguments
2. `SessionManager::get_session_dir(session_id)` creates or retrieves persistent temp directory
3. Session temp directory passed to CLI subprocess via config `cwd` setting
4. CLI maintains state files in session directory across multiple tool invocations
5. Session directory persists until SessionManager is dropped (within application lifetime)

**Setup Registration Flow:**

1. User runs `rig-provider setup [--dry-run]`
2. `run_setup()` discovers target config files:
   - `~/.claude.json` for Claude Code (JSON MCP format)
   - `~/.opencode.json` for OpenCode (JSON MCP format)
   - `~/.codex/config.toml` for Codex (TOML MCP format)
3. For each file, inserts/updates MCP server entry with current executable path
4. Clients auto-discover provider on next launch

**State Management:**

- **Adapter State:** Stateless; each tool invocation creates new subprocess
- **Session State:** Held in `SessionManager::sessions` HashMap (in-memory + persistent temp directories)
- **MCP State:** Stateless server; tool definitions computed once at startup
- **Config State:** Read from user's home directory, not modified except during setup

## Key Abstractions

**Rig `CompletionModel` Trait:**
- Purpose: Standardizes how different AI providers expose completion capabilities
- Examples: `ClaudeModel` in `rig-provider/src/adapters/claude.rs`, `CodexModel`, `OpenCodeModel`
- Pattern: Wraps underlying CLI client, translates `CompletionRequest` to CLI arguments, returns `CompletionResponse` with stdout

**CLI Adapter (`ClaudeCli`, `CodexCli`, `OpenCodeCli`):**
- Purpose: Thin wrapper around subprocess execution with argument construction
- Pattern: `run()` for synchronous completion, `stream()` for event-based streaming
- Located in: `claudecode-adapter/src/lib.rs`, `codex-adapter/src/lib.rs`, `opencode-adapter/src/lib.rs`

**SessionManager:**
- Purpose: Maintains per-session temporary directories for isolated sandbox execution
- Pattern: Lazy creation on first access, HashMap-based tracking, Arc-wrapped TempDir for cleanup
- Located in: `rig-provider/src/sessions.rs`

**JsonSchemaToolkit:**
- Purpose: Declarative configuration for structured output tools (submit, validate, example)
- Pattern: Type-driven schema generation via `schemars::JsonSchema`
- Located in: `mcp/src/tools.rs`

**RigMcpHandler:**
- Purpose: Bridges Rig ToolSet/ToolServer to MCP protocol
- Pattern: Consumes `ToolSet`, pre-computes RMCP `Tool` definitions, handles `call_tool` routing
- Located in: `mcp/src/server.rs`

## Entry Points

**Serve Mode (default):**
- Location: `rig-provider/src/main.rs::run_serve()`
- Triggers: `rig-provider` or `rig-provider serve`
- Responsibilities:
  1. Initializes three adapters (Claude, Codex, OpenCode)
  2. Creates ToolSet with all adapters + JSON schema tools
  3. Converts ToolSet to RigMcpHandler
  4. Starts MCP server over stdio
  5. Blocks indefinitely accepting requests

**Setup Mode:**
- Location: `rig-provider/src/main.rs::main()`, delegates to `run_setup()`
- Triggers: `rig-provider setup [--dry-run]`
- Responsibilities:
  1. Discovers executable path
  2. Reads/modifies CLI config files
  3. Registers MCP server entry in each config
  4. Prints results

**Tool Dispatch:**
- Location: MCP server receives `call_tool` request
- Route: `RigMcpHandler::handle_call_tool()` → `ToolSet::call_tool()` → adapter's `Tool::call()`
- Example: `ClaudeTool::call()` in `rig-provider/src/adapters/claude.rs`

## Error Handling

**Strategy:** Hierarchical error mapping with custom error types at each layer

**Patterns:**

- **CLI Adapter Errors:** Custom enums (`ClaudeError`, `CodexError`, `OpenCodeError`) in each adapter
  - Examples: `ExecutableNotFound`, `ProcessFailed`, `IoError`
  - Located in: `*-adapter/src/error.rs`

- **Provider Errors:** Aggregating enum `ProviderError` wraps all adapter errors via `#[from]`
  - Located in: `rig-provider/src/errors.rs`
  - Used by: `main()`, `run_serve()`

- **Tool Execution Errors:** Rig's `CompletionError` for completion operations, `ToolError` for tool validation
  - Mapped from adapter errors in `CompletionModel` implementations
  - Located in: `rig-provider/src/adapters/*`

- **Setup Errors:** `anyhow::Result<()>` for flexible error propagation
  - Located in: `rig-provider/src/setup.rs`

## Cross-Cutting Concerns

**Logging:**
- Framework: `tracing` with `tracing-subscriber` for setup
- Pattern: Info-level logs for major operations (adapter init, server start)
- Configuration: `env-filter` enables dynamic level control via `RUST_LOG`
- Located in: `rig-provider/src/main.rs::main()`

**Validation:**
- Chat history formatting in `rig-provider/src/utils.rs::format_chat_history()`
- JSON schema validation for structured outputs in `mcp/src/tools.rs::JsonSchemaToolkit`
- Argument construction in CLI adapters `*-adapter/src/cmd.rs`

**Authentication:**
- No built-in authentication; relies on underlying CLI tools' auth
- Setup discovers and registers provider in client configs (implicit trust model)
- Located in: `rig-provider/src/setup.rs::setup_json_mcp()`, `setup_codex()`

**Process Management:**
- Subprocess spawning with configurable working directory and environment variables
- Streaming via line-based parsing of subprocess output
- Timeout handling via `tokio::time::timeout`
- Located in: `*-adapter/src/process.rs`

---

*Architecture analysis: 2026-02-01*
