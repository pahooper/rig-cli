# Codebase Structure

**Analysis Date:** 2026-02-01

## Directory Layout

```
rig-cli/
├── Cargo.toml                          # Workspace root manifest
├── Cargo.lock                          # Dependency lock file
├── rust-toolchain.toml                 # Rust version specification
├── justfile                            # Build task definitions
├── deny.toml                           # Dependency audit configuration
├── README.md                           # Project documentation
├── .gitignore                          # Git ignore patterns
├── .planning/                          # GSD planning artifacts (generated)
│   └── codebase/                       # Architecture analysis documents
├── mcp/                                # MCP server bridge library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                      # Module exports (server, tools, prelude)
│   │   ├── server.rs                   # RigMcpHandler and ServerHandler impl
│   │   └── tools.rs                    # JsonSchemaToolkit for structured outputs
│   └── tests/
│       └── integration.rs              # MCP server integration tests
├── rig-provider/                       # Main MCP server orchestrator
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                     # CLI entry point (serve/setup commands)
│   │   ├── lib.rs                      # Module declarations and re-exports
│   │   ├── errors.rs                   # Aggregating ProviderError enum
│   │   ├── sessions.rs                 # SessionManager for temp directories
│   │   ├── setup.rs                    # Zero-Config registration logic
│   │   ├── utils.rs                    # Chat history formatting utilities
│   │   └── adapters/
│   │       ├── mod.rs                  # Adapter module declarations
│   │       ├── claude.rs               # Claude Code CompletionModel
│   │       ├── codex.rs                # Codex CompletionModel
│   │       └── opencode.rs             # OpenCode CompletionModel
│   └── examples/
│       ├── one_shot.rs                 # Simple tool execution
│       ├── streaming.rs                # Stream handling
│       ├── tool_calling.rs             # Tool invocation patterns
│       ├── agent_workflow.rs           # Agent integration
│       ├── data_extraction.rs          # Structured output extraction
│       ├── session_isolation.rs        # Session management demo
│       ├── claudecode_mcp.rs           # Claude-specific MCP example
│       └── opencode_jsonl.rs           # OpenCode streaming example
├── claudecode-adapter/                 # Claude Code CLI wrapper
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # ClaudeCli public interface
│       ├── error.rs                    # ClaudeError enum
│       ├── discovery.rs                # Executable discovery logic
│       ├── init.rs                     # Claude Code initialization
│       ├── process.rs                  # Subprocess execution and streaming
│       ├── cmd.rs                      # CLI argument construction
│       └── types.rs                    # RunConfig, RunResult, StreamEvent
├── codex-adapter/                      # Codex CLI wrapper
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # CodexCli public interface
│       ├── error.rs                    # CodexError enum
│       ├── discovery.rs                # Executable discovery logic
│       ├── process.rs                  # Subprocess execution and streaming
│       ├── cmd.rs                      # CLI argument construction
│       └── types.rs                    # CodexConfig, RunResult, StreamEvent
└── opencode-adapter/                   # OpenCode CLI wrapper
    ├── Cargo.toml
    └── src/
        ├── lib.rs                      # OpenCodeCli public interface
        ├── error.rs                    # OpenCodeError enum
        ├── discovery.rs                # Executable discovery logic
        ├── process.rs                  # Subprocess execution and streaming
        ├── cmd.rs                      # CLI argument construction
        └── types.rs                    # OpenCodeConfig, RunResult, StreamEvent
```

## Directory Purposes

**`mcp/`:**
- Purpose: Generic MCP server implementation bridging Rig to RMCP protocol
- Contains: Protocol handler, tool definition translation, JSON schema toolkit
- Key files: `server.rs` (383 lines), `tools.rs` (358 lines)
- Used by: All provider instances

**`rig-provider/`:**
- Purpose: Main orchestrator combining all adapters into single MCP server
- Contains: Adapter initialization, session management, setup registration, CLI parsing
- Key files: `main.rs` (99 lines, entry point), `adapters/*` (Claude/Codex/OpenCode models)
- Binary name: `rig-provider`

**`claudecode-adapter/`, `codex-adapter/`, `opencode-adapter/`:**
- Purpose: Thin wrappers around each AI CLI tool
- Contains: Executable discovery, process spawning, output parsing, error handling
- Pattern: Identical structure across all three adapters
- Key files: `process.rs` (subprocess execution), `discovery.rs` (binary location), `types.rs` (data structures)

**`examples/`:**
- Purpose: Integration examples demonstrating provider usage
- Contains: One-shot prompts, streaming, agent workflows, structured extraction
- Pattern: Each example is a standalone executable using provider adapters

## Key File Locations

**Entry Points:**
- `rig-provider/src/main.rs`: Binary entrypoint, Clap CLI parsing, delegates to `run_serve()` or `run_setup()`

**Configuration:**
- `rig-provider/src/setup.rs`: Reads/modifies `~/.claude.json`, `~/.opencode.json`, `~/.codex/config.toml`
- `rust-toolchain.toml`: Specifies Rust edition (2021 for provider, 2024 for mcp)

**Core Logic:**
- `mcp/src/server.rs`: MCP ServerHandler implementation, tool definition conversion
- `rig-provider/src/adapters/claude.rs`: Rig CompletionModel for Claude Code (189 lines)
- `rig-provider/src/adapters/codex.rs`: Rig CompletionModel for Codex (168 lines)
- `rig-provider/src/adapters/opencode.rs`: Rig CompletionModel for OpenCode (167 lines)
- `rig-provider/src/sessions.rs`: SessionManager for temp directory lifecycle (37 lines)

**Adapter Execution:**
- `claudecode-adapter/src/process.rs`: Spawn Claude subprocess, capture output (108 lines)
- `codex-adapter/src/process.rs`: Spawn Codex subprocess, capture output (100 lines)
- `opencode-adapter/src/process.rs`: Spawn OpenCode subprocess, capture output (92 lines)

**Testing:**
- `mcp/tests/integration.rs`: MCP server integration tests (94 lines)

## Naming Conventions

**Files:**
- `lib.rs`: Crate root with public module declarations and re-exports
- `main.rs`: Binary entrypoint (only in `rig-provider`)
- `mod.rs`: Module aggregator in `adapters/` subdirectory
- `error.rs`: Custom error types for each crate
- `types.rs`: Configuration and data structures
- `process.rs`: Subprocess execution logic
- `discovery.rs`: Executable location detection
- `cmd.rs`: Command-line argument construction
- `*_test.rs` or `tests/` directory: Test files (currently minimal)

**Directories:**
- Crate directories named with hyphens: `rig-provider`, `claudecode-adapter`
- Module directories named with underscores: `adapters/`
- Configuration at workspace root: `Cargo.toml`, `justfile`

**Functions:**
- Snake case: `discover_claude()`, `run_opencode()`, `format_chat_history()`
- Async functions prefixed clearly: `async fn stream()`, `async fn completion()`

**Types:**
- PascalCase for structs: `ClaudeCli`, `RunConfig`, `RunResult`, `SessionManager`
- PascalCase for enums: `ClaudeError`, `OpenCodeError`, `ProviderError`
- Trait implementations follow Rig patterns: `CompletionModel`, `Tool`

## Where to Add New Code

**New Adapter for Another CLI:**
1. Create new crate directory: `newcli-adapter/`
2. Copy structure from `claudecode-adapter/src/`:
   - `lib.rs`: Public `NewCliCli` struct with `new()`, `run()`, `stream()`
   - `error.rs`: `NewCliError` enum
   - `types.rs`: `RunConfig`, `RunResult`, `StreamEvent`
   - `discovery.rs`: `discover_newcli()` function
   - `process.rs`: `run_newcli()` subprocess implementation
   - `cmd.rs`: CLI argument construction
3. Add crate to workspace `Cargo.toml` members
4. Create adapter in `rig-provider/src/adapters/newcli.rs`:
   - Implement `CompletionModel` trait
   - Add `NewCliTool` wrapper struct
5. Update `rig-provider/src/adapters/mod.rs` to declare new module
6. Update `rig-provider/src/main.rs::run_serve()` to initialize new adapter
7. Update setup functions in `rig-provider/src/setup.rs` for new CLI config location

**New Feature/Tool:**
- Tool implementations go in `mcp/src/tools.rs` or individual adapters
- JSON schema tools use `JsonSchemaToolkit` pattern
- Add to `ToolSet` in `rig-provider/src/main.rs::run_serve()`

**New Session-Based Feature:**
- Session management in `rig-provider/src/sessions.rs`
- Pass `SessionManager` to adapters that need persistent state
- Example: `ClaudeArgs` includes optional `session_id` field

**Utility Functions:**
- Shared helpers in `rig-provider/src/utils.rs`
- Example: `format_chat_history()` converts Rig messages to plain text prompts

**Tests:**
- Unit tests: Co-located with code in same file (use `#[cfg(test)]`)
- Integration tests: `mcp/tests/integration.rs` for MCP protocol testing
- Example code: `rig-provider/examples/` for demonstration patterns

## Special Directories

**`target/`:**
- Purpose: Rust build artifacts and compiled binaries
- Generated: Yes (by Cargo)
- Committed: No (listed in .gitignore)

**`.planning/codebase/`:**
- Purpose: GSD architecture analysis documents (auto-generated)
- Contains: ARCHITECTURE.md, STRUCTURE.md, and other analysis docs
- Generated: Yes (by GSD mapping tools)
- Committed: Yes (tracked for reference)

**`.git/`:**
- Purpose: Git repository metadata
- Generated: Yes (by git init)
- Committed: N/A

## Build and Configuration Files

**`Cargo.toml` (workspace):**
- Declares workspace members and shared lints
- Located at: `/home/pnod/dev/projects/rig-cli/Cargo.toml`
- Contains: `unsafe_code = deny`, pedantic/nursery/perf/cargo warns, unwrap/expect/panic/todo warns

**`Cargo.toml` (per-crate):**
- Specifies dependencies, name, edition
- Located at: Each crate root
- Pattern: Common deps (tokio, serde, serde_json, thiserror, tracing) across all crates

**`rust-toolchain.toml`:**
- Specifies Rust version
- Located at: `/home/pnod/dev/projects/rig-cli/rust-toolchain.toml`

**`justfile`:**
- Build task automation
- Located at: `/home/pnod/dev/projects/rig-cli/justfile`

**`deny.toml`:**
- Dependency security and license audit config
- Located at: `/home/pnod/dev/projects/rig-cli/deny.toml`

---

*Structure analysis: 2026-02-01*
