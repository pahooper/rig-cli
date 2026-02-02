# rig-cli

## What This Is

A Rust crate that turns CLI-based AI coding agents (Claude Code, Codex) into idiomatic Rig 0.29 providers, enabling structured data extraction from agents that only expose a command-line interface. The core mechanism is a trio of MCP tools (submit, validate, example) that force agents to output schema-validated JSON through Rig's native `ToolSet` and `CompletionModel` patterns, making CLI agents behave like first-class Rig providers.

## Core Value

When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably — the agent is forced through MCP tool constraints to submit conforming JSON rather than freeform text.

## Requirements

### Validated

- ✓ Workspace with adapter crates for Claude Code, Codex, OpenCode — existing
- ✓ `CompletionModel` trait implemented for each adapter — existing
- ✓ `JsonSchemaToolkit` with submit/validate/example tools — existing
- ✓ `RigMcpHandler` bridging Rig `ToolSet` to MCP protocol — existing
- ✓ `ToolSetExt` extension trait for ergonomic MCP serving — existing
- ✓ Session-based sandboxing via `SessionManager` — existing
- ✓ Zero-config setup registration for CLI tool configs — existing
- ✓ Builder patterns for toolkit and handler configuration — existing

### Active

- [ ] Payload support — pass context data (file contents, text blobs) alongside prompts for the agent to process
- [ ] Agent containment — use per-CLI flags to restrict agent to only provided MCP tools (no built-in file editing, bash, etc.)
- [ ] Instruction injection — formalized system prompt that forces tool ordering (example → validate → submit) and prevents freeform text responses
- [ ] Retry and recovery loop — when agent submits invalid JSON, feed validation errors back and retry with configurable max attempts
- [ ] CLI flag audit — ensure each adapter passes correct flags for forced tool use, sandbox mode, output format
- [ ] Robust error handling — replace `.expect()` / `.unwrap()` with proper error propagation, handle stream race conditions
- [ ] Bounded channels — replace unbounded mpsc channels with bounded alternatives and backpressure
- [ ] Explicit task cancellation — clean up spawned tokio tasks on timeout/drop
- [ ] Token cost awareness — track or limit retry attempts to prevent cost spiraling
- [ ] Observability — structured tracing for prompt sent, agent response, validation result, retry decisions
- [ ] Mock adapter strategy — test harness that simulates CLI agent behavior without spawning real processes
- [ ] CLI version detection — discover and validate CLI tool versions at startup, warn on unsupported versions
- [ ] API surface polish — ensure all public types, builders, and traits feel native to the Rig ecosystem
- [ ] Documentation — doc comments and examples that show the extraction workflow end-to-end
- [ ] OpenCode adapter deprioritized — maintain but don't invest in hardening until Claude Code and Codex are solid

### Out of Scope

- GUI or web interface — this is a library/CLI tool only
- Custom LLM hosting — relies on whatever model the CLI tools use
- Temperature or model parameter tuning — limited by what CLIs expose
- Real-time chat UI — the chat/streaming paths exist for programmatic use, not interactive UIs
- Framework or plugin system — purpose-built, not a general extension framework
- OpenCode production hardening — deferred until primary adapters are solid
- Additional CLI adapters (Gemini, etc.) — future work, extension point exists

## Context

- Built rapidly via vibe coding; functional but brittle in edge cases
- Codebase analysis identified: stream race conditions, panicking on stream failures, silent JSON parsing failures, unbounded channels, no retry logic
- The project's foundation is sound — Rig's `ToolSet`, `CompletionModel`, `Tool` traits are used correctly
- The gap is hardening: the happy path works but failure paths are incomplete
- Primary consumers are Rust developers building pipelines/workflows where structured agent output must be reliable
- End users are downstream systems where this runs unattended — it must produce valid output without babysitting
- Rig 0.29 is the target — API should feel like it was built by 0xPlaygrounds as a natural Rig extension

## Constraints

- **Tech stack**: Rust, Rig 0.29, RMCP 0.14 — locked, no framework changes
- **CLI dependency**: Reliability bounded by what Claude Code and Codex CLIs expose (flags, output formats, containment mechanisms)
- **Complexity**: Purpose-built system — resist overengineering, no unnecessary abstractions
- **Adapter priority**: Claude Code and Codex are primary; OpenCode is maintained but not hardened for v1.0

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Force structured output via MCP tools rather than prompt-only | MCP tools give schema enforcement at the protocol level, not just hope the agent complies | — Pending |
| Three-tool pattern (submit/validate/example) | Gives agent a workflow: see example, validate draft, submit final — reduces invalid submissions | — Pending |
| Adapter-per-CLI crate structure | Clean separation of concerns, each CLI has different flags/behavior | ✓ Good |
| Best-effort containment per CLI | Each CLI has different sandbox mechanisms; document limitations rather than refuse to support | — Pending |
| Deprioritize OpenCode for v1.0 | Focus on getting two adapters rock solid rather than three mediocre | — Pending |
| Codex/OpenCode: prepend system prompt to user prompt (no --system-prompt flag) | E2E testing revealed flag doesn't exist in either CLI | Applied |
| Adapter-specific MCP config delivery (file vs -c overrides vs env var) | Each CLI has fundamentally different config mechanisms; one-size-fits-all impossible | Applied |
| OpenCode uses opencode/big-pickle model | E2E testing identified correct model for MCP agent execution | Applied |

---
*Last updated: 2026-02-01 after Phase 2.1 E2E testing*
