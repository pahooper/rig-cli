---
phase: 02-retry-validation-loop
plan: 02
subsystem: extraction
tags: [orchestrator, retry-loop, validation-feedback, jsonschema, async]

# Dependency graph
requires:
  - phase: 02-retry-validation-loop
    plan: 01
    provides: ExtractionError, ExtractionMetrics, ExtractionConfig, validation feedback builders
provides:
  - ExtractionOrchestrator with async retry loop and bounded attempts
  - Enhanced ValidateJsonTool with instance paths, schema echo, and submission feedback
  - extract() method returning (Value, ExtractionMetrics) on success
  - extract_typed<T>() convenience method for typed deserialization
affects: [03-payload-instruction-system, extraction-workflows, structured-output-patterns]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Conversation continuation strategy (append feedback to prompt) for retry context"
    - "Immediate retry with no backoff delay (validation errors are synchronous)"
    - "All failure types (parse, schema, callback) count against same retry budget"
    - "Metrics populated on both success and failure paths for complete cost tracking"

key-files:
  created:
    - mcp/src/extraction/orchestrator.rs
  modified:
    - mcp/src/extraction/mod.rs
    - mcp/src/lib.rs
    - mcp/src/tools.rs

key-decisions:
  - "Orchestrator not generic over T - works with serde_json::Value, caller deserializes"
  - "Conversation continuation strategy: append feedback to prompt rather than fresh prompt each time"
  - "Parse failures count against same retry budget as validation failures (no separate tracking)"
  - "ValidateJsonTool enhanced with instance paths and schema echo for richer agent feedback"

patterns-established:
  - "ExtractionOrchestrator::new(schema).max_attempts(5).extract(agent_fn, prompt) - fluent builder"
  - "extract() returns (Value, ExtractionMetrics) - caller deserializes to specific type if needed"
  - "extract_typed<T>() convenience method combines extract() + deserialization in one call"
  - "ValidateJsonTool feedback format: 'At path /field: error' with full schema and submission"

# Metrics
duration: 3min
completed: 2026-02-01
---

# Phase 2 Plan 02: Extraction Orchestrator Summary

**Async retry loop with validation feedback, enhanced tool output, and complete metrics tracking**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-01T22:58:19Z
- **Completed:** 2026-02-01T23:01:01Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- ExtractionOrchestrator struct with schema and ExtractionConfig
- extract() method implementing bounded retry loop (1..=max_attempts)
- Immediate retry with conversation continuation (no exponential backoff)
- Parse failures and validation failures count against same retry budget
- Validation feedback includes full schema, echoed submission, all errors with instance paths, and attempt counter
- extract_typed<T>() convenience method for typed deserialization
- Success path returns (serde_json::Value, ExtractionMetrics) tuple
- Failure path returns ExtractionError::MaxRetriesExceeded with history, raw_output, and metrics
- Enhanced ValidateJsonTool with instance paths ("At path '/field': error"), schema echo, and submission feedback
- Full workspace compiles cleanly with zero new clippy warnings
- Orchestrator exported from mcp crate and available in prelude

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ExtractionOrchestrator with async retry loop** - `28c7f01` (feat)
   - ExtractionOrchestrator struct with new(), with_config(), max_attempts() builder
   - extract() method with bounded retry loop and conversation continuation
   - extract_typed<T>() convenience method
   - Module declarations and prelude exports
   - Const fn for with_config() and max_attempts() per clippy

2. **Task 2: Enhance ValidateJsonTool with richer feedback** - `51f78d4` (feat)
   - Instance paths in error messages: "At path '/field': error description"
   - Full schema echo in validation failure response
   - Agent's submission echoed for comparison
   - Use write! instead of format! to avoid extra allocation
   - Clear guidance: "Please fix all errors above and resubmit using the validate_json tool, then call submit."

## Files Created/Modified

- `mcp/src/extraction/orchestrator.rs` - ExtractionOrchestrator implementation (new)
- `mcp/src/extraction/mod.rs` - Added orchestrator module and re-export
- `mcp/src/lib.rs` - Added ExtractionOrchestrator to prelude
- `mcp/src/tools.rs` - Enhanced ValidateJsonTool::call() with richer feedback

## Decisions Made

**Orchestrator not generic over T**
- Rationale: Works with serde_json::Value for validation and returns Value on success
- Caller deserializes to specific type T after validation succeeds
- Avoids complex generic bounds and PhantomData in orchestrator implementation
- extract_typed<T>() convenience method provides typed API when needed

**Conversation continuation strategy**
- Rationale: Append validation feedback to existing prompt rather than fresh prompt each retry
- Preserves context across attempts (cheaper, faster convergence)
- Per 02-RESEARCH.md: "Continuation preserves context (cheaper, faster); fresh prompt prevents error accumulation"
- Can be made configurable later if context window explosion becomes an issue

**Parse failures count against same retry budget**
- Rationale: All failure modes (parse, schema validation, callback rejection) count against same retry limit
- Simpler budget enforcement - no separate tracking for different failure types
- Prevents agent from getting 3 parse attempts + 3 validation attempts = 6 total tries
- Per 02-CONTEXT.md decision: "All failure types count against the same retry budget"

**ValidateJsonTool enhanced with instance paths and schema echo**
- Rationale: Agent needs full schema reference and exact error locations to correct mistakes
- Instance paths show which field(s) failed validation (e.g., "/user/age")
- Schema echo lets agent compare submission to expected structure
- Per 02-CONTEXT.md: "Include the full JSON schema alongside all validation errors in feedback"

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Clippy warning: format_push_string**
- Issue: Using `format!()` inside `push_str()` creates unnecessary allocation
- Resolution: Imported `std::fmt::Write` and used `writeln!()` macro instead
- Result: Zero new clippy warnings in mcp crate

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Phase 3 (Payload & Instruction System):**
- ExtractionOrchestrator provides complete retry loop foundation
- ValidateJsonTool produces rich feedback for agent self-correction
- Metrics tracking enables cost awareness
- All extraction foundation types in place

**Contracts established:**
- extract() signature: `async fn extract<F, Fut>(&self, agent_fn: F, initial_prompt: String) -> Result<(Value, ExtractionMetrics), ExtractionError>`
- extract_typed<T>() signature: `async fn extract_typed<T, F, Fut>(&self, agent_fn: F, initial_prompt: String) -> Result<(T, ExtractionMetrics), ExtractionError>`
- ValidateJsonTool feedback format includes "At path '/...':" prefix, schema, and submission

**Requirements implemented:**
- EXTR-01: Retry loop with validation feedback - agents receive errors and retry up to max attempts
- EXTR-04: Attempt/cost tracking - ExtractionMetrics tracks attempts, wall time, estimated tokens on both success and failure paths

**No blockers** - Phase 3 can add payload injection and instruction templates on top of this orchestration foundation.

---
*Phase: 02-retry-validation-loop*
*Completed: 2026-02-01*
