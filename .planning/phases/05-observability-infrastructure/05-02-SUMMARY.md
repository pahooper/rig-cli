---
phase: 05-observability-infrastructure
plan: 02
subsystem: observability
tags: [version-detection, semver, tracing, cli-compatibility]
requires: [05-01]
provides:
  - CLI version detection and validation
  - Semver parsing for version strings
  - Structured tracing warnings for version mismatches
affects: []
tech-stack:
  added:
    - semver 1.0
  patterns:
    - Stateless version detection at execution start
    - Const fn version requirement definitions
    - Structured tracing for version events
key-files:
  created: []
  modified:
    - rig-provider/Cargo.toml
    - rig-provider/src/mcp_agent.rs
key-decisions:
  - decision: Version requirements are hardcoded const functions per adapter
    context: Not developer-configurable, tightened in later phases
    rationale: Simple and explicit for Phase 5, flexibility deferred
  - decision: Version detection warns and continues, never blocks execution
    context: Unsupported versions get warning, agent still runs
    rationale: Fail-open policy prevents false negatives
  - decision: Distinct warning events for unsupported vs untested versions
    context: version_unsupported (below min) vs version_untested (above max_tested)
    rationale: Clear distinction for observability and debugging
duration: 2.6min
completed: 2026-02-02
---

# Phase 5 Plan 2: CLI Version Detection Summary

**One-liner:** CLI version awareness via `--version` detection with semver parsing and structured tracing warnings for compatibility monitoring.

## Performance

**Duration:** 2.6 minutes
**Started:** 2026-02-02T23:36:12Z
**Completed:** 2026-02-02T23:38:48Z
**Tasks:** 2/2 complete
**Files modified:** 2

## Accomplishments

**Implemented CLI version detection and validation:**
- Added semver dependency for robust version parsing
- Defined const fn version requirement functions for all 3 adapters (Claude Code, Codex, OpenCode)
- Implemented `detect_and_validate_version` async function that runs `<binary> --version`
- Added `extract_version_string` helper to handle common version string formats (v-prefix, CLI name prefix, prerelease tags)
- Integrated version detection at start of each `run_*` function (run_claude_code, run_codex, run_opencode)
- Emits structured tracing events: version_detected (debug), version_unsupported (warn), version_untested (warn), version_detection_failed (warn), version_parse_failed (warn)

**Test coverage:**
- Added 7 unit tests covering version extraction and validation logic
- Tests verify: simple versions, v-prefix, CLI name prefix, prerelease tags, unparseable fallback, requirement constants, comparison logic
- All tests pass

## Task Commits

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Add semver dependency and implement version detection | 8d15e0f | rig-provider/Cargo.toml, rig-provider/src/mcp_agent.rs, Cargo.lock |
| 2 | Add unit tests for version detection | 4bbb000 | rig-provider/src/mcp_agent.rs |

## Files Created/Modified

**Modified:**
- `rig-provider/Cargo.toml`: Added semver = "1.0" dependency
- `rig-provider/src/mcp_agent.rs`:
  - Added VersionRequirement struct and 3 const fn requirement functions
  - Added detect_and_validate_version async function with tracing integration
  - Added extract_version_string helper for version string normalization
  - Added 3 version detection callsites in run_claude_code, run_codex, run_opencode
  - Added #[cfg(test)] module with 7 unit tests

## Decisions Made

**1. Version requirements as const functions**
- Each adapter has a const fn returning VersionRequirement with min_version and max_tested
- Hardcoded, not developer-configurable
- Intentionally broad ranges for Phase 5 (e.g., Claude Code 1.0.0 to 1.99.0)
- Will be tightened to specific tested ranges in Phases 8-10

**2. Fail-open policy for version mismatches**
- detect_and_validate_version always succeeds (no Result return)
- Version issues emit tracing warnings but never block execution
- Prevents false negatives where version detection fails but CLI works fine

**3. Distinct warning events for unsupported vs untested**
- version_unsupported: detected < min_version (below supported range)
- version_untested: detected > max_tested (above tested range)
- Both include cli, detected, minimum/max_tested fields for structured observability

**4. Stateless version detection**
- Runs once per agent execution (no caching between runs)
- Calls `<binary> --version` synchronously at start of each run_* function
- Simple and predictable behavior

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Implementation was straightforward:
- semver crate handles version parsing reliably
- Version extraction covers common formats (v-prefix, CLI name, prerelease)
- Test coverage validates all code paths

## Next Phase Readiness

**Phase 5 Plan 3 (Telemetry Export):** Ready
- Version detection establishes structured tracing patterns
- Plan 3 will add telemetry sinks (JSON logs, metrics) for these events

**Observability stack building up:**
- 05-01: Structured tracing in ExtractionOrchestrator
- 05-02: CLI version detection with tracing warnings
- 05-03: Telemetry export infrastructure (next)

**Version tightening deferred to Phase 8-10:**
- Current broad ranges (0.1.0-0.99.0 for Codex/OpenCode, 1.0.0-1.99.0 for Claude Code) intentional
- Will narrow to specific tested versions after adapter hardening phases
