# Project Milestones: rig-cli

## v1.0 Production Release (Shipped: 2026-02-03)

**Delivered:** Production-ready Rust crate that turns CLI-based AI coding agents (Claude Code, Codex, OpenCode) into idiomatic Rig 0.29 providers with schema-validated structured extraction.

**Phases completed:** 01-11 (40 plans total, including 02.1 decimal insertion)

**Key accomplishments:**

- Resource management foundation with bounded channels, JoinSet task tracking, and graceful shutdown
- Self-correcting extraction via retry loop with validation feedback (example → validate → submit workflow)
- MCP tool containment forcing agents to output schema-validated JSON through Rig's native ToolSet
- Cross-platform support (Linux + Windows) with per-CLI binary discovery and config delivery
- rig-cli facade crate with CompletionClient integration and two execution paths (agent() vs mcp_agent())
- Production-hardened adapters for Claude Code, Codex, and OpenCode with E2E containment tests
- 9 comprehensive examples demonstrating extraction, payload injection, and error handling

**Stats:**

- 162 files created/modified
- 33,447 lines of Rust
- 12 phases, 40 plans
- 3 days (Feb 1-3, 2026)

**Git range:** `feat(01-01)` → `docs(11)`

**What's next:** v2.0 with circuit breaker, composable retry policies, and Prometheus metrics

---
