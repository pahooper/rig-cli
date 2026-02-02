# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-01)

**Core value:** When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably — the agent is forced through MCP tool constraints to submit conforming JSON rather than freeform text.
**Current focus:** Phase 1 complete. Ready for Phase 2 - Streaming Architecture.

## Current Position

Phase: 2 of 11 (Retry & Validation Loop) - IN PROGRESS
Plan: 2 of 3 in current phase (02-01, 02-02 complete)
Status: In progress
Last activity: 2026-02-01 — Phase 2.1 planning complete (3 plans created)

Progress: [██████░░░░] 2/6 plans complete (Phase 1: 5/5, Phase 2: 2/3)

## Performance Metrics

**Velocity:**
- Total plans completed: 7
- Average duration: 4 min
- Total execution time: 0.5 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-resource-management-foundation | 5 | 23min | 5min |
| 02-retry-validation-loop | 2 | 6min | 3min |

**Recent Trend:**
- Last 5 plans: 01-02 (6min), 01-04 (7min), 01-05 (3min), 02-01 (3min), 02-02 (3min)
- Trend: Phase 2 progressing - retry orchestration complete

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

### Pending Todos

None.

### Roadmap Evolution

- Phase 2.1 added (INSERTED): Transparent MCP Tool Agent — McpToolAgent builder that auto-spawns MCP server, generates config, and wires Claude CLI. Inserted between Phase 2 and Phase 3. Planning complete: 3 plans in 3 waves (sequential).

### Blockers/Concerns

- Pre-existing ~265 missing-docs clippy warnings across adapter crates (not blocking, future documentation pass)

## Session Continuity

Last session: 2026-02-01
Stopped at: Completed 02-02-PLAN.md (extraction orchestrator with async retry loop and enhanced ValidateJsonTool)
Resume file: None
