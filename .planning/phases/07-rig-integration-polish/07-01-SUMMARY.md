---
phase: 07-rig-integration-polish
plan: 01
subsystem: api
tags: [rig, facade, feature-flags, error-handling, rust]

# Dependency graph
requires:
  - phase: 06-platform-hardening
    provides: Cross-platform binary discovery, process management, path handling
provides:
  - rig-cli facade crate with feature flags (claude, codex, opencode)
  - ClientConfig for CLI-specific settings (binary_path, timeout, channel_capacity)
  - Public Error enum with actionable Display messages wrapping ProviderError
  - Workspace structure for Rig-idiomatic public API over internal rig-provider
affects: [07-rig-integration-polish, future-api-consumers]

# Tech tracking
tech-stack:
  added: [rig-cli facade crate]
  patterns: [workspace facade pattern, feature-gated modules, thiserror error wrapping]

key-files:
  created:
    - rig-cli/Cargo.toml
    - rig-cli/src/lib.rs
    - rig-cli/src/config.rs
    - rig-cli/src/errors.rs
    - rig-cli/src/prelude.rs
    - rig-cli/src/claude.rs (placeholder)
    - rig-cli/src/codex.rs (placeholder)
    - rig-cli/src/opencode.rs (placeholder)
  modified:
    - Cargo.toml (workspace members)

key-decisions:
  - "Feature flags control module compilation (not dependency compilation) - adapters always compile, user picks which to use"
  - "Error enum wraps ProviderError with #[from] for automatic conversion while providing actionable Display messages"
  - "ClientConfig defaults: 300s timeout, 100 message channel capacity, auto-discovery for binary path"
  - "Workspace facade pattern preserves existing adapter separation and provides clean public API"

patterns-established:
  - "Feature flags: claude, codex, opencode - all default-on, additive-only"
  - "Public API structure: lib.rs with feature-gated modules, always-available config and errors"
  - "Minimal prelude: only Error and ClientConfig, no builder types or internal implementation details"

# Metrics
duration: 1.7min
completed: 2026-02-03
---

# Phase 07 Plan 01: Facade Crate Skeleton Summary

**rig-cli facade crate with feature flags, ClientConfig defaults, and Error enum wrapping ProviderError**

## Performance

- **Duration:** 1.7 min
- **Started:** 2026-02-03T03:31:44Z
- **Completed:** 2026-02-03T03:33:26Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- Created rig-cli facade crate as workspace member with Rig-idiomatic public API structure
- Established three feature flags (claude, codex, opencode) all defaulting on for zero-config developer experience
- Defined ClientConfig with sensible CLI-specific defaults (300s timeout, 100 message channel, auto-discovery)
- Created Error enum with actionable user-facing messages while preserving internal error chain

## Task Commits

Each task was committed atomically:

1. **Task 1: Create rig-cli crate with Cargo.toml and feature flags** - `a87b6ce` (chore)
2. **Task 2: Create lib.rs, config.rs, and errors.rs** - `1fd50c0` (feat)

## Files Created/Modified

- `rig-cli/Cargo.toml` - Facade crate manifest with feature flags, dependencies on rig-provider and rig-core 0.29
- `rig-cli/src/lib.rs` - Public API root with feature-gated modules and crate documentation
- `rig-cli/src/config.rs` - ClientConfig with binary_path, timeout (300s), channel_capacity (100) defaults
- `rig-cli/src/errors.rs` - Error enum with actionable Display messages, wraps ProviderError and CompletionError
- `rig-cli/src/prelude.rs` - Minimal prelude re-exporting Error and ClientConfig
- `rig-cli/src/{claude,codex,opencode}.rs` - Placeholder modules for future Client implementations
- `Cargo.toml` - Updated workspace members to include rig-cli

## Decisions Made

**Feature flag design:** All three features (claude, codex, opencode) default on for zero-config experience. Features control which modules compile, not which dependencies are included - rig-provider and all adapters always compile. This matches Rig's own provider pattern and is the simplest correct approach.

**Error wrapping strategy:** Public Error enum uses thiserror #[from] to automatically convert ProviderError and CompletionError while providing actionable Display messages for end users. Debug output preserves full error chain for developers.

**ClientConfig defaults:** 300s timeout matches typical CLI agent interaction duration, 100 message channel capacity handles streaming without excessive memory usage, None for binary_path triggers auto-discovery.

**Workspace facade pattern:** New rig-cli crate wraps existing rig-provider internals rather than consolidating into single crate. Preserves adapter separation benefits and provides clean upgrade path.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Facade crate skeleton complete with feature flags, config, and error types. Ready for Plan 02 to implement Client and AgentBuilder for each provider. Compilation verified across all feature flag combinations (all-on, none, individual features).

---
*Phase: 07-rig-integration-polish*
*Completed: 2026-02-03*
