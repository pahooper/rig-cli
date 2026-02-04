---
phase: 11-documentation-examples
plan: 03
subsystem: examples
tags: [mcp, examples, chat, extraction, custom-tools, toolset]

# Dependency graph
requires:
  - phase: 11-01
    provides: README and lib.rs documentation structure
provides:
  - Four working MCP examples: chat_mcp, one_shot_mcp, agent_mcp, agent_extra_tools
  - Example patterns for multi-turn, one-shot, 3-tool, and custom tool workflows
affects: [11-04-additional-examples, future-documentation]

# Tech tracking
tech-stack:
  added: [schemars (dev)]
  patterns: [RIG_MCP_SERVER env var detection, build_toolset helper function]

key-files:
  created:
    - rig-cli/examples/chat_mcp.rs
    - rig-cli/examples/one_shot_mcp.rs
    - rig-cli/examples/agent_mcp.rs
    - rig-cli/examples/agent_extra_tools.rs
  modified:
    - rig-cli/Cargo.toml

key-decisions:
  - "Use prelude for JsonSchemaToolkit, ToolSetExt from tools module"
  - "Custom error types implement std::error::Error for Rig Tool trait"
  - "Each example self-contained with build_toolset() helper"
  - "All examples support dual-mode: client (default) and server (RIG_MCP_SERVER=1)"

patterns-established:
  - "KEY CODE markers delimit copy-paste sections"
  - "Module doc comments describe purpose and run command"
  - "RIG_MCP_SERVER env var gates server mode at start of main()"
  - "Error types defined inline for simple custom tools"

# Metrics
duration: 5min
completed: 2026-02-04
---

# Phase 11 Plan 03: MCP Examples Summary

**Four self-contained MCP examples demonstrating chat, one-shot, 3-tool, and custom tool patterns**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-04T00:46:19Z
- **Completed:** 2026-02-04T00:50:46Z
- **Tasks:** 2
- **Files created:** 4

## Accomplishments
- chat_mcp.rs: Multi-turn conversation with MCP tool responses
- one_shot_mcp.rs: Simplest MCP pattern - one prompt, structured response
- agent_mcp.rs: Standard 3-tool pattern (json_example/validate_json/submit)
- agent_extra_tools.rs: Custom DateExtractor tool alongside extraction toolkit

## Task Commits

Each task was committed atomically:

1. **Task 1: Create chat_mcp.rs and one_shot_mcp.rs** - `58a89b2` (feat)
2. **Task 2: Create agent_mcp.rs and agent_extra_tools.rs** - `bd8c743` (feat)

## Files Created/Modified
- `rig-cli/examples/chat_mcp.rs` - Multi-turn MCP chat with sentiment analysis
- `rig-cli/examples/one_shot_mcp.rs` - One-shot weather extraction example
- `rig-cli/examples/agent_mcp.rs` - Movie review extraction with 3-tool pattern
- `rig-cli/examples/agent_extra_tools.rs` - Event extraction with custom DateExtractor tool
- `rig-cli/Cargo.toml` - Added schemars dev dependency

## Decisions Made
- Used prelude for common imports (JsonSchemaToolkit exported there)
- Custom tool error types require std::error::Error implementation (not just String)
- All examples follow same structure: build_toolset() helper, RIG_MCP_SERVER check, KEY CODE markers

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Rig Tool trait requires Error type implementing std::error::Error - fixed by creating DateExtractorError struct

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Four core MCP examples available for developers
- Patterns established for 11-04 additional examples
- All examples compile and follow project conventions

---
*Phase: 11-documentation-examples*
*Completed: 2026-02-04*
