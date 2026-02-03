# Phase 8: Claude Code Adapter - Context

**Gathered:** 2026-02-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Production-harden the Claude Code adapter as the primary adapter. All containment features (disable builtins, sandbox) and extraction features (retry, validation, payload) must work reliably. Code passes clippy pedantic with zero warnings. CLI flag combinations are tested and documented.

No new features — this is about making existing functionality production-grade.

</domain>

<decisions>
## Implementation Decisions

### Reliability criteria
- Containment flag rejection: warn and continue (best-effort containment, not hard failure)
- Extraction max retries: return all attempts so caller can analyze failure pattern
- CLI quirks: document and workaround where possible (don't ignore upstream bugs)
- Timeout handling: graceful signal then force kill after grace period

### Clippy approach
- Fix all root causes — no `#[allow]` unless truly justified
- Fix missing-docs warnings workspace-wide (not just claudecode-adapter)
- Minor breaking API changes OK since we're pre-1.0
- Same pedantic standards for test code as production code

### Test coverage
- Containment tests: E2E with real Claude CLI (actually verify containment holds)
- Extraction tests: comprehensive failures (invalid JSON, timeout mid-stream, schema violations, partial JSON, max retries)
- No CI — all tests run locally only, E2E tests marked `#[ignore]`
- Test organization: Claude decides per module based on what's being tested

### CLI flag documentation
- Location: both inline comments (context) + module-level reference (discoverability)
- Combinations: document valid AND invalid flag combos (what conflicts/breaks)
- Version tracking: note when flags were added/changed/removed by CLI version
- External refs: link to official Anthropic CLI docs where available

### Claude's Discretion
- Specific workaround implementations for CLI quirks
- Test file organization (inline vs tests/ directory) per module
- Which specific clippy lints require `#[allow]` with justification

</decisions>

<specifics>
## Specific Ideas

- "Return all attempts" for failed extractions mirrors the existing ExtractionError::MaxRetriesExceeded design from Phase 2
- Workspace-wide missing-docs fix expands scope beyond Phase 8 but addresses the ~265 warning debt noted in STATE.md
- E2E tests with real CLI means tests require Claude Code installed locally — document this requirement

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-claude-code-adapter*
*Context gathered: 2026-02-03*
