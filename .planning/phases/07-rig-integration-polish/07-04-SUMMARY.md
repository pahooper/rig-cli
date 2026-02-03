---
phase: 07-rig-integration-polish
plan: 04
subsystem: api
tags: [rig, facade, prelude, mcp, escape-hatches, feature-flags]

# Dependency graph
requires:
  - phase: 07-02
    provides: Claude Client with CompletionClient trait implementation
  - phase: 07-03
    provides: Codex and OpenCode Client implementations following same pattern
provides:
  - Ergonomic prelude module with feature-gated Client exports
  - MCP type re-exports (JsonSchemaToolkit, ExtractionOrchestrator, etc.)
  - Escape hatches (.cli(), .config()) on all Client types
  - debug-output feature flag for enhanced error diagnostics
  - All feature flag combinations compile cleanly
affects: [08-documentation, users]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Prelude pattern for common imports (use rig_cli::prelude::*)"
    - "Escape hatch pattern (.cli(), .config()) for advanced adapter access"
    - "Feature flag for conditional debug output (debug-output)"
    - "MCP type re-exports through facade modules (extraction, tools)"

key-files:
  created:
    - rig-cli/src/prelude.rs
  modified:
    - rig-cli/src/lib.rs
    - rig-cli/src/claude.rs
    - rig-cli/src/codex.rs
    - rig-cli/src/opencode.rs
    - rig-cli/Cargo.toml

key-decisions:
  - "Prelude exports minimal set: Client types, Error, Rig traits (Prompt, Chat), MCP types"
  - "Re-export rig crate at lib root for user access to Rig types"
  - "MCP types re-exported through extraction and tools modules for discoverability"
  - "Escape hatches return CLI handle directly (not internal Model) for adapter-specific access"
  - "debug-output feature is opt-in only, not in default features"
  - "Feature flags control module compilation, not dependency compilation"

patterns-established:
  - "use rig_cli::prelude::* brings in common types for typical usage"
  - "client.cli() provides escape hatch to underlying adapter for advanced users"
  - "client.config() provides access to ClientConfig for inspection"
  - "Compile-time feature flags for conditional debug output in errors"

# Metrics
duration: 3.4min
completed: 2026-02-03
---

# Phase 7 Plan 4: API Polish & Ergonomics Summary

**Prelude with MCP re-exports, escape hatches for advanced adapter access, and debug-output feature flag**

## Performance

- **Duration:** 3.4 min
- **Started:** 2026-02-03T03:46:15Z
- **Completed:** 2026-02-03T03:49:40Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Prelude module exports feature-gated Clients, Error, Rig traits, and MCP types
- MCP extraction types (ExtractionOrchestrator, JsonSchemaToolkit, etc.) re-exported at crate level
- Escape hatches (.cli(), .config()) added to all Client types for advanced users
- debug-output feature flag enables enhanced error diagnostics (opt-in)
- All feature flag combinations verified (no-default-features, all-features, individual features)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create prelude, add re-exports, and wire MCP types** - `33b7ab4` (feat)
2. **Task 2: Add escape hatches and debug-output feature** - `a1533de` (feat)

## Files Created/Modified
- `rig-cli/src/prelude.rs` - Common re-exports for ergonomic imports
- `rig-cli/src/lib.rs` - Crate root with MCP extraction and tools modules
- `rig-cli/src/claude.rs` - Added .cli() and .config() escape hatches, debug-output error handling
- `rig-cli/src/codex.rs` - Added .cli() and .config() escape hatches, debug-output error handling
- `rig-cli/src/opencode.rs` - Added .cli() and .config() escape hatches, debug-output error handling
- `rig-cli/Cargo.toml` - Added debug-output feature flag

## Decisions Made
- Prelude exports minimal set per Rig convention (Client types, Error, Prompt/Chat traits, MCP types)
- MCP types split across extraction and tools modules for logical grouping
- Escape hatches return CLI handle directly (not internal Model) for adapter-specific access
- debug-output feature is opt-in only to avoid leaking sensitive data in production by default
- Re-export rig crate at lib root so users can access Rig types via `rig_cli::rig::...`

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all feature flag combinations compiled successfully on first attempt.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- rig-cli API surface is complete and polished
- Ready for documentation pass (Phase 8)
- All three providers (Claude, Codex, OpenCode) follow identical patterns
- MCP integration prepared for extractor pattern usage
- Feature flags provide flexibility for minimal builds and debug scenarios

---
*Phase: 07-rig-integration-polish*
*Completed: 2026-02-03*
