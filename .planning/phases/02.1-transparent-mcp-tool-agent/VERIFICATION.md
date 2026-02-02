---
phase: 02.1-transparent-mcp-tool-agent
verified: 2026-02-01T21:30:00Z
status: passed
score: 6/6 must-haves verified
---

# Phase 2.1: Transparent MCP Tool Agent Verification Report

**Phase Goal:** User provides ToolSet + prompt, system handles all MCP plumbing transparently -- no manual config, no dual-mode boilerplate, no RunConfig construction
**Verified:** 2026-02-01T21:30:00Z
**Status:** PASSED
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | McpToolAgent builder accepts a ToolSet and prompt, auto-spawns the current binary as MCP server via env var detection | VERIFIED | `mcp_agent.rs:154-250` -- `run()` takes toolset+prompt+adapter, uses `std::env::current_exe()` to build McpConfig with `RIG_MCP_SERVER=1` env var, launches the CLI. Example at `mcp_tool_agent_e2e.rs:52-53` checks `RIG_MCP_SERVER` env var for server mode. |
| 2 | MCP config JSON is auto-generated with correct server name, tool names, and env vars | VERIFIED | `mcp_agent.rs:176-220` -- Builds `McpConfig` with server name, command, and `RIG_MCP_SERVER` env var. Writes adapter-specific formats via `to_claude_json()` (line 201), `to_codex_toml()` (line 208), `to_opencode_json()` (line 214). All three methods confirmed in `mcp/src/server.rs:167-216`. |
| 3 | Tool names are auto-computed as `mcp__<server_name>__<tool_name>` from ToolSet definitions | VERIFIED | `mcp_agent.rs:188-191` -- `allowed_tools` computed via `format!("mcp__{}__{}",  self.server_name, def.name)`. Same pattern in `compute_tool_names()` at line 136. |
| 4 | Claude CLI is discovered and launched with correct --mcp-config, --allowed-tools flags | VERIFIED | `mcp_agent.rs:260-290` -- ClaudeCode path: `claudecode_adapter::init(None)` discovers CLI, builds `RunConfig` with `McpPolicy.configs` (--mcp-config), `ToolPolicy.allowed` (--allowed-tools), `SystemPromptMode::Append` (--system-prompt). Codex: `codex_adapter::discover_codex()` at line 298. OpenCode: `opencode_adapter::discover_opencode()` at line 326. |
| 5 | Temp files are auto-cleaned via RAII guards | VERIFIED | `mcp_agent.rs:194-223` -- `tempfile::NamedTempFile::new()` creates temp file, `config_file.into_temp_path()` at line 223 creates RAII guard `_config_guard` that auto-deletes on drop after CLI execution completes. `tempfile = "3.10"` in `rig-provider/Cargo.toml`. |
| 6 | Existing mcp_extraction_e2e example reduces from ~300 lines to ~50 lines using new API | VERIFIED | `mcp_extraction_e2e.rs` is 301 lines. `mcp_tool_agent_e2e.rs` is 73 lines (under 80-line target). New example uses `McpToolAgent::builder().toolset().prompt().adapter().server_name().run().await` -- no manual config JSON, no manual temp file management, no manual RunConfig construction. |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `codex-adapter/src/types.rs` | CodexConfig with mcp_config_path, system_prompt, env_vars | VERIFIED (108 lines) | Lines 50-55: `system_prompt: Option<String>`, `env_vars: Vec<(String, String)>`, `mcp_config_path: Option<PathBuf>`. All have doc comments. Defaults: None, Vec::new(), None. |
| `codex-adapter/src/cmd.rs` | system_prompt wired to --system-prompt flag | VERIFIED (68 lines) | Lines 60-63: `if let Some(ref sp) = config.system_prompt { args.push("--system-prompt"); args.push(sp); }` |
| `codex-adapter/src/process.rs` | env_vars injection into subprocess | VERIFIED (250 lines) | Lines 79-81 in `spawn_child()`: `for (k, v) in &config.env_vars { cmd.env(k, v); }`. Signature takes `config: &CodexConfig` (not bare cwd). |
| `opencode-adapter/src/types.rs` | OpenCodeConfig with env_vars, mcp_config_path | VERIFIED (80 lines) | Lines 23-25: `env_vars: Vec<(String, String)>`, `mcp_config_path: Option<PathBuf>`. Both have doc comments. Defaults: Vec::new(), None. |
| `opencode-adapter/src/cmd.rs` | prompt wired to --system-prompt flag | VERIFIED (45 lines) | Lines 37-39: `if let Some(ref sp) = config.prompt { args.push("--system-prompt"); args.push(sp); }` |
| `opencode-adapter/src/process.rs` | env_vars + OPENCODE_CONFIG injection | VERIFIED (319 lines) | Lines 89-95 in `spawn_child()`: env_vars loop + `if let Some(ref mcp_path) = config.mcp_config_path { cmd.env("OPENCODE_CONFIG", mcp_path); }` |
| `rig-provider/src/mcp_agent.rs` | McpToolAgent builder, CliAdapter, run() | VERIFIED (345 lines) | CliAdapter enum (lines 12-20), McpToolAgentResult (lines 23-33), McpToolAgent namespace (lines 37-45), McpToolAgentBuilder (lines 52-251), run_claude_code/run_codex/run_opencode helpers (lines 253-345). |
| `rig-provider/src/lib.rs` | Module declaration + re-exports | VERIFIED (25 lines) | Line 23: `pub mod mcp_agent;`. Line 25: `pub use mcp_agent::{CliAdapter, McpToolAgent, McpToolAgentBuilder, McpToolAgentResult};` |
| `rig-provider/src/errors.rs` | McpToolAgent error variant | VERIFIED (41 lines) | Lines 38-40: `#[error("MCP tool agent error: {0}")] McpToolAgent(String)` |
| `rig-provider/examples/mcp_tool_agent_e2e.rs` | Simplified example under 80 lines | VERIFIED (73 lines) | Uses McpToolAgent::builder(), CliAdapter::ClaudeCode, env var server detection. No manual config JSON, temp files, or RunConfig. |
| `rig-provider/examples/mcp_extraction_e2e.rs` | Old example still compiles | VERIFIED (301 lines) | `cargo check --example mcp_extraction_e2e -p rig-provider` passes. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `codex-adapter/src/types.rs` | `codex-adapter/src/cmd.rs` | config.system_prompt in build_args | VERIFIED | cmd.rs line 60: `config.system_prompt` accessed in build_args |
| `codex-adapter/src/types.rs` | `codex-adapter/src/process.rs` | config.env_vars in spawn_child | VERIFIED | process.rs line 79: `config.env_vars` iterated in spawn_child |
| `opencode-adapter/src/types.rs` | `opencode-adapter/src/cmd.rs` | config.prompt in build_args | VERIFIED | cmd.rs line 37: `config.prompt` accessed in build_args |
| `opencode-adapter/src/types.rs` | `opencode-adapter/src/process.rs` | config.env_vars + config.mcp_config_path in spawn_child | VERIFIED | process.rs lines 89-95: env_vars loop + OPENCODE_CONFIG env var |
| `rig-provider/src/mcp_agent.rs` | `mcp/src/server.rs` | McpConfig.to_claude_json/to_codex_toml/to_opencode_json | VERIFIED | mcp_agent.rs lines 201, 208, 214 call McpConfig methods; server.rs lines 171-216 implement them |
| `rig-provider/src/mcp_agent.rs` | `claudecode-adapter` | init(), ClaudeCli, RunConfig | VERIFIED | mcp_agent.rs lines 260-282: init(), ClaudeCli::new(), RunConfig with McpPolicy+ToolPolicy |
| `rig-provider/src/mcp_agent.rs` | `codex-adapter` | discover_codex(), CodexCli, CodexConfig | VERIFIED | mcp_agent.rs lines 298-310: discover_codex(), CodexCli::new(), CodexConfig with new fields |
| `rig-provider/src/mcp_agent.rs` | `opencode-adapter` | discover_opencode(), OpenCodeCli, OpenCodeConfig | VERIFIED | mcp_agent.rs lines 326-337: discover_opencode(), OpenCodeCli::new(), OpenCodeConfig with new fields |
| `mcp_tool_agent_e2e.rs` | `rig-provider/src/mcp_agent.rs` | McpToolAgent::builder(), CliAdapter | VERIFIED | Example line 57: `McpToolAgent::builder()`, line 64: `CliAdapter::ClaudeCode` |
| `mcp_tool_agent_e2e.rs` | `mcp/src/server.rs` | ToolSetExt::into_handler().serve_stdio() | VERIFIED | Example line 53: `build_toolset().into_handler().await?.serve_stdio().await?` |

### Requirements Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| Phase 2.1 core value: agent forced through MCP tool constraints to submit conforming JSON | VERIFIED | McpToolAgent builder generates MCP config, sets allowed-tools whitelist, wires system prompt instructing agent to use tools. Full pipeline from ToolSet to CLI execution is automated. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No TODO, FIXME, PLACEHOLDER, HACK, XXX, .unwrap(), .expect() found in any new code |

### Compilation Checks

| Check | Status |
|-------|--------|
| `cargo check --workspace` | PASS (zero errors) |
| `cargo clippy --workspace` | PASS (zero warnings) |
| `cargo check --example mcp_tool_agent_e2e -p rig-provider` | PASS |
| `cargo check --example mcp_extraction_e2e -p rig-provider` | PASS |

### Human Verification Required

### 1. Claude Code End-to-End Run

**Test:** Run `cargo run --example mcp_tool_agent_e2e` with Claude Code CLI installed
**Expected:** Agent discovers CLI, spawns MCP server, calls tools (json_example, validate_json, submit), returns structured output
**Why human:** Requires Claude Code CLI binary installed and authenticated. Cannot verify network/CLI availability programmatically.

### 2. Codex End-to-End Run

**Test:** Build with Codex adapter: modify example to use `CliAdapter::Codex`, run
**Expected:** Agent discovers Codex binary, spawns MCP server, processes prompt through tools
**Why human:** Requires Codex CLI installed. TOML config format delivery mechanism may need testing.

### 3. OpenCode End-to-End Run

**Test:** Build with OpenCode adapter: modify example to use `CliAdapter::OpenCode`, run
**Expected:** Agent discovers OpenCode binary, OPENCODE_CONFIG env var points to generated config
**Why human:** Requires OpenCode CLI installed. OPENCODE_CONFIG env var mechanism may need testing.

### 4. Temp File Cleanup

**Test:** After a successful run, verify no temp files remain in system temp directory
**Expected:** NamedTempFile RAII guard deletes config file on drop
**Why human:** Requires running the binary and inspecting filesystem state

### Gaps Summary

No gaps found. All 6 observable truths are verified. All 11 artifacts pass existence, substantive, and wiring checks at all three levels. All 10 key links are verified. No anti-patterns detected. Workspace compiles cleanly with zero errors and zero clippy warnings. Both old and new examples compile.

The phase goal -- "User provides ToolSet + prompt, system handles all MCP plumbing transparently" -- is achieved. The `McpToolAgent::builder().toolset(ts).prompt(p).adapter(CliAdapter::ClaudeCode).run().await` API eliminates ~230 lines of boilerplate compared to the manual approach in `mcp_extraction_e2e.rs`.

---

*Verified: 2026-02-01T21:30:00Z*
*Verifier: Claude (gsd-verifier)*
