# Roadmap: rig-cli

## Overview

This roadmap transforms rig-cli from functional prototype to production-ready library. The journey spans 11 phases, starting with critical resource management fixes (bounded channels, task cleanup, zombie prevention) that prevent OOM kills and resource exhaustion. We then layer in the retry-with-validation loop that enables self-correcting structured extraction, followed by agent containment, observability, and platform hardening. The final phases polish the Rig integration and harden all three adapters (Claude Code, Codex, and OpenCode) to production quality, then deliver comprehensive documentation. Each phase builds on stable foundations to deliver a CLI-agent provider that feels native to the Rig ecosystem.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Resource Management Foundation** - Bounded channels, task tracking, subprocess cleanup, error propagation
- [x] **Phase 2: Retry & Validation Loop** - Self-correcting extraction with validation feedback
- [x] **Phase 2.1: Transparent MCP Tool Agent** - INSERTED: McpToolAgent builder that auto-spawns MCP server, generates config, and wires Claude CLI
- [x] **Phase 3: Payload & Instruction System** - Context data injection and forced tool workflow
- [x] **Phase 4: Agent Containment** - MCP tool boundaries and sandbox enforcement
- [x] **Phase 5: Observability Infrastructure** - Structured tracing and CLI version detection
- [x] **Phase 6: Platform Hardening** - Cross-platform reliability for Linux and Windows
- [x] **Phase 7: Rig Integration Polish** - Native Rig ecosystem feel
- [x] **Phase 8: Claude Code Adapter** - Production hardening for primary adapter
- [x] **Phase 9: Codex Adapter** - Production hardening for secondary adapter
- [x] **Phase 10: OpenCode Adapter** - Production hardening for third adapter (full parity)
- [x] **Phase 11: Documentation & Examples** - End-to-end examples and comprehensive doc comments

## Phase Details

### Phase 1: Resource Management Foundation
**Goal**: Subprocess execution is stable with bounded resources and no leaks
**Depends on**: Nothing (first phase)
**Requirements**: RSRC-01, RSRC-02, RSRC-03, RSRC-04, RSRC-05
**Success Criteria** (what must be TRUE):
  1. All stdout/stderr streams use bounded channels with configurable backpressure (no OOM from unbounded queues)
  2. All spawned tokio tasks are tracked via JoinHandles and properly aborted/awaited on timeout
  3. Subprocesses are killed and awaited without leaving zombie processes
  4. Stream readers fully drain before process exit (no data loss from race conditions)
  5. All error paths propagate errors instead of panicking (no .expect() or .unwrap() in stream handling)
**Plans**: 5 plans

Plans:
- [x] 01-01-PLAN.md — Rewrite claudecode-adapter with bounded channels, JoinSet, graceful shutdown, rich errors
- [x] 01-02-PLAN.md — Rewrite codex-adapter with bounded channels, JoinSet, graceful shutdown, rich errors
- [x] 01-03-PLAN.md — Rewrite opencode-adapter with bounded channels, JoinSet, graceful shutdown, rich errors
- [x] 01-04-PLAN.md — Update rig-provider callers to bounded channels, workspace-wide RSRC verification
- [x] 01-05-PLAN.md — Fix graceful_shutdown in codex/opencode: proper error returns, child reaping, no zombies

### Phase 2: Retry & Validation Loop
**Goal**: Agent self-corrects on validation errors through bounded retry attempts
**Depends on**: Phase 1
**Requirements**: EXTR-01, EXTR-04
**Success Criteria** (what must be TRUE):
  1. When agent submits invalid JSON, validation errors are fed back with specific field-level feedback
  2. Retry loop attempts up to configurable max (default 3) with immediate retry
  3. Token cost or attempt count is tracked per extraction
  4. Retry loop terminates after max attempts with clear failure indication
**Plans**: 2 plans

Plans:
- [x] 02-01-PLAN.md — Foundation types: ExtractionError, ExtractionMetrics, AttemptRecord, ExtractionConfig, validation feedback builder
- [x] 02-02-PLAN.md — ExtractionOrchestrator retry loop, enhanced ValidateJsonTool feedback, module wiring

### Phase 2.1: Transparent MCP Tool Agent (INSERTED)
**Goal**: User provides ToolSet + prompt, system handles all MCP plumbing transparently — no manual config, no dual-mode boilerplate, no RunConfig construction
**Depends on**: Phase 2
**Requirements**: Core value — agent forced through MCP tool constraints to submit conforming JSON
**Success Criteria** (what must be TRUE):
  1. McpToolAgent builder accepts a ToolSet and prompt, auto-spawns the current binary as MCP server via env var detection
  2. MCP config JSON is auto-generated with correct server name, tool names, and env vars
  3. Tool names are auto-computed as `mcp__<server_name>__<tool_name>` from ToolSet definitions
  4. Claude CLI is discovered and launched with correct --mcp-config, --allowed-tools flags
  5. Temp files are auto-cleaned via RAII guards
  6. Existing mcp_extraction_e2e example reduces from ~300 lines to ~50 lines using new API
**Plans**: 3 plans

Plans:
- [x] 02.1-01-PLAN.md — Add MCP config fields to Codex and OpenCode adapters
- [x] 02.1-02-PLAN.md — McpToolAgent builder, CliAdapter enum, and config generation
- [x] 02.1-03-PLAN.md — Simplified mcp_tool_agent_e2e example and workspace verification

### Phase 3: Payload & Instruction System
**Goal**: Developer can pass context data to agents and force tool workflow
**Depends on**: Phase 2.1
**Requirements**: EXTR-02, EXTR-03, EXTR-05
**Success Criteria** (what must be TRUE):
  1. Developer can attach file contents or text blobs to extraction request
  2. Built-in instruction template forces agents to use example → validate → submit workflow
  3. Agents cannot respond with freeform text instead of tool calls
  4. Three-tool pattern (example/validate/submit) is the enforced extraction mechanism
**Plans**: 2 plans

Plans:
- [x] 03-01-PLAN.md — Builder extensions (.payload(), .instruction_template()) and enhanced prompt construction with workflow enforcement
- [x] 03-02-PLAN.md — Payload extraction E2E example and workspace verification

### Phase 4: Agent Containment
**Goal**: Agents are locked to MCP tools only, no builtin tool escape
**Depends on**: Phase 3
**Requirements**: CONT-01, CONT-02, CONT-03, CONT-04
**Success Criteria** (what must be TRUE):
  1. Default configuration disables all agent builtin tools (file editing, bash, etc.)
  2. Developer can explicitly opt-in to allow specific builtin tools when needed
  3. Claude Code and Codex CLI flags are audited and applied for forced tool use
  4. Agent execution is sandboxed to session temp directory by default
  5. When provided only MCP tools, agent cannot access host filesystem
**Plans**: 2 plans

Plans:
- [x] 04-01-PLAN.md — Containment-first defaults and opt-in escape hatches on McpToolAgentBuilder
- [x] 04-02-PLAN.md — CLI flag audit tests for Claude Code and Codex containment flags

### Phase 5: Observability Infrastructure
**Goal**: Extraction workflow is fully traceable with version awareness
**Depends on**: Phase 4
**Requirements**: OBSV-01, OBSV-02
**Success Criteria** (what must be TRUE):
  1. Structured tracing logs every extraction stage (prompt sent, agent response, validation result, retry decisions)
  2. CLI tool versions are detected and validated at startup
  3. Clear warnings are shown when CLI tool version is unsupported
  4. Trace output enables debugging retry loops and agent behavior
**Plans**: 2 plans

Plans:
- [x] 05-01-PLAN.md — Instrument ExtractionOrchestrator with structured tracing spans and events at every extraction stage
- [x] 05-02-PLAN.md — CLI version detection and validation with semver parsing and structured warnings

### Phase 6: Platform Hardening
**Goal**: Full functionality works on Linux and Windows
**Depends on**: Phase 5
**Requirements**: PLAT-01, PLAT-02, PLAT-05
**Success Criteria** (what must be TRUE):
  1. Subprocess spawning, temp directories, and config paths work identically on Pop!_OS and Windows
  2. CLI binary discovery handles .exe extensions and PATH differences correctly
  3. Setup registration works on both platforms without platform-specific code paths
  4. All external crate dependencies are well-maintained and stable
**Plans**: 4 plans

Plans:
- [x] 06-01-PLAN.md — Cross-platform signal handling (cfg(unix)/cfg(windows) for all adapter process.rs)
- [x] 06-02-PLAN.md — Binary discovery with fallback locations, install hints, standardized 3-tier pattern
- [x] 06-03-PLAN.md — Path handling: dirs::home_dir() in setup.rs, OsString migration, example fixes
- [x] 06-04-PLAN.md — Dependency audit: justfile targets for cargo audit, semver strategy verification

### Phase 7: Rig Integration Polish
**Goal**: API surface feels like native Rig extension built by 0xPlaygrounds
**Depends on**: Phase 6
**Requirements**: PLAT-03, PLAT-04, QUAL-02
**Success Criteria** (what must be TRUE):
  1. CompletionModel, Tool, and ToolSet integrations use idiomatic Rig 0.29 patterns
  2. JsonSchemaToolkit and RigMcpHandler follow current MCP-centered approach
  3. Public API is simple and obvious for Rust developers
  4. Builder patterns and extension traits feel consistent with Rig's design language
  5. Two execution paths: agent() for direct CLI, mcp_agent() for MCP-enforced extraction
**Plans**: 7 plans

Plans:
- [x] 07-01-PLAN.md — Create rig-cli facade crate with Cargo.toml, feature flags, ClientConfig, and Error types
- [x] 07-02-PLAN.md — Claude Code Client implementing CompletionClient trait (reference provider pattern)
- [x] 07-03-PLAN.md — Codex and OpenCode Clients following Claude pattern
- [x] 07-04-PLAN.md — Prelude, escape hatches, debug-output feature, and API verification
- [x] 07-05-PLAN.md — CliAgent infrastructure: McpToolAgent streaming, CliAgent/CliAgentBuilder with Prompt/Chat traits
- [x] 07-06-PLAN.md — Integrate mcp_agent() into Claude, Codex, OpenCode Clients
- [x] 07-07-PLAN.md — Wire payload injection, fix dead code warnings, verify streaming parity

### Phase 8: Claude Code Adapter
**Goal**: Claude Code adapter is production-hardened as primary adapter
**Depends on**: Phase 7
**Requirements**: ADPT-01, QUAL-01
**Success Criteria** (what must be TRUE):
  1. All containment features (disable builtins, sandbox) work reliably
  2. All extraction features (retry, validation, payload) work reliably
  3. Passes clippy pedantic with zero warnings (root causes fixed, not suppressed)
  4. CLI flag combinations are tested and documented
**Plans**: 4 plans

Plans:
- [x] 08-01-PLAN.md — Fix all clippy pedantic warnings workspace-wide (doc_markdown, const fn, truncation)
- [x] 08-02-PLAN.md — Add CLI flag documentation and combination tests to cmd.rs
- [x] 08-03-PLAN.md — Add E2E containment tests with real Claude CLI (tests/e2e_containment.rs)
- [x] 08-04-PLAN.md — Add comprehensive extraction failure tests to orchestrator.rs

### Phase 9: Codex Adapter
**Goal**: Codex adapter is production-hardened as secondary adapter
**Depends on**: Phase 8
**Requirements**: ADPT-02
**Success Criteria** (what must be TRUE):
  1. All containment features work reliably with Codex CLI flags
  2. All extraction features work reliably with Codex response format
  3. Codex-specific CLI flags are audited and documented
  4. Passes clippy pedantic with zero warnings
**Plans**: 2 plans

Plans:
- [x] 09-01-PLAN.md — Add ApprovalPolicy enum, CLI flag documentation, and flag combination tests
- [x] 09-02-PLAN.md — Add E2E containment tests with real Codex CLI (tests/e2e_containment.rs)

### Phase 10: OpenCode Adapter
**Goal**: OpenCode adapter is production-hardened to full parity with Claude Code and Codex
**Depends on**: Phase 9
**Requirements**: ADPT-03
**Success Criteria** (what must be TRUE):
  1. All containment features work reliably with OpenCode CLI flags
  2. All extraction features work reliably with OpenCode response format
  3. OpenCode-specific CLI flags are audited and documented
  4. Passes clippy pedantic with zero warnings
  5. E2E containment tests pass with real OpenCode CLI
**Plans**: 2 plans

Plans:
- [x] 10-01-PLAN.md — Add comprehensive module-level documentation to cmd.rs and lib.rs, clippy pedantic pass
- [x] 10-02-PLAN.md — Add E2E containment tests with real OpenCode CLI (tests/e2e_containment.rs)

### Phase 11: Documentation & Examples
**Goal**: Developer can understand and use library end-to-end from documentation
**Depends on**: Phase 10
**Requirements**: QUAL-03, QUAL-04
**Success Criteria** (what must be TRUE):
  1. End-to-end examples demonstrate extraction workflow with real CLI agents
  2. Examples show payload injection, retry handling, and error recovery
  3. All public types and methods have doc comments
  4. Doc comments explain the "why" not just the "what"
**Plans**: 5 plans

Plans:
- [x] 11-01-PLAN.md — README + crate-level docs rewrite (concept-first, adapter comparison)
- [x] 11-02-PLAN.md — Enable #![warn(missing_docs)] on adapters + MCP, fix gaps
- [x] 11-03-PLAN.md — User story examples 1-4 (chat_mcp, one_shot, agent_mcp, agent_extra_tools)
- [x] 11-04-PLAN.md — User story examples 5-8 (multiagent, extraction, payload_chat, mcp_deterministic)
- [x] 11-05-PLAN.md — Error handling example + final verification

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 2.1 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Resource Management Foundation | 5/5 | Complete | 2026-02-01 |
| 2. Retry & Validation Loop | 2/2 | Complete | 2026-02-01 |
| 2.1 Transparent MCP Tool Agent | 3/3 | Complete | 2026-02-01 |
| 3. Payload & Instruction System | 2/2 | Complete | 2026-02-02 |
| 4. Agent Containment | 2/2 | Complete | 2026-02-02 |
| 5. Observability Infrastructure | 2/2 | Complete | 2026-02-02 |
| 6. Platform Hardening | 4/4 | Complete | 2026-02-03 |
| 7. Rig Integration Polish | 7/7 | Complete | 2026-02-03 |
| 8. Claude Code Adapter | 4/4 | Complete | 2026-02-03 |
| 9. Codex Adapter | 2/2 | Complete | 2026-02-03 |
| 10. OpenCode Adapter | 2/2 | Complete | 2026-02-03 |
| 11. Documentation & Examples | 5/5 | Complete | 2026-02-04 |
