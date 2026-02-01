---
phase: 02-retry-validation-loop
plan: 01
subsystem: extraction
tags: [jsonschema, thiserror, validation, retry, metrics, token-estimation]

# Dependency graph
requires:
  - phase: 01-resource-management-foundation
    provides: Workspace conventions (thiserror, no .unwrap()/.expect(), bounded channels)
provides:
  - ExtractionError enum with MaxRetriesExceeded variant including attempt history and metrics
  - ExtractionMetrics struct for tracking attempts, wall time, and estimated tokens
  - ExtractionConfig for configurable retry behavior
  - Validation feedback builders with complete error context
  - Token estimation using UTF-8-safe chars().count().div_ceil(4)
affects: [02-02-extraction-orchestrator, structured-extraction-workflows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Token estimation using chars().count().div_ceil(4) for UTF-8 safety"
    - "Validation feedback includes full schema + echoed submission + all errors"
    - "Typed error variants with structured context for pattern matching"

key-files:
  created:
    - mcp/src/extraction/mod.rs
    - mcp/src/extraction/error.rs
    - mcp/src/extraction/metrics.rs
    - mcp/src/extraction/config.rs
    - mcp/src/extraction/feedback.rs
  modified:
    - mcp/src/lib.rs

key-decisions:
  - "Use chars().count() not len() for token estimation to handle UTF-8 correctly"
  - "ExtractionError::MaxRetriesExceeded holds complete history, raw output, and metrics"
  - "Validation feedback always includes schema, submission, all errors, and attempt counter"
  - "CallbackRejection as separate error variant for business logic rejections"

patterns-established:
  - "estimate_tokens function uses chars().count().div_ceil(4) - standard 4-chars-per-token heuristic with ceiling division"
  - "build_validation_feedback includes attempt counter, all errors, schema, and echoed submission"
  - "collect_validation_errors uses iter_errors() for complete error collection with instance paths"

# Metrics
duration: 3min
completed: 2026-02-01
---

# Phase 2 Plan 01: Extraction Foundation Types Summary

**Typed error enums, attempt tracking, validation feedback builders, and UTF-8-safe token estimation for extraction retry loops**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-01T22:51:26Z
- **Completed:** 2026-02-01T22:54:09Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- ExtractionError enum with 5 variants (MaxRetriesExceeded, ParseError, SchemaError, AgentError, CallbackRejection) using thiserror
- AttemptRecord struct capturing submission JSON, validation errors, raw output, and elapsed time
- ExtractionMetrics for tracking attempts, wall time, estimated input/output tokens
- ExtractionConfig with defaults (max_attempts: 3, include_schema_in_feedback: true) and builder methods
- Validation feedback builders that include schema, submission echo, all errors, and attempt counter
- UTF-8-safe token estimation using chars().count().div_ceil(4)

## Task Commits

Each task was committed atomically:

1. **Task 1-2: Create extraction module with error types, metrics, config, and feedback** - `d098fb3` (feat)

**Note:** Tasks 1 and 2 were combined into a single commit as they form a cohesive module (error types depend on metrics, feedback uses both).

## Files Created/Modified

- `mcp/src/extraction/mod.rs` - Module declarations and re-exports
- `mcp/src/extraction/error.rs` - ExtractionError enum and AttemptRecord struct
- `mcp/src/extraction/metrics.rs` - ExtractionMetrics struct and estimate_tokens function
- `mcp/src/extraction/config.rs` - ExtractionConfig with builder methods
- `mcp/src/extraction/feedback.rs` - Validation feedback builders (build_validation_feedback, collect_validation_errors, build_parse_error_feedback)
- `mcp/src/lib.rs` - Added extraction module export and prelude re-exports

## Decisions Made

**Token estimation using chars().count() not len()**
- Rationale: UTF-8 characters can be multiple bytes; chars().count() gives accurate character count while len() gives byte count
- Example: "你好" is 2 chars but 6 bytes - chars().count() / 4 = 1 token (correct), len() / 4 = 1.5 tokens (wrong)
- Per 02-RESEARCH.md Pitfall 8: "Use chars().count() not bytes().len() to avoid UTF-8 estimation errors"

**ExtractionError::MaxRetriesExceeded includes history, raw_output, and metrics**
- Rationale: Complete debugging context - caller can inspect what went wrong across all attempts
- Enables post-mortem analysis of why extraction failed and at what point
- Metrics included in error path ensures cost tracking on both success and failure

**Validation feedback includes schema + submission + all errors**
- Rationale: Agent needs full schema reference to correct mistakes; echoed submission enables comparison
- collect_validation_errors uses iter_errors() for ALL validation failures, not just first
- Per 02-CONTEXT.md decision: "Include the full JSON schema alongside all validation errors in feedback"

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all types compiled cleanly with zero clippy warnings.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Phase 2 Plan 02 (Extraction Orchestrator):**
- Error types defined with all necessary context
- Metrics tracking structure in place
- Feedback builders ready for orchestrator to use
- Token estimation function available
- Configuration defaults established

**Contracts established:**
- ExtractionError variants specify all failure modes
- ExtractionMetrics tracks attempts and token usage
- Feedback builders produce complete error context
- estimate_tokens provides UTF-8-safe heuristic

**No blockers** - orchestrator can consume these types immediately.

---
*Phase: 02-retry-validation-loop*
*Completed: 2026-02-01*
