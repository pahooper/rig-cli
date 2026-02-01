# Roadmap: rig-cli

## Overview

This roadmap transforms rig-cli from functional prototype to production-ready library. The journey spans 11 phases, starting with critical resource management fixes (bounded channels, task cleanup, zombie prevention) that prevent OOM kills and resource exhaustion. We then layer in the retry-with-validation loop that enables self-correcting structured extraction, followed by agent containment, observability, and platform hardening. The final phases polish the Rig integration, harden Claude Code and Codex adapters to production quality, maintain OpenCode functionality, and deliver comprehensive documentation. Each phase builds on stable foundations to deliver a CLI-agent provider that feels native to the Rig ecosystem.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Resource Management Foundation** - Bounded channels, task tracking, subprocess cleanup, error propagation
- [ ] **Phase 2: Retry & Validation Loop** - Self-correcting extraction with validation feedback
- [ ] **Phase 3: Payload & Instruction System** - Context data injection and forced tool workflow
- [ ] **Phase 4: Agent Containment** - MCP tool boundaries and sandbox enforcement
- [ ] **Phase 5: Observability Infrastructure** - Structured tracing and CLI version detection
- [ ] **Phase 6: Platform Hardening** - Cross-platform reliability for Linux and Windows
- [ ] **Phase 7: Rig Integration Polish** - Native Rig ecosystem feel
- [ ] **Phase 8: Claude Code Adapter** - Production hardening for primary adapter
- [ ] **Phase 9: Codex Adapter** - Production hardening for secondary adapter
- [ ] **Phase 10: OpenCode Maintenance** - Functional baseline without production hardening
- [ ] **Phase 11: Documentation & Examples** - End-to-end examples and comprehensive doc comments

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
  2. Retry loop attempts up to configurable max (default 3) with exponential backoff
  3. Token cost or attempt count is tracked per extraction
  4. Retry loop terminates after max attempts with clear failure indication
**Plans**: TBD

Plans:
- [ ] 02-01: TBD

### Phase 3: Payload & Instruction System
**Goal**: Developer can pass context data to agents and force tool workflow
**Depends on**: Phase 2
**Requirements**: EXTR-02, EXTR-03, EXTR-05
**Success Criteria** (what must be TRUE):
  1. Developer can attach file contents or text blobs to extraction request
  2. Built-in instruction template forces agents to use example → validate → submit workflow
  3. Agents cannot respond with freeform text instead of tool calls
  4. Three-tool pattern (example/validate/submit) is the enforced extraction mechanism
**Plans**: TBD

Plans:
- [ ] 03-01: TBD

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
**Plans**: TBD

Plans:
- [ ] 04-01: TBD

### Phase 5: Observability Infrastructure
**Goal**: Extraction workflow is fully traceable with version awareness
**Depends on**: Phase 4
**Requirements**: OBSV-01, OBSV-02
**Success Criteria** (what must be TRUE):
  1. Structured tracing logs every extraction stage (prompt sent, agent response, validation result, retry decisions)
  2. CLI tool versions are detected and validated at startup
  3. Clear warnings are shown when CLI tool version is unsupported
  4. Trace output enables debugging retry loops and agent behavior
**Plans**: TBD

Plans:
- [ ] 05-01: TBD

### Phase 6: Platform Hardening
**Goal**: Full functionality works on Linux and Windows
**Depends on**: Phase 5
**Requirements**: PLAT-01, PLAT-02, PLAT-05
**Success Criteria** (what must be TRUE):
  1. Subprocess spawning, temp directories, and config paths work identically on Pop!_OS and Windows
  2. CLI binary discovery handles .exe extensions and PATH differences correctly
  3. Setup registration works on both platforms without platform-specific code paths
  4. All external crate dependencies are well-maintained and stable
**Plans**: TBD

Plans:
- [ ] 06-01: TBD

### Phase 7: Rig Integration Polish
**Goal**: API surface feels like native Rig extension built by 0xPlaygrounds
**Depends on**: Phase 6
**Requirements**: PLAT-03, PLAT-04, QUAL-02
**Success Criteria** (what must be TRUE):
  1. CompletionModel, Tool, and ToolSet integrations use idiomatic Rig 0.29 patterns
  2. JsonSchemaToolkit and RigMcpHandler follow current MCP-centered approach
  3. Public API is simple and obvious for Rust developers
  4. Builder patterns and extension traits feel consistent with Rig's design language
**Plans**: TBD

Plans:
- [ ] 07-01: TBD

### Phase 8: Claude Code Adapter
**Goal**: Claude Code adapter is production-hardened as primary adapter
**Depends on**: Phase 7
**Requirements**: ADPT-01, QUAL-01
**Success Criteria** (what must be TRUE):
  1. All containment features (disable builtins, sandbox) work reliably
  2. All extraction features (retry, validation, payload) work reliably
  3. Passes clippy pedantic with zero warnings (root causes fixed, not suppressed)
  4. CLI flag combinations are tested and documented
**Plans**: TBD

Plans:
- [ ] 08-01: TBD

### Phase 9: Codex Adapter
**Goal**: Codex adapter is production-hardened as secondary adapter
**Depends on**: Phase 8
**Requirements**: ADPT-02
**Success Criteria** (what must be TRUE):
  1. All containment features work reliably with Codex CLI flags
  2. All extraction features work reliably with Codex response format
  3. Codex-specific CLI flags are audited and documented
  4. Passes clippy pedantic with zero warnings
**Plans**: TBD

Plans:
- [ ] 09-01: TBD

### Phase 10: OpenCode Maintenance
**Goal**: OpenCode adapter is functional but not production-hardened
**Depends on**: Phase 9
**Requirements**: ADPT-03
**Success Criteria** (what must be TRUE):
  1. OpenCode adapter compiles and runs basic extraction workflow
  2. Happy path works (valid JSON extraction on first try)
  3. Failure paths are documented but not hardened
  4. No investment in retry tuning or edge case handling
**Plans**: TBD

Plans:
- [ ] 10-01: TBD

### Phase 11: Documentation & Examples
**Goal**: Developer can understand and use library end-to-end from documentation
**Depends on**: Phase 10
**Requirements**: QUAL-03, QUAL-04
**Success Criteria** (what must be TRUE):
  1. End-to-end examples demonstrate extraction workflow with real CLI agents
  2. Examples show payload injection, retry handling, and error recovery
  3. All public types and methods have doc comments
  4. Doc comments explain the "why" not just the "what"
**Plans**: TBD

Plans:
- [ ] 11-01: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Resource Management Foundation | 5/5 | Complete | 2026-02-01 |
| 2. Retry & Validation Loop | 0/TBD | Not started | - |
| 3. Payload & Instruction System | 0/TBD | Not started | - |
| 4. Agent Containment | 0/TBD | Not started | - |
| 5. Observability Infrastructure | 0/TBD | Not started | - |
| 6. Platform Hardening | 0/TBD | Not started | - |
| 7. Rig Integration Polish | 0/TBD | Not started | - |
| 8. Claude Code Adapter | 0/TBD | Not started | - |
| 9. Codex Adapter | 0/TBD | Not started | - |
| 10. OpenCode Maintenance | 0/TBD | Not started | - |
| 11. Documentation & Examples | 0/TBD | Not started | - |
