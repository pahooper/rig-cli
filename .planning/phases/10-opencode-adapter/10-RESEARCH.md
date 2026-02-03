# Phase 10: OpenCode Adapter - Research

**Researched:** 2026-02-03
**Domain:** Rust CLI subprocess adapter development, process lifecycle management
**Confidence:** HIGH

## Summary

OpenCode adapter currently has basic functionality (discovery, CLI execution, streaming) but lacks the production hardening present in Claude Code and Codex adapters. This phase brings OpenCode to full parity by adding:

1. **Documentation parity**: Module-level docs matching Claude/Codex quality and depth
2. **Testing parity**: Full test suite including E2E containment tests with `#[ignore]` pattern
3. **Error handling parity**: Complete error variant coverage with context preservation
4. **Code quality parity**: Zero clippy pedantic warnings, same standards as other adapters
5. **MCP integration parity**: Full MCP config delivery via env var + file path

**Primary recommendation:** Follow the exact patterns established in Phases 8 (Claude Code) and 9 (Codex) for documentation, testing, and error handling. OpenCode is architecturally simpler (no approval policies or sandbox flags), so hardening focuses on process management, graceful shutdown, and stream parsing robustness.

## Standard Stack

### Core Dependencies (Already Present)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.0 | Async runtime, process spawning | Industry standard for async Rust |
| serde/serde_json | 1.0 | Config serialization, JSONL parsing | De facto standard for Rust serialization |
| thiserror | 1.0 | Error type derivation | Ergonomic error handling pattern |
| which | 6.0 | CLI binary discovery | Cross-platform PATH resolution |
| nix (Unix) | 0.29 | Signal handling (SIGTERM/SIGKILL) | POSIX signal APIs for graceful shutdown |

### Testing Dependencies (To Add)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3 | Temporary directories for E2E tests | Already used in Claude/Codex E2E tests |

**Installation:**
```bash
# Already in opencode-adapter/Cargo.toml, just add dev-dependencies:
[dev-dependencies]
tempfile = "3"
```

**Note:** OpenCode adapter already has all core dependencies. Testing parity requires adding `tempfile` for E2E containment tests.

## Architecture Patterns

### Adapter Module Structure (Established Pattern)
```
opencode-adapter/src/
├── lib.rs           # Public API, module docs, exports
├── cmd.rs           # CLI arg construction, unit tests
├── discovery.rs     # Binary path resolution
├── error.rs         # Error enum with rich context
├── process.rs       # Subprocess lifecycle, streaming
└── types.rs         # Config, RunResult, StreamEvent
```

**Current state:** OpenCode has this structure. Hardening adds:
- E2E tests in `tests/e2e_containment.rs`
- Enhanced module-level documentation in `lib.rs`
- Version compatibility helpers (functions, not consts due to discovery)

### Pattern 1: CLI Argument Construction with Unit Tests
**What:** `cmd.rs` builds `Vec<OsString>` args from typed config, with comprehensive unit tests verifying flag generation.

**When to use:** Every adapter needs this. Tests document CLI contract.

**Example (from codex-adapter/src/cmd.rs):**
```rust
// Source: /home/pnod/dev/projects/rig-cli/codex-adapter/src/cmd.rs:55-121
pub fn build_args(prompt: &str, config: &CodexConfig) -> Vec<OsString> {
    let mut args = Vec::new();
    args.push(OsString::from("exec"));

    if let Some(ref model) = config.model {
        args.push(OsString::from("--model"));
        args.push(OsString::from(model));
    }

    // ... more flag logic

    args.push(OsString::from(prompt));
    args
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sandbox_readonly_flag() {
        let config = CodexConfig {
            sandbox: Some(SandboxMode::ReadOnly),
            ..CodexConfig::default()
        };
        let args = build_args("test", &config);
        let args_str: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();

        assert!(
            args_str.windows(2).any(|w| w[0] == "--sandbox" && w[1] == "read-only"),
            "Expected '--sandbox read-only' but got: {:?}",
            args_str
        );
    }
}
```

**OpenCode specifics:**
- OpenCode has 6 unit tests in `cmd.rs` already (documented in notes)
- Pattern is established, just needs coverage parity for all config fields
- Test containment via `cwd` set through `Command::current_dir()`, not CLI args
- MCP config via `OPENCODE_CONFIG` env var, not CLI args

### Pattern 2: E2E Containment Tests with `#[ignore]`
**What:** Integration tests that require real CLI installed, marked `#[ignore]` to prevent CI failures.

**When to use:** Verify containment flags actually work with the real CLI binary.

**Example (from claudecode-adapter/tests/e2e_containment.rs):**
```rust
// Source: /home/pnod/dev/projects/rig-cli/claudecode-adapter/tests/e2e_containment.rs:47-111
#[tokio::test]
#[ignore = "Requires Claude CLI installed"]
async fn e2e_containment_no_builtins() {
    let cli = match get_claude_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: Claude CLI not found");
            return;
        }
    };

    let config = RunConfig {
        tools: ToolPolicy {
            builtin: BuiltinToolSet::None, // --tools ""
            allowed: None,
            disallowed: None,
            disable_slash_commands: true,
        },
        timeout: Duration::from_secs(60),
        ..RunConfig::default()
    };

    let result = run_claude(&cli.path, "List all files...", &config, None).await;

    match result {
        Ok(run_result) => {
            let output = run_result.stdout.to_lowercase();
            let indicates_limitation = output.contains("cannot") || ...;
            assert!(indicates_limitation, "Expected containment...");
        }
        Err(e) => {
            // Timeout/error acceptable for containment
            eprintln!("CLI error (acceptable): {e}");
        }
    }
}
```

**OpenCode adaptations:**
- Test working directory containment (no filesystem sandbox flags available)
- Test timeout handling with graceful shutdown
- Test MCP config delivery via env var
- Accept timeout/error as valid outcomes (LLM non-determinism)

### Pattern 3: Process Lifecycle with Graceful Shutdown
**What:** Spawn subprocess with piped streams, bounded channels for output, timeout with SIGTERM → SIGKILL escalation.

**When to use:** All adapters need this for production reliability.

**Example (from claudecode-adapter/src/process.rs structure):**
```rust
// Source: Documented pattern from Phase 8/9 implementations
pub async fn run_claude(...) -> Result<RunResult, ClaudeError> {
    let start_time = Instant::now();
    let mut child = spawn_child(path, &args, config)?;

    let stdout = child.stdout.take().ok_or(ClaudeError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(ClaudeError::NoStderr)?;
    let pid = child.id().ok_or(ClaudeError::NoPid)?;

    let (stdout_tx, mut stdout_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel(CHANNEL_CAPACITY);

    let mut tasks = JoinSet::new();
    tasks.spawn(drain_stdout_bounded(stdout, stdout_tx, ...));
    tasks.spawn(drain_stderr_bounded(stderr, stderr_tx, ...));

    if let Ok(result) = timeout(config.timeout, execute_and_collect(...)).await {
        result
    } else {
        handle_timeout(&mut child, pid, ...).await // SIGTERM → wait → SIGKILL
    }
}
```

**OpenCode has this pattern already** (confirmed in process.rs). Hardening verifies:
- All error paths preserve context (exit code, PID, elapsed time, partial output)
- JoinSet cleanup prevents zombie tasks
- Signal handling is platform-gated (`#[cfg(unix)]`)

### Anti-Patterns to Avoid
- **Don't spawn blocking threads for stream reading**: Use tokio tasks with `JoinSet` for cleanup
- **Don't use `unwrap()` on PID/streams**: Return `NoPid`/`NoStdout`/`NoStderr` errors instead
- **Don't hard-kill immediately on timeout**: SIGTERM with 5s grace period, then SIGKILL
- **Don't mix containment mechanisms**: OpenCode uses working directory + env vars, not CLI flags

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Subprocess lifecycle | Custom process manager | `tokio::process::Command` + `JoinSet` | Handles async I/O, signals, cleanup |
| Stream parsing | Regex line matching | `serde_json::from_str` with `StreamEvent` enum | Type-safe, graceful unknown handling |
| Timeout + graceful shutdown | `thread::sleep` + `kill -9` | `tokio::time::timeout` + SIGTERM → SIGKILL | Gives process time to cleanup |
| Binary discovery | Manual PATH walking | `which` crate | Cross-platform, handles edge cases |
| Platform-specific signals | Direct syscalls | `nix` crate with `#[cfg(unix)]` gates | POSIX-correct, cross-platform safe |

**Key insight:** Process management has more edge cases than it appears. Zombie processes, unclosed streams, partial output loss, and platform differences make custom solutions fragile. The tokio + nix stack handles these correctly.

## Common Pitfalls

### Pitfall 1: Platform-Specific Code Without cfg Gates
**What goes wrong:** Signal handling (SIGTERM/SIGKILL) compiles on Unix but fails on Windows.

**Why it happens:** Developers test on one platform, forget cross-platform builds.

**How to avoid:**
```rust
#[cfg(unix)]
use nix::sys::signal::{self, Signal};

#[cfg(unix)]
async fn send_signal(pid: u32, sig: Signal) -> Result<(), Error> {
    // Unix-specific code
}

#[cfg(not(unix))]
async fn send_signal(pid: u32, _sig: ()) -> Result<(), Error> {
    // Windows fallback or no-op
}
```

**Warning signs:** Compilation failures on Windows, or `nix` imports outside `#[cfg(unix)]`.

### Pitfall 2: Forgetting to Join Background Tasks
**What goes wrong:** Stream reader tasks panic or hang, but main function returns success because `JoinSet` wasn't awaited.

**Why it happens:** Tasks spawned with `tokio::spawn` detach and won't fail the parent.

**How to avoid:**
```rust
let mut tasks = JoinSet::new();
tasks.spawn(drain_stdout(stdout, tx));
tasks.spawn(drain_stderr(stderr, tx));

// Later: wait for all tasks
while let Some(result) = tasks.join_next().await {
    result.map_err(|e| Error::StreamFailed { source: e, ... })??;
}
```

**Warning signs:** Tests pass but real runs show partial output or zombie processes.

### Pitfall 3: Timeout Without Graceful Shutdown
**What goes wrong:** Process killed immediately with SIGKILL, losing partial output and preventing cleanup.

**Why it happens:** Developers use `timeout` but forget to drain streams and send SIGTERM first.

**How to avoid:**
1. On timeout, immediately start draining remaining channel data
2. Send SIGTERM to allow graceful exit
3. Wait `GRACE_PERIOD` (5 seconds)
4. If still alive, send SIGKILL
5. Collect partial stdout/stderr in error context

**Warning signs:** Lost output on timeout, or processes stuck in zombie state.

### Pitfall 4: Unbounded Output Accumulation
**What goes wrong:** CLI produces 100MB of output, adapter OOMs or hangs.

**Why it happens:** Infinite loop in stream reader with no size limit.

**How to avoid:**
```rust
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB

let mut total_bytes = 0;
while let Some(line) = lines.next_line().await? {
    total_bytes += line.len();
    if total_bytes > MAX_OUTPUT_BYTES {
        return Err(Error::OutputTruncated {
            captured_bytes: total_bytes,
            limit_bytes: MAX_OUTPUT_BYTES,
        });
    }
    output_lines.push(line);
}
```

**Warning signs:** Slow tests, memory spikes, or hangs on large outputs.

### Pitfall 5: Missing Documentation on CLI Quirks
**What goes wrong:** Future maintainers don't know why certain parsing is "best-effort" or why some flags are missing.

**Why it happens:** Knowledge stays in commit messages or discussion, not in code.

**How to avoid:**
```rust
// OpenCode has no --system-prompt flag (unlike Claude/Codex).
// We prepend the system prompt to the user message instead.
// This is a documented OpenCode CLI limitation, not a bug.
let effective_message = config.prompt
    .as_ref()
    .map_or_else(|| message.to_string(), |sp| format!("{sp}\n\n{message}"));
```

**Warning signs:** Code comments say "workaround" without explaining why, or tests have unexplained assertions.

## Code Examples

### Example 1: Version Detection (Discovery Pattern)
```rust
// Source: claudecode-adapter pattern
// Note: OpenCode doesn't have init/capabilities like Claude, but should detect version
use which::which;

pub fn discover_opencode(override_path: Option<PathBuf>) -> Result<PathBuf, OpenCodeError> {
    if let Some(path) = override_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(OpenCodeError::ExecutableNotFound(format!(
            "Override path does not exist: {}",
            path.display()
        )));
    }

    // Check environment variable
    if let Ok(env_path) = std::env::var("OPENCODE_BIN") {
        let path = PathBuf::from(env_path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fall back to which
    which("opencode").map_err(OpenCodeError::WhichError)
}

pub async fn check_version(path: &Path) -> Result<String, OpenCodeError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| OpenCodeError::SpawnFailed {
            stage: "version check".to_string(),
            source: e,
        })?;

    if !output.status.success() {
        return Err(OpenCodeError::ExecutableNotFound(
            "Version check failed".to_string()
        ));
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string()
        .pipe(Ok)
}
```

### Example 2: MCP Config Delivery (OpenCode-Specific)
```rust
// Source: opencode-adapter/src/process.rs pattern (existing)
// OpenCode delivers MCP config via OPENCODE_CONFIG env var, not CLI args

fn spawn_child(
    path: &Path,
    message: &str,
    config: &OpenCodeConfig,
) -> Result<(Child, u32), OpenCodeError> {
    let args = crate::cmd::build_args(message, config);
    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env_vars {
        cmd.env(k, v);
    }

    // OpenCode-specific: MCP config via env var
    if let Some(ref mcp_path) = config.mcp_config_path {
        cmd.env("OPENCODE_CONFIG", mcp_path);
    }

    let mut child = cmd.spawn().map_err(|e| OpenCodeError::SpawnFailed {
        stage: "opencode spawn".to_string(),
        source: e,
    })?;

    let pid = child.id().ok_or(OpenCodeError::NoPid)?;
    Ok((child, pid))
}
```

### Example 3: E2E Test Pattern
```rust
// Source: Adapted from claudecode-adapter/tests/e2e_containment.rs

use opencode_adapter::{discover_opencode, run_opencode, OpenCodeCli, OpenCodeConfig};
use std::time::Duration;
use tempfile::TempDir;

async fn get_opencode_cli() -> Option<OpenCodeCli> {
    match discover_opencode(None) {
        Ok(path) => {
            let cli = OpenCodeCli::new(path);
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
#[ignore = "Requires OpenCode CLI installed"]
async fn e2e_working_directory_containment() {
    let cli = match get_opencode_cli().await {
        Some(cli) => cli,
        None => {
            eprintln!("Skipping: OpenCode CLI not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = OpenCodeConfig {
        cwd: Some(temp_dir.path().to_path_buf()),
        timeout: Duration::from_secs(60),
        ..OpenCodeConfig::default()
    };

    // Verify working directory is set correctly
    let result = run_opencode(
        &cli.path,
        "What is the current working directory? Use pwd or equivalent.",
        &config,
        None,
    )
    .await;

    match result {
        Ok(run_result) => {
            let output = run_result.stdout;
            let temp_path_str = temp_dir.path().to_string_lossy();
            assert!(
                output.contains(&*temp_path_str),
                "Expected working directory {} in output, got: {}",
                temp_path_str,
                output
            );
        }
        Err(e) => {
            eprintln!("CLI error (may be acceptable): {e}");
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Thread-based subprocess readers | Tokio async tasks with JoinSet | Phase 1 (Jan 2026) | Prevents zombie tasks, enables proper cleanup |
| Immediate SIGKILL on timeout | SIGTERM with grace period, then SIGKILL | Phase 1 (Jan 2026) | Allows process cleanup, captures partial output |
| Unbounded output accumulation | 10MB limit with OutputTruncated error | Phase 1 (Jan 2026) | Prevents OOM, documents limit in error |
| Per-adapter discovery patterns | Standardized discovery with env var override | Phase 6 (Feb 2026) | Consistent across all three adapters |
| Missing clippy lints | Workspace-level pedantic/nursery/perf | Phase 1 (Jan 2026) | Catches common bugs, enforces best practices |

**Deprecated/outdated:**
- **Manual signal handling without `nix` crate**: Replaced with `nix::sys::signal` for POSIX correctness
- **`unwrap()` on subprocess operations**: Now returns typed errors with context
- **Ad-hoc error messages**: Now use `thiserror` with structured fields

## Open Questions

1. **OpenCode CLI version compatibility**
   - What we know: OpenCode has `--version` flag, supports `--model` override
   - What's unclear: Minimum version for MCP support, when OPENCODE_CONFIG env var was added
   - Recommendation: Document tested version in E2E tests, add version detection in discovery

2. **OpenCode JSONL streaming format**
   - What we know: OpenCode has streaming (process.rs handles it)
   - What's unclear: Exact event schema compared to Claude's `stream-json`
   - Recommendation: Best-effort parsing with `StreamEvent::Unknown` for unrecognized events

3. **OpenCode MCP config file format**
   - What we know: Different JSON format than Claude (documented in prior notes)
   - What's unclear: Schema differences, whether strict validation is needed
   - Recommendation: Document format differences in code comments, fail gracefully on parse errors

## Sources

### Primary (HIGH confidence)
- `/home/pnod/dev/projects/rig-cli/claudecode-adapter/src/` - Complete Claude adapter implementation
- `/home/pnod/dev/projects/rig-cli/codex-adapter/src/` - Complete Codex adapter implementation
- `/home/pnod/dev/projects/rig-cli/opencode-adapter/src/` - Current OpenCode adapter state
- `/home/pnod/dev/projects/rig-cli/Cargo.toml` - Workspace clippy configuration (pedantic standard)
- Phase 8 and 9 completed plans and research (adapter patterns established)

### Secondary (MEDIUM confidence)
- [OpenCode Models Documentation](https://opencode.ai/docs/models/) - CLI flags (`--model`)
- [OpenCode MCP Servers Documentation](https://opencode.ai/docs/mcp-servers/) - MCP integration capability
- Git commit history for opencode-adapter (shows evolution of patterns)

### Tertiary (LOW confidence - marked for validation)
- [Big Pickle Model Specifications](https://www.crackedaiengineering.com/ai-models/opencode-big-pickle) - Model capabilities (not needed for adapter)
- [OpenCode Tutorial 2026](https://www.nxcode.io/resources/news/opencode-tutorial-2026) - General usage (not adapter-specific)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All dependencies already in use, patterns established
- Architecture: HIGH - Exact patterns from Phase 8/9, just need to apply
- Pitfalls: HIGH - All documented from Claude/Codex hardening work
- CLI quirks: MEDIUM - OpenCode less documented than Claude/Codex, need best-effort parsing

**Research date:** 2026-02-03
**Valid until:** 30 days (stable Rust ecosystem, adapter patterns locked)

**Key decision:** OpenCode is production-equal to Claude and Codex. Same test coverage, same error handling depth, same documentation quality. The fact that OpenCode has fewer containment flags doesn't make it "lesser" - it just means tests focus on working directory isolation and MCP config delivery instead of sandbox flags.
