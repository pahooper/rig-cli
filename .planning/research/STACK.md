# Technology Stack Research

**Project:** rig-cli
**Domain:** Rust Rig 0.29 provider for CLI-based AI coding agents with structured extraction
**Researched:** 2026-02-01
**Confidence:** HIGH

## Executive Summary

This research validates and enhances the existing stack for hardening rig-cli to v1.0 production quality. The core dependencies (rig-core, rmcp, tokio, serde, schemars) are correctly chosen and production-ready. The focus for v1.0 hardening is adding production-grade error handling, observability, bounded concurrency, and testing infrastructure.

**Key finding:** Version discrepancy detected. The codebase uses rig-core 0.29.0 and rmcp 0.14.0, but current crates.io shows rig-core 0.23.1 and rmcp 0.14.0. **Action required:** Verify rig-core version - 0.29.0 may be unreleased/local or the search results are outdated.

## Recommended Stack

### Core Rig Integration (LOCKED - No Changes)

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **rig-core** | 0.29.0 | AI tool and completion model abstraction | Provides `CompletionModel`, `Tool`, `ToolSet` traits that define the idiomatic Rig provider interface. Version 0.29.0 required for API compatibility. **Note:** Crates.io shows 0.23.1 - verify this version exists. |
| **rmcp** | 0.14.0 | MCP server implementation | Official Rust MCP SDK with server/transport-io features. Provides `#[tool]` macro, `ServerHandler`, task lifecycle, and JSON schema support for MCP protocol compliance. Version 0.14.0 confirmed current. |
| **tokio** | 1.0 (latest stable) | Async runtime | Industry-standard async runtime for Rust. Required for subprocess management, bounded channels, task spawning, timeouts. Use "full" features for complete runtime. |

**Confidence:** HIGH (rmcp verified via official docs, tokio is stable standard, rig-core version needs verification)

### Serialization & Schema Validation

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **serde** | 1.0 | Data serialization/deserialization | Rust standard for JSON handling. Use with `derive` feature for struct serialization. |
| **serde_json** | 1.0 | JSON format support | De facto standard for JSON in Rust. Used for parsing CLI output and MCP protocol messages. |
| **schemars** | 1.0+ | JSON Schema generation | Generates JSON schemas from Rust types. Integrates with `#[derive(JsonSchema)]` for MCP tool parameters. Supports validation library attributes (validator, garde). **Use 1.0+ for stability.** |
| **jsonschema** | 0.26+ | JSON schema validation | High-performance validator for runtime validation of agent submissions. Supports pattern validation with configurable regex engine (fancy-regex default, can switch to regex for linear-time safety). **Use 0.26+ for performance improvements.** |

**Confidence:** HIGH (all verified via docs.rs and official documentation)

### Error Handling & Observability (PRODUCTION HARDENING)

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **thiserror** | 1.0 | Library error types | Define clear, matchable error enums for library consumers. Use `#[error]` for Display impl, `#[from]` for From/source. **Best practice for library crates (mcp/, adapters/).** |
| **anyhow** | 1.0 | Application error handling | Simplify error handling in application code (rig-provider/). Provides context chaining, automatic conversion via `?`. **Best practice for application crates.** |
| **tracing** | 0.1 (latest) | Structured logging and spans | Application-level tracing for async/concurrent systems. Use spans to track prompt → agent → validation → retry flow. Async-native, zero-cost abstractions. |
| **tracing-subscriber** | 0.3 (latest) | Tracing backend | Use `env-filter` feature for runtime log level control. Configure JSON output for production, pretty output for development. |
| **tracing-opentelemetry** | 0.28+ (if needed) | OpenTelemetry integration | **Optional for v1.0.** Only add if distributed tracing required. Initialize early, use semantic conventions, flush on shutdown. |

**Confidence:** HIGH (verified via 2026 best practices articles and official docs)

**Pattern:** Use thiserror in `mcp/`, `claudecode-adapter/`, `codex-adapter/`, `opencode-adapter/` for typed errors. Use anyhow in `rig-provider/` for aggregated error reporting.

### Subprocess & Containment

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **tokio::process** | (via tokio 1.0) | Async subprocess spawning | Use `tokio::process::Command` for async CLI spawning. Provides stdout/stderr streaming, timeout support via `tokio::time::timeout`. |
| **tempfile** | 3.10+ | Session sandboxing | Create isolated temporary directories per session. Provides automatic cleanup on drop. Current dependency is correct. |
| **which** | 6.0+ | CLI binary discovery | Locate Claude Code, Codex, OpenCode in PATH. Current dependency is correct. |

**Confidence:** HIGH (standard Rust ecosystem libraries)

**CLI Containment Strategy:**
- **Claude Code:** Use `--strict-mcp-config` + `--mcp-config '{...}'` to limit tools. As of Jan 2026, `--no-mcp` flag doesn't exist; use strict config workaround. MCP Tool Search enabled by default (reduces tokens 85%, ~8.7K vs ~77K). Use `--append-system-prompt` for instruction injection (safest option).
- **Codex:** Research CLI flags for tool containment (not found in search results - needs CLI-specific investigation).
- **OpenCode:** Deprioritized for v1.0 - maintain but don't harden.

**Confidence:** MEDIUM (Claude Code flags verified from Jan 2026 docs, Codex/OpenCode flags need deeper research)

### Testing Infrastructure (NEW FOR V1.0)

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **test-binary** | latest | Integration test subprocess helpers | Returns paths of built test binaries for subprocess tests. Compile test binaries with specific exit codes/output patterns to simulate CLI agents without spawning real processes. |
| **assert_cmd** | 2.0+ | CLI testing assertions | Run rig-provider binary as subprocess in integration tests. Provides fluent assertions for stdout/stderr/exit codes. Standard for CLI testing. |
| **tempfile** | 3.10+ (already present) | Test isolation | Create per-test temp directories for session sandboxing tests. |
| **tokio-test** | 0.4+ | Async test utilities | Test utilities for tokio-based code. Provides `block_on` for tests, mock time, etc. |

**Confidence:** HIGH (standard Rust testing libraries, verified via CLI testing guides)

**Testing Strategy:**
1. **Unit tests:** Test toolkit, handler, adapter logic in isolation with mock trait implementations.
2. **Integration tests (mock):** Use `test-binary` to create fake CLI agents that output controlled JSON/errors. Test extraction workflow end-to-end without real CLI dependencies.
3. **Integration tests (real):** Optional smoke tests with real Claude Code/Codex if available in CI.

### Concurrency & Safety (HARDENING PRIORITIES)

| Technology | Version | Purpose | Rationale |
|------------|---------|---------|-----------|
| **tokio::sync::mpsc::channel** | (via tokio 1.0) | Bounded channels with backpressure | **Replace unbounded channels.** Use bounded capacity (e.g., 32-128) for CLI stdout/validation messages. Provides automatic backpressure when channel fills. **Critical for production reliability.** |
| **tokio::time** | (via tokio 1.0) | Timeouts and delays | Use `timeout()` for bounded retry attempts, CLI response deadlines. Prevents infinite hangs. |
| **tokio_util::sync::CancellationToken** | 0.11+ (via tokio-util) | Graceful task cancellation | **Add if not present.** Coordinate shutdown of spawned tasks (CLI processes, MCP server). Allows cleanup before termination. |

**Confidence:** HIGH (verified via Tokio official docs and 2026 backpressure patterns)

**Backpressure Pattern:**
```rust
// BEFORE (unbounded - dangerous)
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

// AFTER (bounded with backpressure)
let (tx, rx) = tokio::sync::mpsc::channel(64); // Application-specific bound
// When full, tx.send().await yields until space available
```

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **clippy** | Linter | Workspace lints configured: pedantic, nursery, perf, cargo. `unwrap_used`, `expect_used`, `panic` warnings enforced. Good baseline. |
| **rustfmt** | Formatter | Standard Rust formatting. |
| **cargo-deny** | License/security checking | Configured via `deny.toml`. Run in CI. |
| **cargo-audit** | Dependency vulnerability scanning | Run in CI to catch CVEs. |
| **cargo-nextest** | Faster test runner | **Recommended for CI.** Parallel test execution, clean output. |
| **cargo-llvm-cov** | Code coverage | **Optional for v1.0.** Track test coverage for hardening confidence. |

**Confidence:** HIGH (standard Rust development ecosystem)

## Alternatives Considered

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Error handling (library) | thiserror | eyre | eyre is for applications. Use thiserror for library error enums. |
| Error handling (app) | anyhow | eyre | anyhow is more widely adopted, simpler API. eyre adds features (hooks, custom reports) that rig-cli doesn't need. |
| JSON validation | jsonschema | valico | jsonschema is actively maintained, high-performance, supports latest JSON Schema drafts. valico is less active. |
| Subprocess testing | test-binary + assert_cmd | subprocess crate | subprocess is synchronous. test-binary integrates with Cargo test harness. assert_cmd is fluent API standard. |
| Async runtime | tokio | async-std | tokio has larger ecosystem, better maintained, de facto standard for production Rust. |
| Tracing | tracing | log | tracing provides structured spans, async-aware context. log is flat logging only. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `std::process::Command` in async context | Blocks executor. Race conditions with async I/O. | `tokio::process::Command` |
| `tokio::sync::mpsc::unbounded_channel` | No backpressure. Will OOM under load. | `tokio::sync::mpsc::channel(bound)` |
| `unwrap()` / `expect()` in library code | Panics are unrecoverable for library consumers. | `Result<T, E>` with proper error propagation |
| `panic!()` in error paths | Ungraceful failure. Hard to recover. | Return errors, let caller decide recovery strategy |
| `JoinHandle::abort()` without awaiting | Task may not be cancelled yet. Resource leaks. | Await the JoinHandle after abort to ensure completion |
| Unbounded retry loops | Token cost spiral. Infinite hangs. | Bounded attempts (e.g., 3) + exponential backoff |
| High-cardinality tracing fields | Performance hit. Log explosion. | Use semantic conventions, avoid dynamic IDs in spans |

## Version Compatibility Matrix

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| rig-core | 0.29.0 | rmcp 0.14.0 | **Verify 0.29.0 exists on crates.io** (search shows 0.23.1) |
| rmcp | 0.14.0 | tokio 1.0 | Confirmed current version. Uses tokio for async. |
| tokio | 1.0 (latest) | All async crates | Stable 1.x API. Update to latest 1.x patch for security fixes. |
| schemars | 1.0+ | jsonschema 0.26+ | Both support latest JSON Schema drafts. |
| thiserror | 1.0 | anyhow 1.0 | Can use both in same workspace (library vs app pattern). |
| tracing | 0.1 | tokio 1.0 | tracing-subscriber 0.3 required for full functionality. |
| tokio-util | 0.11+ (if using CancellationToken) | tokio 1.0 | Provides utilities beyond core tokio. |

## Installation

### Core Dependencies (Already Present)

```toml
# Workspace Cargo.toml - already configured
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = "warn"
nursery = "warn"
perf = "warn"
cargo = "warn"
unwrap_used = "warn"    # Good
expect_used = "warn"    # Good
panic = "warn"          # Good
todo = "warn"           # Good
```

### Additions for V1.0 Hardening

Add to `rig-provider/Cargo.toml` (application crate):

```toml
[dependencies]
# Error handling
anyhow = "1.0"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Concurrency (if CancellationToken needed)
tokio-util = { version = "0.11", features = ["sync"] }

[dev-dependencies]
# Testing
assert_cmd = "2.0"
tokio-test = "0.4"
```

Add to library crates (`mcp/`, `claudecode-adapter/`, `codex-adapter/`):

```toml
[dependencies]
# Error handling (replace anyhow with thiserror)
thiserror = "1.0"

# Observability
tracing = "0.1"

[dev-dependencies]
# Testing
test-binary = "3"
tempfile = "3.10"  # Already present, ensure in dev-dependencies
```

### CI Additions

```bash
# In CI pipeline
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check licenses
cargo audit
```

### Optional for V1.0

```bash
# Code coverage (optional)
cargo install cargo-llvm-cov
cargo llvm-cov --html

# Faster tests in CI (recommended)
cargo install cargo-nextest
cargo nextest run
```

## Stack Patterns by Use Case

### If building additional CLI adapters:

1. Create new crate in workspace: `yourCLI-adapter/`
2. Use `thiserror` for adapter-specific error enums
3. Implement `CompletionModel` trait from rig-core
4. Use `tokio::process::Command` for subprocess spawning
5. Use bounded `mpsc::channel` for stdout/stderr streaming
6. Add `#[instrument]` tracing to key methods
7. Return `Result<T, YourAdapterError>` from all public methods

### If adding retry/recovery logic:

1. Use `tokio::time::timeout` for bounded wait
2. Use exponential backoff: `tokio::time::sleep(Duration::from_millis(100 * 2^attempt))`
3. Limit max attempts: `for attempt in 0..MAX_RETRIES`
4. Log retry decisions with `tracing::warn!`
5. Return validation errors to agent via MCP error response
6. Track cost metrics: `tracing::info!(tokens_used = ?, attempt = ?, ...)`

### If implementing graceful shutdown:

1. Add `tokio-util` with `sync` feature
2. Create `CancellationToken` at server startup
3. Pass clones to spawned tasks
4. Check `token.is_cancelled()` in task loops
5. On SIGTERM/SIGINT, call `token.cancel()`
6. Await `JoinHandle`s after cancellation
7. Flush tracing subscriber before exit

## Production Readiness Checklist

For v1.0 production hardening, verify:

- [ ] **Error handling:** All `unwrap()`/`expect()` replaced with `?` or proper error handling
- [ ] **Bounded concurrency:** All channels are bounded with application-appropriate capacity
- [ ] **Timeouts:** All CLI interactions have timeout bounds (e.g., 30s-120s)
- [ ] **Retry limits:** Max retry attempts configured (e.g., 3 attempts)
- [ ] **Observability:** Tracing spans cover: prompt sent, agent response, validation, retry decision, success/failure
- [ ] **Task cleanup:** Spawned tasks properly cancelled on timeout/error
- [ ] **Testing:** Mock adapter tests + integration tests with test-binary fake CLIs
- [ ] **CLI flags audited:** Each adapter documents containment flags (Claude Code: `--strict-mcp-config`, Codex: TBD)
- [ ] **Version verification:** Confirm rig-core 0.29.0 availability (search shows 0.23.1)
- [ ] **Resource limits:** Session temp directories cleaned up on Drop
- [ ] **Structured logging:** JSON output for production, env-filter for log levels

## Open Questions & Verification Needed

1. **rig-core version:** Codebase uses 0.29.0, but crates.io shows 0.23.1 as latest. Is 0.29.0 a local/unreleased version? **Action:** Check `Cargo.lock` for source (git dep? local path?).

2. **Codex CLI flags:** Containment flags for Codex not found in search results. **Action:** Review Codex CLI `--help` and documentation for sandbox/tool-restriction flags.

3. **OpenCode CLI flags:** Deprioritized for v1.0 but document any containment flags discovered.

4. **Bounded channel capacity:** What's the right bound for CLI stdout streaming? **Recommendation:** Start with 64-128, tune based on testing. Too small = backpressure delays, too large = memory pressure.

5. **Retry timeout strategy:** What's appropriate timeout per attempt? **Recommendation:** Start with 30s for initial, 60s for retries, 3 max attempts = 150s total worst case.

## Sources

### Context7 & Official Documentation (HIGH confidence)
- [rmcp 0.14.0 documentation](https://docs.rs/rmcp) - Server features, tool macros, task lifecycle verified
- [Tokio channels documentation](https://tokio.rs/tokio/tutorial/channels) - Bounded channel backpressure patterns
- [Tokio graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown) - CancellationToken patterns
- [Schemars documentation](https://graham.cool/schemars/) - JSON Schema generation patterns

### Web Search Verified (MEDIUM-HIGH confidence)
- [Rig GitHub repository](https://github.com/0xPlaygrounds/rig) - Rig framework overview
- [rig-core on crates.io](https://crates.io/crates/rig-core) - Version 0.23.1 shown (conflicts with 0.29.0 in code)
- [rmcp on crates.io](https://crates.io/crates/rmcp) - Current version verification
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) - CLI flags and options
- [Claude Code MCP containment](https://github.com/anthropics/claude-code/issues/20873) - `--no-mcp` flag feature request (not yet available)
- [thiserror/anyhow best practices (2026)](https://github.com/oneuptime/blog/tree/master/posts/2026-01-25-error-types-thiserror-anyhow-rust) - Library vs application patterns
- [Rust tracing structured logging (2026)](https://oneuptime.com/blog/post/2026-01-07-rust-tracing-structured-logs/view) - Production configuration
- [Rust tokio task cancellation patterns](https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/) - Cleanup and graceful shutdown
- [Testing Rust CLI applications](https://rust-cli.github.io/book/tutorial/testing.html) - Subprocess testing strategies
- [test-binary crate](https://docs.rs/test-binary/latest/test_binary/) - Integration test helpers
- [jsonschema crate](https://github.com/Stranger6667/jsonschema) - High-performance validation

### Community Patterns (MEDIUM confidence)
- [Mastering Tokio channels](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260) - Backpressure patterns
- [Rust subprocess containment](https://github.com/ebkalderon/bastille) - Sandboxing libraries
- [Rust error handling guide](https://momori.dev/posts/rust-error-handling-thiserror-anyhow/) - Production patterns

---

*Stack research for: rig-cli v1.0 production hardening*
*Researched: 2026-02-01*
*Confidence: HIGH (with version verification caveat for rig-core 0.29.0)*
