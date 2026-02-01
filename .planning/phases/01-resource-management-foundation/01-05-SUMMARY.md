---
phase: 01-resource-management-foundation
plan: 05
subsystem: infra
tags: [subprocess, zombie-prevention, graceful-shutdown, error-handling]

# Dependency graph
requires:
  - phase: 01-resource-management-foundation
    provides: Adapter process.rs rewrites from plans 01-01, 01-02, 01-03
provides:
  - Consistent graceful_shutdown across all three adapters
  - Zero zombie processes on timeout (child always reaped)
  - Proper error propagation in opencode-adapter shutdown
affects: [all adapter crates]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "graceful_shutdown takes &mut child for proper reaping"
    - "timeout(GRACE_PERIOD, child.wait()) replaces sleep(GRACE_PERIOD)"
    - "Timeout paths use let _ = to never mask Timeout error"

key-files:
  created: []
  modified:
    - opencode-adapter/src/process.rs
    - codex-adapter/src/process.rs

key-decisions:
  - "Match claudecode-adapter's graceful_shutdown pattern exactly across all adapters"
  - "Use let _ = for shutdown errors in timeout path (shutdown is best-effort, Timeout is the real error)"

patterns-established:
  - "All three adapters have identical graceful_shutdown: SIGTERM → timeout(5s, child.wait()) → child.kill() → child.wait()"

# Metrics
duration: 3min
completed: 2026-02-01
---

# Phase 01 Plan 05: Graceful Shutdown Consistency

**Fixed graceful_shutdown in codex-adapter and opencode-adapter to prevent zombie processes and use proper error propagation**

## Performance

- **Duration:** 3 min
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Eliminated `eprintln!` from opencode-adapter graceful_shutdown, replaced with proper `Result` returns
- Both codex and opencode now call `child.wait()` to reap the process (prevents zombies)
- Replaced `sleep(GRACE_PERIOD)` with `timeout(GRACE_PERIOD, child.wait())` for early exit when process dies
- Added `child.kill()` + `child.wait()` fallback when grace period expires
- Fixed codex timeout path: `?` replaced with `let _ =` to never mask Timeout error with SignalFailed
- All three adapters now have identical graceful_shutdown structure

## Task Commits

1. **Fix graceful_shutdown in codex/opencode** - `6b3a793`

## Changes

### opencode-adapter/src/process.rs
- `graceful_shutdown` signature: `(pid: u32)` → `(child: &mut Child, pid: u32) -> Result<(), OpenCodeError>`
- Removed `sleep` import (no longer needed)
- Replaced `eprintln!` with `map_err` returning `SignalFailed`/`SpawnFailed`
- Added `child.kill()` + `child.wait()` when grace period expires
- Timeout caller: `graceful_shutdown(pid)` → `let _ = graceful_shutdown(&mut child, pid)`

### codex-adapter/src/process.rs
- `graceful_shutdown` signature: `(pid: u32, tasks: &mut JoinSet<...>)` → `(child: &mut Child, pid: u32, tasks: &mut JoinSet<...>)`
- Removed `sleep` import
- Replaced `sleep(GRACE_PERIOD)` with `timeout(GRACE_PERIOD, child.wait())`
- Added `child.kill()` + `child.wait()` when grace period expires
- Timeout caller: `graceful_shutdown(pid, &mut tasks).await?` → `let _ = graceful_shutdown(&mut child, pid, &mut tasks).await`

## Deviations from Plan

None.

## Issues Encountered

None.

---
*Phase: 01-resource-management-foundation*
*Completed: 2026-02-01*
