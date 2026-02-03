---
phase: 09-codex-adapter
plan: 01
subsystem: adapter
tags: [codex, cli-adapter, containment, approval-policy, sandbox]

# Dependency graph
requires:
  - phase: 08-claude-code-adapter
    provides: CLI flag documentation pattern and clippy pedantic standards
provides:
  - ApprovalPolicy enum with 4 Codex CLI values
  - ask_for_approval field on CodexConfig for containment
  - Module-level CLI flag documentation matching Claude Code pattern
  - Flag combination unit tests using windows(2) pattern
affects: [09-02, 09-03, 10-opencode-adapter]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Module-level CLI flag documentation with Flag Reference, Combinations, Version Notes, Known Limitations
    - windows(2) pattern for flag-value pair verification in tests
    - Containment-first defaults (ApprovalPolicy::default() returns Untrusted)

key-files:
  created: []
  modified:
    - codex-adapter/src/types.rs
    - codex-adapter/src/cmd.rs

key-decisions:
  - "ApprovalPolicy::default() returns Untrusted for containment-first operation (locked decision)"
  - "ask_for_approval field is Option<ApprovalPolicy> - None means no flag, Some passes explicit policy"
  - "Document full-auto override behavior in tests (full-auto overrides sandbox and approval at CLI level)"

patterns-established:
  - "Flag combination tests document both valid and conflicting combinations"
  - "MCP sandbox bypass limitations documented inline in Known Limitations section"

# Metrics
duration: 3min
completed: 2026-02-03
---

# Phase 9 Plan 1: Codex ApprovalPolicy and CLI Flag Documentation Summary

**ApprovalPolicy enum with 4 CLI values, --ask-for-approval flag generation, module-level CLI flag documentation with Known Limitations for MCP sandbox bypass #4152**

## Performance

- **Duration:** 3 min (169 seconds)
- **Started:** 2026-02-03T22:34:26Z
- **Completed:** 2026-02-03T22:37:15Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- Added ApprovalPolicy enum with Untrusted, OnFailure, OnRequest, Never variants
- Untrusted is #[default] variant honoring containment-first locked decision
- build_args() generates --ask-for-approval flag with all 4 policy values
- Module-level documentation with Flag Reference, Combinations, Version Notes, Known Limitations
- MCP sandbox bypass issue #4152 documented in Known Limitations
- 8 new flag combination tests using windows(2) pattern
- Clippy pedantic pass: zero warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ApprovalPolicy enum and update CodexConfig** - `b9b93d4` (feat)
2. **Task 2: Update build_args() and add module documentation** - `874102d` (docs)
3. **Task 3: Add flag combination unit tests and clippy pedantic pass** - `de936d9` (test)

## Files Created/Modified
- `codex-adapter/src/types.rs` - ApprovalPolicy enum, ask_for_approval field on CodexConfig
- `codex-adapter/src/cmd.rs` - Module-level CLI flag documentation, --ask-for-approval generation, 8 new tests

## Decisions Made
- ApprovalPolicy::default() returns Untrusted (locked decision for containment-first)
- ask_for_approval is Option<ApprovalPolicy>: None = no flag, Some = explicit policy
- Documented that --full-auto overrides sandbox and approval settings at CLI level (conflict documented in tests)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ApprovalPolicy enum ready for use in Codex containment configuration
- Flag documentation pattern established, ready for Plans 02 and 03
- All must_haves verified: enum, flag generation, documentation, tests, clippy pedantic

---
*Phase: 09-codex-adapter*
*Completed: 2026-02-03*
