---
phase: 01-resource-management-foundation
plan: 01
subsystem: infra
tags: [tokio, nix, signal-handling, error-handling, subprocess-management, bounded-channels]

# Dependency graph
requires:
  - phase: none (foundation)
    provides: n/a
provides:
  - Bounded subprocess execution with 100-capacity mpsc channels
  - Graceful shutdown via SIGTERM with 5-second grace period before SIGKILL
  - Rich error context with PID, elapsed time, partial output, and pipeline stage
  - 10MB output limit enforcement with OutputTruncated error
  - Zero-panic subprocess handling via Result propagation
affects: [01-02, 01-03, 01-04, all adapter crates, subprocess-intensive phases]

# Tech tracking
tech-stack:
  added: [nix 0.29 with signal support]
  patterns: [bounded-channel subprocess execution, JoinSet task tracking, graceful SIGTERM→SIGKILL shutdown, staged error context]

key-files:
  created: []
  modified:
    - claudecode-adapter/Cargo.toml
    - claudecode-adapter/src/error.rs
    - claudecode-adapter/src/process.rs
    - claudecode-adapter/src/lib.rs

key-decisions:
  - "Use bounded mpsc::channel(100) instead of unbounded for backpressure"
  - "Track reader tasks via JoinSet for proper lifecycle management and abort_all on timeout"
  - "Send SIGTERM before SIGKILL with 5-second grace period for graceful shutdown"
  - "Enforce 10MB MAX_OUTPUT_BYTES limit to prevent unbounded memory growth"
  - "Replace Arc<Mutex<String>> with Vec<String> for output accumulation"

patterns-established:
  - "Error variants carry subprocess context: PID, elapsed time, partial output, pipeline stage"
  - "All fallible operations use ? operator with stage-aware ClaudeError::SpawnFailed mapping"
  - "Reader tasks stop gracefully when receiver drops (no explicit ChannelClosed errors in happy path)"
  - "Timeout path drains remaining buffered output before cleanup"

# Metrics
duration: 4min
completed: 2026-02-01
---

# Phase 01 Plan 01: Bounded Subprocess Execution Summary

**Rewrote claudecode-adapter subprocess execution with bounded mpsc channels (100-capacity), JoinSet task tracking, SIGTERM→SIGKILL graceful shutdown (5s grace period), 10MB output limits, and rich error context propagation**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-01T19:44:39Z
- **Completed:** 2026-02-01T19:48:54Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Eliminated all panic-prone .expect() calls from subprocess execution
- Replaced unbounded channels with bounded mpsc::channel(100) for backpressure
- Implemented graceful shutdown: SIGTERM + 5s grace period before SIGKILL
- Added rich error context with PID, elapsed time, partial output, and pipeline stage
- Enforced 10MB output limit to prevent unbounded memory growth

## Task Commits

Each task was committed atomically:

1. **Task 1: Add nix dependency and rewrite ClaudeError with rich subprocess context** - `e81551f` (feat)
2. **Task 2: Rewrite process.rs with bounded channels, JoinSet, graceful shutdown** - `cc24125` (refactor)

## Files Created/Modified
- `claudecode-adapter/Cargo.toml` - Added nix dependency with signal support
- `claudecode-adapter/src/error.rs` - Rich ClaudeError enum with subprocess context variants (Timeout, SpawnFailed, StreamFailed, SignalFailed, NoStdout, NoStderr, NoPid, OutputTruncated, ChannelClosed)
- `claudecode-adapter/src/process.rs` - Complete rewrite with bounded channels, JoinSet, graceful shutdown, and helper functions (drain_stdout_bounded, drain_stderr_bounded, graceful_shutdown, collect_remaining)
- `claudecode-adapter/src/lib.rs` - Updated stream() signature to use bounded Sender

## Decisions Made

**Bounded channel capacity:** Chose 100-item capacity as balance between buffering and backpressure. Claude Code typically produces moderate output volumes where 100 lines provides sufficient buffering without unbounded growth.

**Grace period duration:** Set 5-second grace period between SIGTERM and SIGKILL based on typical CLI tool shutdown times. Most well-behaved processes exit within 1-2 seconds; 5 seconds provides ample margin.

**Output limit:** Set 10MB MAX_OUTPUT_BYTES limit to prevent OOM from pathological output while allowing legitimate large responses (e.g., multi-file diffs, extended diagnostics).

**Error context fields:** Included PID, elapsed time, partial output, and pipeline stage in error variants to maximize debugging utility when subprocess operations fail.

**Vec<String> accumulation:** Replaced Arc<Mutex<String>> with Vec<String> to eliminate holding mutex across await points and simplify ownership (no Arc cloning needed).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Borrow checker error with tokio::select! pattern matching:** Initial implementation pattern-matched on `Some(line) = rx.recv()` and `None = rx.recv()` separately, which created two mutable borrows of the same receiver. Fixed by pattern-matching on `result = rx.recv()` once and using match inside the arm.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for parallel rollout:** This pattern is now established for claudecode-adapter and should be replicated in codex-adapter (01-02) and opencode-adapter (01-03) with identical architecture.

**Remaining work in phase:** Plans 01-02, 01-03, and 01-04 will apply this pattern to other adapters and add integration tests to verify bounded resource behavior under load.

---
*Phase: 01-resource-management-foundation*
*Completed: 2026-02-01*
