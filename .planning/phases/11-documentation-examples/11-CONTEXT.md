# Phase 11: Documentation & Examples - Context

**Gathered:** 2026-02-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Developer can understand and use library end-to-end from documentation. Includes end-to-end examples demonstrating extraction workflow with real CLI agents, payload injection, retry handling, error recovery, and comprehensive doc comments explaining the "why" not just the "what".

</domain>

<decisions>
## Implementation Decisions

### Example Structure
- Both standalone `examples/` files AND doc tests — standalone for complete workflows, doc tests for simple inline snippets
- Feature-named examples (extraction.rs, mcp_agent.rs) — no numbered progression, find by feature
- Self-contained examples — each includes all setup, copy-paste friendly, no shared helpers
- Both full main() and highlighted key code — complete runnable example plus commented "key code" section

### Doc Comment Depth
- Comprehensive documentation — purpose, parameters explained, return value, example snippet, panics/errors documented
- Selectively include code examples — examples for non-obvious APIs, builders, entry points; skip trivial getters
- Dedicated `# Errors` section — list each error variant and when it occurs
- Comprehensive internals documentation — full documentation throughout for contributors, not just public API

### Learning Path
- Concept overview first — explain what CLI agents are, why MCP matters, then show code
- README flow: Concept → Quick start → Features → Examples
- README + rustdoc only — no separate GUIDE.md or mdBook site
- Comparison table for adapter differences — feature matrix showing Claude/Codex/OpenCode capabilities at a glance

### Code Coverage
- Claude as primary adapter in examples — note that Codex/OpenCode follow same pattern
- Dedicated error handling example — separate error_handling.rs showing retry exhaustion, parse failures, timeout
- Fix all ~265 missing-docs warnings — clean slate, every public item gets documentation

### Required User Story Examples
1. **Chat with MCP and sessions** — multi-turn conversation with MCP tool responses
2. **One-shot with MCP** — single prompt, structured MCP response
3. **Agent with MCP** — standard 3-tool pattern (example/validate/submit)
4. **Agent with MCP and extra tools** — 3-tool pattern plus additional custom tools
5. **Multiagent with extra tools and MCP** — multiple agents coordinating with tools
6. **Extraction agent with MCP** — structured data extraction via MCP validation
7. **Chat about file via payload** — both single Q&A and multi-turn conversation about injected file content
8. **MCP Agent + deterministic tool** — 3-tool pattern plus custom date extraction tool, showing full tool definition

### Claude's Discretion
- Exact example file names
- Order of sections within README
- Which APIs get inline doc examples vs entry-point-only
- Internal module documentation depth per module

</decisions>

<specifics>
## Specific Ideas

- Examples should demonstrate both single Q&A and multi-turn patterns for payload-based chat
- Custom tool example (date extraction) should show full tool definition, not assume it exists
- User stories represent real developer workflows, not synthetic demos

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 11-documentation-examples*
*Context gathered: 2026-02-03*
