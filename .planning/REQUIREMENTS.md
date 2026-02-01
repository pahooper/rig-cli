# Requirements: rig-cli

**Defined:** 2026-02-01
**Core Value:** When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably.

## v1 Requirements

Requirements for v1.0 production release. Each maps to roadmap phases.

### Resource Management

- [ ] **RSRC-01**: All mpsc channels are bounded with configurable capacity and backpressure
- [ ] **RSRC-02**: All spawned tokio tasks are tracked via JoinHandles and aborted on timeout or drop
- [ ] **RSRC-03**: Subprocesses are properly killed and awaited (no zombie processes)
- [ ] **RSRC-04**: Stream readers are fully drained before process exit to prevent data loss
- [ ] **RSRC-05**: All `.expect()` and `.unwrap()` calls replaced with proper error propagation

### Structured Extraction

- [ ] **EXTR-01**: Retry loop feeds validation errors back to the agent and re-attempts (configurable max, default 3)
- [ ] **EXTR-02**: Developer can pass payload data (file contents, text blobs) alongside prompts for the agent to process
- [ ] **EXTR-03**: Built-in instruction template forces agents to use the submit tool workflow, not freeform text
- [ ] **EXTR-04**: Token cost or attempt count is tracked per extraction to enable cost awareness
- [ ] **EXTR-05**: The three-tool workflow (example/validate/submit) is the enforced extraction mechanism

### Agent Containment

- [ ] **CONT-01**: Default posture disables agent builtin tools (no file editing, bash, etc.) — only provided MCP tools
- [ ] **CONT-02**: Developer can explicitly opt-in to allow specific builtin tools or filesystem access when needed
- [ ] **CONT-03**: Per-CLI flags audited and applied for Claude Code and Codex to lock down agent behavior
- [ ] **CONT-04**: Agent execution is sandboxed to session temp directory by default, not host filesystem

### Observability

- [ ] **OBSV-01**: Structured tracing logs: prompt sent, agent response, validation result, retry decisions
- [ ] **OBSV-02**: CLI tool version detected and validated at startup with clear warning on unsupported versions

### Platform & Compatibility

- [ ] **PLAT-01**: Full functionality on Pop!_OS (Linux) and Windows — subprocess spawning, temp directories, config paths, setup registration
- [ ] **PLAT-02**: CLI binary discovery works reliably on both Linux and Windows (handles .exe, PATH differences)
- [ ] **PLAT-03**: Integrates with Rig 0.29 using idiomatic patterns (CompletionModel, Tool, ToolSet, extraction)
- [ ] **PLAT-04**: Uses current MCP-centered approach (JsonSchemaToolkit, RigMcpHandler, RMCP protocol)
- [ ] **PLAT-05**: External crates are well-maintained and stable (no experimental or abandoned dependencies)

### Code Quality

- [ ] **QUAL-01**: Passes clippy pedantic with zero warnings — root causes fixed, not suppressed with `#[allow]`
- [ ] **QUAL-02**: API surface is simple and obvious for Rust developers — feels like a native Rig extension
- [ ] **QUAL-03**: End-to-end examples demonstrate extraction workflow with real CLI agents
- [ ] **QUAL-04**: Doc comments on all public types and methods

### Adapters

- [ ] **ADPT-01**: Claude Code adapter is production-hardened with all containment and extraction features
- [ ] **ADPT-02**: Codex adapter is production-hardened with all containment and extraction features
- [ ] **ADPT-03**: OpenCode adapter is maintained and functional but not production-hardened for v1.0

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Extended Adapters

- **ADPT-04**: OpenCode adapter production-hardened to same standard as Claude Code and Codex
- **ADPT-05**: Gemini CLI adapter (or other future CLIs) supported via the same adapter pattern

### Advanced Features

- **ADVF-01**: Circuit breaker pattern — stop retrying after repeated failures
- **ADVF-02**: Composable retry policies — custom backoff strategies per adapter
- **ADVF-03**: Session TTL and LRU eviction for long-running servers
- **ADVF-04**: Prometheus metrics export
- **ADVF-05**: Streaming extraction progress — surface retry/validation status during streaming
- **ADVF-06**: Mock test harness for CI without real CLI agents

## Out of Scope

| Feature | Reason |
|---------|--------|
| GUI or web interface | Library/CLI tool only |
| Custom LLM hosting | Relies on whatever model the CLI tools use |
| Temperature/model parameter tuning | Limited by what CLIs expose |
| Framework or plugin system | Purpose-built, not a general extension framework |
| Real-time chat UI | Streaming paths are for programmatic use |
| MCP authentication/authorization | Relies on CLI tools' own auth; stdio transport is local |
| Graceful shutdown with CancellationToken | v2 operational hardening |
| Health check endpoints | v2 operational hardening |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| RSRC-01 | — | Pending |
| RSRC-02 | — | Pending |
| RSRC-03 | — | Pending |
| RSRC-04 | — | Pending |
| RSRC-05 | — | Pending |
| EXTR-01 | — | Pending |
| EXTR-02 | — | Pending |
| EXTR-03 | — | Pending |
| EXTR-04 | — | Pending |
| EXTR-05 | — | Pending |
| CONT-01 | — | Pending |
| CONT-02 | — | Pending |
| CONT-03 | — | Pending |
| CONT-04 | — | Pending |
| OBSV-01 | — | Pending |
| OBSV-02 | — | Pending |
| PLAT-01 | — | Pending |
| PLAT-02 | — | Pending |
| PLAT-03 | — | Pending |
| PLAT-04 | — | Pending |
| PLAT-05 | — | Pending |
| QUAL-01 | — | Pending |
| QUAL-02 | — | Pending |
| QUAL-03 | — | Pending |
| QUAL-04 | — | Pending |
| ADPT-01 | — | Pending |
| ADPT-02 | — | Pending |
| ADPT-03 | — | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 0
- Unmapped: 28

---
*Requirements defined: 2026-02-01*
*Last updated: 2026-02-01 after initial definition*
