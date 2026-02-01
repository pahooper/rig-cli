---
phase: 01-resource-management-foundation
plan: 03
subsystem: infra
tags: [rust, tokio, nix, subprocess, resource-management, bounded-channels]

# Dependency graph
requires:
  - phase: 01-resource-management-foundation
    provides: Resource management patterns from claude-adapter and codex-adapter rewrites
provides:
  - OpenCode adapter with bounded channels, JoinSet task tracking, and graceful shutdown
  - Zero-panic subprocess execution in opencode-adapter
  - Rich error context with PID, elapsed time, and partial output
  - 10MB output limit enforcement
affects: [02-mcp-server-foundation, testing, production-hardening]

# Tech tracking
tech-stack:
  added: [nix 0.29 with signal features]
  patterns:
    - Bounded mpsc channels (capacity: 100) for internal communication
    - JoinSet for reader task lifecycle management
    - SIGTERM → SIGKILL graceful shutdown with 5s grace period
    - Bounded output accumulation with MAX_OUTPUT_BYTES enforcement
    - Rich subprocess error context (PID, elapsed time, partial output)

key-files:
  created: []
  modified:
    - opencode-adapter/Cargo.toml
    - opencode-adapter/src/error.rs
    - opencode-adapter/src/process.rs
    - opencode-adapter/src/lib.rs

key-decisions:
  - "Apply resource management fixes to opencode-adapter despite deprioritization (infrastructure-level stability concern)"
  - "Use same bounded channel architecture as claude-adapter and codex-adapter for consistency"
  - "Remove AnyhowError and Other variants in favor of specific subprocess error types"

patterns-established:
  - "Consistent resource management across all adapters (same constants, same patterns)"
  - "Zero-panic subprocess handling via ? operator and proper error context"

# Metrics
duration: 3min
completed: 2026-02-01
---

# Phase 01 Plan 03: OpenCode Adapter Resource Management

**OpenCode adapter rewritten with bounded channels, JoinSet task tracking, SIGTERM/SIGKILL graceful shutdown, and zero panics**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-01T19:46:25Z
- **Completed:** 2026-02-01T19:49:31Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Replaced unbounded channels with bounded (capacity: 100) for internal stream communication
- Removed all Arc<Mutex<String>> in favor of bounded mpsc channels with proper backpressure
- Implemented JoinSet for reader task lifecycle (proper abort and cleanup)
- Added graceful shutdown: SIGTERM → 5s grace period → SIGKILL
- Enforced 10MB output limit with OutputTruncated error
- Eliminated all .expect(), .unwrap(), and panic! calls (zero panics verified)
- Added rich error context: PID, elapsed time, partial output on timeout
- Updated lib.rs stream() API from UnboundedSender to bounded Sender

## Task Commits

Each task was committed atomically:

1. **Task 1: Add nix dependency and rewrite OpenCodeError** - `d2473df` (feat)
   - Added nix 0.29 with signal features
   - Replaced simple error variants with rich subprocess context
   - Added PID, elapsed time, partial output to Timeout
   - Added subprocess-specific variants: StreamFailed, SignalFailed, NoStdout, NoStderr, NoPid, OutputTruncated, ChannelClosed
   - Removed AnyhowError and Other variants

2. **Task 2: Rewrite process.rs with bounded resources** - `d3b6536` (feat)
   - Bounded internal channels (CHANNEL_CAPACITY: 100)
   - JoinSet for reader task tracking and cleanup
   - Graceful shutdown (SIGTERM → GRACE_PERIOD: 5s → SIGKILL)
   - Output limit enforcement (MAX_OUTPUT_BYTES: 10MB)
   - Drain buffered output before cleanup
   - Updated lib.rs stream() to use bounded Sender
   - Fixed check_health() to use SpawnFailed instead of Other

## Files Created/Modified
- `opencode-adapter/Cargo.toml` - Added nix dependency with signal features
- `opencode-adapter/src/error.rs` - Rich subprocess error variants with PID, elapsed time, partial output
- `opencode-adapter/src/process.rs` - Complete rewrite with bounded channels, JoinSet, graceful shutdown, zero panics
- `opencode-adapter/src/lib.rs` - Updated stream() API to use bounded Sender, fixed check_health() error handling

## Decisions Made

None - plan executed exactly as specified. Applied same resource management patterns as claude-adapter and codex-adapter for consistency.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- OpenCode adapter now has same resource management guarantees as claude-adapter and codex-adapter
- All three adapters use consistent bounded channel architecture
- Ready for MCP server integration (Phase 02)
- Zero panics verified via cargo clippy with explicit unwrap/expect/panic lints

**No blockers.** Infrastructure-level resource management is complete for opencode-adapter.

---
*Phase: 01-resource-management-foundation*
*Completed: 2026-02-01*
