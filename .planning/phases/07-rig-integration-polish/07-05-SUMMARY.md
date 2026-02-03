---
phase: 07-rig-integration-polish
plan: 05
subsystem: api
tags: [streaming, mcp, toolset, rig-provider, async, tokio]

# Dependency graph
requires:
  - phase: 07-04
    provides: API polish with prelude, escape hatches, and debug-output feature
  - phase: 02.1
    provides: McpToolAgent foundation with run() method
provides:
  - McpStreamEvent enum for unified streaming events across adapters
  - McpToolAgent.stream() method for async streaming execution
  - CliAgent type holding ToolSet directly for MCP-enforced execution
  - CliAgentBuilder fluent API for configuration
  - Streaming helper functions (run_claude_code_stream, run_codex_stream, run_opencode_stream)
affects: [gap-closure, streaming-apis, mcp-enforcement]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Unified McpStreamEvent enum with adapter-specific conversion logic"
    - "Background tokio tasks for streaming with channel-based event forwarding"
    - "CliAgent consumes self pattern since ToolSet doesn't implement Clone"

key-files:
  created: []
  modified:
    - rig-provider/src/mcp_agent.rs
    - rig-provider/src/lib.rs

key-decisions:
  - "McpStreamEvent enum unified across all three adapters (Claude has ToolCall/ToolResult, Codex/OpenCode have Text/Error only)"
  - "Streaming helpers spawn tokio tasks for background execution with event conversion"
  - "CliAgent prompt() and chat() methods consume self since ToolSet lacks Clone"
  - "CliAgent provides methods, not Rig Prompt/Chat trait implementations (trait signatures incompatible with simple pattern)"

patterns-established:
  - "Stream channel capacity: 100 messages (consistent with existing adapter patterns)"
  - "Adapter event conversion: tokio::spawn with channel forwarding and type mapping"
  - "CliAgent builder pattern mirrors McpToolAgentBuilder for consistency"

# Metrics
duration: 8min
completed: 2026-02-03
---

# Phase 07 Plan 05: MCP Streaming and CliAgent Summary

**MCP-enforced streaming infrastructure with unified McpStreamEvent enum and CliAgent abstraction holding ToolSet directly**

## Performance

- **Duration:** 8 minutes
- **Started:** 2026-02-03T04:33:44Z
- **Completed:** 2026-02-03T04:42:16Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added streaming support to McpToolAgent via stream() method returning Receiver<McpStreamEvent>
- Created unified McpStreamEvent enum with Text, ToolCall, ToolResult, Error variants
- Implemented three streaming helpers (run_claude_code_stream, run_codex_stream, run_opencode_stream) with adapter-specific event conversion
- Created CliAgent and CliAgentBuilder types providing Rig-idiomatic API for MCP-enforced execution

## Task Commits

Each task was committed atomically:

1. **Task 1: Add streaming infrastructure to McpToolAgent** - `716bbb7` (feat)
2. **Task 2: Create CliAgent and CliAgentBuilder types** - `bb17ef6` (feat)

## Files Created/Modified
- `rig-provider/src/mcp_agent.rs` - Added McpStreamEvent enum, stream() method, streaming helpers, CliAgent/CliAgentBuilder structs
- `rig-provider/src/lib.rs` - Re-exported CliAgent, CliAgentBuilder, McpStreamEvent

## Decisions Made

**1. Unified McpStreamEvent enum across adapters**
- Claude adapter has ToolCall/ToolResult variants (rich streaming events)
- Codex and OpenCode adapters only have Text/Error/Unknown (simpler event model)
- McpStreamEvent provides superset: all adapters map to common type

**2. Streaming via background tokio tasks**
- stream() spawns tokio task for CLI execution
- Internal channel converts adapter-native StreamEvent to McpStreamEvent
- Outer channel returns to caller immediately
- Pattern: async streaming without blocking builder

**3. CliAgent consumes self in prompt() and chat()**
- ToolSet doesn't implement Clone (Rig core limitation)
- McpToolAgentBuilder.toolset() requires owned ToolSet
- Solution: prompt() and chat() take `self` not `&self`
- Alternative (Arc<ToolSet>) considered but adds complexity without clear benefit

**4. CliAgent methods, not Rig trait implementations**
- Plan specified implementing Prompt and Chat traits
- Reality: those traits have generic signatures with WasmCompatSend bounds and IntoFuture returns
- Implemented regular async methods instead (prompt, chat)
- Provides same developer experience without trait complexity

## Deviations from Plan

### Architectural Adjustments

**1. [Deviation - Trait Implementation] CliAgent uses methods instead of trait impls**
- **Found during:** Task 2 (CliAgent implementation)
- **Issue:** rig::completion::Prompt and rig::completion::Chat traits have complex generic signatures incompatible with simple &self pattern described in plan
- **Resolution:** Implemented `pub async fn prompt(self, prompt: &str)` and `pub async fn chat(self, prompt: &str, chat_history: &[String])` as regular methods
- **Files modified:** rig-provider/src/mcp_agent.rs
- **Rationale:** Provides same API surface and developer experience without fighting trait system
- **Committed in:** bb17ef6 (Task 2)

**2. [Deviation - Ownership] CliAgent methods consume self**
- **Found during:** Task 2 (implementing prompt method)
- **Issue:** ToolSet doesn't implement Clone, McpToolAgentBuilder.toolset() requires ownership
- **Resolution:** Changed from `pub async fn prompt(&self, ...)` to `pub async fn prompt(self, ...)`
- **Files modified:** rig-provider/src/mcp_agent.rs
- **Rationale:** Clean solution without introducing Arc complexity or hacks
- **Impact:** CliAgent is single-use (must rebuild for multiple prompts)
- **Committed in:** bb17ef6 (Task 2)

---

**Total deviations:** 2 architectural adjustments
**Impact on plan:** Both necessary due to Rig library constraints. Core functionality delivered: streaming support and CliAgent abstraction both work as intended. Developer experience equivalent to plan's vision.

## Issues Encountered

**ToolSet Clone limitation**
- ToolSet (from rig-core) doesn't derive or implement Clone
- Attempted Arc<ToolSet> wrapper but requires unwrapping for McpToolAgentBuilder
- Considered reconstructing ToolSet from definitions but can't rebuild Tool trait objects
- Resolution: Accept consume-on-use pattern (self not &self)
- Impact: Minimal - CliAgentBuilder is cheap to call multiple times for different prompts

## Next Phase Readiness

### Ready for Use
- Streaming API complete for all three adapters (Claude, Codex, OpenCode)
- CliAgent provides clean builder-based API for MCP-enforced execution
- McpStreamEvent enum unifies streaming across adapters

### Architecture Notes
- CliAgent's consume-on-use pattern is intentional given ToolSet constraints
- For reusable agents, developers should use CliAgentBuilder fresh each time
- Streaming infrastructure uses same bounded channel pattern (100-message capacity) as existing adapters

### No Blockers
- All planned functionality delivered
- Tests passing
- Documentation complete

---
*Phase: 07-rig-integration-polish*
*Completed: 2026-02-03*
