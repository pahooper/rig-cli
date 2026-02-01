# Feature Research

**Domain:** CLI Agent to Rig Provider (Structured Extraction System)
**Researched:** 2026-02-01
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Schema-validated JSON extraction** | Core value proposition; the entire point is forcing structured output | LOW | ✓ Already implemented via JsonSchemaToolkit (submit/validate/example tools) |
| **Retry loop with validation feedback** | LLM outputs fail validation frequently; retries with error context are production standard | MEDIUM | Missing; currently single-shot execution without retry on validation failure |
| **Bounded retry attempts** | Unbounded retries = runaway costs; industry standard is 3-5 attempts with backoff | LOW | Missing; no retry limit or cost awareness |
| **Session isolation** | Different extraction tasks must not interfere with each other; stateful workflows need sandboxing | LOW | ✓ Already implemented via SessionManager with persistent temp directories |
| **Error propagation** | Failures should surface with actionable context, not silent degradation | MEDIUM | Partial; has error types but uses .expect() in critical paths (stream handling) |
| **Basic observability** | Must log prompt sent, agent response, validation result to diagnose failures | MEDIUM | Missing; no tracing for extraction workflow stages |
| **MCP protocol compliance** | Standard for CLI agent tool integration; without it, agents can't discover tools | LOW | ✓ Already implemented via RigMcpHandler and RMCP library |
| **Multi-adapter support** | Users expect to use whichever CLI agent they have installed (Claude Code, Codex) | MEDIUM | ✓ Already implemented with adapter pattern for 3 CLIs |
| **Structured error messages** | LLMs need detailed validation errors to self-correct; generic errors rarely work | LOW | ✓ Already implemented in ValidateJsonTool with schema error details |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Idiomatic Rig integration** | Feels like native Rig, not bolted-on; uses ToolSet, CompletionModel, Tool traits correctly | MEDIUM | ✓ Already implemented; architecture follows Rig patterns |
| **Type-driven schema generation** | Define struct once, schema auto-generated via schemars; no manual JSON schema writing | LOW | ✓ Already implemented via JsonSchemaToolkit<T> with schemars::JsonSchema |
| **Three-tool workflow pattern** | example → validate → submit gives agent a clear workflow; reduces trial-and-error | LOW | ✓ Already implemented in JsonSchemaToolkit |
| **Per-CLI containment flags** | Each CLI has different sandbox mechanisms; exposing per-adapter isolation controls = power user feature | HIGH | Missing; current RunConfig doesn't expose containment flags per CLI |
| **Payload injection** | Pass file contents, context blobs alongside prompts for agent to process; enables batch extraction workflows | MEDIUM | Missing; no mechanism to inject data payloads into prompts |
| **Token cost tracking** | Expose token usage per extraction attempt; prevent runaway retry costs | MEDIUM | Missing; no cost awareness or token usage reporting |
| **Streaming extraction progress** | Show agent thinking, tool calls, validation attempts in real-time; valuable for debugging and UX | MEDIUM | Partial; streaming exists but not tied to extraction workflow visibility |
| **Composable retry policies** | Configurable backoff (exponential, linear, jitter), custom retry predicates (retry only on schema errors, not timeouts) | HIGH | Missing; no retry infrastructure at all currently |
| **Circuit breaker pattern** | After N consecutive failures, stop attempting; prevents retry storms | MEDIUM | Missing; no failure tracking across attempts |
| **Declarative toolkit builder** | Configure submit/validate/example tools via builder pattern; users customize tool names, descriptions, success messages | LOW | ✓ Already implemented via JsonSchemaToolkitBuilder |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Automatic schema inference from examples** | "I don't want to write a struct, just give it an example" | LLMs hallucinate structure; example-based schemas are ambiguous and unreliable | Require explicit schemars::JsonSchema struct; one-time cost, permanent clarity |
| **Unlimited retry attempts** | "Keep trying until it works" | Runaway costs, retry storms, DDoS on provider APIs; production systems need bounded failure | Hard limit of 3-5 retries with exponential backoff; fail loudly after limit |
| **Full filesystem access for agents** | "Let the agent do whatever it needs" | Security nightmare; agents can read SSH keys, AWS credentials, modify system files | Strict sandbox with read-only root, read-write only in session temp directory |
| **Automatic prompt optimization** | "Make the prompt better automatically" | Adds unpredictability; prompt engineering should be explicit and version-controlled | Provide good default instructions, but let users customize via builder |
| **Caching extraction results** | "Don't re-extract the same data" | Cache invalidation is hard; stale data is worse than slow data; extraction inputs are rarely identical | No caching; optimize retry loop instead; use session isolation for stateful workflows |
| **Synchronous blocking API** | "I don't want to deal with async" | Blocks threads during long-running agent execution; doesn't compose with Rig's async ecosystem | Async-only API using Tokio; align with Rig's CompletionModel trait (already async) |
| **OpenCode production hardening** | "Support all three adapters equally" | OpenCode has different maturity; spreading effort thin = three mediocre adapters | Focus on Claude Code + Codex for v1.0; maintain OpenCode but don't harden |

## Feature Dependencies

```
[Schema-validated extraction]
    ├──requires──> [MCP protocol compliance] (must expose tools)
    ├──requires──> [Structured error messages] (validation feedback)
    └──requires──> [Multi-adapter support] (works with user's CLI)

[Retry loop with validation feedback]
    ├──requires──> [Bounded retry attempts] (prevents runaway costs)
    ├──requires──> [Error propagation] (must surface validation errors)
    ├──requires──> [Structured error messages] (agent needs error details)
    └──enhances──> [Token cost tracking] (cost awareness per retry)

[Per-CLI containment flags]
    ├──requires──> [Multi-adapter support] (different CLIs = different flags)
    └──requires──> [Session isolation] (sandbox boundary enforcement)

[Payload injection]
    ├──requires──> [Schema-validated extraction] (payload structure must be typed)
    └──enhances──> [Three-tool workflow] (validate payload + extraction together)

[Basic observability]
    ├──enhances──> [Retry loop] (trace retry attempts)
    ├──enhances──> [Error propagation] (structured logs for debugging)
    └──enhances──> [Token cost tracking] (log cost per stage)

[Circuit breaker pattern]
    ├──requires──> [Retry loop] (no retry = no circuit to break)
    └──requires──> [Basic observability] (track failure patterns)

[Composable retry policies]
    ├──requires──> [Retry loop] (policy configures retry behavior)
    └──conflicts──> [Unlimited retry attempts] (policy enforces bounds)
```

### Dependency Notes

- **Retry loop is foundational:** Enables cost tracking, observability, circuit breaker, and composable policies; must be implemented before those enhancements.
- **Observability multiplies value:** Every feature becomes easier to debug and optimize with tracing; implement early.
- **Containment is adapter-specific:** Each CLI (Claude Code, Codex, OpenCode) has different sandbox mechanisms; can't be generic.
- **Payload injection is orthogonal:** Doesn't depend on retry or observability; can be implemented independently.

## MVP Definition

### Launch With (v1.0)

Minimum viable product — what's needed to validate the concept.

- [x] **Schema-validated JSON extraction** — Core value; already works via JsonSchemaToolkit
- [x] **MCP protocol compliance** — Table stakes for CLI agent integration; already working
- [x] **Multi-adapter support** — Users need Claude Code + Codex; architecture supports this
- [x] **Session isolation** — Prevents interference between extraction tasks; already implemented
- [x] **Structured error messages** — LLMs need validation error details; already in ValidateJsonTool
- [ ] **Retry loop with validation feedback** — Production standard; currently missing (CRITICAL GAP)
- [ ] **Bounded retry attempts** — Prevents runaway costs; currently missing (CRITICAL GAP)
- [ ] **Error propagation** — Replace .expect() with proper error handling; partially done (CRITICAL GAP)
- [ ] **Basic observability** — Log extraction workflow stages; currently missing (CRITICAL GAP)

### Add After Validation (v1.1-v1.5)

Features to add once core is working.

- [ ] **Token cost tracking** — Trigger: Users report unexpected costs from retries
- [ ] **Payload injection** — Trigger: Users request batch extraction workflows (file contents, etc.)
- [ ] **Per-CLI containment flags** — Trigger: Security-conscious users request tighter sandboxing
- [ ] **Streaming extraction progress** — Trigger: Users want visibility into long-running extractions
- [ ] **Composable retry policies** — Trigger: Users need custom backoff (e.g., linear for fast APIs)
- [ ] **Circuit breaker pattern** — Trigger: Users report retry storms during outages

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Advanced observability** — Metrics, dashboards, alerting; defer until scale demands it
- [ ] **OpenCode production hardening** — Defer until Claude Code + Codex are rock solid
- [ ] **Additional CLI adapters** — Gemini, other agents; extension point exists
- [ ] **Custom validation predicates** — User-defined validation beyond JSON schema; complex to design
- [ ] **Multi-stage extraction pipelines** — Chain multiple extraction tasks; solve with external orchestration first

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Retry loop with validation feedback | HIGH | MEDIUM | P1 |
| Bounded retry attempts | HIGH | LOW | P1 |
| Error propagation (remove .expect()) | HIGH | LOW | P1 |
| Basic observability (tracing) | HIGH | MEDIUM | P1 |
| Token cost tracking | MEDIUM | MEDIUM | P2 |
| Payload injection | MEDIUM | MEDIUM | P2 |
| Per-CLI containment flags | MEDIUM | HIGH | P2 |
| Streaming extraction progress | MEDIUM | MEDIUM | P2 |
| Composable retry policies | LOW | HIGH | P3 |
| Circuit breaker pattern | LOW | MEDIUM | P3 |
| Advanced observability | LOW | HIGH | P3 |
| OpenCode hardening | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for v1.0 launch (production-ready)
- P2: Should have, add after v1.0 validation
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | Instructor (Python) | LlamaExtract | Our Approach (rig-cli) |
|---------|---------------------|--------------|------------------------|
| **Structured extraction** | ✓ Pydantic models | ✓ Managed service | ✓ Rust structs via schemars |
| **Retry logic** | ✓ Auto-retry on validation failure | ✓ Built-in fault tolerance | ❌ Missing (P1 gap) |
| **Validation feedback** | ✓ Pydantic errors to LLM | ✓ Schema enforcement | ✓ ValidateJsonTool with detailed errors |
| **CLI agent support** | ❌ API-based only | ❌ API-based only | ✓ Purpose-built for CLI agents |
| **Multi-provider** | ✓ OpenAI, Anthropic, etc. | ❌ Managed service | ✓ Claude Code, Codex, OpenCode |
| **Session isolation** | ❌ Stateless | ❌ Managed service | ✓ SessionManager with temp directories |
| **Token tracking** | ✓ Via API wrappers | ✓ Managed service | ❌ Missing (P2) |
| **Streaming** | ✓ Streaming support | ❌ Batch only | ✓ Partial (not extraction-aware) |
| **Type safety** | ✓ Python typing | ❌ REST API | ✓ Rust compile-time safety |
| **Cost awareness** | ✓ Max retries configurable | ✓ Managed service | ❌ Missing (P2) |

**Key differentiation:**
- **rig-cli is the only CLI-agent-focused structured extraction system** — Instructor and LlamaExtract target API-based LLMs, not CLI tools.
- **Rust type safety is unique** — Competitors are Python (runtime errors) or REST (no typing).
- **Missing retry logic is critical gap** — Competitors have this; we must implement for parity.

## Production Readiness Checklist

Based on research into production LLM systems (2026):

**Essential for production deployment:**
- [x] Schema enforcement at protocol level (MCP tools)
- [x] Session-based sandboxing for isolation
- [ ] Retry loop with exponential backoff + jitter (CRITICAL GAP)
- [ ] Bounded retry attempts (3-5 max, industry standard)
- [ ] Observability: trace IDs, structured logs, stage markers (CRITICAL GAP)
- [ ] Error handling: no panics in production paths (CRITICAL GAP)
- [ ] Bounded channels: replace unbounded mpsc (CONCERNS.md identified this)
- [ ] Explicit task cancellation: cleanup spawned tasks (CONCERNS.md identified this)

**Important for production quality:**
- [ ] Token cost tracking per extraction attempt
- [ ] Validation error details returned to LLM (already have this)
- [ ] Timeout handling with configurable limits
- [ ] Health checks for CLI availability
- [ ] Metrics: success rate, latency, retry count

**Nice to have:**
- [ ] Circuit breaker after consecutive failures
- [ ] Dead letter queue for failed extractions
- [ ] Custom retry predicates (retry on schema error, not timeout)

## Research-Informed Recommendations

### Immediate (v1.0 Launch)

1. **Implement retry loop with validation feedback** — This is the #1 gap vs. competitors. Pattern: exponential backoff with jitter, max 3-5 attempts, feed validation errors back to agent. See: [Instructor retry logic](https://python.useinstructor.com/), [LLM retry patterns 2026](https://medium.com/@komalbaparmar007/llm-tool-calling-in-production-rate-limits-retries-and-the-infinite-loop-failure-mode-you-must-2a1e2a1e84c8).

2. **Add structured tracing** — Production systems in 2026 require trace IDs linking prompt → agent response → validation → retry. See: [AI observability 2026](https://www.braintrust.dev/articles/best-ai-observability-tools-2026), [Agent harness observability](https://skywork.ai/blog/ai-agent/observability-manus-1-5-agents-best-practices/).

3. **Remove .expect() and .unwrap()** — CONCERNS.md identified panics in stream handling. Replace with proper error propagation using `?` operator.

4. **Bounded channels** — Replace unbounded mpsc with bounded channels (e.g., `channel(1000)`) to prevent memory exhaustion. See CONCERNS.md.

### Post-Launch (v1.1+)

5. **Token cost tracking** — Track usage per attempt, log cumulative cost. Prevents runaway retry costs. See: [LLM cost awareness](https://portkey.ai/blog/the-complete-guide-to-llm-observability/).

6. **Payload injection** — Enable passing file contents, context blobs alongside prompts for batch extraction workflows.

7. **Per-CLI containment flags** — Expose sandbox controls per adapter. Claude Code uses bubblewrap/seatbelt, Codex may differ. See: [Claude Code sandboxing](https://code.claude.com/docs/en/sandboxing), [Docker sandboxes for agents](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/).

### Future (v2+)

8. **Circuit breaker** — After N consecutive failures, stop retrying. Prevents retry storms. See: [Circuit breakers for LLMs](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/).

9. **Composable retry policies** — Builder pattern for custom backoff, retry predicates. Deferred until users request specific policies.

## Sources

**Rig Framework:**
- [Rig documentation](https://docs.rig.rs/)
- [Rig GitHub](https://github.com/0xPlaygrounds/rig)
- [Model provider clients in Rig](https://docs.rig.rs/docs/concepts/provider_clients)

**Structured Extraction Patterns:**
- [Instructor - Multi-Language Library for Structured LLM Outputs](https://python.useinstructor.com/)
- [Structured Data Extraction | LlamaIndex Python Documentation](https://docs.llamaindex.ai/en/stable/use_cases/extraction/)
- [LLMs for Structured Data Extraction from PDFs in 2026](https://unstract.com/blog/comparing-approaches-for-using-llms-for-structured-data-extraction-from-pdfs/)
- [How JSON Schema Works for LLM Tools & Structured Outputs](https://blog.promptlayer.com/how-json-schema-works-for-structured-outputs-and-tool-integration/)

**Retry and Error Handling:**
- [LLM Tool-Calling in Production: Rate Limits, Retries, and the "Infinite Loop" Failure Mode](https://medium.com/@komalbaparmar007/llm-tool-calling-in-production-rate-limits-retries-and-the-infinite-loop-failure-mode-you-must-2a1e2a1e84c8)
- [Retries, fallbacks, and circuit breakers in LLM apps](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)
- [Backoff and Retry Strategies for LLM Failures](https://palospublishing.com/backoff-and-retry-strategies-for-llm-failures/)
- [Error Handling in MCP Tools](https://apxml.com/courses/getting-started-model-context-protocol/chapter-3-implementing-tools-and-logic/error-handling-reporting)

**Agent Harness & Observability:**
- [The importance of Agent Harness in 2026](https://www.philschmid.de/agent-harness-2026)
- [Top 5 AI Agent Observability Platforms 2026 Guide](https://o-mega.ai/articles/top-5-ai-agent-observability-platforms-the-ultimate-2026-guide)
- [AI observability tools: A buyer's guide to monitoring AI agents in production (2026)](https://www.braintrust.dev/articles/best-ai-observability-tools-2026)
- [Observability for Manus 1.5 Agents: Logs, Retries, Error Budgets](https://skywork.ai/blog/ai-agent/observability-manus-1-5-agents-best-practices/)
- [The complete guide to LLM observability for 2026](https://portkey.ai/blog/the-complete-guide-to-llm-observability/)

**Agent Containment & Sandboxing:**
- [Docker Sandboxes: A New Approach for Coding Agent Safety](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/)
- [Sandboxing - Claude Code Docs](https://code.claude.com/docs/en/sandboxing)
- [Anthropic Engineering: Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)
- [Show HN: Fence – Sandbox CLI commands with network/filesystem restrictions](https://news.ycombinator.com/item?id=46695467)
- [What's the best code execution sandbox for AI agents in 2026?](https://northflank.com/blog/best-code-execution-sandbox-for-ai-agents)

**Testing & Mocking:**
- [Rust Mock Shootout](https://asomers.github.io/mock_shootout/)
- [mockall - Rust](https://docs.rs/mockall)
- [Test Harness in Software Testing](https://testrigor.com/blog/test-harness-in-software-testing/)

---
*Feature research for: CLI Agent to Rig Provider (Structured Extraction System)*
*Researched: 2026-02-01*
