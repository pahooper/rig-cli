---
phase: 03-payload-instruction-system
plan: 01
subsystem: orchestration
tags: [mcp, builder-pattern, prompt-engineering, xml-structure]

# Dependency graph
requires:
  - phase: 02.1-transparent-mcp-tool-agent
    provides: McpToolAgentBuilder with MCP config generation and CLI execution
provides:
  - McpToolAgentBuilder.payload() for context data injection
  - McpToolAgentBuilder.instruction_template() for custom workflow templates
  - DEFAULT_WORKFLOW_TEMPLATE constant enforcing example -> validate -> submit workflow
  - 4-block XML prompt structure when payload is present
  - Enhanced system prompt with workflow enforcement steps
affects: [03-02-core-orchestrator, 03-03-typed-extraction-api]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "4-block XML prompt structure (<context>, <task>, <output_format>) for payload injection"
    - "Workflow template injection into system prompt for tool sequence enforcement"
    - "Backward compatible builder enhancement (new methods optional)"

key-files:
  created: []
  modified:
    - rig-provider/src/mcp_agent.rs
    - rig-provider/src/lib.rs

key-decisions:
  - "Use DEFAULT_WORKFLOW_TEMPLATE as default, allow override via instruction_template()"
  - "4-block XML structure only when payload is present (backward compatible)"
  - "Inject workflow steps into system prompt for all executions (enhancement)"

patterns-established:
  - "Optional builder fields with .unwrap_or() for defaults"
  - "Move owned fields before match block to avoid partial-move issues"
  - "Export public constants from lib.rs for developer access"

# Metrics
duration: 2.5min
completed: 2026-02-02
---

# Phase 03 Plan 01: Payload & Instruction System Summary

**McpToolAgentBuilder enhanced with payload injection and workflow template enforcement via 4-block XML prompts**

## Performance

- **Duration:** 2.5 min
- **Started:** 2026-02-02T03:22:25Z
- **Completed:** 2026-02-02T03:24:54Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added .payload() and .instruction_template() builder methods
- Created DEFAULT_WORKFLOW_TEMPLATE constant with numbered workflow steps
- Implemented 4-block XML prompt structure for payload injection
- Enhanced system prompt with workflow enforcement language
- Maintained backward compatibility (no payload = unchanged behavior)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add builder fields, methods, and DEFAULT_WORKFLOW_TEMPLATE constant** - `e335a0c` (feat)
2. **Task 2: Refactor prompt and system prompt construction in run()** - `7af0885` (feat)

## Files Created/Modified
- `rig-provider/src/mcp_agent.rs` - Added payload/instruction_template fields, methods, DEFAULT_WORKFLOW_TEMPLATE constant, enhanced prompt construction
- `rig-provider/src/lib.rs` - Exported DEFAULT_WORKFLOW_TEMPLATE

## Decisions Made

None - followed plan as specified. All key decisions were in the plan:
- Use DEFAULT_WORKFLOW_TEMPLATE as default with override capability
- 4-block XML structure only when payload present
- Inject workflow steps into system prompt for all executions

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Clippy flagged needless raw string hashes (r#"..."# → r"..."), fixed immediately.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Phase 03-02 (Core Orchestrator):
- Builder has all fields needed for payload/instruction injection
- DEFAULT_WORKFLOW_TEMPLATE defines the three-tool workflow
- 4-block XML structure ready for context data
- Backward compatible: existing code unaffected

No blockers. Next phase can implement the orchestrator using these new builder capabilities.

---
*Phase: 03-payload-instruction-system*
*Completed: 2026-02-02*
