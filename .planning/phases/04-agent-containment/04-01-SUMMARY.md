---
phase: 04-agent-containment
plan: 01
subsystem: security
tags: [containment, sandbox, mcp, tempfile, builtin-tools, codex, claude-code, opencode]

# Dependency graph
requires:
  - phase: 02.1-transparent-mcp-tool-agent
    provides: McpToolAgent builder with per-adapter execution functions
provides:
  - Containment-first McpToolAgentBuilder with MCP-only defaults
  - Opt-in escape hatches for builtin tools (allow_builtins)
  - Temp directory sandboxing via TempDir RAII
  - Codex read-only sandbox mode by default
  - Claude Code strict MCP mode with slash commands disabled
affects: [05-orchestrator-integration, 06-platform-hardening, examples]

# Tech tracking
tech-stack:
  added: [tempfile (temp directory RAII cleanup)]
  patterns: [Containment-first defaults with opt-in escapes, TempDir RAII lifecycle management]

key-files:
  created: []
  modified: [rig-provider/src/mcp_agent.rs]

key-decisions:
  - "Default to MCP-only mode: disable all builtin tools unless developer explicitly opts in"
  - "Temp directory by default: agents execute in isolated temp dir with RAII cleanup"
  - "Codex full_auto: false preserves sandbox and approval safety layers"
  - "Claude Code strict: true forces MCP-only config, ignores global MCP configs"
  - "Best-effort per-CLI containment: use each CLI's native flags to full extent, document limitations"

patterns-established:
  - "Pattern: Containment fields with secure defaults - builtin_tools: None, sandbox_mode: ReadOnly, working_dir: None (temp)"
  - "Pattern: TempDir lifecycle - create before adapter call, keep alive until result consumed"
  - "Pattern: Per-adapter containment application - each adapter gets containment params propagated to config"

# Metrics
duration: 2.4min
completed: 2026-02-02
---

# Phase 4 Plan 1: Agent Containment Defaults Summary

**MCP-only mode by default with temp directory sandboxing, opt-in builtin tools, and Codex read-only sandbox enforced at builder level**

## Performance

- **Duration:** 2.4 min
- **Started:** 2026-02-02T18:21:55Z
- **Completed:** 2026-02-02T18:24:21Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- McpToolAgentBuilder defaults lock agents to MCP-only mode (no builtins, no slash commands, strict MCP)
- Developer opt-in for specific builtins via `.allow_builtins(vec!["Bash"])`
- All adapter executions run in temp directory by default with RAII cleanup
- Codex uses read-only sandbox instead of full_auto (preserves safety layers)
- Workspace compiles cleanly with no downstream breakage

## Task Commits

Each task was committed atomically:

1. **Task 1: Add containment fields and opt-in API to McpToolAgentBuilder** - `c7d3fdc` (feat)
2. **Task 2: Apply containment settings in per-adapter run functions** - `d4c4d55` (feat)

## Files Created/Modified
- `rig-provider/src/mcp_agent.rs` - Added containment fields (builtin_tools, sandbox_mode, working_dir), builder methods (allow_builtins, sandbox_mode, working_dir), temp dir RAII in run(), propagated containment to per-adapter run functions

## Decisions Made
- Default to MCP-only mode: `builtin_tools: None` disables all builtin tools unless developer explicitly opts in via `.allow_builtins()`
- Temp directory by default: `working_dir: None` creates auto-cleaned temp dir; developer can override via `.working_dir(path)`
- Codex full_auto: false instead of true to preserve sandbox and approval safety layers (research doc explicitly warns full_auto reduces safety)
- Claude Code strict: true for MCP-only mode, forces use of --strict-mcp-config flag
- Codex sandbox: SandboxMode::ReadOnly default (most restrictive), developer can override via `.sandbox_mode()`
- Best-effort containment per CLI: use each CLI's native flags to full extent (Claude: --tools/--strict-mcp, Codex: --sandbox/--ask-for-approval, OpenCode: cwd only)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Phase 4 Plan 2:** Containment verification and E2E testing.

**What's ready:**
- McpToolAgentBuilder has containment-first defaults implemented
- All three adapters receive and apply containment settings
- TempDir RAII prevents resource leaks
- Opt-in escape hatches available for developers needing broader access

**No blockers.**

---
*Phase: 04-agent-containment*
*Completed: 2026-02-02*
