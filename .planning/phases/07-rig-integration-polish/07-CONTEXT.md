# Phase 7: Rig Integration Polish - Context

**Gathered:** 2026-02-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Make rig-cli's API surface feel like a native Rig 0.29 extension. Developers should be able to use CLI-backed agents with the same patterns they use for cloud API providers (OpenAI, Anthropic). This phase wraps existing internals (adapters, orchestrator, MCP plumbing) behind Rig-idiomatic Client/Agent/Builder types. No new capabilities — only reshaping the interface.

</domain>

<decisions>
## Implementation Decisions

### Public API shape
- Single `rig-cli` crate with feature flags for adapters (claude, codex, opencode) — all enabled by default
- API must mirror Rig's cloud provider pattern: `let client = rig_cli::claude::Client::new(); let agent = client.agent("model").preamble("...").build();`
- Developer should not know they're using CLI agents under the hood — it should feel like using normal Rig
- All interactions go through MCP server, even simple `.prompt()` calls — agents always submit responses via tool calls (core value enforcement)
- Multi-turn chat follows whatever pattern Rig 0.29 provides for chat
- Internal types (ExtractionOrchestrator, AdapterConfig, RunConfig) hidden by default; escape hatch available via `.raw()` or `.inner()` for advanced users
- Prelude module: `use rig_cli::prelude::*;` for common types, matching Rig convention
- Crate name is `rig-cli`
- Target Rig 0.29 specifically
- Workspace structure decision: Claude's discretion — pick the approach (facade over workspace vs consolidation) that follows the best engineering principles

### Builder ergonomics
- CLI-specific configuration (binary path, timeout, channel capacity) lives at Client level — all agents from a client inherit settings
- MCP tool configuration is fully automatic — developer passes ToolSet via `.tools()`, everything else (server spawning, config generation, tool wiring) is invisible
- Containment defaults (disable builtins, sandbox to temp dir) are hidden and always on — no builder methods to weaken them; advanced users use escape hatch
- Builder chaining must match Rig's exact style: `client.agent("model").preamble("...").build()`
- CLI-specific builder methods (like `.payload()`) follow whatever pattern is most Rig-like — Claude's discretion on integration approach
- `.build()` sync vs async: Claude's discretion based on Rig's own pattern
- Retry/validation config (max attempts, strategy) uses internal defaults only — not exposed on builder

### Rig trait alignment
- CompletionModel trait implementation approach: Claude's discretion based on how other Rig providers structure their impls
- ToolSet-only vs raw JSON schema: Claude's discretion based on Rig's Tool trait requirements
- Return Rig's response types (CompletionResponse, ChatResponse, etc.) from trait methods — extraction metadata available via extension methods
- Implement streaming traits from the start — CLI agents already stream stdout, surface that through Rig's streaming interface
- Integrate with Rig's tool execution pipeline — tools registered via Rig's API, results flow through Rig's pipeline
- Research actual Rig 0.29 source during planning to match current patterns (not assumptions from roadmap creation time)
- Completion/extraction provider only — no embedding traits (CLI tools don't support embeddings)

### Error surface
- Use Rig's error types where possible — only use custom errors for CLI-specific failures (binary not found, process crash, version mismatch)
- Error messages: Display shows actionable message for users ("Claude CLI not found. Install: npm i -g @anthropic-ai/claude-code"), Debug shows technical details for developers
- Extraction failure error design: Claude's discretion based on what consumers need
- Error pattern (thiserror, custom, etc.): Claude's discretion, match Rig's approach
- Raw CLI output in errors behind feature flag: `features = ["debug-output"]` to avoid leaking large outputs in production

### Claude's Discretion
- Workspace structure decision (facade vs consolidation) — pick best engineering approach
- CompletionModel trait impl structure (direct on client vs wrapper type)
- Tool schema source (ToolSet-only vs ToolSet + raw JSON)
- CLI-specific builder method integration pattern
- `.build()` sync vs async
- Extraction error variant design
- Error implementation pattern (match Rig's approach)

</decisions>

<specifics>
## Specific Ideas

- Rig cloud provider pattern is the gold standard. User provided concrete example:
  ```rust
  let client = openai::Client::from_env();
  let agent = client.agent("gpt-5.2")
      .preamble("You are a comedian...")
      .build();
  let response = agent.prompt("Entertain me!").await?;
  ```
  rig-cli must look exactly like this, just with `rig_cli::claude::Client` instead of `openai::Client`.

- "I want it to look like you're using normal Rig" — the CLI execution is an implementation detail, not a user-facing concern

</specifics>

<deferred>
## Deferred Ideas

- Local ONNX embedding model in rig-cli for embeddings support — new capability, separate phase

</deferred>

---

*Phase: 07-rig-integration-polish*
*Context gathered: 2026-02-02*
