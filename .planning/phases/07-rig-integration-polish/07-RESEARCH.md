# Phase 7: Rig Integration Polish - Research

**Researched:** 2026-02-02
**Domain:** Rust library API design, Rig 0.29 provider patterns, workspace architecture
**Confidence:** MEDIUM-HIGH

## Summary

Phase 7 wraps existing CLI adapter internals (ClaudeModel, CodexModel, OpenCodeModel, McpToolAgent) behind a Rig-idiomatic public API that matches the cloud provider pattern used by OpenAI, Anthropic, and other Rig providers. The project already has CompletionModel implementations for all three adapters and an McpToolAgent builder that handles MCP orchestration. This phase creates a clean facade that hides these internals and exposes only Client/Agent/Builder types that feel native to Rig 0.29.

The standard Rig provider pattern is: `Client::new() → client.agent("model") → .preamble()/.tools()/.build() → agent.prompt().await`. The codebase already implements the hard parts (CLI spawning, MCP config generation, streaming, tool execution). This phase is purely about API reshaping to match Rig's design language.

**Primary recommendation:** Use workspace facade pattern with `rig-cli` crate as thin public API over existing `rig-provider` internals. Keep CompletionModel implementations, add ProviderClient + Client wrappers, make `.build()` synchronous (construction without I/O), surface CLI-specific config at Client level only.

## Standard Stack

The established libraries/tools for Rig provider integration:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rig-core | 0.29.0 | Rig framework foundation | Required for CompletionModel, Tool, ToolSet traits |
| thiserror | 1.0 | Library error types | Rust ecosystem standard for library error handling |
| serde/serde_json | 1.0 | Serialization | Required for CompletionRequest/Response, tool schemas |
| tokio | 1.0 | Async runtime | Required by Rig's async trait methods |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| schemars | 1.2 | JSON schema generation | Tool parameter schema generation |
| tempfile | 3.10 | Temp directory management | Sandbox containment (already in use) |
| semver | 1.0 | Version validation | CLI version checking (already in use) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| thiserror | anyhow | anyhow is for applications, not libraries; consumers need structured errors |
| Workspace facade | Single crate consolidation | Consolidation loses adapter separation benefits |

**Installation:**
```bash
# Project uses workspace, core dependencies already present
cargo add rig-core@0.29.0 thiserror@1.0
```

## Architecture Patterns

### Recommended Project Structure (Workspace Facade)
```
rig-cli/                      # Project root
├── Cargo.toml                # Workspace manifest
├── rig-cli/                  # PUBLIC FACADE CRATE (new)
│   ├── Cargo.toml            # Depends on rig-provider, re-exports
│   ├── src/
│   │   ├── lib.rs            # Public API root
│   │   ├── prelude.rs        # Common imports (Client, Agent, ToolSet)
│   │   ├── claude.rs         # rig_cli::claude::Client + AgentBuilder
│   │   ├── codex.rs          # rig_cli::codex::Client + AgentBuilder
│   │   ├── opencode.rs       # rig_cli::opencode::Client + AgentBuilder
│   │   ├── errors.rs         # Public error types (wraps ProviderError)
│   │   └── config.rs         # ClientConfig shared across adapters
├── rig-provider/             # INTERNAL IMPLEMENTATION (existing)
│   ├── src/
│   │   ├── adapters/         # CompletionModel impls (keep as-is)
│   │   ├── mcp_agent.rs      # McpToolAgent (keep as-is)
│   │   └── ...               # Existing infrastructure
├── claudecode-adapter/       # CLI wrappers (unchanged)
├── codex-adapter/            # CLI wrappers (unchanged)
├── opencode-adapter/         # CLI wrappers (unchanged)
└── mcp/                      # MCP server (unchanged)
```

**Rationale:** Facade preserves existing adapter separation, provides clean upgrade path, keeps internal complexity isolated. Workspace already uses flat layout (best practice for 10k-1M LOC projects per matklad).

### Pattern 1: ProviderClient + Client Wrapper
**What:** Implement Rig's ProviderClient trait on a new Client type that wraps existing CompletionModel
**When to use:** For each adapter (claude, codex, opencode)
**Example:**
```rust
// Source: https://docs.rig.rs/docs/concepts/provider_clients
// Adapted for CLI provider pattern

use rig::client::ProviderClient;
use rig_provider::adapters::claude::ClaudeModel;

#[derive(Clone)]
pub struct Client {
    inner: ClaudeModel,
    config: ClientConfig, // CLI-specific: binary_path, timeout, channel_capacity
}

impl ProviderClient for Client {
    type Input = (); // No input needed, discovery automatic

    fn new(_input: Self::Input) -> Result<Self, rig::client::Error> {
        // Calls claudecode_adapter::init(), wraps in Client
        let report = claudecode_adapter::init(None).await?;
        Ok(Self {
            inner: ClaudeModel::new(report.claude_path, report.capabilities),
            config: ClientConfig::default(),
        })
    }

    fn from_env() -> Self {
        Self::new(()).expect("Claude CLI discovery failed")
    }
}

// Extension trait for agent creation
impl Client {
    pub fn agent(&self, model: impl Into<String>) -> AgentBuilder {
        AgentBuilder::new(self.clone(), model.into())
    }
}
```

### Pattern 2: AgentBuilder Facade
**What:** Builder that constructs Rig Agent with CLI-specific config hidden
**When to use:** For creating agents from Client
**Example:**
```rust
// Source: https://docs.rig.rs/docs/concepts/agent
// Adapted for facade over McpToolAgent

pub struct AgentBuilder {
    client: Client,
    model: String,
    preamble: Option<String>,
    tools: Option<rig::tool::ToolSet>,
    temperature: Option<f64>,
    // CLI-specific fields hidden from user:
    // - timeout (inherited from client.config)
    // - builtin tools (always disabled)
    // - sandbox mode (always temp dir)
}

impl AgentBuilder {
    pub fn preamble(mut self, text: impl Into<String>) -> Self {
        self.preamble = Some(text.into());
        self
    }

    pub fn tools(mut self, toolset: rig::tool::ToolSet) -> Self {
        self.tools = Some(toolset);
        self
    }

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    // SYNC build - construction without I/O
    pub fn build(self) -> rig::agent::Agent<ClaudeModel> {
        // Wraps CompletionModel in Rig Agent
        let mut agent = rig::agent::Agent::new(self.client.inner)
            .model(self.model);

        if let Some(p) = self.preamble {
            agent = agent.preamble(p);
        }
        if let Some(t) = self.tools {
            agent = agent.tools(t);
        }
        if let Some(temp) = self.temperature {
            agent = agent.temperature(temp);
        }

        agent
    }
}
```

### Pattern 3: Feature Flags for Adapters
**What:** Optional compilation of adapters via Cargo features
**When to use:** Always - allows users to minimize binary size
**Example:**
```toml
# rig-cli/Cargo.toml
[features]
default = ["claude", "codex", "opencode"]
claude = ["rig-provider/claude-adapter"]
codex = ["rig-provider/codex-adapter"]
opencode = ["rig-provider/opencode-adapter"]

[dependencies]
rig-provider = { path = "../rig-provider" }
```

### Anti-Patterns to Avoid
- **Async .build():** Construction should be sync; Rig providers use sync build, async only for I/O operations like .prompt()
- **Leaking internal types:** Don't expose McpToolAgent, AdapterConfig, RunConfig in public API - use extension methods or feature flags for escape hatches
- **Per-agent MCP config:** MCP tools should be invisible; developer passes ToolSet, everything else automatic
- **Magic preludes:** Don't export everything into prelude - only truly ubiquitous types (Client, Error, maybe ToolSet re-export)

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CompletionModel trait impl | Custom wrapper | Existing ClaudeModel/CodexModel/OpenCodeModel | Already implements async completion, streaming, tool support correctly |
| MCP server spawning | Manual process management | Existing McpToolAgent | Handles config generation, temp files, tool name computation, version checking |
| Tool schema conversion | Manual JSON schema building | Rig's ToolSet + schemars | ToolSet.get_tool_definitions() provides correct format, schemars generates schemas |
| Client configuration | Custom config struct | Standard builder pattern + ProviderClient | Rig ecosystem expects ProviderClient trait, builders are Rust idiom |
| Error conversion | Custom error wrapping | thiserror with #[from] | Auto-implements From trait, preserves error chains correctly |
| Streaming responses | Custom stream adapters | Existing StreamingCompletionResponse wrappers | Already converts CLI stdout to RawStreamingChoice correctly |

**Key insight:** The hard work is already done in rig-provider, claudecode-adapter, codex-adapter, opencode-adapter. This phase is pure API reshaping, not capability building.

## Common Pitfalls

### Pitfall 1: Making .build() Async
**What goes wrong:** Async build() forces users to .await construction, breaking Rig's pattern where only execution is async
**Why it happens:** CLI discovery (init/discover_codex/discover_opencode) is async in adapters
**How to avoid:** Move discovery to Client::new()/from_env(), cache result in Client. AgentBuilder.build() just wraps existing CompletionModel in Rig Agent (sync operation).
**Warning signs:** If .build() signature returns Future, you've diverged from Rig's pattern

### Pitfall 2: Exposing Too Much in Prelude
**What goes wrong:** Prelude becomes grab-bag of types, pollutes user namespace
**Why it happens:** Copying standard library pattern without understanding "always needed" threshold
**How to avoid:** Only export types used in 90%+ of interactions. For rig-cli: probably just Client types and Error. NOT: AgentBuilder (accessed via client.agent()), RunConfig (internal), McpToolAgent (internal).
**Warning signs:** Prelude has >5 items; prelude exports builder types; prelude exports internal config structs

### Pitfall 3: Feature Flag Explosions
**What goes wrong:** 2^N build combinations create untestable matrix
**Why it happens:** Making every optional dependency a separate feature without grouping
**How to avoid:** Three features only: `claude`, `codex`, `opencode`. All default on. No sub-features like `claude-streaming` or `codex-mcp`.
**Warning signs:** More than 5 features in [features] section; features that don't correspond to adapters; features for implementation details

### Pitfall 4: Trying to Hide CLI Reality Completely
**What goes wrong:** Users hit CLI-specific errors (binary not found, version mismatch) with no escape hatch
**Why it happens:** Over-abstracting to match cloud provider pattern which has no binary dependencies
**How to avoid:** Provide .with_binary_path() on Client for custom CLI locations, .raw() or .inner() on Agent for McpToolAgent access, clear error messages ("Claude CLI not found. Install: npm i -g @anthropic-ai/claude-code").
**Warning signs:** No way to specify CLI path; errors say "provider error" not "claude binary not found"; no way to access underlying adapter for debugging

### Pitfall 5: Non-Additive Features
**What goes wrong:** Disabling a feature breaks existing code, SemVer violation
**Why it happens:** Features that change API shape instead of just enabling/disabling optional code
**How to avoid:** Features must be pure additive. Disabling `codex` feature removes `rig_cli::codex` module entirely, but doesn't change `rig_cli::claude` API.
**Warning signs:** Conditional compilation (#[cfg]) inside module that changes struct fields; methods that exist only with certain features; default types that change based on features

### Pitfall 6: Leaking Adapter Internal Errors
**What goes wrong:** User sees ClaudeError::NonZeroExit with internal details, confusing for high-level API
**Why it happens:** Direct #[from] conversion without Display/Debug customization
**How to avoid:** rig-cli errors wrap ProviderError with actionable Display ("Claude execution failed. Check CLI is installed and up to date."), preserve full error chain in Debug for developers.
**Warning signs:** Error messages reference internal types (RunResult, AdapterConfig); stack traces show 5+ frames of internal adapter code; no "what to do" guidance

## Code Examples

Verified patterns from official sources and current codebase:

### Rig Agent Creation (Standard Pattern)
```rust
// Source: https://docs.rig.rs/docs/concepts/agent
// This is what rig-cli MUST match

use rig::providers::openai;

let client = openai::Client::from_env();
let agent = client.agent("gpt-4o")
    .preamble("You are a helpful assistant")
    .build();
let response = agent.prompt("Hello!").await?;
```

### CompletionModel Implementation (Already Exists)
```rust
// Source: /home/pnod/dev/projects/rig-cli/rig-provider/src/adapters/claude.rs
// Example of what's already working - DON'T rebuild

impl CompletionModel for ClaudeModel {
    type Response = RunResult;
    type StreamingResponse = ();
    type Client = ClaudeCli;

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let prompt_str = format_chat_history(&request);
        let config = RunConfig::default();
        let result = self.cli.run(&prompt_str, &config).await?;

        Ok(CompletionResponse {
            choice: OneOrMany::one(AssistantContent::text(result.stdout.clone())),
            usage: Usage::default(),
            raw_response: result,
        })
    }

    async fn stream(...) -> Result<StreamingCompletionResponse<...>, CompletionError> {
        // Already implemented, streams via ReceiverStream
    }
}
```

### McpToolAgent Usage (Internal - Will Be Hidden)
```rust
// Source: /home/pnod/dev/projects/rig-cli/rig-provider/src/mcp_agent.rs
// This is what AgentBuilder.build() will use internally

let result = McpToolAgent::builder()
    .toolset(my_toolset)
    .prompt("Extract data...")
    .adapter(CliAdapter::ClaudeCode)
    .system_prompt("You are an extractor")
    .run()
    .await?;
```

### Error Handling with thiserror (Recommended)
```rust
// Source: Current ProviderError pattern
// https://docs.rs/thiserror

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Claude CLI not found. Install: npm i -g @anthropic-ai/claude-code")]
    ClaudeNotFound,

    #[error("CLI execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Provider error: {0}")]
    Provider(#[from] rig_provider::errors::ProviderError),

    #[error("Rig completion error: {0}")]
    Completion(#[from] rig::completion::CompletionError),
}
```

### Feature-Gated Module (Standard Pattern)
```rust
// Source: Cargo feature best practices
// https://doc.rust-lang.org/cargo/reference/features.html

// In rig-cli/src/lib.rs
#[cfg(feature = "claude")]
pub mod claude;

#[cfg(feature = "codex")]
pub mod codex;

#[cfg(feature = "opencode")]
pub mod opencode;

// Public error types always available (no feature gate)
pub mod errors;

// Prelude only exports what's available based on features
pub mod prelude {
    #[cfg(feature = "claude")]
    pub use crate::claude::Client as ClaudeClient;

    #[cfg(feature = "codex")]
    pub use crate::codex::Client as CodexClient;

    pub use crate::errors::Error;
}
```

### ProviderClient Implementation (Pattern)
```rust
// Source: https://docs.rig.rs/docs/concepts/provider_clients
// Combined with current codebase structure

use rig::client::ProviderClient;

impl ProviderClient for Client {
    type Input = ();

    fn new(_: Self::Input) -> Result<Self, rig::client::Error> {
        // Discovery happens here, cached in Client
        let report = claudecode_adapter::init(None)
            .await
            .map_err(|e| rig::client::Error::ProviderError(e.to_string()))?;

        Ok(Self {
            model: ClaudeModel::new(report.claude_path, report.capabilities),
            config: ClientConfig::default(),
        })
    }

    fn from_env() -> Self {
        Self::new(()).expect("Failed to initialize Claude client")
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Direct CompletionModel usage | Client → Agent builder → CompletionModel | Rig 0.13+ | ProviderClient trait required, agent() method standard |
| Manual tool schema JSON | ToolSet with get_tool_definitions() | Rig 0.16+ | Simplified tool integration, MCP SDK support |
| Custom error types per provider | Unified CompletionError | Rig 0.x | Consistent error handling across providers |
| Separate crates for each provider | Monorepo with feature flags | Rig 0.29 | Single rig-core with optional provider features |
| Streaming via custom iterators | StreamingCompletionResponse | Rig 0.x | Standardized streaming interface |

**Deprecated/outdated:**
- **DynClientBuilder without ProviderClient:** Pre-0.13 pattern, clients must now implement ProviderClient trait
- **Direct JSON schema for tools:** Use ToolSet.get_tool_definitions(), not manual serde_json construction
- **Async build() in providers:** Current providers (OpenAI, Anthropic via Rig) use sync build, async only for execution

## Open Questions

Things that couldn't be fully resolved:

1. **Exact ProviderClient::new() signature for CLI discovery**
   - What we know: ProviderClient trait has new() and from_env() methods
   - What's unclear: Whether new() can be async (CLI discovery requires await)
   - Recommendation: Make discovery happen in lazy fashion on first agent.prompt(), or cache in thread-local/once_cell. Investigate Rig 0.29 source for async trait method support.

2. **Extension methods for extraction metadata**
   - What we know: CompletionResponse returns raw_response with extraction metadata
   - What's unclear: Best pattern for surfacing ExtractionMetrics (attempts, validation errors)
   - Recommendation: Provide extension trait `RigCliResponseExt` with .extraction_metrics() method, gated behind feature flag or always available

3. **CLI-specific builder methods integration**
   - What we know: CLI agents need .payload() for context data (4-block XML format)
   - What's unclear: Whether to add .payload() to AgentBuilder or hide in .prompt() variant
   - Recommendation: Add as builder method matching Rig style: .context(data) to align with existing .context() for documents, OR provide ClaudeAgent::prompt_with_payload(prompt, payload) extension method

4. **Streaming with tool calls**
   - What we know: CLI adapters emit StreamEvent::ToolCall during streaming
   - What's unclear: Whether Rig's StreamingCompletionResponse supports tool call chunks
   - Recommendation: Verify RawStreamingChoice enum supports ToolCall variant (seen in current code, appears correct)

## Sources

### Primary (HIGH confidence)
- [Rig official documentation](https://docs.rig.rs/) - Provider pattern, agent builders, tool integration
- [rig-core 0.29.0 on docs.rs](https://docs.rs/rig-core/0.29.0/rig/) - Trait signatures, types
- Current rig-cli codebase at `/home/pnod/dev/projects/rig-cli/` - Existing CompletionModel implementations, McpToolAgent builder
- [Cargo Features documentation](https://doc.rust-lang.org/cargo/reference/features.html) - Feature flag best practices
- [thiserror documentation](https://docs.rs/thiserror) - Error handling patterns for libraries

### Secondary (MEDIUM confidence)
- [Write Your Own Provider guide](https://docs.rig.rs/guides/extension/write_your_own_provider) - Provider implementation walkthrough
- [Provider Clients concept](https://docs.rig.rs/docs/concepts/provider_clients) - ProviderClient trait usage
- [Rig Completion concept](https://docs.rig.rs/docs/concepts/completion) - CompletionModel requirements
- [Rust workspace best practices (matklad)](https://matklad.github.io/2021/08/22/large-rust-workspaces.html) - Flat layout for 10k-1M LOC
- [Error handling in Rust 2026](https://www.shakacode.com/blog/thiserror-anyhow-or-how-i-handle-errors-in-rust-apps/) - thiserror vs anyhow guidance

### Tertiary (LOW confidence - requires validation)
- WebSearch: "Rust builder pattern sync vs async" - Consensus on sync build, async execution (needs official Rig verification)
- WebSearch: "Rust prelude best practices" - Guidance to use sparingly (general advice, not Rig-specific)
- Community discussions on ProviderClient async methods - Needs verification against Rig 0.29 actual source code

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - rig-core 0.29.0 requirement is absolute, thiserror is Rust standard for libraries
- Architecture: HIGH - Workspace facade pattern matches existing structure, matklad guidance for project size
- Rig trait alignment: MEDIUM-HIGH - CompletionModel implementations exist and work, but ProviderClient async handling needs source verification
- Builder pattern: MEDIUM - Standard Rig pattern clear from docs, but CLI-specific methods (.payload()) integration pattern needs validation
- Error handling: HIGH - thiserror for libraries is established best practice, pattern matches current ProviderError approach
- Feature flags: HIGH - Cargo feature documentation is authoritative, three-adapter pattern is straightforward
- Pitfalls: MEDIUM - Based on general Rust/API design wisdom and Rig patterns, but CLI-specific edge cases may emerge

**Research date:** 2026-02-02
**Valid until:** 30 days (Rig 0.29 is stable, patterns unlikely to change rapidly)
