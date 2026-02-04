# rig-cli

## What This Is

A Rust crate that turns CLI-based AI coding agents (Claude Code, Codex, OpenCode) into idiomatic Rig 0.29 providers, enabling structured data extraction from agents that only expose a command-line interface. The core mechanism is a trio of MCP tools (submit, validate, example) that force agents to output schema-validated JSON through Rig's native `ToolSet` and `CompletionModel` patterns, making CLI agents behave like first-class Rig providers.

## Core Value

When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably — the agent is forced through MCP tool constraints to submit conforming JSON rather than freeform text.

## Current State (v1.0 Shipped)

**Version:** v1.0 Production Release (2026-02-03)
**LOC:** 33,447 lines of Rust
**Tech stack:** Rust, Rig 0.29, RMCP 0.14

### What Was Built

- **rig-cli facade crate** with CompletionClient integration and two execution paths:
  - `client.agent()` → Direct CLI execution for simple prompts
  - `client.mcp_agent()` → MCP-enforced extraction with schema validation
- **Three production-hardened adapters**: Claude Code, Codex, OpenCode
- **Resource management**: Bounded channels (100 capacity), JoinSet task tracking, graceful shutdown (SIGTERM → SIGKILL)
- **Self-correcting extraction**: Retry loop with validation feedback, example → validate → submit workflow
- **Agent containment**: Default disables builtins, opt-in via `.allow_builtins()`, temp dir sandboxing
- **Cross-platform support**: Linux + Windows with platform-specific binary discovery and signal handling
- **Observability**: Structured tracing spans, CLI version detection at startup
- **Documentation**: 9 examples covering extraction, payload injection, multi-agent, error handling

### Primary Users

- Rust developers building pipelines/workflows where structured agent output must be reliable
- Downstream systems running unattended that need valid output without babysitting

## Requirements

### Validated (v1.0)

- ✓ RSRC-01 through RSRC-05: Resource management (bounded channels, JoinSet, graceful shutdown, error propagation) — v1.0
- ✓ EXTR-01 through EXTR-05: Structured extraction (retry loop, payload, instruction template, three-tool workflow) — v1.0
- ✓ CONT-01 through CONT-04: Agent containment (disable builtins, opt-in, CLI flags, temp dir sandboxing) — v1.0
- ✓ OBSV-01, OBSV-02: Observability (structured tracing, CLI version detection) — v1.0
- ✓ PLAT-01 through PLAT-05: Platform compatibility (Linux/Windows, binary discovery, Rig 0.29 integration) — v1.0
- ✓ QUAL-01 through QUAL-04: Code quality (clippy pedantic, simple API, examples, doc comments) — v1.0
- ✓ ADPT-01 through ADPT-03: Production-hardened adapters (Claude Code, Codex, OpenCode) — v1.0

### Active (v2.0)

- [ ] ADVF-01: Circuit breaker pattern — stop retrying after repeated failures
- [ ] ADVF-02: Composable retry policies — custom backoff strategies per adapter
- [ ] ADVF-03: Session TTL and LRU eviction for long-running servers
- [ ] ADVF-04: Prometheus metrics export
- [ ] ADVF-05: Streaming extraction progress — surface retry/validation status during streaming
- [ ] ADVF-06: Mock test harness for CI without real CLI agents
- [ ] ADPT-05: Gemini CLI adapter (or other future CLIs) via the same adapter pattern

### Out of Scope

- GUI or web interface — this is a library/CLI tool only
- Custom LLM hosting — relies on whatever model the CLI tools use
- Temperature or model parameter tuning — limited by what CLIs expose
- Real-time chat UI — the chat/streaming paths exist for programmatic use, not interactive UIs
- Framework or plugin system — purpose-built, not a general extension framework
- MCP authentication/authorization — relies on CLI tools' own auth; stdio transport is local

## Constraints

- **Tech stack**: Rust, Rig 0.29, RMCP 0.14 — locked, no framework changes
- **CLI dependency**: Reliability bounded by what Claude Code, Codex, and OpenCode CLIs expose (flags, output formats, containment mechanisms)
- **Complexity**: Purpose-built system — resist overengineering, no unnecessary abstractions

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Force structured output via MCP tools rather than prompt-only | MCP tools give schema enforcement at the protocol level, not just hope the agent complies | ✓ Good |
| Three-tool pattern (submit/validate/example) | Gives agent a workflow: see example, validate draft, submit final — reduces invalid submissions | ✓ Good |
| Adapter-per-CLI crate structure | Clean separation of concerns, each CLI has different flags/behavior | ✓ Good |
| Best-effort containment per CLI | Each CLI has different sandbox mechanisms; document limitations rather than refuse to support | ✓ Good |
| Two execution paths: agent() vs mcp_agent() | Direct CLI for simple prompts, MCP-enforced for structured extraction | ✓ Good |
| Codex/OpenCode: prepend system prompt to user prompt (no --system-prompt flag) | E2E testing revealed flag doesn't exist in either CLI | Applied |
| Adapter-specific MCP config delivery (file vs -c overrides vs env var) | Each CLI has fundamentally different config mechanisms; one-size-fits-all impossible | Applied |
| OpenCode uses opencode/big-pickle model | E2E testing identified correct model for MCP agent execution | Applied |
| Codex: removed ApprovalPolicy/ask_for_approval (v0.91.0 dropped --ask-for-approval) | Codex exec mode is inherently non-interactive; flag no longer exists | Applied |
| Codex: added skip_git_repo_check for temp dir containment | Temp directory containment creates non-git dirs; Codex requires --skip-git-repo-check | Applied |

## Tech Debt (v1.0)

Minor items tracked from milestone audit:

- opencode-adapter graceful_shutdown uses eprintln! instead of error propagation (inconsistency)
- codex/opencode adapters don't call child.wait() after kill in timeout path (brief zombie)
- error_handling.rs doc comment mentions "retry exhaustion" but example shows timeout/fallback

---
*Last updated: 2026-02-03 after v1.0 milestone*
