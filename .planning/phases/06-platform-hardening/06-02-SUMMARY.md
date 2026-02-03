---
phase: 06-platform-hardening
plan: 02
subsystem: adapters
tags: [discovery, dirs, cross-platform, path-resolution, fallback-locations]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: Basic adapter structure with PATH-based discovery
provides:
  - Standardized 5-step binary discovery across all adapters
  - Platform-specific fallback location checks
  - Helpful install hints in error messages
  - Cross-platform home directory resolution via dirs crate
affects: [06-03-path-handling, future-adapter-implementations]

# Tech tracking
tech-stack:
  added: [dirs-5.0]
  patterns: [5-step-discovery-pattern, platform-specific-fallbacks, cfg-unix-windows]

key-files:
  created: []
  modified:
    - claudecode-adapter/src/discovery.rs
    - codex-adapter/src/discovery.rs
    - opencode-adapter/src/discovery.rs
    - claudecode-adapter/Cargo.toml
    - codex-adapter/Cargo.toml
    - opencode-adapter/Cargo.toml

key-decisions:
  - "All adapters follow same 5-step discovery pattern: explicit path, env var, PATH, fallbacks, helpful error"
  - "Use dirs::home_dir() for cross-platform home directory resolution instead of HOME env var"
  - "Platform-specific fallback locations use cfg(unix)/cfg(windows) compilation flags"
  - "Windows npm installs use .cmd wrappers, Go binaries use .exe extension"

patterns-established:
  - "Discovery pattern: explicit_path -> env var -> PATH -> platform-specific fallbacks -> install hint error"
  - "Fallback locations tailored to installation method: npm uses ~/.npm/bin, Go uses ~/go/bin"
  - "Error messages include both what was searched and how to install"

# Metrics
duration: 3min
completed: 2026-02-02
---

# Phase 6 Plan 2: Standardized Binary Discovery Summary

**All three adapters now have 5-step discovery with platform-specific fallback locations, dirs crate for cross-platform paths, and helpful install hints**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-03T01:40:18Z
- **Completed:** 2026-02-03T01:43:36Z
- **Tasks:** 2
- **Files modified:** 6 core files + 5 caller files

## Accomplishments
- Standardized discovery pattern across Claude, Codex, and OpenCode adapters
- Platform-specific fallback locations for Unix and Windows (npm and Go install paths)
- Helpful error messages guide users to correct installation commands
- Cross-platform home directory resolution via dirs crate

## Task Commits

Each task was committed atomically:

1. **Task 1: Enhance Claude discovery with fallback locations and install hints** - `cb3f04f` (feat)
2. **Task 2: Standardize Codex and OpenCode discovery to match Claude pattern** - `9fdab05` (feat)

## Files Created/Modified

**Core discovery files:**
- `claudecode-adapter/src/discovery.rs` - 5-step discovery with fallback locations and install hints
- `codex-adapter/src/discovery.rs` - Standardized to match Claude pattern
- `opencode-adapter/src/discovery.rs` - Standardized to match Claude pattern with Go-specific paths
- `claudecode-adapter/Cargo.toml` - Added dirs = "5.0"
- `codex-adapter/Cargo.toml` - Added dirs = "5.0"
- `opencode-adapter/Cargo.toml` - Added dirs = "5.0"

**Caller updates (signature change to accept Option<PathBuf>):**
- `rig-provider/src/mcp_agent.rs` - Updated run_codex and run_opencode to pass None
- `rig-provider/src/adapters/codex.rs` - Updated new() to pass None
- `rig-provider/src/adapters/opencode.rs` - Updated new() to pass None
- `rig-provider/examples/session_isolation.rs` - Updated to pass None
- `rig-provider/examples/streaming.rs` - Updated to pass None
- `rig-provider/examples/opencode_jsonl.rs` - Updated to pass None
- `rig-provider/examples/data_extraction.rs` - Updated to pass None

## Decisions Made

**1. 5-step discovery pattern for all adapters:**
- Step 1: Explicit path if provided
- Step 2: Environment variable (CC_ADAPTER_CLAUDE_BIN, CODEX_ADAPTER_BIN, OPENCODE_ADAPTER_BIN)
- Step 3: PATH lookup via which crate
- Step 4: Platform-specific fallback locations
- Step 5: Helpful error with install instructions

**2. Platform-specific fallback locations:**
- Unix npm: ~/.npm/bin, ~/.local/bin, /usr/local/bin
- Windows npm: AppData/Roaming/npm (with .cmd extension)
- Unix Go: ~/go/bin, ~/.local/bin, /usr/local/bin
- Windows Go: ~/go/bin (with .exe extension)

**3. dirs crate for cross-platform home resolution:**
- Replaces direct HOME env var access
- Handles HOME/USERPROFILE/HOMEPATH differences
- Safer for Windows compatibility

**4. Signature change to accept explicit_path:**
- Changed from `discover_X()` to `discover_X(explicit_path: Option<PathBuf>)`
- Enables future extensibility for custom binary paths
- Updated all callers to pass None for current behavior

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Discovery standardization complete. Ready for next hardening tasks:
- Path handling improvements (06-03)
- Signal handling cross-platform compatibility
- Dependency auditing

All three adapters now have consistent, robust binary discovery with clear user guidance when binaries are missing.

---
*Phase: 06-platform-hardening*
*Completed: 2026-02-02*
