# Phase 2: Retry & Validation Loop - Context

**Gathered:** 2026-02-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the automated retry/validation feedback loop for structured extraction. When an agent submits invalid JSON via the existing validate or submit tools, the system feeds back validation errors and the agent retries — up to a configurable limit. The three tools (validate, example, submit) already exist in `mcp/src/tools.rs`. This phase adds the orchestration layer that tracks attempts, feeds errors back, and enforces retry limits. Payload injection, instruction templates, and forced tool workflow are Phase 3 concerns.

</domain>

<decisions>
## Implementation Decisions

### Validation feedback design
- Include the full JSON schema alongside all validation errors in feedback
- Echo back the agent's invalid submission so it can compare against the schema
- Show attempt number and max (e.g., "Attempt 2 of 3") in the feedback message
- Unified error format — no distinction between parse failures and schema validation failures; if it doesn't conform to the schema, it's wrong and needs to be redone
- Errors are the correction mechanism — no artificial delay between retries

### Retry behavior & limits
- Immediate retry after validation failure (no backoff delay) — feedback itself drives correction
- Default max attempts: 3 (configurable by developer)
- All failure types count against the same retry budget (schema failures, callback rejections — same pool)
- Existing validate/example/submit tools remain; this phase adds the orchestration loop around them

### Failure terminal state
- Typed Rust enum variant for extraction failure: `ExtractionError::MaxRetriesExceeded { attempts, history, raw_output, metrics }`
- Structured error includes full attempt history: each attempt's JSON submission and corresponding validation errors
- Include raw agent text output (not just JSON attempts) for debugging why the agent got confused
- Caller can pattern-match on the error variant for specific handling

### Cost & attempt tracking
- Track per extraction: attempt count, wall time, and token estimates
- Token estimates use text-length heuristic (chars/4 approximation) when CLI doesn't report actual token usage
- Metrics returned alongside the result on success: `Result<(T, ExtractionMetrics)>`
- Metrics included in error variant on failure — always available regardless of outcome

### Claude's Discretion
- Retry context strategy (conversation continuation vs fresh prompt) — optimize for fastest path to conforming output
- Internal orchestration architecture (where the loop lives in the crate structure)
- How token estimation heuristic is calibrated
- Whether validation tool output format changes to support richer feedback

</decisions>

<specifics>
## Specific Ideas

- The three-tool pattern (example, validate, submit) already exists in `mcp/src/tools.rs` — the validate tool returns human-readable error strings, the example tool returns sample JSON, and the submit tool accepts deserialized data with an optional callback
- Current state: tools work individually but there's no automated retry loop, no attempt tracking, no structured error feedback re-injection
- Developer already has `on_submit` callback pattern in SubmitTool — callback rejections should feed into the same retry loop as schema failures
- "Whatever is going to better contribute to our goal — correct answers that conform to our JSON schema as soon as possible" — this is the north star for retry strategy decisions

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-retry-validation-loop*
*Context gathered: 2026-02-01*
