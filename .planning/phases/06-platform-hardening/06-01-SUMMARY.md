---
phase: 06-platform-hardening
plan: 01
subsystem: infra
tags: [rust, cross-platform, windows, unix, nix, process-management, signals]

# Dependency graph
requires:
  - phase: 01-phase-2.1
    provides: Unified subprocess pattern across all adapters with graceful shutdown
provides:
  - Cross-platform subprocess management for Windows and Unix
  - Platform-specific signal handling via cfg gates
  - Unix: SIGTERM → grace period → SIGKILL
  - Windows: immediate TerminateProcess
affects: [cross-compilation, windows-deployment, ci-cd]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cfg(unix)/cfg(windows) conditional compilation for platform-specific code"
    - "Platform-neutral error types using String descriptions instead of platform-specific types"
    - "Nix crate gated behind target-specific dependencies"

key-files:
  created: []
  modified:
    - claudecode-adapter/src/error.rs
    - claudecode-adapter/src/process.rs
    - claudecode-adapter/Cargo.toml
    - codex-adapter/src/error.rs
    - codex-adapter/src/process.rs
    - codex-adapter/Cargo.toml
    - opencode-adapter/src/error.rs
    - opencode-adapter/src/process.rs
    - opencode-adapter/Cargo.toml

key-decisions:
  - "Use cfg(unix)/cfg(windows) instead of runtime platform detection for zero-cost abstraction"
  - "Windows graceful shutdown uses immediate Child::kill() (TerminateProcess) - documented limitation"
  - "Platform-neutral error types carry string descriptions instead of Unix-specific errno types"
  - "Nix imports moved inside cfg(unix) function bodies, not at top-level"

patterns-established:
  - "Platform-specific code: import platform-specific crates inside cfg-gated functions, not at file top-level"
  - "Cross-platform error handling: use String descriptions for platform-specific errors, not platform-specific types in public API"
  - "Target-specific dependencies: gate Unix-only crates behind [target.'cfg(unix)'.dependencies]"

# Metrics
duration: 3min
completed: 2026-02-03
---

# Phase 06 Plan 01: Cross-Platform Process Management Summary

**Unix and Windows subprocess management with cfg-gated signal handling: Unix preserves SIGTERM graceful shutdown, Windows uses immediate TerminateProcess**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-03T01:39:11Z
- **Completed:** 2026-02-03T01:42:35Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- All three adapters (claudecode-adapter, codex-adapter, opencode-adapter) compile on both Unix and Windows targets
- Unix behavior preserved exactly: SIGTERM → 5-second grace period → SIGKILL
- Windows implementation uses immediate TerminateProcess via Child::kill()
- Platform-neutral error types eliminate Unix-specific dependencies in public API

## Task Commits

Each task was committed atomically:

1. **Task 1: Make error types platform-neutral across all adapters** - `95a14ab` (refactor)
2. **Task 2: Add cfg(unix)/cfg(windows) to process.rs and gate nix dependency** - `64703fb` (feat)

## Files Created/Modified
- `claudecode-adapter/src/error.rs` - SignalFailed variant now uses reason: String instead of nix::errno::Errno
- `claudecode-adapter/src/process.rs` - Unix/Windows graceful_shutdown implementations with cfg gates
- `claudecode-adapter/Cargo.toml` - Nix dependency gated behind [target.'cfg(unix)'.dependencies]
- `codex-adapter/src/error.rs` - SignalFailed variant now uses reason: String instead of nix::errno::Errno
- `codex-adapter/src/process.rs` - Unix/Windows graceful_shutdown implementations with cfg gates
- `codex-adapter/Cargo.toml` - Nix dependency gated behind [target.'cfg(unix)'.dependencies]
- `opencode-adapter/src/error.rs` - SignalFailed variant now uses reason: String instead of nix::errno::Errno
- `opencode-adapter/src/process.rs` - Unix/Windows graceful_shutdown implementations with cfg gates, removed pid_to_nix helper
- `opencode-adapter/Cargo.toml` - Nix dependency gated behind [target.'cfg(unix)'.dependencies]

## Decisions Made
- **cfg gates instead of runtime detection:** Using compile-time conditional compilation (cfg attributes) provides zero-cost abstraction - Windows builds never include Unix signal code and vice versa
- **Windows limitation documented:** Windows console processes cannot be gracefully shut down like Unix processes. Child::kill() calls TerminateProcess which immediately terminates. This is a platform limitation, not a bug
- **Nix imports inside functions:** All nix crate imports moved inside cfg(unix) function bodies to prevent any unconditional dependency on Unix-only code
- **Platform-neutral errors:** Error types use String for platform-specific error descriptions instead of carrying Unix-specific types like nix::errno::Errno, making the error API fully cross-platform

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all three adapters compiled successfully on first attempt after changes. All existing tests passed without modification. The plan's detailed implementation guidance allowed for straightforward execution.

## User Setup Required

None - no external service configuration required. This is a pure code refactoring for cross-platform compatibility.

## Next Phase Readiness

All adapters are now cross-platform compatible:
- ✅ Compile on both Unix (Linux, macOS) and Windows targets
- ✅ Unix behavior preserved exactly (no regressions)
- ✅ Windows support added with documented limitations
- ✅ All existing tests pass
- ✅ No new clippy warnings introduced

Ready for Phase 06 Plan 02: Cross-compilation verification and Windows CI testing.

**Blocker check:** None - all dependencies are met for CI/CD integration testing.

**Technical debt:** Windows graceful shutdown limitation is a platform constraint, not addressable without OS-level changes. This is acceptable for CLI subprocess management.

---
*Phase: 06-platform-hardening*
*Completed: 2026-02-03*
