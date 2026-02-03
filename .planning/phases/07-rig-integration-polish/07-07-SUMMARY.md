---
phase: 07-rig-integration-polish
plan: 07
subsystem: api
tags: [rig, mcp, payload-injection, cli-provider, xml-context]

# Dependency graph
requires:
  - phase: 07-05
    provides: CliAgent MCP-enforced execution
  - phase: 07-06
    provides: Client mcp_agent() method for all providers
provides:
  - Payload injection via XML context/task tags in all three providers
  - Shared CliResponse type used consistently across providers
  - Preamble and timeout properly wired to adapter configs
  - Zero compilation warnings in rig-cli
affects: [Phase 8 (if API enhancements needed), end-user examples, documentation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "XML <context>/<task> wrapping for payload injection"
    - "Shared CliResponse type across providers"
    - "Consistent preamble/timeout wiring pattern"

key-files:
  created: []
  modified:
    - "rig-cli/src/claude.rs"
    - "rig-cli/src/codex.rs"
    - "rig-cli/src/opencode.rs"
    - "rig-cli/src/response.rs"

key-decisions:
  - "Payload wraps prompts with XML <context>/<task> structure for instruction/data separation"
  - "Codex uses system_prompt field, OpenCode uses prompt field for preamble"
  - "Remove duplicate CliResponse from claude.rs, use shared type from response.rs"
  - "model_name field allowed as dead_code with justification (API consistency)"

patterns-established:
  - "Pattern: Payload injection via XML wrapping in completion() and stream()"
  - "Pattern: Preamble wired to adapter-specific config fields (system_prompt or prompt)"
  - "Pattern: Timeout always wired from self.config.timeout to adapter config"

# Metrics
duration: 3.2min
completed: 2026-02-03
---

# Phase 7 Plan 07: Payload Wiring and Dead Code Cleanup Summary

**All three providers wire payload via XML context/task tags, use shared CliResponse, properly wire preamble/timeout to adapters - zero compilation warnings**

## Performance

- **Duration:** 3.2 min
- **Started:** 2026-02-03T04:55:22Z
- **Completed:** 2026-02-03T04:58:34Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Payload injection working in all three providers (Claude, Codex, OpenCode) via XML context wrapping
- Removed duplicate CliResponse from claude.rs, all providers now use shared type
- Preamble and timeout properly wired to each adapter's configuration
- Zero compilation warnings from cargo check -p rig-cli

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire payload into Claude completion() and fix CliResponse duplication** - `74653ff` (feat)
2. **Task 2: Wire payload, preamble, timeout into Codex and OpenCode** - `5a360e2` (feat)

## Files Created/Modified
- `rig-cli/src/claude.rs` - Removed duplicate CliResponse, added XML payload wrapping, wired preamble/timeout
- `rig-cli/src/codex.rs` - Added XML payload wrapping, wired preamble to system_prompt, wired timeout
- `rig-cli/src/opencode.rs` - Added XML payload wrapping, wired preamble to prompt field, wired timeout

## Decisions Made

**Payload XML format:**
Prompts are wrapped with `<context>` and `<task>` XML tags when payload is set, matching the MCP agent format. This provides clean instruction/data separation.

**Preamble field mapping:**
- Claude: Uses `SystemPromptMode::Append(preamble)`
- Codex: Uses `config.system_prompt = Some(preamble)`
- OpenCode: Uses `config.prompt = Some(preamble)` (different field name)

**Shared CliResponse:**
Removed local CliResponse definition from claude.rs. All three providers now use `crate::response::CliResponse` via the `from_run_result()` factory method.

**model_name dead code:**
Added `#[allow(dead_code)]` with justification: "Model identifier stored for API consistency, CLI agents don't use per-request model selection."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Removed unused serde imports from claude.rs**
- **Found during:** Task 2 verification
- **Issue:** After removing CliResponse struct from claude.rs, Serialize and Deserialize imports were unused
- **Fix:** Removed unused serde imports
- **Files modified:** rig-cli/src/claude.rs
- **Verification:** cargo check shows zero warnings
- **Committed in:** 5a360e2 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical - cleanup)
**Impact on plan:** Cleanup to achieve zero-warning goal. No functional changes.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All payload wiring complete across CLI providers
- MCP enforcement via CliAgent works (from 07-06)
- Direct CLI path supports payload injection via XML wrapping
- Ready for Phase 8 (CLI polish and documentation) or Phase 9 (framework-agnostic extractor API)

---
*Phase: 07-rig-integration-polish*
*Completed: 2026-02-03*
