---
phase: 04-agent-containment
plan: 02
subsystem: security
tags: [containment, testing, cli-flags, rust, unit-tests, claudecode, codex]

# Dependency graph
requires:
  - phase: 04-01
    provides: Containment-first McpToolAgentBuilder defaults
provides:
  - Unit tests verifying Claude Code containment flags generate correct CLI arguments
  - Unit tests verifying Codex containment flags generate correct CLI arguments
  - Documentation of Codex Issue #4152 (MCP sandbox bypass)
affects: [05-orchestrator-integration, 06-platform-hardening]

# Tech tracking
tech-stack:
  added: []
  patterns: [CLI flag generation verification through unit tests, Test coverage for containment enforcement]

key-files:
  created: []
  modified:
    - claudecode-adapter/src/cmd.rs (added test module)
    - codex-adapter/src/cmd.rs (added test module)

key-decisions:
  - "Unit tests use windows(2) pattern to find adjacent flag-value pairs in CLI args"
  - "Default config tests verify full_auto absence to ensure containment posture"
  - "Codex MCP sandbox bypass limitation documented inline as known external issue"

patterns-established:
  - "Pattern: Test structure with RunConfig/CodexConfig default override for targeted assertions"
  - "Pattern: OsString conversion to &str via to_str().unwrap() for string comparisons"
  - "Pattern: Integration test for full containment config verifying all flags together"

# Metrics
duration: 2.0min
completed: 2026-02-02
---

# Phase 4 Plan 2: CLI Containment Flag Verification Summary

**Unit tests verify Claude Code and Codex containment flags generate correct CLI arguments for CONT-01/02/03/04 assertions**

## Performance

- **Duration:** 2.0 min
- **Started:** 2026-02-02T04:13:34Z
- **Completed:** 2026-02-02T04:15:34Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- 6 unit tests in claudecode-adapter verify: BuiltinToolSet::None → --tools "", BuiltinToolSet::Explicit → --tools "Bash", disable_slash_commands → --disable-slash-commands, strict MCP → --strict-mcp-config, allowed tools → --allowed-tools, full containment integration
- 6 unit tests in codex-adapter verify: SandboxMode::ReadOnly → --sandbox read-only, SandboxMode::WorkspaceWrite → --sandbox workspace-write, ApprovalPolicy::Never → --ask-for-approval never, cd → --cd flag, full_auto absence by default, full containment integration
- Workspace compiles cleanly with no regressions
- Codex Issue #4152 (MCP tools bypass sandbox) documented inline

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Claude Code containment flag tests to cmd.rs** - `3e8d862` (test)
2. **Task 2: Add Codex containment flag tests and workspace verification** - `bb568d9` (test)

## Files Created/Modified
- `claudecode-adapter/src/cmd.rs` - Added 6 unit tests for containment flag CLI arg generation, testing CONT-01, CONT-02, CONT-03 assertions
- `codex-adapter/src/cmd.rs` - Added 6 unit tests for containment flag CLI arg generation, testing CONT-04 and full_auto absence, documented Codex Issue #4152

## Decisions Made
- Used `windows(2).any()` pattern for finding adjacent flag-value pairs in CLI args - more robust than string matching
- Explicit test for full_auto absence by default verifies CONT-03 audit finding that full_auto bypasses containment
- Documented Codex MCP sandbox bypass as inline comment in test module - establishes this as known external limitation, not rig-cli bug

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Phase 4 Plan 3 (if exists) or Phase 5:** CLI containment enforcement verified.

**What's ready:**
- Claude Code containment flag generation verified through 6 unit tests
- Codex containment flag generation verified through 6 unit tests
- All success criteria met: --tools "", --disable-slash-commands, --strict-mcp-config, --allowed-tools (Claude Code) and --sandbox read-only, --ask-for-approval never, --cd (Codex) proven correct
- Workspace compiles and all tests pass
- Known limitations documented (Codex MCP sandbox bypass)

**No blockers.**

---
*Phase: 04-agent-containment*
*Completed: 2026-02-02*
