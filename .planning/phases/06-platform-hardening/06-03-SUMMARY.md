---
phase: 06-platform-hardening
plan: 03
subsystem: infra
tags: [cross-platform, dirs, pathbuf, windows, linux, macos, path-handling]

# Dependency graph
requires:
  - phase: 06-02
    provides: Platform-specific fallback locations for config discovery
provides:
  - Cross-platform home directory resolution via dirs crate
  - PathBuf-based path handling throughout setup and MCP agent
  - Documented to_string_lossy() usage at serialization boundaries
affects: [future-windows-support, config-management, cross-platform-compatibility]

# Tech tracking
tech-stack:
  added: [dirs 5.0]
  patterns: [PathBuf-first path handling, display().to_string() at serialization boundaries, dirs::home_dir() for cross-platform home resolution]

key-files:
  created: []
  modified: [rig-provider/src/setup.rs, rig-provider/src/mcp_agent.rs, rig-provider/examples/claudecode_mcp.rs, rig-provider/examples/opencode_jsonl.rs, rig-provider/Cargo.toml]

key-decisions:
  - "Use dirs::home_dir() instead of HOME env var for cross-platform home directory resolution"
  - "Keep paths as PathBuf/&Path as long as possible, convert to String only at serialization boundaries"
  - "Use display().to_string() over to_string_lossy() for idiomatic path-to-string conversion"
  - "Document all to_string_lossy() usage with inline comments explaining why conversion is acceptable"

patterns-established:
  - "Path handling: Stay in PathBuf/&Path domain as long as possible, convert to String only when required by external formats (JSON, TOML, CLI args)"
  - "Cross-platform home directory: Use dirs::home_dir() instead of platform-specific env vars"
  - "Path serialization: Use display().to_string() for display/logging, to_string_lossy() only when data loss is acceptable"

# Metrics
duration: 3min 44s
completed: 2026-02-03
---

# Phase 6 Plan 3: Cross-Platform Path Handling Summary

**Migrated from Unix-only HOME env var to cross-platform dirs::home_dir() and PathBuf-first path handling**

## Performance

- **Duration:** 3 min 44 sec
- **Started:** 2026-02-03T01:47:58Z
- **Completed:** 2026-02-03T01:51:43Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Replaced Unix-only `std::env::var("HOME")` with cross-platform `dirs::home_dir()` in setup.rs
- Changed setup helper functions to accept `&Path` instead of `&str` for exe_path parameter
- Kept paths as PathBuf throughout, converting to String only at JSON/TOML serialization points
- Documented all to_string_lossy() usage in mcp_agent.rs with inline comments
- Migrated example files (claudecode_mcp.rs, opencode_jsonl.rs) from HOME env var to dirs::home_dir()

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate setup.rs to dirs::home_dir() and PathBuf-based exe_path** - `01008f4` (feat)
2. **Task 2: Audit and document to_string_lossy usage in mcp_agent.rs and examples** - `b83d83a` (docs)

## Files Created/Modified
- `rig-provider/Cargo.toml` - Added dirs = "5.0" dependency
- `rig-provider/src/setup.rs` - Replaced HOME env var with dirs::home_dir(), changed helper functions to accept &Path for exe_path, convert to string only at serialization
- `rig-provider/src/mcp_agent.rs` - Added inline comments at two to_string_lossy() call sites (lines 418 and 526) documenting why conversion is acceptable
- `rig-provider/examples/claudecode_mcp.rs` - Migrated from HOME env var to dirs::home_dir()
- `rig-provider/examples/opencode_jsonl.rs` - Migrated from HOME env var to dirs::home_dir()

## Decisions Made

1. **Use dirs::home_dir() for cross-platform home resolution**: The `std::env::var("HOME")` approach only works on Unix systems. Windows uses `USERPROFILE` or `HOMEPATH`. The `dirs` crate abstracts this platform difference and provides a single API.

2. **PathBuf-first path handling**: Paths stay as `PathBuf` or `&Path` as long as possible through the call chain. Only convert to `String` at the point where serialization requires it (JSON values, TOML values).

3. **display().to_string() for serialization**: Use `display().to_string()` instead of `to_string_lossy().to_string()` where possible. Both are acceptable for serialization, but `display()` is the idiomatic Rust way.

4. **Document to_string_lossy() usage**: All instances of `to_string_lossy()` now have inline comments explaining why the conversion is acceptable (e.g., "JSON requires UTF-8 strings", "temp file paths are always valid UTF-8").

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - straightforward migration. The dirs crate worked as expected, and all tests passed after migration.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Cross-platform path handling is complete. The codebase now:
- Works on Windows, macOS, and Linux without platform-specific code in setup.rs
- Uses PathBuf-based path handling following Rust best practices
- Documents all path-to-string conversions with clear rationale

Ready for Phase 6 Plan 4 (Dependency Audit Infrastructure).

---
*Phase: 06-platform-hardening*
*Completed: 2026-02-03*
