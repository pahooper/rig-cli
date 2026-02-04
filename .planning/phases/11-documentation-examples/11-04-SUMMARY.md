---
phase: 11-documentation-examples
plan: 04
subsystem: documentation
tags: [examples, multiagent, extraction, payload, deterministic-tools, mcp]

# Dependency graph
requires:
  - phase: 11-01
    provides: README and crate-level documentation
provides:
  - Four advanced user story examples (multiagent, extraction, payload_chat, mcp_deterministic)
  - Demonstration of multi-agent coordination pattern
  - Demonstration of payload injection for file analysis
  - Demonstration of mixing MCP tools with deterministic operations
affects: [11-05-error-handling-example]

# Tech tracking
tech-stack:
  added: [chrono]
  patterns: [dual-mode MCP server/client via env var, multi-agent coordination]

key-files:
  created:
    - rig-cli/examples/multiagent.rs
    - rig-cli/examples/extraction.rs
    - rig-cli/examples/payload_chat.rs
    - rig-cli/examples/mcp_deterministic.rs
  modified:
    - rig-cli/Cargo.toml

key-decisions:
  - "Use async fn for Tool trait implementations (Rig 0.29 pattern)"
  - "Dual-mode examples detect RIG_MCP_SERVER env var for server mode"
  - "Custom tools need thiserror-based Error types for Tool trait"

patterns-established:
  - "Multi-agent pattern: researcher agent -> summarizer agent coordination"
  - "Deterministic tool pattern: full Tool trait impl with async fn definition/call"
  - "Payload pattern: with_payload() on Client for context injection"

# Metrics
duration: 5min
completed: 2026-02-04
---

# Phase 11 Plan 04: Advanced Examples Summary

**Four advanced user story examples demonstrating multiagent coordination, PersonInfo extraction, payload-based chat, and MCP with deterministic CurrentDateTool**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-04T00:46:23Z
- **Completed:** 2026-02-04T00:51:22Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Created multiagent.rs demonstrating researcher + summarizer agent coordination
- Created extraction.rs showing PersonInfo extraction with full 3-tool workflow
- Created payload_chat.rs demonstrating with_payload() for file content analysis (both single Q&A and multi-turn patterns)
- Created mcp_deterministic.rs with full CurrentDateTool implementation mixing AI and deterministic operations
- Added chrono dev-dependency for date tool example

## Task Commits

Each task was committed atomically:

1. **Task 1: Create multiagent.rs and extraction.rs** - `bf1311c` (feat)
2. **Task 2: Create payload_chat.rs and mcp_deterministic.rs** - `02c99f4` (feat)

**Note:** chrono dependency was added via an amend to a 11-03 commit (`bd8c743`) due to linter intervention

## Files Created/Modified
- `rig-cli/examples/multiagent.rs` - Two-agent coordination (researcher extracts, summarizer condenses)
- `rig-cli/examples/extraction.rs` - PersonInfo extraction with 3-tool workflow
- `rig-cli/examples/payload_chat.rs` - File content analysis with payload injection
- `rig-cli/examples/mcp_deterministic.rs` - CurrentDateTool mixing AI and deterministic ops
- `rig-cli/Cargo.toml` - Added chrono dev-dependency

## Decisions Made
- Used `async fn` pattern for Tool trait (Rig 0.29 pattern, not Pin<Box<dyn Future>>)
- Custom tool errors must impl std::error::Error (used thiserror::Error derive)
- CliAgent::prompt() returns String directly, not struct with raw_output field

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Tool trait signature**
- **Found during:** Task 2 (mcp_deterministic.rs creation)
- **Issue:** Plan's Tool impl used Pin<Box<dyn Future>> return type which doesn't satisfy Rig 0.29 trait bounds
- **Fix:** Changed to async fn pattern matching existing tool implementations
- **Files modified:** rig-cli/examples/mcp_deterministic.rs
- **Verification:** Example compiles successfully
- **Committed in:** 02c99f4 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed error type for Tool trait**
- **Found during:** Task 2 (mcp_deterministic.rs creation)
- **Issue:** String doesn't implement std::error::Error required by Tool::Error
- **Fix:** Created DateToolError enum with thiserror::Error derive
- **Files modified:** rig-cli/examples/mcp_deterministic.rs
- **Verification:** Example compiles successfully
- **Committed in:** 02c99f4 (Task 2 commit)

**3. [Rule 1 - Bug] Fixed API usage in examples**
- **Found during:** Task 1 (multiagent.rs and extraction.rs)
- **Issue:** Plan used `.raw_output` field but CliAgent::prompt() returns String
- **Fix:** Used String directly instead of struct field access
- **Files modified:** multiagent.rs, extraction.rs
- **Verification:** Examples compile successfully
- **Committed in:** bf1311c (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (3 bugs)
**Impact on plan:** All fixes were necessary to match actual Rig 0.29 API. No scope creep.

## Issues Encountered
- Linter removed chrono from dev-dependencies during commit; re-added via amend

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Four advanced examples complete and compiling
- Ready for Plan 11-05 (Error handling example + final verification)
- All examples have KEY CODE markers for copy-paste discoverability

---
*Phase: 11-documentation-examples*
*Completed: 2026-02-04*
