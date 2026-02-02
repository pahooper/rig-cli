# Phase 5: Observability Infrastructure - Context

**Gathered:** 2026-02-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Add structured tracing to the extraction orchestrator and CLI version detection/validation across all three adapters. Scoped to OBSV-01 (extraction tracing) and OBSV-02 (version detection). Adapter-internal tracing is deferred to their respective hardening phases (8, 9, 10).

</domain>

<decisions>
## Implementation Decisions

### Trace verbosity & filtering
- Default tracing level: warn-only. Happy path produces no output unless developer configures a subscriber.
- Library emits tracing events, consuming application sets up subscriber. Standard Rust library pattern — no library-owned subscriber initialization.
- Never log prompt or response content, not even at TRACE level. Log character counts only (prompt_chars, output_chars). Security-first posture.
- Document tracing targets (module paths) so developers can filter precisely with RUST_LOG.

### Version validation behavior
- Warn and continue on unsupported versions. Never block execution on version mismatch.
- Version detection happens once per agent execution (stateless, no caching).
- Warn on untested (newer than max_tested) versions with a distinct message from unsupported (below minimum).
- Version requirements are hardcoded constants per adapter in code, not configurable by developer.

### Output format & destination
- Add tracing-subscriber with json + env-filter features to the mcp crate Cargo.toml (for examples and tests).
- Event messages use snake_case identifiers (prompt_sent_to_agent, validation_failed, retry_decision) for machine-parseable grep/filter.
- Include elapsed timing (elapsed_ms) on completion events and total_duration_ms on extraction outcome.
- Tracing examples deferred to Phase 11 (Documentation & Examples).

### What gets traced
- All five extraction stages traced: prompt_sent, agent_response, validation_result, retry_decision, extraction_outcome.
- Character counts only on prompt/response events — no approximate token estimates.
- Top-level #[instrument] span on extract(), flat events with attempt=N field. No nested attempt spans (avoids async Span::enter() pitfall).
- Scope limited to extraction orchestrator + version validation. Adapter init/run tracing deferred to Phases 8/9/10.

### Claude's Discretion
- Exact tracing field names beyond the snake_case convention
- Whether extract_typed() also gets #[instrument]
- Internal structuring of the version module

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches within the decisions above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-observability-infrastructure*
*Context gathered: 2026-02-02*
