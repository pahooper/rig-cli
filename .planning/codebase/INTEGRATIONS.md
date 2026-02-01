# External Integrations

**Analysis Date:** 2026-02-01

## APIs & External Services

**AI Code Assistants (CLI Tools):**
- Claude Code - Anthropic's code assistant CLI
  - SDK/Client: `claudecode-adapter` crate (`/home/pnod/dev/projects/rig-cli/claudecode-adapter`)
  - Discovery: Environment variable `CC_ADAPTER_CLAUDE_BIN` or via PATH lookup using `which` crate
  - Binary name: `claude`
  - Integration: Process-based via spawned subprocess with full stdio/stderr capture

- Codex - Codex CLI tool
  - SDK/Client: `codex-adapter` crate (`/home/pnod/dev/projects/rig-cli/codex-adapter`)
  - Discovery: Via PATH lookup
  - Integration: Process-based via spawned subprocess

- OpenCode - OpenCode CLI tool
  - SDK/Client: `opencode-adapter` crate (`/home/pnod/dev/projects/rig-cli/opencode-adapter`)
  - Discovery: Via PATH lookup
  - Integration: Process-based via spawned subprocess

## Data Storage

**Configuration Storage:**
- Claude Code config: `~/.claude.json`
  - Format: JSON with `mcpServers` object containing MCP server definitions
  - Auto-registration: Handled by `setup_json_mcp()` in `rig-provider/src/setup.rs`

- OpenCode config: `~/.opencode.json`
  - Format: JSON with `mcpServers` object
  - Auto-registration: Handled by `setup_json_mcp()` in `rig-provider/src/setup.rs`

- Codex config: `~/.codex/config.toml`
  - Format: TOML with `[mcp_servers.{provider_name}]` sections
  - Auto-registration: Handled by `setup_codex()` in `rig-provider/src/setup.rs`

**Session Storage:**
- Per-session temporary directories for sandboxed execution
- Managed by `SessionManager` in `rig-provider/src/sessions.rs`
- Uses `tempfile` crate for temporary directory creation
- Session persistence across multiple tool calls within a session

**Databases:**
- Not used - This is a stateless MCP server provider

**File Storage:**
- Local filesystem only - No cloud storage integration

**Caching:**
- None - Stateless per-request execution

## Authentication & Identity

**Auth Provider:**
- Custom - No centralized authentication provider
- Authentication delegated to individual CLI tools (Claude Code, Codex, OpenCode)
- Each tool handles its own authentication independently
- Zero-config setup approach: Provider registers itself in existing CLI tool configs

**Session Identity:**
- UUID v4-based session IDs for persistent session tracking
- Generated in adapter implementations using `uuid` crate with v4 feature
- Session ID passed to Claude Code adapter as optional parameter
- Sessions map to persistent temporary directories for isolated execution

## Monitoring & Observability

**Error Tracking:**
- None - No external error tracking service integrated

**Logs:**
- Built-in tracing via `tracing` and `tracing-subscriber` crates
- Structured logging aligned with Rig's internal tracing style
- Environment-based filtering via `tracing_subscriber::fmt()` with env-filter support
- Default log level: INFO
- Configurable via RUST_LOG environment variable through env-filter feature
- No persistent logging - stdout/stderr only

**Tracing Implementation:**
- `tracing` 0.1 for event generation
- `tracing_subscriber` 0.3 with env-filter for runtime log level control
- Format: Minimal console output with target names disabled

## CI/CD & Deployment

**Hosting:**
- Standalone MCP server executable
- Designed for integration as MCP server into Claude Code, Codex, OpenCode
- CLI-based deployment: Binary can be registered via `rig-provider setup` command

**CI Pipeline:**
- No external CI service integration detected
- Local development: Cargo build system with quality checks via justfile
- Quality checks: `cargo fmt`, `cargo clippy`, `cargo test`
- Optional security checks: cargo-deny, cargo-audit (commented out in justfile)

**Build Artifacts:**
- Compiled binary: `rig-provider` (main entry point)
- Adapter binaries: Not directly executable, used as libraries
- Configuration: Setup command configures target CLI applications

## Environment Configuration

**Required env vars:**
- HOME - Required for locating configuration files (Claude, OpenCode, Codex paths)

**Optional env vars:**
- CC_ADAPTER_CLAUDE_BIN - Override Claude Code binary path (otherwise uses PATH lookup)
- RUST_LOG - Control logging level via tracing-subscriber env-filter
- Custom env vars - Passed through to spawned CLI processes via RunConfig

**Secrets location:**
- No secrets managed by provider
- External CLI tools manage their own authentication secrets
- Configuration files stored in user home directory (standard Unix convention)

## Webhooks & Callbacks

**Incoming:**
- MCP tool calls via stdio - Standard Model Context Protocol tool invocation
- No HTTP endpoints - Stdio-based transport only

**Outgoing:**
- Subprocess streams - Output from Claude Code, Codex, OpenCode processes captured and relayed
- Error callbacks - Stderr/stdout captured and returned to MCP client
- Stream events - JSON-formatted streaming events from Claude Code parsed and forwarded

## MCP Server Configuration

**MCP Servers Registered:**
- Provider name: "rig-provider"
- Command: Path to compiled `rig-provider` binary
- Args: Empty array (no command-line arguments)
- Env: Empty object (environment inherited from parent)

**MCP Tools Exposed:**
Three adapters provide tool integration:
- ClaudeTool - Wraps Claude Code as an MCP tool
- CodexTool - Wraps Codex as an MCP tool
- OpenCodeTool - Wraps OpenCode as an MCP tool

Plus three built-in tools from JsonSchemaToolkit:
- SubmitTool - Submits structured data matching a JSON schema
- ValidateJsonTool - Validates JSON against schema
- JsonExampleTool - Provides examples of valid schema instances

## Process Integration

**Subprocess Management:**
- Claude Code: Spawned via `run_claude()` in `claudecode-adapter/src/process.rs`
  - Stdout/stderr captured via tokio subprocess pipes
  - Output streamed via UnboundedSender channels
  - Timeout configuration via config.timeout
  - Optional output format: JSON streaming format parsed and forwarded

- Codex: Spawned via similar subprocess mechanism in `codex-adapter`
  - Default configuration used if not specified
  - Output captured as plain text

- OpenCode: Spawned via subprocess in `opencode-adapter`
  - Default configuration used
  - Output captured as plain text

**Configuration Propagation:**
- RunConfig passed to each adapter specifying:
  - Current working directory (cwd)
  - Environment variables
  - Allowed tools list
  - Output format preferences
  - Timeout settings

---

*Integration audit: 2026-02-01*
