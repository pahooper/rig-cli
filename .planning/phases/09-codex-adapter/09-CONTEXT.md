# Phase 9: Codex Adapter - Context

**Gathered:** 2026-02-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Production hardening the Codex CLI adapter to the same standard as Claude Code. All containment features, extraction features, CLI flag documentation, and tests should match Claude Code quality. Codex becomes a first-class adapter alongside Claude Code.

</domain>

<decisions>
## Implementation Decisions

### Containment Flags
- Test and document the MCP sandbox bypass (#4152) as a known limitation rather than skip sandbox tests
- Research whether Codex has replacement approval mechanisms (--ask-for-approval IS available, contrary to earlier decision)
- Default approval policy: `untrusted` (only run trusted commands without approval - most restrictive automated mode)
- E2E containment tests use same pattern as Claude Code (real CLI, #[ignore], helper function for CLI discovery)

### Response Parsing
- Focus on `codex exec` output format only - that's what MCP extraction uses
- Same retry budget as Claude Code (3 attempts default, same ExtractionConfig)
- Explicitly verify Codex stream events match expected simpler enum (Text/Error/Unknown only)
- Generic extraction error format - same ExtractionError variants, adapter-agnostic feedback

### CLI Documentation
- Same structure as Claude Code cmd.rs: Flag Reference, Combinations, Version Notes, Known Limitations
- MCP sandbox bypass (#4152) documented in BOTH cmd.rs Known Limitations AND crate-level docs
- Test both valid and invalid flag combinations (document conflicts like sandbox + full_auto)
- Documentation scope: same as Claude Code (containment-relevant flags, not exhaustive)

### Test Coverage
- E2E tests in tests/e2e_containment.rs with #[ignore] - mirrors Claude Code structure
- Rely on shared orchestrator tests for extraction failures (no Codex-specific duplication)
- Use windows(2) pattern for unit test flag pair verification
- Sandbox mode test coverage: Claude's discretion (default containment mode vs all three)

### Parity with Claude Code
- Both API consistency AND behavior consistency across adapters
- Codex-specific clippy pedantic pass (not relying on workspace-wide 08-01)
- Codex should match Claude Code capabilities; OpenCode stays at maintenance baseline per roadmap

### Claude's Discretion
- Sandbox mode E2E test coverage (all three modes vs default only)
- Internal implementation details of flag combination validation

</decisions>

<specifics>
## Specific Ideas

- Reference /home/pnod/foryou.md for current Codex CLI help output (--ask-for-approval IS available with untrusted/on-failure/on-request/never policies)
- Codex has richer sandbox options: read-only, workspace-write, danger-full-access
- --full-auto is a convenience alias for (-a on-request, --sandbox workspace-write)

</specifics>

<deferred>
## Deferred Ideas

- OpenCode production hardening to same level as Claude Code and Codex - note for Phase 10 scope discussion (user wants all three adapters equal, but Phase 10 roadmap says "maintenance only")

</deferred>

---

*Phase: 09-codex-adapter*
*Context gathered: 2026-02-03*
