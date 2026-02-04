# Phase 11: Documentation & Examples - Research

**Researched:** 2026-02-03
**Domain:** Rust documentation (rustdoc), examples organization, doc comments
**Confidence:** HIGH

## Summary

Rust has well-established conventions for documentation through rustdoc, with official API guidelines and extensive tooling support. The standard approach is rustdoc-first with comprehensive doc comments on all public items, accompanied by feature-named examples in the `examples/` directory. Documentation tests are first-class citizens that validate code examples.

The user has decided on: (1) both standalone examples and doc tests, (2) comprehensive documentation with Errors sections, (3) README + rustdoc only (no separate guide), and (4) 8 specific user story examples. Research focused on official Rust conventions, documentation structure, common pitfalls, and patterns from well-documented crates like Tokio and Serde.

**Primary recommendation:** Follow official Rust API Guidelines strictly - enable `#![warn(missing_docs)]` crate-wide, document all public items with summary/details/examples structure, use dedicated `# Errors` and `# Panics` sections, place feature-named examples in `examples/` directory, and leverage doc tests for inline validation.

## Standard Stack

The established tools/patterns for Rust documentation:

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| rustdoc | 1.85+ | Documentation generation | Built into Rust toolchain, first-class citizen |
| cargo doc | 1.85+ | Doc generation orchestrator | Official Cargo command, workspace-aware |
| doc comments (`///`, `//!`) | N/A | Markdown in source code | Native Rust syntax, supports CommonMark |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `#![warn(missing_docs)]` | N/A | Enforce doc coverage | Enable at crate root for all public APIs |
| `cargo doc --document-private-items` | N/A | Generate internal docs | For contributor documentation |
| doc test attributes | N/A | Control test behavior | `no_run`, `ignore`, `should_panic` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rustdoc only | mdBook + rustdoc | mdBook for guides - user decided against separate guide |
| `#![doc = include_str!()]` | Separate README + lib docs | User wants separate README, not included in rustdoc |
| cargo-readme | Manual sync | Generates README from docs - opposite direction of need |

**Installation:**
```bash
# Built into Rust toolchain - no installation needed
cargo doc --open
```

## Architecture Patterns

### Recommended Project Structure
```
rig-cli/
├── examples/
│   ├── chat_with_mcp.rs           # Multi-turn conversation
│   ├── one_shot_mcp.rs            # Single prompt
│   ├── agent_mcp.rs               # 3-tool pattern
│   ├── agent_extra_tools.rs       # 3-tool + custom tools
│   ├── multiagent.rs              # Multiple agents coordinating
│   ├── extraction.rs              # Structured data extraction
│   ├── payload_chat.rs            # File content injection
│   ├── mcp_deterministic.rs      # Custom date tool example
│   └── error_handling.rs          # Error scenarios
├── src/
│   ├── lib.rs                     # Crate-level docs (//!)
│   ├── claude.rs                  # Module docs + item docs
│   └── ...
└── README.md                      # Concept → Quick start → Features → Examples
```

### Pattern 1: Crate-Level Documentation
**What:** Top-level overview in `lib.rs` using `//!` comments
**When to use:** Every crate root to establish purpose and learning path
**Example:**
```rust
// Source: Official Rust API Guidelines
//! # rig-cli
//!
//! Turn CLI-based AI agents into idiomatic Rig 0.29 providers.
//!
//! ## Quick Start
//!
//! ```no_run
//! # use rig_cli::prelude::*;
//! let client = rig_cli::claude::Client::new().await?;
//! let agent = client.agent("claude-sonnet-4").build();
//! let response = agent.prompt("Hello!").await?;
//! ```
//!
//! ## Two Execution Paths
//!
//! | Method | When to Use |
//! |--------|-------------|
//! | `client.agent("model")` | Simple prompts, chat |
//! | `client.mcp_agent("model")` | Structured extraction |
```

### Pattern 2: Item Documentation with Standard Sections
**What:** Structured doc comments with summary, details, examples, errors, panics
**When to use:** All public functions, methods, types
**Example:**
```rust
// Source: Rust API Guidelines - https://rust-lang.github.io/api-guidelines/documentation.html
/// Creates a new Claude Code client with auto-discovery.
///
/// Discovers the CLI binary via `$CLAUDE_BIN`, PATH, or standard locations.
/// Uses default configuration (300s timeout, 100 message capacity).
///
/// # Examples
///
/// ```no_run
/// # use rig_cli::claude::Client;
/// let client = Client::new().await?;
/// ```
///
/// # Errors
///
/// Returns `Error::ClaudeNotFound` if the CLI binary cannot be found.
/// Returns `Error::Provider` if initialization fails.
pub async fn new() -> Result<Self, Error> {
    // ...
}
```

### Pattern 3: Standalone Examples with Key Code Sections
**What:** Self-contained examples with full main() + highlighted sections
**When to use:** All feature demonstrations in examples/ directory
**Example:**
```rust
// Source: Cargo documentation - https://doc.rust-lang.org/cargo/guide/project-layout.html
//! Example: Chat with MCP and sessions
//!
//! Demonstrates multi-turn conversation with MCP tool responses.

use rig_cli::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- KEY CODE: Client setup ---
    let client = rig_cli::claude::Client::new().await?;
    let agent = client.mcp_agent("sonnet")
        .toolset(my_tools)
        .build()?;
    // --- END KEY CODE ---

    // Full working example continues...
    Ok(())
}
```

### Pattern 4: Documentation Tests for Validation
**What:** Code examples in doc comments that compile and run as tests
**When to use:** All non-trivial public API surface
**Example:**
```rust
// Source: rustdoc documentation - https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
/// Adds payload data for context injection.
///
/// # Examples
///
/// ```
/// # use rig_cli::claude::Client;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new().await?
///     .with_payload("File content here");
/// # Ok(())
/// # }
/// ```
pub fn with_payload(mut self, data: impl Into<String>) -> Self {
    // ...
}
```

### Pattern 5: README Structure (Concept-First)
**What:** User-facing README separate from rustdoc, concept before code
**When to use:** Project root README.md
**Example:**
```markdown
# rig-cli

Turn CLI-based AI agents into idiomatic Rig 0.29 providers.

## What are CLI Agents?

[Explain concept - what, why MCP matters]

## Quick Start

[Minimal working example]

## Features

| Feature | Description |
|---------|-------------|

## Examples

See `examples/` directory:
- [chat_with_mcp.rs](examples/chat_with_mcp.rs) - Multi-turn conversation
- ...

## Adapter Comparison

| Feature | Claude | Codex | OpenCode |
|---------|--------|-------|----------|
```

### Anti-Patterns to Avoid

- **Including README in rustdoc via `include_str!()`**: User decided against this - README and rustdoc serve different purposes (concept vs API reference)
- **Trivial documentation**: "Gets the value" for a getter - adds no information beyond signature
- **Using `unwrap()` in examples**: Examples should use `?` operator for error handling per API guidelines
- **Missing summary line**: First line should be standalone sentence, reused in search results
- **Implementation details in public docs**: Focus on "what/how" not "why designed this way" unless counter-intuitive

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Extracting README snippets | Custom parser | `cargo test --doc` | Doc tests already validate examples |
| Enforcing doc coverage | Pre-commit script | `#![warn(missing_docs)]` | Compiler-enforced, zero maintenance |
| Generating docs from templates | Custom codegen | `rustdoc` | Official tool, IDE-integrated |
| Cross-referencing items | Manual links | Intra-doc links with backticks | Compiler-validated, refactor-safe |
| Testing doc examples | Manual test files | Doc tests (triple backticks) | Automatic via `cargo test` |

**Key insight:** Rust's documentation tooling is comprehensive and first-class. Leverage it instead of external solutions.

## Common Pitfalls

### Pitfall 1: Missing `# Errors` and `# Panics` Sections
**What goes wrong:** Functions that return `Result<T, E>` or can panic lack documented conditions
**Why it happens:** Developers focus on happy path, forget to document failure modes
**How to avoid:**
- Every function returning `Result<T, E>` needs `# Errors` section listing when `Err` is returned
- Every function that can panic needs `# Panics` section unless panics are from caller-provided implementations (e.g., Display)
**Warning signs:** API guidelines lint passes but users file issues asking "when does this fail?"

### Pitfall 2: Code Examples That Don't Compile
**What goes wrong:** Doc examples become stale as API evolves, breaking user copy-paste
**Why it happens:** Examples aren't tested automatically by default
**How to avoid:**
- Use triple backticks for all examples (compiled by default)
- Use `no_run` for examples that compile but shouldn't execute (I/O, network)
- Use `ignore` only when truly incompatible with test environment
- Run `cargo test` to validate all doc tests
**Warning signs:** GitHub issues with "this example doesn't work"

### Pitfall 3: Mixing README and Rustdoc Content
**What goes wrong:** Using `#![doc = include_str!("README.md")]` makes both worse
**Why it happens:** Desire to avoid duplication
**How to avoid:**
- README: Concept-first, explain "when/why" to use library, marketing-friendly
- Rustdoc: API reference, assume reader chose library, focus on "how"
- Intra-doc links in rustdoc (e.g., `[`Board::default()`]`) don't work in README
**Warning signs:** README has too much API detail or rustdoc lacks conceptual overview

### Pitfall 4: Under-Documenting for "It's Obvious"
**What goes wrong:** Assumptions about what's obvious lead to sparse docs
**Why it happens:** Author familiarity bias - what's obvious to you isn't to users
**How to avoid:**
- Document all public items, period - "rarely does anyone complain about too much documentation"
- Even simple getters benefit from one-line summary for rustdoc tooltips
- Document invariants, edge cases, and relationships between items
**Warning signs:** 265 missing-docs warnings (user's current state)

### Pitfall 5: Non-Copyable Examples
**What goes wrong:** Examples require setup code not shown, can't be copy-pasted
**Why it happens:** Focus on brevity over usability
**How to avoid:**
- Standalone examples in `examples/` must be fully self-contained
- Use `# ` prefix in doc tests to hide boilerplate but keep it compilable
- Every example should work if user copy-pastes exactly as shown
**Warning signs:** User asks "how do I actually run this?"

### Pitfall 6: Ignoring `missing_docs` Lint Warnings
**What goes wrong:** Public API lacks comprehensive coverage
**Why it happens:** Warnings seen as optional, not enforced
**How to avoid:**
- Enable `#![warn(missing_docs)]` at crate root for all public APIs
- Can use `#[allow(missing_docs)]` sparingly for truly internal-only public items
- Treat warnings as errors in CI with `#![deny(missing_docs)]` for critical crates
**Warning signs:** ~265 missing-docs warnings (user's current count)

## Code Examples

Verified patterns from official sources:

### Crate-Level Doc Comment Structure
```rust
// Source: Rust API Guidelines - https://rust-lang.github.io/api-guidelines/documentation.html
//! # crate-name
//!
//! [One-sentence summary of the crate's purpose]
//!
//! [More detailed explanation, key concepts]
//!
//! ## Quick Start
//!
//! ```no_run
//! // Minimal working example
//! ```
//!
//! ## Feature Overview
//!
//! | Feature | Description |
//! |---------|-------------|
```

### Function with All Sections
```rust
// Source: Rust API Guidelines
/// Creates a new client with custom configuration.
///
/// Allows overriding defaults for timeout, channel capacity, and binary path.
/// For most users, [`Client::new()`] with auto-discovery is recommended.
///
/// # Examples
///
/// ```no_run
/// # use rig_cli::claude::Client;
/// # use std::time::Duration;
/// let config = ClientConfig::default()
///     .with_timeout(Duration::from_secs(600));
/// let client = Client::with_config(config).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns `Error::ClaudeNotFound` if the specified binary path is invalid.
/// Returns `Error::InvalidConfig` if timeout is zero or capacity exceeds u16::MAX.
///
/// # Panics
///
/// Panics if the configuration's `binary_path` contains invalid UTF-8.
pub async fn with_config(config: ClientConfig) -> Result<Self, Error> {
    // ...
}
```

### Standalone Example Template
```rust
// Source: Cargo documentation - https://doc.rust-lang.org/cargo/guide/project-layout.html
//! Example: Agent with MCP and extra tools
//!
//! Demonstrates the 3-tool pattern (example/validate/submit) plus
//! additional custom tools for date extraction.

use rig_cli::prelude::*;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

// --- TOOL DEFINITION ---
#[derive(Deserialize)]
struct DateExtractorArgs {
    text: String,
}

#[derive(Serialize)]
struct DateExtractorOutput {
    dates: Vec<String>,
}

#[derive(Tool)]
#[tool(
    name = "extract_dates",
    description = "Extracts dates from text using regex"
)]
struct DateExtractor;

impl DateExtractor {
    async fn call(args: DateExtractorArgs) -> Result<DateExtractorOutput, Error> {
        // Implementation here
    }
}
// --- END TOOL DEFINITION ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- KEY CODE: Setup with extra tools ---
    let client = rig_cli::claude::Client::new().await?;

    let tools = ToolSet::new()
        .with_tool(DateExtractor);

    let agent = client.mcp_agent("sonnet")
        .toolset(tools)
        .preamble("Extract dates and structured data")
        .build()?;
    // --- END KEY CODE ---

    let response = agent.prompt("Find all dates in: Jan 1 2026, Feb 3").await?;
    println!("Extracted: {:?}", response);

    Ok(())
}
```

### Doc Test with Hidden Setup
```rust
// Source: rustdoc documentation
/// Builds the agent with the configured parameters.
///
/// # Examples
///
/// ```
/// # use rig_cli::claude::Client;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let client = Client::new().await?;
/// let agent = client.agent("claude-sonnet-4")
///     .preamble("You are a helpful assistant")
///     .temperature(0.7)
///     .build();
/// # Ok(())
/// # }
/// ```
pub fn build(self) -> Agent {
    // ...
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Separate docs site (mdBook) | Rustdoc-first with optional mdBook | ~2018-2020 | Rustdoc improvements made it sufficient for most crates |
| Manual example testing | Doc tests via `cargo test` | Since Rust 1.0 | Examples validated automatically, always current |
| `try!()` macro in examples | `?` operator | Rust 1.13 (2016) | API guidelines now mandate `?` in examples |
| Plain markdown links | Intra-doc links with backticks | Rust 1.48 (2020) | Compiler-validated cross-references |
| `#[doc(hidden)]` for internal items | `pub(crate)` visibility | Rust 1.18 (2017) | Cleaner API surface, better encapsulation |

**Deprecated/outdated:**
- `try!()` macro in examples: Use `?` operator per API guidelines
- `cargo-readme` for docs → README: User wants README separate, not generated
- Including README in rustdoc: Creates tension between marketing (README) and reference (rustdoc)

## Open Questions

Things that couldn't be fully resolved:

1. **Exact example file names**
   - What we know: User wants feature-named (e.g., extraction.rs, mcp_agent.rs), 8 specific user stories
   - What's unclear: Naming convention for combined scenarios (e.g., "MCP Agent + deterministic tool" - is it `mcp_deterministic.rs` or `agent_custom_tool.rs`?)
   - Recommendation: Use planner discretion per CONTEXT.md, prioritize clarity over brevity

2. **Internal module documentation depth**
   - What we know: User wants "comprehensive internals documentation" for contributors
   - What's unclear: Whether to use `#![warn(missing_docs)]` on private items or just `--document-private-items`
   - Recommendation: Enable lint only on public items, use `cargo doc --document-private-items` for contributor builds

3. **Doc example vs entry-point-only APIs**
   - What we know: User wants selective examples - non-obvious APIs get examples, trivial getters don't
   - What's unclear: Where's the threshold? (e.g., does `with_payload()` need inline example or just in Quick Start?)
   - Recommendation: Planner should add examples to: builders, entry points, non-obvious methods, anything with edge cases

## Sources

### Primary (HIGH confidence)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html) - Official documentation standards
- [rustdoc book - How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html) - Official writing guide
- [Cargo Book - Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) - Examples directory convention

### Secondary (MEDIUM confidence)
- [Tangram Vision - Making Great Docs with Rustdoc](https://www.tangramvision.com/blog/making-great-docs-with-rustdoc) - Practical patterns (2024)
- [Tokio documentation](https://docs.rs/tokio) - Well-documented crate example
- [Serde documentation](https://docs.rs/serde) - API reference patterns

### Tertiary (LOW confidence - marked for validation)
- [Leapcell - 9 Rust Pitfalls](https://leapcell.io/blog/nine-rust-pitfalls) - Common mistakes (July 2025)
- [Sherlock - Rust Security Guide 2026](https://sherlock.xyz/post/rust-security-auditing-guide-2026) - Documentation gaps in security context

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Official Rust toolchain, stable since 1.0
- Architecture: HIGH - Verified with official API guidelines and Cargo documentation
- Pitfalls: HIGH - Cross-referenced with official lints and community patterns
- Examples: HIGH - Sourced from official Rust documentation and established crates

**Research date:** 2026-02-03
**Valid until:** 2026-05-03 (90 days - Rust documentation conventions are stable)
