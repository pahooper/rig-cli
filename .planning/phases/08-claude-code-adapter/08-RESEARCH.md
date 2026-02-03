# Phase 8: Claude Code Adapter - Research

**Researched:** 2026-02-03
**Domain:** Production-hardening Rust CLI subprocess adapter
**Confidence:** HIGH

## Summary

Phase 8 production-hardens the Claude Code adapter by making containment and extraction features reliable, eliminating clippy pedantic warnings, and testing CLI flag combinations. The research focused on understanding Rust production code standards (clippy pedantic approach, documentation requirements), E2E testing patterns for subprocess CLIs, graceful shutdown implementations, and CLI flag behavior.

The current codebase already has strong foundations: bounded channel architecture (100 msg capacity, 10MB limit, 5s grace period), graceful SIGTERM→SIGKILL shutdown on Unix, extraction retry loop with full attempt history, and 6 existing unit tests verifying CLI arg generation. The adapter is functionally complete but needs production polish: ~154 clippy pedantic warnings workspace-wide, no E2E tests validating containment holds with real CLI, incomplete CLI flag documentation, and missing comprehensive extraction failure tests.

**Primary recommendation:** Fix clippy warnings incrementally (missing docs workspace-wide, format string inlining, unwrap elimination), add E2E tests marked `#[ignore]` requiring local Claude CLI, document CLI flag combinations inline and at module level, and test extraction edge cases (timeout mid-stream, partial JSON, max retries exhausted).

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | Async runtime for subprocess I/O | Industry standard for async Rust, used throughout adapter crates |
| clippy | 1.92.0+ | Linting and code quality | Built into Rust toolchain, pedantic group production-ready |
| jsonschema | Latest | JSON Schema validation | High-performance validator, used in extraction orchestrator (Phase 2) |
| assert_cmd | Latest | CLI subprocess testing | De facto standard for Rust CLI E2E tests |
| thiserror | Latest | Error type derivation | Standard for Rust library error types |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| nix | 0.29+ | Unix signal handling | Platform-specific (Unix only), for SIGTERM/SIGKILL |
| which | Latest | PATH-based binary discovery | Cross-platform executable resolution |
| dirs | Latest | Cross-platform home directory | Used in discovery fallback locations |
| tracing | Latest | Structured observability | Security-first logging (skip_all, no prompt content) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| jsonschema | serde_valid | serde_valid integrates validation with deserialization but less flexible for retry feedback |
| assert_cmd | subprocess_test | subprocess_test has advanced features but assert_cmd is simpler and more widely used |
| nix | libc | nix provides safe wrappers vs raw libc bindings |

**Installation:**
```bash
# All dependencies already in Cargo.toml
# For E2E tests, requires Claude CLI installed:
npm install -g @anthropic-ai/claude-code
```

## Architecture Patterns

### Test Organization Pattern
**Unit tests:** Inline with `#[cfg(test)]` module per file for private API access
**Integration tests:** `tests/` directory for public API, marked `#[ignore]` for E2E requiring external CLI

```
claudecode-adapter/
├── src/
│   ├── cmd.rs           # CLI arg builder
│   │   └── #[cfg(test)] mod tests { ... }  // Unit tests for arg combinations
│   ├── process.rs       # Subprocess execution
│   └── lib.rs          # Public API
└── tests/
    └── e2e_containment.rs  // #[ignore] tests requiring real `claude` binary
```

**Why:** Unit tests validate logic without external dependencies, E2E tests validate behavior with real CLI. `#[ignore]` prevents CI failures when CLI not installed, explicit `cargo test -- --ignored` runs E2E locally.

### Graceful Shutdown Pattern (SIGTERM → SIGKILL)
**What:** Unix processes receive SIGTERM for graceful shutdown, escalate to SIGKILL after grace period if unresponsive.

**Implementation:**
```rust
// Source: claudecode-adapter/src/process.rs (existing implementation)
#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let nix_pid = Pid::from_raw(pid.cast_signed());
    // Step 1: Send SIGTERM for graceful exit
    signal::kill(nix_pid, Signal::SIGTERM)?;

    // Step 2: Wait up to GRACE_PERIOD (5 seconds)
    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(status)) => Ok(status),
        // Step 3: Force kill with SIGKILL if timeout
        Err(_) => {
            child.kill().await?;
            child.wait().await
        }
    }
}

#[cfg(windows)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    _pid: u32,
) -> Result<std::process::ExitStatus, ClaudeError> {
    // Windows: no graceful mechanism, immediate TerminateProcess
    child.kill().await?;
    child.wait().await
}
```

**Why:** Gives processes time to clean up resources (close files, flush buffers) before force termination. 5-second grace period balances responsiveness with cleanup time. Windows documented limitation (no SIGTERM equivalent for console processes).

### CLI Flag Documentation Pattern
**Location strategy:** Both inline comments (context for reader) and module-level reference doc (discoverability).

**Example:**
```rust
// cmd.rs module-level doc
//! ## Flag Combinations and Compatibility
//!
//! ### Containment Flags (Added: CLI v0.45.0+)
//! - `--tools ""`: Disables all builtin tools
//! - `--allowed-tools`: Explicit allowlist (requires MCP tools)
//! - `--disable-slash-commands`: Disables interactive commands
//! - `--strict-mcp-config`: Ignores global MCP config (known issue: doesn't override disabledMcpServers)
//!
//! ### Invalid Combinations
//! - `--tools default` + `--tools ""`: Last flag wins (undefined which)
//! - `--allowed-tools` without MCP tools: No tools available (empty set)
//!
//! ### Version Notes
//! - v0.45.0: Added `--strict-mcp-config` flag
//! - v0.91.0 (Codex): Removed `--ask-for-approval` flag
//!
//! **Official docs:** https://code.claude.com/docs/en/cli-reference

// Inline context for flag usage
if mcp.strict {
    // --strict-mcp-config: Only use MCP servers from --mcp-config,
    // ignoring all other MCP configurations.
    // Known limitation: doesn't override disabledMcpServers in ~/.claude.json
    // See: https://github.com/anthropics/claude-code/issues/14490
    args.push(OsString::from("--strict-mcp-config"));
}
```

**Why:** Module docs provide overview and discoverability, inline comments provide context at usage site. Version tracking helps with compatibility testing across CLI versions. External refs link to canonical source.

### Extraction Retry Testing Pattern
**What:** Comprehensive tests for all extraction failure modes from Phase 2.

**Test cases:**
```rust
#[tokio::test]
async fn extraction_invalid_json_retries() {
    // Agent returns non-JSON on attempt 1, valid JSON on attempt 2
    // Assert: ExtractionMetrics shows 2 attempts, result is Ok
}

#[tokio::test]
async fn extraction_timeout_mid_stream() {
    // Agent times out during response streaming
    // Assert: ClaudeError::Timeout with partial output captured
}

#[tokio::test]
async fn extraction_schema_violation_retries() {
    // Agent returns JSON with missing required field on attempt 1-3
    // Assert: ExtractionError::MaxRetriesExceeded with all 3 attempts in history
}

#[tokio::test]
async fn extraction_partial_json_parse_failure() {
    // Agent output cuts off mid-JSON (incomplete bracket)
    // Assert: ExtractionError::ParseError on final attempt
}

#[tokio::test]
async fn extraction_max_retries_history_complete() {
    // Exhaust all retries, verify AttemptRecord includes all fields
    // Assert: history.len() == max_attempts, each record has submission + errors + raw output
}
```

**Why:** Extraction failures are high-value test cases — these are the scenarios where retry loop must work correctly. Tests validate error types match design from Phase 2 (ExtractionError::MaxRetriesExceeded with complete history).

### Clippy Pedantic Workflow
**Strategy:** Enable pedantic workspace-wide, fix incrementally by category, use `#[allow]` sparingly with justification.

**Fix priority order:**
1. Missing docs (workspace debt, ~9 crate-level + public items)
2. Format string inlining (mechanical, ~28 occurrences)
3. Unwrap elimination (safety, 11 occurrences)
4. Const fn candidates (optimization, 9 occurrences)
5. Stylistic nits (if/let, map_or, etc.)

**Allow pattern:**
```rust
// Justified allow for test code assertion formatting
#[allow(clippy::uninlined_format_args)]
#[test]
fn test_complex_assertion() {
    // Complex assertion where format args would reduce readability
    assert!(condition, "Expected {expected} but got {actual}", expected = x, actual = y);
}
```

**Why:** Pedantic is production-ready but aggressive (false positives expected). Fix root causes first, allow only when warning doesn't apply. Workspace-wide policy enforces consistency.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON Schema validation | Custom validator parsing schema | jsonschema crate | Handles all JSON Schema draft versions, complex validation rules (pattern, format, dependencies), detailed error paths |
| CLI subprocess testing | Manual Command::spawn + assertions | assert_cmd crate | Handles stdout/stderr capture, exit code assertions, timeout handling, predicate matching |
| Graceful shutdown | Manual SIGTERM + sleep loop | Existing graceful_shutdown fn | Already handles Unix/Windows differences, timeout logic, error cases |
| Retry with backoff | Custom loop with delay | Existing ExtractionOrchestrator | Already tracks metrics, attempt history, validation feedback, immediate retry (no backoff needed) |
| Structured logging | println! with JSON | tracing crate with instrument | Already integrated (Phase 5), structured fields, security-first (skip_all), machine-parseable |

**Key insight:** Subprocess management and validation are complex domains with edge cases (zombie processes, signal handling, schema format variants). Existing crates handle these edge cases; custom solutions introduce bugs.

## Common Pitfalls

### Pitfall 1: Clippy Pedantic False Positives in Test Code
**What goes wrong:** Test assertions with complex error messages trigger `uninlined_format_args` warnings, reducing readability when "fixed."

**Why it happens:** Clippy pedantic optimizes for runtime performance and consistency, but test code prioritizes readability and explicit variable names in assertion messages.

**How to avoid:**
- Use `#[allow(clippy::uninlined_format_args)]` on test functions where format args reduce clarity
- Add comment explaining why allow is justified: "Test assertion readability over format optimization"
- Still fix warnings in production code paths

**Warning signs:**
- Test diff becomes harder to read after "fixing" clippy warning
- Variable names in assertion messages get lost in inline formatting

### Pitfall 2: E2E Tests Failing in CI Without Claude CLI
**What goes wrong:** Integration tests requiring real `claude` binary fail in CI/CD environments where CLI isn't installed.

**Why it happens:** E2E tests validate adapter behavior against real CLI but CI environments don't have Claude CLI pre-installed (requires npm + authentication).

**How to avoid:**
- Mark E2E tests with `#[ignore]` attribute
- Document requirement in test module docstring: "Requires `claude` CLI installed: npm install -g @anthropic-ai/claude-code"
- Run E2E tests explicitly: `cargo test -- --ignored`
- Keep E2E tests separate from unit tests (tests/ directory vs inline #[cfg(test)])

**Warning signs:**
- CI fails with "claude not found" errors
- Tests pass locally but fail in clean environments
- Test failures in environments without npm/node

### Pitfall 3: Missing Docs Warnings from Workspace Inheritance
**What goes wrong:** Workspace-level `missing_docs = "warn"` applies to all crates, but some adapter crates have minimal public APIs and incomplete docs, generating ~9 crate-level warnings.

**Why it happens:** Workspace lint inheritance (added Phase 6) applies uniformly across all crates regardless of maturity. Adapter crates were built incrementally across phases.

**How to avoid:**
- Add crate-level doc comments to all lib.rs files: `//! Brief description of adapter purpose`
- Document all public items (pub fn, pub struct, pub enum)
- Use clippy fix suggestions: `cargo clippy --fix --allow-dirty`
- Phase 8 scope includes workspace-wide doc fixes (not just claudecode-adapter)

**Warning signs:**
- 9 "missing documentation for the crate" warnings
- Public functions without `///` doc comments
- Cargo doc build warnings about undocumented items

### Pitfall 4: CLI Flag Combinations Not Tested
**What goes wrong:** Adapter generates valid CLI args in isolation but combinations may conflict (last flag wins, empty tool set, etc.).

**Why it happens:** Unit tests verify individual flags but not flag interactions. Claude CLI flag behavior isn't fully documented (some combinations undefined).

**How to avoid:**
- Test containment flag combinations: `--tools ""` + `--allowed-tools` + `--disable-slash-commands` + `--strict-mcp-config`
- Test conflicting flags: multiple `--tools` flags (undefined which wins)
- Test edge cases: `--allowed-tools` without MCP config (no tools available)
- Document known issues inline: "Known limitation: --strict-mcp-config doesn't override disabledMcpServers"

**Warning signs:**
- Containment features fail in production despite passing unit tests
- CLI rejects flag combination with unclear error
- Adapter generates args that work individually but fail together

### Pitfall 5: Unwrap in Error Paths
**What goes wrong:** Using `.unwrap()` or `.expect()` in error handling paths causes panics instead of returning typed errors.

**Why it happens:** Quick prototyping leaves unwraps in place, clippy pedantic warns but doesn't block compilation.

**How to avoid:**
- Replace `.unwrap()` with `?` operator in functions returning Result
- Use `.unwrap_or_default()` for safe fallback values
- Use `.ok_or_else()` to convert Option to Result with descriptive error
- Reserve `.expect()` for truly unreachable cases with detailed panic message

**Warning signs:**
- Clippy warns: "used `unwrap()` on a `Result` value" (7 occurrences)
- Clippy warns: "used `unwrap()` on an `Option` value" (4 occurrences)
- Panics in error scenarios instead of graceful error propagation

## Code Examples

Verified patterns from official sources:

### E2E Test with #[ignore] Attribute
```rust
// Source: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
// https://www.slingacademy.com/article/controlling-test-execution-with-ignore-in-rust/

/// E2E test validating containment flags with real Claude CLI.
///
/// **Requires:** `claude` CLI installed locally
/// **Install:** npm install -g @anthropic-ai/claude-code
/// **Run:** cargo test -- --ignored
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_containment_mcp_only() {
    use assert_cmd::Command;

    // Verify claude binary exists
    let cli = ClaudeCli::new(/* ... */);

    // Run with containment flags
    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::None,
            allowed: Some(vec!["mcp__test__tool".to_string()]),
            ..Default::default()
        },
        mcp: Some(McpPolicy {
            configs: vec!["test-mcp.json".to_string()],
            strict: true,
        }),
        ..Default::default()
    };

    let result = cli.run("echo 'test'", &config).await.unwrap();

    // Assert builtin tools weren't used (validate containment held)
    assert!(!result.stdout.contains("Bash"));
    assert!(!result.stderr.contains("Read"));
}
```

### Clippy Fix Pattern for Missing Docs
```rust
// Source: Workspace Cargo.toml + https://doc.rust-lang.org/clippy/usage.html

// BEFORE (triggers missing_docs warning):
pub fn discover_claude(explicit_path: Option<PathBuf>) -> Result<PathBuf, ClaudeError> {
    // ...
}

// AFTER (satisfies clippy):
/// Locates the Claude CLI executable.
///
/// Resolution order:
/// 1. `explicit_path` if provided and the file exists.
/// 2. The path in the `CC_ADAPTER_CLAUDE_BIN` environment variable.
/// 3. `claude` resolved via `$PATH`.
/// 4. Common install location fallbacks (platform-specific).
/// 5. Helpful error with install instructions.
///
/// # Errors
///
/// Returns `ClaudeError::ExecutableNotFound` when no valid executable can be
/// located.
pub fn discover_claude(explicit_path: Option<PathBuf>) -> Result<PathBuf, ClaudeError> {
    // ...
}
```

### CLI Flag Combination Test
```rust
// Source: claudecode-adapter/src/cmd.rs (existing pattern extended)

#[test]
fn test_containment_flag_combination() {
    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::None,
            allowed: Some(vec!["mcp__rig__submit".to_string()]),
            disallowed: None,
            disable_slash_commands: true,
        },
        mcp: Some(McpPolicy {
            configs: vec!["mcp.json".to_string()],
            strict: true,
        }),
        ..Default::default()
    };

    let args = build_args("test prompt", &config);
    let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

    // Verify all containment flags present
    assert!(args_str.windows(2).any(|w| w[0] == "--tools" && w[1] == ""));
    assert!(args_str.windows(2).any(|w| w[0] == "--allowed-tools" && w[1] == "mcp__rig__submit"));
    assert!(args_str.contains(&"--disable-slash-commands"));
    assert!(args_str.contains(&"--strict-mcp-config"));

    // Verify flag order (tools restriction before allowlist)
    let tools_idx = args_str.iter().position(|&s| s == "--tools").unwrap();
    let allowed_idx = args_str.iter().position(|&s| s == "--allowed-tools").unwrap();
    assert!(tools_idx < allowed_idx, "Tools flag must come before allowed-tools");
}
```

### Extraction Failure Test Pattern
```rust
// Source: mcp/src/extraction/orchestrator.rs (extend existing tests)

#[tokio::test]
async fn extraction_max_retries_includes_all_attempts() {
    use serde_json::json;

    let schema = json!({
        "type": "object",
        "properties": { "id": { "type": "string" } },
        "required": ["id"]
    });

    let orchestrator = ExtractionOrchestrator::new(schema)
        .max_attempts(3);

    let mut attempt = 0;
    let agent_fn = |_prompt: String| async move {
        attempt += 1;
        // Always return invalid JSON (missing required field)
        Ok(r#"{"value": 123}"#.to_string())
    };

    let result = orchestrator.extract(agent_fn, "initial prompt".to_string()).await;

    match result {
        Err(ExtractionError::MaxRetriesExceeded {
            attempts,
            history,
            metrics,
            ..
        }) => {
            assert_eq!(attempts, 3);
            assert_eq!(history.len(), 3);
            // Verify each attempt has complete record
            for (i, record) in history.iter().enumerate() {
                assert_eq!(record.attempt_number, i + 1);
                assert!(!record.validation_errors.is_empty());
                assert!(!record.raw_agent_output.is_empty());
            }
            assert!(metrics.total_attempts == 3);
        }
        _ => panic!("Expected MaxRetriesExceeded error"),
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[allow(clippy::pedantic)]` crate-wide | Fix root causes incrementally | Phase 8 | Workspace adheres to production code standards |
| No E2E containment tests | E2E tests with real CLI marked `#[ignore]` | Phase 8 | Validates containment actually works vs just generating args |
| Minimal CLI flag docs | Inline + module-level flag documentation | Phase 8 | Developers understand flag combinations and limitations |
| Basic extraction error test | Comprehensive failure mode coverage | Phase 8 | All retry/timeout/validation edge cases verified |
| Windows immediate kill | Documented platform limitation | Phase 6 | No false expectation of graceful shutdown on Windows |

**Deprecated/outdated:**
- Codex `--ask-for-approval` flag: Removed in CLI v0.91.0, ApprovalPolicy enum removed from codex-adapter
- Global `#[allow(clippy::pedantic)]`: Replaced with workspace-level warn + selective allows

## Open Questions

Things that couldn't be fully resolved:

1. **--strict-mcp-config behavior with disabledMcpServers**
   - What we know: GitHub issue #14490 reports flag doesn't override disabledMcpServers in ~/.claude.json
   - What's unclear: Whether this is intended behavior or bug, if fix is planned
   - Recommendation: Document known limitation inline, test actual behavior in E2E, don't rely on strict isolation

2. **Multiple --tools flags behavior**
   - What we know: Official docs show `--tools ""` (disable all) and `--tools "Bash,Edit,Read"` (explicit list)
   - What's unclear: What happens if `--tools` specified multiple times (last wins? merge? error?)
   - Recommendation: Test empirically in E2E, document observed behavior, avoid multiple tools flags in adapter

3. **Claude CLI minimum version for containment flags**
   - What we know: --strict-mcp-config added in ~v0.45.0, --disable-slash-commands present in current docs
   - What's unclear: Exact version requirements for each containment flag, backward compatibility
   - Recommendation: Document version requirements in module-level docs, link to official changelog

4. **Extraction timeout during streaming**
   - What we know: Timeout triggers graceful_shutdown, partial output captured
   - What's unclear: Whether partial JSON in stdout should attempt parse or immediate error
   - Recommendation: Test both scenarios (timeout with valid partial JSON vs invalid), document behavior

## Sources

### Primary (HIGH confidence)
- Official Claude Code CLI Reference: https://code.claude.com/docs/en/cli-reference (flag documentation)
- Rust Book - Test Organization: https://doc.rust-lang.org/book/ch11-03-test-organization.html
- Tokio Graceful Shutdown Guide: https://tokio.rs/tokio/topics/shutdown
- Rust CLI Book - Testing: https://rust-cli.github.io/book/tutorial/testing.html
- jsonschema crate docs: https://docs.rs/jsonschema (validation API)
- Existing codebase: claudecode-adapter, mcp/extraction modules (Phase 1-7 implementations)

### Secondary (MEDIUM confidence)
- Clippy Pedantic Best Practices: https://moldstud.com/articles/p-enhance-your-rust-coding-skills-how-clippy-can-help-you-write-idiomatic-rust-code
- Effective Rust - Item 29 Clippy: https://effective-rust.com/clippy.html
- Rust Compiler Dev Guide - CLI arguments: https://rustc-dev-guide.rust-lang.org/cli.html
- Sling Academy - Rust E2E Testing: https://www.slingacademy.com/article/approaches-for-end-to-end-testing-in-rust-cli-applications/
- Sling Academy - #[ignore] Attribute: https://www.slingacademy.com/article/controlling-test-execution-with-ignore-in-rust/

### Tertiary (LOW confidence - flag for validation)
- Claude MCP Config Bug Article: https://www.petegypps.uk/blog/claude-code-mcp-configuration-bug-documentation-error-november-2025 (--strict-mcp-config issue)
- GitHub Issue #14490: https://github.com/anthropics/claude-code/issues/14490 (disabledMcpServers limitation)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Libraries verified in existing codebase (tokio, jsonschema, tracing all in use)
- Architecture: HIGH - Patterns documented in official Rust/Tokio docs, existing graceful_shutdown implementation verified
- Pitfalls: HIGH - Identified from current clippy output (154 warnings counted), existing codebase patterns
- CLI flags: MEDIUM - Official docs verified but some edge cases (multiple flags, version requirements) unclear
- E2E testing: MEDIUM - Pattern well-documented but specific containment validation approach requires testing

**Research date:** 2026-02-03
**Valid until:** 30 days (stable domain - Rust stdlib/ecosystem changes slowly, Claude CLI flags may evolve)
