# Technology Stack

**Analysis Date:** 2026-02-01

## Languages

**Primary:**
- Rust (Edition 2021) - Core provider and adapter implementations
- Rust (Edition 2024) - MCP server library

**Secondary:**
- JSON - Configuration files for Claude Code and OpenCode
- TOML - Configuration for Codex adapter

## Runtime

**Environment:**
- Rust stable toolchain (configured in `rust-toolchain.toml`)

**Build System:**
- Cargo workspace with 5 member crates
- Resolver version 2

## Frameworks

**Core Protocol:**
- RMCP (Rust Model Context Protocol) 0.14.0 - MCP server implementation with server and transport-io features
- Rig (rig-core) 0.29.0 - AI tool and completion model abstraction framework

**Async Runtime:**
- Tokio 1.0 with full features (all runtime components enabled)
- Tokio-stream 0.1.18 - Stream utilities for async operations

**Serialization:**
- Serde 1.0 with derive feature - Data serialization/deserialization
- Serde JSON 1.0 - JSON format support

**Schema & Validation:**
- Schemars 1.0-1.2 - JSON Schema generation from Rust types
- JSONSchema 0.26 - JSON schema validation

**CLI:**
- Clap 4.5 with derive feature - Command-line argument parsing

## Key Dependencies

**Critical:**
- `rig-core` 0.29.0 - Provides CompletionModel, Tool, and ToolSet traits for integrating AI models
- `rmcp` 0.14.0 - Model Context Protocol server implementation for MCP compliance
- `tokio` 1.0 - Async runtime enabling concurrent operation of multiple adapters

**Infrastructure:**
- `tracing` 0.1 - Distributed tracing and logging
- `tracing-subscriber` 0.3 with env-filter - Tracing subscriber with environment-based filtering
- `async-trait` 0.1 - Async trait implementations
- `thiserror` 1.0 - Error type derivation and display
- `anyhow` 1.0 - Flexible error handling with context
- `futures` 0.3 - Futures utilities for async composition
- `uuid` 1.20.0 with v4 feature - UUID generation for session IDs
- `tempfile` 3.10 - Temporary file/directory creation for session sandboxing
- `which` 6.0 - Binary discovery from PATH environment
- `schemars` 1.0-1.2 - JSON Schema generation for tool parameters

**Local Dependencies (workspace):**
- `rig-mcp-server` - Core MCP server bridge library in `mcp/`
- `claudecode-adapter` - Claude Code CLI adapter in `claudecode-adapter/`
- `codex-adapter` - Codex CLI adapter in `codex-adapter/`
- `opencode-adapter` - OpenCode CLI adapter in `opencode-adapter/`

## Configuration

**Environment:**
- HOME env var - Required for locating CLI configuration files
- CC_ADAPTER_CLAUDE_BIN - Optional override for Claude Code binary path (checked via `which` if not set)
- Optional env vars passed to spawned CLI processes via adapter RunConfig

**Build:**
- `Cargo.toml` - Workspace configuration with workspace lints
- `Cargo.lock` - Exact dependency lock file
- `rust-toolchain.toml` - Rust version specification (stable channel)
- `deny.toml` - Cargo deny configuration for license/security checking

**Lint Configuration:**
- Workspace-wide clippy lints: pedantic, nursery, perf, cargo
- Additional checks: unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro warnings
- Allowed: missing_errors_doc, missing_panics_doc, module_name_repetitions
- Unsafe code: denied

## Platform Requirements

**Development:**
- Rust stable toolchain with rustfmt and clippy
- Cargo (distributed with Rust)
- External CLI tools: Claude Code, Codex, OpenCode (discovered at runtime)
- Optional: cargo-deny, cargo-audit, cargo-machete, typos (for quality checks)

**Production:**
- Rust stable runtime (no additional runtime required)
- External CLI tools must be installed and in PATH or discoverable:
  - Claude Code (binary name: `claude`)
  - Codex
  - OpenCode
- Configuration files for target CLIs:
  - ~/.claude.json (JSON-based MCP configuration)
  - ~/.opencode.json (JSON-based MCP configuration)
  - ~/.codex/config.toml (TOML-based configuration)

## Workspace Structure

**Crates:**
- `rig-provider` - Main MCP server and adapter orchestrator
- `mcp` (rig-mcp-server) - Core MCP server library implementing RMCP protocol bridge
- `claudecode-adapter` - Anthropic Claude Code CLI integration
- `codex-adapter` - Codex CLI integration
- `opencode-adapter` - OpenCode CLI integration

**Key Binaries:**
- `rig-provider` - Main entry point supporting `serve` and `setup` commands

---

*Stack analysis: 2026-02-01*
