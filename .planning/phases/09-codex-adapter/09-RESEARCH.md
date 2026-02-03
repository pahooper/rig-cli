# Phase 9: Codex Adapter - Research

**Researched:** 2026-02-03
**Domain:** CLI adapter production hardening (Rust subprocess wrapper)
**Confidence:** HIGH

## Summary

Phase 9 brings Codex adapter to production-ready status matching Claude Code adapter quality. The research confirms Codex CLI has all necessary containment and approval mechanisms available, though with different flag semantics than Claude Code. Key findings:

1. **Containment model differs**: Codex uses sandbox + approval policies vs Claude Code's tool-based containment
2. **Response format is simpler**: Codex `StreamEvent` enum has only 3 variants (Text/Error/Unknown) vs Claude Code's 5 (includes ToolCall/ToolResult)
3. **MCP sandbox bypass (#4152) is confirmed**: Known external issue affecting all Codex versions, must be documented
4. **Clippy pedantic passes**: codex-adapter already passes pedantic checks (verified)

**Primary recommendation:** Mirror Claude Code adapter structure exactly - same cmd.rs documentation format, same E2E test patterns, same clippy pedantic standards. Codex and Claude Code are peers, not alternatives.

## Standard Stack

The established libraries/tools for CLI subprocess adapters:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.0 | Async runtime + process management | Industry standard for Rust async I/O, required for subprocess streaming |
| serde/serde_json | 1.0 | JSON parsing for JSONL streams | Universal Rust serialization, zero-copy streaming support |
| thiserror | 1.0 | Error type derivation | Idiomatic Rust error handling with Display impl |
| nix | 0.29 | Unix signal handling (SIGTERM/SIGKILL) | Low-level Unix syscall access for graceful shutdown |
| which | 6.0 | Binary discovery in PATH | Cross-platform executable resolution |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3.0 | Temporary directories for E2E tests | Required for isolated containment testing |
| jsonschema | 0.18+ | Schema validation (orchestrator) | Extraction feature only, not adapter-level |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio | async-std | Tokio has better ecosystem for subprocess (tokio::process) and is workspace standard |
| nix | libc | nix provides safer Rust wrappers for signals; libc is too low-level |
| which | manual PATH parsing | which handles cross-platform PATH semantics correctly |

**Installation:**
```bash
# Already in codex-adapter/Cargo.toml
cargo add tokio --features full,process
cargo add serde --features derive
cargo add serde_json thiserror anyhow which dirs
cargo add nix --features signal --target 'cfg(unix)'
cargo add tempfile --dev
```

## Architecture Patterns

### Recommended Project Structure
```
codex-adapter/
├── src/
│   ├── lib.rs           # Public API (CodexCli struct)
│   ├── cmd.rs           # CLI flag documentation + build_args()
│   ├── types.rs         # Config/Result/StreamEvent types
│   ├── process.rs       # Subprocess execution with streaming
│   ├── discovery.rs     # Binary discovery (PATH + env var)
│   └── error.rs         # CodexError enum with thiserror
├── tests/
│   └── e2e_containment.rs  # #[ignore] tests requiring real CLI
└── Cargo.toml
```

### Pattern 1: CLI Flag Documentation in cmd.rs
**What:** Module-level rustdoc with sections for Flag Reference, Combinations, Version Notes, Known Limitations
**When to use:** All CLI adapters (parity requirement)
**Example:**
```rust
//! Command-line argument builder for Codex CLI invocations.
//!
//! ## Flag Reference
//!
//! ### Containment Flags
//! - `-s, --sandbox <mode>`: Filesystem isolation (read-only | workspace-write | danger-full-access)
//! - `-a, --ask-for-approval <policy>`: Approval gating (untrusted | on-failure | on-request | never)
//! - `--full-auto`: Convenience alias (-a on-request, --sandbox workspace-write)
//!
//! ### Working Directory Flags
//! - `-C, --cd <dir>`: Set working directory
//! - `--add-dir <dir>`: Additional writable directories (repeatable)
//! - `--skip-git-repo-check`: Allow non-git directories
//!
//! ## Flag Combinations and Compatibility
//!
//! ### Valid Containment Combinations
//! | Combination | Effect |
//! |-------------|--------|
//! | `--sandbox read-only` | Landlock enforces read-only filesystem access |
//! | `--ask-for-approval untrusted` | Only known-safe commands auto-run |
//! | `--sandbox read-only -a untrusted` | Maximum containment (both layers) |
//!
//! ### Invalid/Conflict Combinations
//! | Combination | Issue |
//! |-------------|-------|
//! | `--full-auto` + `--sandbox read-only` | full-auto overrides to workspace-write |
//! | `--full-auto` + `-a untrusted` | full-auto overrides to on-request |
//!
//! ## Known Limitations
//! - MCP tools bypass Landlock sandbox restrictions (Codex Issue #4152)
//!   ([GitHub #4152](https://github.com/openai/codex/issues/4152))
//! - `--dangerously-bypass-approvals-and-sandbox` disables ALL containment
//!
//! ## External References
//! - [Codex CLI Reference](https://developers.openai.com/codex/cli/reference/)
```
**Source:** Mirror of claudecode-adapter/src/cmd.rs structure (Phase 8)

### Pattern 2: E2E Tests with #[ignore]
**What:** Integration tests requiring real CLI, marked #[ignore] to prevent CI failures
**When to use:** Validating containment behavior (not just flag generation)
**Example:**
```rust
//! ## Running E2E Tests
//!
//! ```bash
//! # Run all ignored E2E tests
//! cargo test -p codex-adapter -- --ignored
//! ```

/// Discovers Codex CLI, returns None if not available.
async fn get_codex_cli() -> Option<CodexCli> {
    match discover_codex(None) {
        Ok(path) => {
            let cli = CodexCli::new(path);
            if cli.check_health().await.is_ok() {
                Some(cli)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

#[tokio::test]
#[ignore = "Requires Codex CLI installed"]
async fn e2e_containment_sandbox_readonly() {
    let cli = match get_codex_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Codex CLI not found");
            return;
        }
    };

    let config = CodexConfig {
        sandbox: Some(SandboxMode::ReadOnly),
        ask_for_approval: Some(ApprovalPolicy::Untrusted),
        skip_git_repo_check: true,
        ..Default::default()
    };

    // Test containment behavior
    let result = cli.run("write a file named test.txt", &config).await;

    // Verify sandbox prevented write (or approval blocked it)
    // NOTE: Due to #4152, MCP tools may bypass this
}
```
**Source:** claudecode-adapter/tests/e2e_containment.rs (Phase 8)

### Pattern 3: Retry Budget Configuration
**What:** Adapter-agnostic ExtractionConfig with max_attempts parameter
**When to use:** Orchestrator-level configuration (not adapter-specific)
**Example:**
```rust
// mcp/src/extraction/config.rs
pub struct ExtractionConfig {
    pub max_attempts: usize,  // Default: 3
    pub include_schema_in_feedback: bool,
}

// Codex adapter DOES NOT duplicate this - uses shared orchestrator
```
**Source:** mcp/src/extraction/config.rs (verified shared across all adapters)

### Anti-Patterns to Avoid
- **Adapter-specific extraction retry logic:** Use shared orchestrator from mcp crate
- **Suppressing clippy warnings without justification:** Add inline comments explaining WHY
- **Duplicating Claude Code tests:** Rely on shared orchestrator tests where behavior is identical

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Extraction retry with feedback | Codex-specific retry loop | `mcp::extraction::ExtractionOrchestrator` | Generic across all adapters, tracks metrics, handles parse/validation/callback errors uniformly |
| Graceful process shutdown | `child.kill()` | nix SIGTERM + timeout + SIGKILL pattern | Gives process grace period to flush buffers, prevents corrupted partial output |
| CLI flag validation | Runtime error on spawn | Unit tests with windows(2) pattern | Catches invalid combinations at test time, documents valid patterns |
| JSON streaming | Manual line-by-line parse | tokio BufReader + serde_json::from_str per line | Handles partial lines, EOF, malformed JSON gracefully |

**Key insight:** CLI adapters are thin wrappers around subprocess execution. The complex logic (extraction, validation, feedback) lives in the orchestrator layer.

## Common Pitfalls

### Pitfall 1: Assuming full_auto=false means containment
**What goes wrong:** `full_auto: false` only prevents the convenience alias; it doesn't enable sandbox or approval policies
**Why it happens:** Misunderstanding flag semantics - Codex defaults to NO sandbox unless explicitly set
**How to avoid:** Always explicitly set `sandbox` and `ask_for_approval` for containment; don't rely on negating full_auto
**Warning signs:** E2E tests pass without containment configured

### Pitfall 2: Trusting sandbox mode alone for security
**What goes wrong:** MCP tools bypass Landlock sandbox (Issue #4152), allowing file writes even in read-only mode
**Why it happens:** External Codex bug - MCP tools aren't subject to sandbox enforcement
**How to avoid:** Document limitation prominently in both cmd.rs and crate-level docs; recommend external Docker sandboxes for strong isolation
**Warning signs:** E2E tests show MCP tools working in read-only mode

### Pitfall 3: Duplicating extraction error handling
**What goes wrong:** Adding Codex-specific ExtractionError variants or retry logic
**Why it happens:** Not realizing mcp::extraction is adapter-agnostic by design
**How to avoid:** Use shared ExtractionError enum; Codex adapter only maps CodexError to AgentError variant
**Warning signs:** New error variants in codex-adapter that mirror extraction concerns

### Pitfall 4: Mixing clippy pedantic fixes with feature work
**What goes wrong:** Single commit/plan contains both clippy fixes and new containment tests
**Why it happens:** Temptation to "fix everything at once"
**How to avoid:** Separate clippy pedantic pass (plan 09-01) from containment features (09-02+); makes review easier and git history cleaner
**Warning signs:** Plan mixes "fix doc_markdown warnings" with "add E2E tests"

### Pitfall 5: Not testing flag pair combinations
**What goes wrong:** Valid flag pairs (like --sandbox + --ask-for-approval) not tested together
**Why it happens:** Unit tests only check individual flags
**How to avoid:** Use windows(2) iterator pattern to test all adjacent flag pairs
**Warning signs:** Test verifies `--sandbox read-only` alone but not with `--ask-for-approval untrusted`

## Code Examples

Verified patterns from current implementation:

### Unit Test with windows(2) Pattern
```rust
// Source: claudecode-adapter/src/cmd.rs tests
#[test]
fn test_full_containment_config() {
    let config = CodexConfig {
        sandbox: Some(SandboxMode::ReadOnly),
        ask_for_approval: Some(ApprovalPolicy::Untrusted),
        skip_git_repo_check: true,
        cd: Some(PathBuf::from("/tmp/isolated")),
        full_auto: false,
        ..CodexConfig::default()
    };
    let args = build_args("test prompt", &config);
    let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

    // Verify all containment flags present
    assert!(
        args_str.windows(2).any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
        "Expected '--sandbox read-only'"
    );
    assert!(
        args_str.windows(2).any(|w| w[0] == "--ask-for-approval" && w[1] == "untrusted"),
        "Expected '--ask-for-approval untrusted'"
    );
    assert!(
        args_str.contains(&"--skip-git-repo-check"),
        "Expected '--skip-git-repo-check'"
    );
}
```

### Graceful Shutdown with Signals
```rust
// Source: codex-adapter/src/process.rs (existing)
#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
    tasks: &mut JoinSet<StreamOutput>,
) -> Result<(), CodexError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let raw_pid = i32::try_from(pid).map_err(|_| CodexError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: "PID value exceeds i32::MAX".to_string(),
    })?;
    let nix_pid = Pid::from_raw(raw_pid);

    signal::kill(nix_pid, Signal::SIGTERM).map_err(|e| CodexError::SignalFailed {
        signal: "SIGTERM".to_string(),
        pid,
        reason: e.to_string(),
    })?;

    match timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(_status)) => {}
        Ok(Err(e)) => return Err(CodexError::SpawnFailed {
            stage: "graceful_shutdown wait".to_string(),
            source: e,
        }),
        Err(_) => {
            // Grace period expired, force kill
            child.kill().await.map_err(|e| CodexError::SpawnFailed {
                stage: "SIGKILL".to_string(),
                source: e,
            })?;
        }
    }

    tasks.abort_all();
    Ok(())
}
```

### StreamEvent Enum (Simpler than Claude Code)
```rust
// Source: codex-adapter/src/types.rs (existing)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// A chunk of text output.
    Text { text: String },
    /// An error message from the subprocess.
    Error { message: String },
    /// An unrecognised JSON value.
    Unknown(serde_json::Value),
}

// NOTE: Unlike Claude Code, NO ToolCall/ToolResult variants
// Codex exec output is simpler - just text stream
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Codex v0.91.0 dropped --ask-for-approval | v0.92.0+ re-added with 4 policies | Late 2025 | Prior STATE.md decision about removed flag is obsolete; approval mechanism available |
| Manual clippy warnings | clippy::pedantic workspace-wide | Phase 8 (08-01) | All workspace crates must pass pedantic; codex-adapter already compliant |
| Separate extraction retry per adapter | Shared orchestrator pattern | Phase 7 | Codex uses mcp::extraction like all adapters |
| Tool-based containment (Claude Code) | Sandbox + approval containment (Codex) | N/A (different CLIs) | Both valid; document differences in cmd.rs |

**Deprecated/outdated:**
- STATE.md claim "Codex CLI v0.91.0 dropped --ask-for-approval flag": Contradicted by current help output showing 4 approval policies
- Assumption that full_auto=false enables containment: Must explicitly set sandbox and approval policy

## Open Questions

Things that couldn't be fully resolved:

1. **Should E2E tests cover all 3 sandbox modes?**
   - What we know: Claude Code tests default mode only; Codex has 3 modes (read-only, workspace-write, danger-full-access)
   - What's unclear: Whether testing all modes adds value or just increases E2E test time
   - Recommendation: Start with default containment mode (read-only + untrusted); marked as Claude's discretion in CONTEXT.md

2. **How to test MCP sandbox bypass (#4152)?**
   - What we know: Known external issue, should be documented not skipped
   - What's unclear: Whether to write an E2E test that expects bypass (demonstrates limitation) or just document
   - Recommendation: Add E2E test with comment explaining expected bypass behavior; validates documentation accuracy

3. **ApprovalPolicy enum vs field naming**
   - What we know: Current types.rs has no ApprovalPolicy enum (only SandboxMode)
   - What's unclear: Whether to add enum now or defer to extraction phase
   - Recommendation: Add ApprovalPolicy enum in containment features task (mirrors SandboxMode pattern)

## Sources

### Primary (HIGH confidence)
- Codex CLI help output: /home/pnod/foryou.md (verified current version with --ask-for-approval flag)
- Existing codex-adapter implementation: /home/pnod/dev/projects/rig-cli/codex-adapter/src/*.rs
- Claude Code adapter implementation: /home/pnod/dev/projects/rig-cli/claudecode-adapter/src/*.rs (Phase 8 baseline)
- Extraction orchestrator: /home/pnod/dev/projects/rig-cli/mcp/src/extraction/*.rs (shared across adapters)
- Phase 8 plans: .planning/phases/08-claude-code-adapter/08-01-PLAN.md, 08-03-PLAN.md

### Secondary (MEDIUM confidence)
- [Codex CLI Issue #4152: MCP tools bypass sandbox](https://github.com/openai/codex/issues/4152) (verified via WebSearch)
- [Codex CLI approval policies documentation](https://developers.openai.com/codex/security/) (WebSearch)
- [Rust Clippy pedantic best practices](https://doc.rust-lang.org/clippy/usage.html) (WebSearch)

### Tertiary (LOW confidence)
- None - all critical findings verified with primary sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in use, versions verified from Cargo.toml
- Architecture: HIGH - Direct code inspection of working adapters (Claude Code, Codex, OpenCode)
- Pitfalls: HIGH - Derived from CONTEXT.md decisions and actual implementation gaps
- Flag semantics: HIGH - Verified from current Codex CLI help output in foryou.md

**Research date:** 2026-02-03
**Valid until:** 2026-03-03 (30 days - Codex CLI is stable, flag semantics unlikely to change)

**Key decision validation:**
- ✅ --ask-for-approval IS available (CONTEXT.md decision updated, contradicts STATE.md)
- ✅ MCP sandbox bypass #4152 confirmed as real issue (not assumption)
- ✅ Clippy pedantic already passes for codex-adapter (verified via cargo clippy)
- ✅ Codex StreamEvent enum has 3 variants (Text/Error/Unknown) - simpler than Claude Code's 5

**Planning readiness:** Research is sufficient to create detailed PLAN.md files matching Phase 8 structure. No blocking unknowns.
