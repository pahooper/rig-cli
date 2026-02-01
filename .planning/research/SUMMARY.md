# Project Research Summary

**Project:** rig-cli
**Domain:** CLI-agent Rig provider with structured extraction and MCP integration
**Researched:** 2026-02-01
**Confidence:** HIGH

## Executive Summary

This project hardens an existing CLI-agent Rig provider from prototype to production (v1.0). The system wraps CLI agents (Claude Code, Codex, OpenCode) as Rig CompletionModel implementations, exposing them via MCP protocol for structured JSON extraction. Research confirms the core architecture is sound—Rig 0.29 integration patterns are idiomatic, the adapter layering is clean, and the three-tool extraction workflow (example/validate/submit) is production-ready. However, critical production gaps exist: unbounded channels risk OOM, orphaned async tasks leak resources, zombie processes accumulate, and retry logic is completely missing despite being industry-standard for LLM structured extraction.

The recommended hardening approach prioritizes resource management first (bounded channels, task cleanup, zombie prevention), then implements the missing retry-with-validation-feedback loop that competitors like Instructor have proven essential. Agent containment comes next—ensuring CLI agents can't escape MCP tool boundaries via builtin tool fallbacks. Final phases add observability (tracing spans, metrics) and polish (payload injection, cost tracking). This ordering prevents the production-blocking resource exhaustion bugs from blocking adoption while delivering the core structured-extraction value proposition incrementally.

Key risks are manageable: the rig-core 0.29.0 version discrepancy needs verification (crates.io shows 0.23.1), Codex CLI containment flags require investigation (not documented), and stream race conditions between process exit and pipe closure need careful sequencing. These are all solvable during implementation. The architectural foundation is solid; hardening focuses on production reliability patterns that are well-documented in the Tokio and LLM observability ecosystems.

## Key Findings

### Recommended Stack

The existing stack is production-appropriate with targeted additions needed. Core dependencies (rig-core 0.29, rmcp 0.14, tokio 1.x, serde, schemars) are correctly chosen. Research validates adding thiserror for library error types, anyhow for application errors, tracing/tracing-subscriber for observability, and replacing unbounded channels with bounded mpsc::channel. The version discrepancy for rig-core (0.29.0 in code vs 0.23.1 on crates.io) requires immediate verification—likely a local/unreleased version.

**Core technologies:**
- **rig-core 0.29.0**: CompletionModel trait abstraction — idiomatic provider pattern (verify version availability)
- **rmcp 0.14.0**: MCP server with #[tool] macro — standard for tool protocol compliance
- **tokio 1.x**: Async runtime with process spawning — required for subprocess lifecycle and backpressure
- **thiserror + anyhow**: Layered error handling — library crates use thiserror, application uses anyhow
- **tracing**: Structured observability — async-aware spans for prompt→agent→validation flow
- **bounded mpsc channels**: Backpressure control — prevents OOM from unbounded queues (critical fix)

**Testing additions:**
- **test-binary**: Mock CLI agents for integration tests — avoids dependency on real CLI tools
- **assert_cmd**: CLI subprocess testing — standard for testing binary invocations

### Expected Features

Research confirms structured extraction with retry-on-validation-failure is table stakes, not optional. Competitors (Instructor, LlamaExtract) all provide automatic retry with error feedback. The current implementation has the validation tools but no retry loop—agents fail on first validation error instead of self-correcting.

**Must have (table stakes):**
- **Schema-validated JSON extraction** — already implemented via JsonSchemaToolkit ✓
- **Retry loop with validation feedback** — CRITICAL GAP: currently single-shot, needs 3-5 retry attempts
- **Bounded retry attempts** — CRITICAL GAP: prevents cost spirals from unbounded retries
- **Error propagation without panics** — CRITICAL GAP: .expect() in stream handling causes crashes
- **Basic observability** — CRITICAL GAP: no tracing of extraction stages (prompt→response→validation)
- **MCP protocol compliance** — already implemented ✓
- **Session isolation** — already implemented via SessionManager ✓

**Should have (competitive advantage):**
- **Type-driven schema generation** — already implemented via schemars::JsonSchema ✓
- **Three-tool workflow** — already implemented (example/validate/submit) ✓
- **Token cost tracking** — missing, valuable for preventing retry cost spirals
- **Payload injection** — missing, enables batch extraction workflows
- **Per-CLI containment flags** — missing, needed for security-conscious deployments
- **Streaming extraction progress** — partial (streaming exists but not extraction-aware)

**Defer (v2+):**
- **Circuit breaker pattern** — nice-to-have, only after retry loop exists
- **Composable retry policies** — defer until custom backoff requirements emerge
- **OpenCode production hardening** — maintain but focus on Claude Code + Codex for v1.0

### Architecture Approach

The adapter pattern is well-executed. CLI adapters (claudecode-adapter, codex-adapter, opencode-adapter) implement subprocess management and output parsing. Provider layer (rig-provider) wraps adapters as Rig CompletionModel implementations. MCP layer (mcp crate) bridges Rig ToolSet to RMCP protocol. This clean separation enables adapter-specific hardening without touching MCP logic.

**Major components:**
1. **CLI Adapters** — subprocess spawning, stream parsing, retry logic (needs bounded channels, task tracking)
2. **Provider Models** — CompletionModel trait implementation, RunConfig construction, session management (needs retry orchestration)
3. **MCP Handler** — RMCP ServerHandler, tool definition translation, call routing (production-ready)
4. **JsonSchemaToolkit** — three-tool extraction workflow (submit/validate/example) (production-ready)
5. **SessionManager** — isolated temp directories per session (production-ready, could use LRU eviction)

**Key patterns validated:**
- **Extraction loop**: Multi-stage validation (parse→schema→semantic) with error budget
- **Bounded resource pool**: Semaphore-limited concurrent subprocesses to prevent exhaustion
- **Layered error propagation**: ClaudeError→ProviderError→CompletionError with From impls
- **Subprocess lifecycle**: Track JoinHandles, abort on timeout, await cleanup

### Critical Pitfalls

Research identified seven critical pitfalls with production impact. Top five for v1.0:

1. **Unbounded channel memory explosion** — Current code uses unbounded mpsc for streaming. Fast CLI output outpaces slow consumer, channel queue grows unbounded, OOM kill. Fix: Replace with mpsc::channel(1000) for backpressure. (Phase 1)

2. **Orphaned async tasks from untracked spawns** — Stdout/stderr reader tasks spawned without storing JoinHandles. On timeout, tasks continue running, leaking memory. Fix: Store handles, call abort(), then await to ensure cleanup. (Phase 1)

3. **Zombie process accumulation** — child.kill() without child.wait() leaves zombies consuming process table slots. Fix: Always kill().await then wait().await. Test by running 100 timeout scenarios and checking for <defunct> processes. (Phase 1)

4. **LLM schema drift despite tool constraints** — LLMs rename fields, add unexpected keys, wrap JSON in markdown despite schema. Fix: Implement retry loop that feeds validation errors back to agent with specific corrections. (Phase 2)

5. **Agent containment bypass via builtin tool fallback** — Current RunConfig sets builtin:Default, allowing agents to escape MCP tools and use file system/bash. Fix: Set builtin:None, test by providing insufficient MCP tools and verifying agent fails rather than escapes. (Phase 2)

**Secondary pitfalls for later phases:**
6. **Stream race condition** — Process exits before reader tasks consume all buffered output, losing final chunk. (Phase 1)
7. **MCP tool poisoning** — Malicious tool descriptions can inject instructions or shadow trusted tools. (Phase 3)

## Implications for Roadmap

Based on research, suggested phase structure balances production-blocking fixes with value delivery:

### Phase 1: Error Handling & Resource Management
**Rationale:** Production-blocking bugs first. Unbounded channels, task leaks, and zombie processes cause OOM kills and resource exhaustion in long-running servers. These must be fixed before any feature work or deployment.

**Delivers:** Stable subprocess execution with bounded resources
- Replace unbounded channels with bounded mpsc::channel(1000)
- Track JoinHandles and implement abort→await cleanup pattern
- Fix zombie processes via kill().await + wait().await sequencing
- Replace .expect() in stream handling with proper error propagation
- Implement stream race condition fix (await readers before process)

**Uses:** tokio::sync::mpsc::channel, tokio::process patterns, thiserror for adapter errors

**Avoids:**
- Pitfall 1: Unbounded channel OOM
- Pitfall 2: Orphaned async tasks
- Pitfall 3: Zombie processes
- Pitfall 6: Stream race conditions

**Research flag:** SKIP — Well-documented Tokio patterns, official docs comprehensive

### Phase 2: Retry Logic & Validation Loop
**Rationale:** Core value proposition requires retry. Competitors have this; we must match. Enables self-correcting structured extraction that handles LLM schema drift.

**Delivers:** Production-quality structured extraction
- Implement retry loop in CLI adapters (exponential backoff, max 3-5 attempts)
- Feed validation errors from ValidateJsonTool back to agent with specific field-level feedback
- Add bounded retry policy configuration (max attempts, timeout per attempt)
- Track retry metrics (attempt count, success rate, cumulative cost)

**Features from FEATURES.md:**
- Retry loop with validation feedback (table stakes)
- Bounded retry attempts (table stakes)

**Avoids:**
- Pitfall 4: LLM schema drift
- Technical debt: Unbounded retry loops causing cost spirals

**Research flag:** MEDIUM — Retry patterns well-documented, but adapter-specific integration needs design

### Phase 3: Agent Containment & Observability
**Rationale:** Security model depends on MCP tool boundaries. Tracing enables debugging retry loops and cost tracking.

**Delivers:** Secure sandboxing + visibility into extraction workflow
- Set builtin:None in RunConfig to prevent tool escape
- Audit CLI containment flags (Claude Code verified, Codex needs investigation)
- Add tracing spans: prompt_sent, agent_response, validation, retry_decision
- Implement structured logging (JSON for prod, pretty for dev)
- Add basic metrics (success rate, latency p50/p99, retry count)

**Features from FEATURES.md:**
- Basic observability (table stakes)
- Per-CLI containment flags (competitive advantage, partial)

**Avoids:**
- Pitfall 5: Agent containment bypass
- Pitfall 7: MCP tool poisoning (via tool allowlisting)

**Research flag:** HIGH for Codex — CLI flags not documented, need --help investigation

### Phase 4: Payload Injection & Cost Tracking
**Rationale:** Enables batch workflows and cost awareness. Both enhance competitive position but not blocking.

**Delivers:** Enhanced extraction capabilities
- Add payload support to RunConfig (FileContent, JsonData types)
- Extend CompletionRequest with payload injection mechanism
- Track token usage per extraction attempt (parse from stderr/metadata)
- Expose cost metrics in response metadata

**Features from FEATURES.md:**
- Payload injection (competitive advantage)
- Token cost tracking (competitive advantage)

**Avoids:**
- UX pitfall: Bill shock from unexpected retry costs

**Research flag:** SKIP — Standard pattern, no novel research needed

### Phase 5: Production Hardening
**Rationale:** Final polish for deployment readiness. Graceful shutdown, health checks, session cleanup.

**Delivers:** Production deployment infrastructure
- Implement graceful shutdown (CancellationToken pattern)
- Add session TTL and LRU eviction to SessionManager
- Implement health checks (CLI availability, disk space)
- Add Prometheus metrics export
- Production logging configuration (log levels, rotation)

**Research flag:** SKIP — Standard production patterns

### Phase Ordering Rationale

- **Resource management first** because OOM/zombie bugs prevent any deployment testing
- **Retry loop second** because it depends on stable subprocess execution and delivers core value
- **Observability third** because retry debugging needs tracing visibility
- **Payload/cost fourth** because they're enhancements on top of working extraction
- **Hardening last** because it requires all prior phases stable

**Dependencies identified:**
- Phase 2 (retry) requires Phase 1 (stable subprocess) — can't retry if tasks leak
- Phase 3 (observability) enhances Phase 2 (retry) — trace retry attempts
- Phase 4 (payload) is orthogonal — can parallelize with Phase 3 if needed
- Phase 5 (hardening) consumes all prior phases

**Grouping rationale:**
- Phase 1 groups all tokio resource management fixes (channels, tasks, processes)
- Phase 2 groups all LLM interaction patterns (retry, validation, feedback)
- Phase 3 groups security + visibility (related by debugging threat model)
- Phase 4 groups user-facing enhancements (payload, cost)
- Phase 5 groups operational concerns (shutdown, health, metrics)

### Research Flags

**Needs deeper research during planning:**
- **Phase 3:** Codex CLI containment flags (run `codex --help`, audit sandbox options)
- **Phase 3:** MCP tool allowlisting patterns (verify rmcp supports per-tool authz)

**Standard patterns (skip phase-specific research):**
- **Phase 1:** Tokio subprocess lifecycle, channel backpressure (official docs comprehensive)
- **Phase 2:** Retry with exponential backoff (well-documented pattern, see BackON crate)
- **Phase 4:** Payload injection (standard arg construction pattern)
- **Phase 5:** Graceful shutdown with CancellationToken (Tokio shutdown guide covers this)

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | rmcp, tokio, serde verified via official docs; rig-core 0.29.0 version needs verification against crates.io (shows 0.23.1) |
| Features | HIGH | Retry loop patterns verified from Instructor, LlamaExtract, production LLM guides; table stakes vs competitive features well-researched |
| Architecture | MEDIUM-HIGH | Rig patterns verified from codebase, Tokio patterns from official docs; some production patterns extrapolated from 2026 best practices |
| Pitfalls | HIGH | Unbounded channels, task leaks, zombies verified from Tokio issues and docs; LLM drift and MCP security verified from 2026 research |

**Overall confidence:** HIGH

Research is comprehensive enough to inform roadmap. Minor gaps don't block planning.

### Gaps to Address

**During roadmap planning:**
- **rig-core version**: Check Cargo.lock for source (git dependency? local path?). If 0.29.0 is unreleased, may need to pin or adjust API usage.
- **Codex CLI flags**: Run `codex --help` to discover containment flags. If undocumented, may need to deprioritize Codex hardening for v1.0.
- **Channel bound sizing**: Research suggests 1000-10000; tune based on testing. Too small = backpressure delays, too large = memory pressure.
- **Retry timeout strategy**: Start with 30s initial, 60s retry, 3 max attempts (150s worst case). Validate with real extraction workloads.

**During implementation:**
- **Test zombie cleanup**: Integration test must verify no <defunct> processes remain after timeout scenarios
- **Benchmark channel backpressure**: Measure memory usage with bounded vs unbounded under heavy streaming load
- **Validate schema retry loop**: Test with deliberate schema violations to verify agent self-corrects
- **Audit builtin tool escape**: Provide only MCP tools, verify agent cannot access file system

## Sources

### Primary (HIGH confidence)
- **Context7 documentation**:
  - rmcp 0.14.0 — Server features, tool macros, task lifecycle
  - Tokio channels — Bounded channel backpressure patterns
  - Tokio process — Subprocess spawning, zombie prevention
  - Tokio graceful shutdown — CancellationToken patterns
  - Schemars — JSON Schema generation
- **Official documentation**:
  - [Rig framework](https://docs.rig.rs/) — CompletionModel trait patterns
  - [Tokio documentation](https://tokio.rs/) — Process, channels, task spawning, shutdown
  - [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — CLI flags, containment options

### Secondary (MEDIUM confidence)
- **Web search verified**:
  - [rig-core on crates.io](https://crates.io/crates/rig-core) — Version 0.23.1 shown (conflicts with 0.29.0 in code)
  - [rmcp on crates.io](https://crates.io/crates/rmcp) — Current version 0.14.0 verified
  - [thiserror/anyhow best practices (2026)](https://github.com/oneuptime/blog/tree/master/posts/2026-01-25-error-types-thiserror-anyhow-rust) — Library vs application patterns
  - [Rust tracing structured logging (2026)](https://oneuptime.com/blog/post/2026-01-07-rust-tracing-structured-logs/view) — Production configuration
  - [Instructor library](https://python.useinstructor.com/) — Retry with validation feedback patterns
  - [LLM tool calling in production (2026)](https://medium.com/@komalbaparmar007/llm-tool-calling-in-production-rate-limits-retries-and-the-infinite-loop-failure-mode-you-must-2a1e2a1e84c8) — Retry loops, rate limits
  - [AI observability tools 2026](https://www.braintrust.dev/articles/best-ai-observability-tools-2026) — Trace IDs, structured logs
  - [MCP security risks 2026](https://www.redhat.com/en/blog/model-context-protocol-mcp-understanding-security-risks-and-controls) — Tool poisoning, prompt injection
  - [Tokio task cancellation patterns](https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/) — JoinHandle abort and cleanup

### Tertiary (LOW confidence)
- **Community patterns**:
  - [Mastering Tokio channels](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260) — Backpressure patterns
  - [Rust subprocess containment](https://github.com/ebkalderon/bastille) — Sandboxing libraries
  - [BackON retry crate](https://xuanwo.io/2024/08-backon-reaches-v1/) — Retry API design

### Existing Codebase Analysis
- `/home/pnod/dev/projects/rig-cli/.planning/codebase/CONCERNS.md` — Identified unbounded channels, task leaks, panics
- `/home/pnod/dev/projects/rig-cli/.planning/codebase/ARCHITECTURE.md` — Validated adapter pattern, session isolation
- `/home/pnod/dev/projects/rig-cli/.planning/PROJECT.md` — Confirmed v1.0 hardening goal

---
*Research completed: 2026-02-01*
*Ready for roadmap: yes*
