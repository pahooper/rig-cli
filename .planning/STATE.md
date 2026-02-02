# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-01)

**Core value:** When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably — the agent is forced through MCP tool constraints to submit conforming JSON rather than freeform text.
**Current focus:** Phase 4 complete. E2E testing documented. Ready for Phase 5 - Observability Infrastructure.

## Current Position

Phase: 4 of 11 (Agent Containment) — COMPLETE
Plan: 2 of 2 in current phase (2 complete)
Status: Phase complete
Last activity: 2026-02-02 — Documented Phase 4 E2E testing findings and adapter fixes

Progress: [███████████] 14/14 plans complete (Phase 1: 5/5, Phase 2: 2/2, Phase 2.1: 3/3, Phase 3: 2/2, Phase 4: 2/2)

## Performance Metrics

**Velocity:**
- Total plans completed: 14
- Average duration: 3.2 min
- Total execution time: 0.79 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-resource-management-foundation | 5 | 23min | 5min |
| 02-retry-validation-loop | 2 | 6min | 3min |
| 02.1-transparent-mcp-tool-agent | 3 | 8min | 3min |
| 03-payload-instruction-system | 2 | 4.5min | 2.25min |
| 04-agent-containment | 2 | 4.4min | 2.2min |

**Recent Trend:**
- Last 5 plans: 03-01 (2.5min), 03-02 (2min), 04-01 (2.4min), 04-02 (2min)
- Trend: Phase 4 complete — containment defaults + CLI flag verification complete

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Force structured output via MCP tools rather than prompt-only (gives schema enforcement at protocol level)
- Three-tool pattern (submit/validate/example) for workflow guidance
- Adapter-per-CLI crate structure (clean separation of concerns)
- Best-effort containment per CLI (document limitations rather than refuse to support)
- Deprioritize OpenCode for v1.0 (focus on getting two adapters rock solid)
- Apply resource management fixes to opencode-adapter despite deprioritization (infrastructure-level stability concern)
- Use same bounded channel architecture across all adapters for consistency (01-01, 01-02, 01-03)
- Standardize on 100-message channel capacity, 10MB output limit, 5s grace period across all adapters
- Use pid: 0 placeholder in rig-provider NonZeroExit since RunResult doesn't carry PID (01-04)
- Match claudecode-adapter's graceful_shutdown pattern exactly across all adapters (01-05)
- Use chars().count() not len() for token estimation to handle UTF-8 correctly (02-01)
- ExtractionError::MaxRetriesExceeded holds complete history, raw output, and metrics (02-01)
- Validation feedback includes schema, submission echo, all errors, and attempt counter (02-01)
- Orchestrator not generic over T - works with serde_json::Value, caller deserializes (02-02)
- Conversation continuation strategy: append feedback to prompt for retry context (02-02)
- Parse failures count against same retry budget as validation failures (02-02)
- McpToolAgent uses free functions (not &self methods) for per-adapter execution to avoid partial-move issues (02.1-02)
- Env var detection (RIG_MCP_SERVER=1) for server mode instead of --server CLI flag (02.1-03)
- Codex and OpenCode lack --system-prompt flag; prepend system prompt to user prompt instead (E2E testing)
- Each adapter manages its own MCP config delivery: Claude uses file path, Codex uses -c overrides (CodexConfig.overrides), OpenCode uses env var + file with different JSON format (E2E testing)
- OpenCode uses opencode/big-pickle model for MCP agent execution (E2E testing)
- Default to MCP-only mode: disable all builtin tools unless developer explicitly opts in (04-01)
- Temp directory by default: agents execute in isolated temp dir with RAII cleanup (04-01)
- Codex full_auto: false preserves sandbox and approval safety layers (04-01)
- Claude Code strict: true forces MCP-only config, ignores global MCP configs (04-01)
- Best-effort per-CLI containment: use each CLI's native flags to full extent, document limitations (04-01)
- Unit tests use windows(2) pattern to find adjacent flag-value pairs in CLI args (04-02)
- Default config tests verify full_auto absence to ensure containment posture (04-02)
- Codex MCP sandbox bypass limitation documented inline as known external issue (04-02)
- Codex CLI v0.91.0 dropped --ask-for-approval flag; removed ApprovalPolicy enum and ask_for_approval field from codex-adapter (E2E testing)
- Codex requires --skip-git-repo-check for non-git temp directory containment; added skip_git_repo_check field to CodexConfig (E2E testing)
- OpenCode adapter now has 6 unit tests for CLI arg generation in cmd.rs (E2E testing)

### Pending Todos

None.

### Roadmap Evolution

- Phase 2.1 added (INSERTED): Transparent MCP Tool Agent — McpToolAgent builder that auto-spawns MCP server, generates config, and wires Claude CLI. Inserted between Phase 2 and Phase 3. COMPLETE: 3 plans executed, verified 6/6 must-haves.

### Blockers/Concerns

- Pre-existing ~265 missing-docs clippy warnings across adapter crates (not blocking, future documentation pass)
- Codex Issue #4152: MCP tools bypass Landlock sandbox restrictions (known external limitation, documented inline)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 002 | Save Phase 2.1 plan files to GSD planning system | 2026-02-01 | abd49bc | [002-save-phase-2-1-plans-to-gsd](./quick/002-save-phase-2-1-plans-to-gsd/) |
| 003 | Update planning docs with E2E testing findings | 2026-02-02 | 0616a58 | [003-update-planning-docs-for-e2e-testing-f](./quick/003-update-planning-docs-for-e2e-testing-f/) |
| 004 | Document E2E testing findings and adapter fixes from Phase 4 | 2026-02-02 | d9198b2 | [004-document-e2e-testing-findings-and-adapte](./quick/004-document-e2e-testing-findings-and-adapte/) |

## Session Continuity

Last session: 2026-02-02
Stopped at: Phase 4 E2E findings documented — ready for Phase 5 (Observability Infrastructure)
Resume file: None
