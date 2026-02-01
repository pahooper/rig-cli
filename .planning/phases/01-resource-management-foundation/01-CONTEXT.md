# Phase 1: Resource Management Foundation - Context

**Gathered:** 2026-02-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Subprocess execution is stable with bounded resources and no leaks. Covers bounded channels, task tracking, subprocess cleanup, stream draining, and error propagation. Retry logic, validation, payload injection, and containment are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Channel bounds & backpressure
- Stdout and stderr use separate bounded channels
- Stdout carries JSONL protocol messages; actual extraction content arrives via MCP submit tool, not the stream
- Stderr is captured and surfaced to callers (available in error results), not just logged
- Channel capacity is hardcoded (not configurable via builder API) — keep API simple
- Claude's discretion: channel capacity size and backpressure strategy (block vs drop)

### Timeout & cleanup behavior
- Default timeout exists (generous, hardcoded — not configurable via builder API)
- Shutdown sequence: SIGTERM first, wait for grace period, then SIGKILL if still alive
- On timeout, partial output collected so far is returned with the error (useful for debugging)
- Timeout value is hardcoded — can expose later if needed

### Error propagation strategy
- Rich context errors: include adapter name, pipeline stage, subprocess PID, elapsed time
- Non-zero exit code does NOT automatically mean error — check if valid MCP tool call was received first, then treat as success
- Zero panics in library code: every fallible operation returns Result, library never panics
- Use thiserror for typed error enums that callers can match on

### Stream draining semantics
- On normal exit: wait until streams are fully drained before returning (no timeout, no data loss)
- On kill (timeout/error): still attempt to drain remaining buffered output before cleanup
- Hard limit on total accumulated stream output to prevent OOM from runaway CLI
- When buffer limit is hit: truncate and warn (stop accumulating, let process continue, return what was captured with truncation warning)

### Claude's Discretion
- Exact bounded channel capacity
- Backpressure strategy (block producer vs drop oldest)
- Default timeout duration
- Grace period between SIGTERM and SIGKILL
- Buffer size hard limit value
- Internal task tracking implementation (JoinHandle management)

</decisions>

<specifics>
## Specific Ideas

- Agent communicates via JSONL on stdout — channel carries protocol messages, not payload content
- MCP submit tool is the actual content delivery mechanism, streams are for protocol coordination
- Exit code handling should be lenient: some CLIs exit non-zero on warnings but still produce valid output

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-resource-management-foundation*
*Context gathered: 2026-02-01*
