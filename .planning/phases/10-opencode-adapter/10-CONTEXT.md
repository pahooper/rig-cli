# Phase 10: OpenCode Adapter - Context

**Gathered:** 2026-02-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Production-harden OpenCode adapter to full parity with Claude Code and Codex adapters. Same features, same quality standards, same testing, same documentation depth. OpenCode is a first-class production adapter, not maintenance-only.

**Note:** ROADMAP.md will be updated to reflect production status (currently says "maintenance").

</domain>

<decisions>
## Implementation Decisions

### Feature parity
- Match Claude/Codex baseline: full extraction workflow, MCP support, containment
- Same ExtractionConfig defaults: 3 retries, same timeout, same validation
- Same error handling: propagate ExtractionError with attempt history
- Document + best-effort parse for OpenCode CLI quirks (note in code comments, fail gracefully on unknowns)

### Documentation
- Module-level doc comments with same quality/depth as Claude/Codex
- Version compatibility: min_version/max_tested_version const functions, version detection at startup
- Same tracing warn events for known limitations (continue execution)

### Testing
- Match Claude/Codex test pattern: CLI flag tests, config tests, unit tests for cmd.rs
- E2E containment tests: #[ignore] tests with real CLI, verify containment behavior
- Same test failure resolution: fix if reasonable, skip if disproportionate effort
- Clippy pedantic: zero warnings, same standards as other adapters

### Production status
- No deprecation signals or maintenance warnings
- OpenCode is equal-status production adapter alongside Claude Code and Codex
- Same investment level in edge cases and hardening

### Claude's Discretion
- Exact parsing strategy for OpenCode-specific response format quirks
- Order of implementation tasks within the parity goal
- Specific test case selection within the established patterns

</decisions>

<specifics>
## Specific Ideas

- "Same as Claude Code and Codex CLIs" — user explicitly wants identical behavior patterns
- Production-first mindset: this is not a "lesser" adapter, it's a full peer

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-opencode-adapter*
*Context gathered: 2026-02-03*
