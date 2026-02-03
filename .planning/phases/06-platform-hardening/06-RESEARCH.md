# Phase 06: Platform Hardening - Research

**Researched:** 2026-02-02
**Domain:** Cross-platform Rust development for Linux and Windows
**Confidence:** HIGH

## Summary

Platform hardening for Linux and Windows requires addressing four core areas: (1) path and home directory handling using platform-aware crates, (2) subprocess spawning with Windows-specific signal alternatives, (3) binary discovery with .exe extension awareness, and (4) dependency health verification.

The current codebase has Unix-specific signal handling via the `nix` crate (which doesn't support Windows), uses `HOME` environment variable directly instead of cross-platform directory resolution, and relies on `to_string_lossy()` for path-to-string conversion which can lose information on Windows with non-ASCII usernames. The `tempfile` crate is already correctly used with RAII cleanup and works cross-platform without changes needed.

Binary discovery via the `which` crate (version 6.0) already handles .exe extensions automatically on Windows, but lacks common install location fallbacks and helpful error messages when binaries aren't found.

**Primary recommendation:** Replace `nix` signal handling with conditional compilation using Windows-native APIs, migrate `HOME` to `dirs::home_dir()`, replace `to_string_lossy()` with `OsString`/`OsStr` where possible, add common install location fallbacks to discovery functions, and establish `cargo audit` as a justfile target for ongoing dependency health monitoring.

## Standard Stack

The established libraries/tools for cross-platform Rust development:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| dirs | 5.0+ | Cross-platform directory resolution | Standard for home/config paths, handles HOME/USERPROFILE/HOMEPATH |
| tempfile | 3.10+ | Temporary file/directory with RAII cleanup | Secure, cross-platform, automatic cleanup, already in use |
| which | 6.0 | Binary discovery in PATH | Already in use, handles .exe on Windows automatically |
| cargo-audit | Latest | Vulnerability scanning for dependencies | Official RustSec tool, integrates with RustSec Advisory Database |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::ffi::OsString/OsStr | stdlib | Path handling without UTF-8 assumptions | Windows non-ASCII paths, avoid `to_string_lossy()` |
| cfg(target_os) | stdlib | Conditional compilation per platform | Platform-specific signal handling, path conventions |
| std::process::Command | stdlib | Subprocess spawning | Cross-platform, handles shell differences automatically |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| nix (Unix-only) | Platform-specific std APIs with cfg | Lose Unix convenience but gain Windows support |
| to_string_lossy() | OsString/OsStr/PathBuf | More verbose but preserves non-UTF-8 paths |
| dirs | Manual env var checks | Dirs handles platform quirks, env vars incomplete |

**Installation:**
```bash
cargo add dirs --version "^5.0"
# tempfile, which already present in Cargo.toml
cargo install cargo-audit
```

## Architecture Patterns

### Recommended Project Structure
Current structure is already appropriate:
```
{adapter}-adapter/src/
├── discovery.rs      # Binary PATH resolution + fallback locations
├── process.rs        # Subprocess execution (needs Windows signal handling)
├── cmd.rs            # CLI argument building
├── types.rs          # Config types
└── error.rs          # Error types
rig-provider/src/
├── setup.rs          # Config path resolution (needs dirs crate)
└── mcp_agent.rs      # MCP config generation (needs OsString)
```

### Pattern 1: Conditional Platform-Specific Code
**What:** Use `cfg(target_os)` attributes for platform-specific implementations.
**When to use:** Signal handling, process termination, platform-specific paths.
**Example:**
```rust
// Source: Rust stdlib std::process documentation + research findings
#[cfg(unix)]
fn terminate_process(pid: u32) -> Result<(), Error> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let pid = Pid::from_raw(pid as i32);
    signal::kill(pid, Signal::SIGTERM)?;
    Ok(())
}

#[cfg(windows)]
fn terminate_process(pid: u32) -> Result<(), Error> {
    // On Windows, Child::kill() uses TerminateProcess
    // For graceful shutdown, Windows apps should handle WM_CLOSE
    // but console processes don't have that mechanism
    // Best we can do is TerminateProcess (equivalent to SIGKILL)
    Err(Error::GracefulTerminationNotSupported)
}
```

### Pattern 2: Platform-Aware Path Handling
**What:** Use `OsString`/`OsStr` for paths instead of String conversions.
**When to use:** Config file paths, executable paths, working directories.
**Example:**
```rust
// Source: https://doc.rust-lang.org/std/ffi/struct.OsString.html
use std::path::PathBuf;
use std::ffi::OsStr;

// BAD: Loses information on Windows with non-ASCII usernames
fn get_config_path_lossy() -> String {
    let home = std::env::var("HOME").unwrap();
    format!("{}/.claude.json", home)
}

// GOOD: Preserves all path information
fn get_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".claude.json")
}

// When you need a string for display only:
fn display_path(path: &std::path::Path) -> String {
    path.display().to_string() // Lossy but acceptable for display
}
```

### Pattern 3: Binary Discovery with Fallbacks
**What:** Check PATH first, then common install locations, then provide helpful error.
**When to use:** All adapter discovery functions (claude, codex, opencode).
**Example:**
```rust
// Source: Research on npm/cargo install locations
use std::path::PathBuf;
use which::which;

fn discover_with_fallbacks(
    binary_name: &str,
    fallback_locations: &[PathBuf],
    install_hint: &str,
) -> Result<PathBuf, Error> {
    // 1. Try PATH via which crate (handles .exe automatically on Windows)
    if let Ok(path) = which(binary_name) {
        return Ok(path);
    }

    // 2. Check common install locations
    for location in fallback_locations {
        if location.exists() {
            return Ok(location.clone());
        }
    }

    // 3. Helpful error with install instructions
    Err(Error::ExecutableNotFound {
        binary: binary_name.to_string(),
        hint: install_hint.to_string(),
    })
}
```

### Pattern 4: Home Directory Resolution
**What:** Use `dirs::home_dir()` instead of `HOME` environment variable.
**When to use:** Setup registration, config path resolution.
**Example:**
```rust
// Source: https://crates.io/crates/dirs
use dirs::home_dir;
use std::path::PathBuf;

// BAD: Unix-only, fails on Windows
fn setup_config_unix_only() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".claude.json")
}

// GOOD: Cross-platform
fn setup_config_cross_platform() -> PathBuf {
    let home = home_dir().expect("Could not determine home directory");
    home.join(".claude.json")
    // Returns: /home/alice/.claude.json on Linux
    // Returns: C:\Users\Alice\.claude.json on Windows
}
```

### Anti-Patterns to Avoid
- **Direct HOME env var access:** Fails on Windows where USERPROFILE or HOMEPATH is used instead. Use `dirs::home_dir()`.
- **Assuming UTF-8 paths:** Windows allows non-UTF-8 paths via extended characters. Use `OsString`/`OsStr` instead of `to_string_lossy()` for anything beyond display.
- **Unix-only signal handling:** The `nix` crate is Unix-only. Use `cfg(target_os)` to provide Windows alternatives.
- **Hardcoded path separators:** Use `PathBuf::join()` instead of string concatenation with `/` or `\`.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform home directory | Parse env vars manually | `dirs::home_dir()` | Handles HOME/USERPROFILE/HOMEPATH, XDG on Linux, edge cases |
| Temporary file cleanup | Manual cleanup in Drop | `tempfile::tempdir()` | RAII cleanup, secure permissions (0o600), handles panics, already used correctly |
| Binary in PATH | Custom PATH parsing | `which` crate | Handles .exe on Windows, PATH splitting, search order, already in use |
| Dependency vulnerabilities | Manual CVE checks | `cargo audit` | RustSec Advisory Database, automated updates, semver-aware |
| Process termination timeout | Manual signal + timer | Pattern in process.rs | SIGTERM → 5s grace → SIGKILL pattern is proven, needs Windows cfg |

**Key insight:** Cross-platform path and process handling has many edge cases (Windows extended paths, non-ASCII usernames, process handles vs PIDs, WM_CLOSE vs SIGTERM). Standard crates encode decades of platform-specific knowledge.

## Common Pitfalls

### Pitfall 1: Unix-Only Signal Handling on Windows
**What goes wrong:** The `nix` crate is Unix-only and won't compile on Windows. Current codebase uses `nix::sys::signal` in all three adapters.
**Why it happens:** Developers test on Linux, Unix signal model is well-known, Windows process termination is different.
**How to avoid:** Use `#[cfg(unix)]` for nix signal code, `#[cfg(windows)]` for Windows alternatives. On Windows, `Child::kill()` uses `TerminateProcess()` (equivalent to SIGKILL, no graceful option for console processes).
**Warning signs:** Compilation errors on Windows target, `use nix::` imports without `cfg(unix)`.
**Evidence:** [The nix crate is Unix-only](https://docs.rs/nix) and does not support Windows. [Windows has no SIGTERM support](https://users.rust-lang.org/t/best-cross-platform-signal-handling/11022) - must use `TerminateProcess()` or `WM_CLOSE`.

### Pitfall 2: to_string_lossy() Data Loss
**What goes wrong:** `to_string_lossy()` replaces invalid UTF-8 with U+FFFD REPLACEMENT CHARACTER, losing path information. On Windows, this can cause "file not found" errors for non-ASCII usernames.
**Why it happens:** Rust developers expect UTF-8 everywhere, Windows WTF-16 encoding is surprising, `to_string_lossy()` is convenient.
**How to avoid:** Use `OsString`/`OsStr` for paths throughout. Only use `display()` or `to_string_lossy()` for final user output (error messages, logging), never for path operations.
**Warning signs:** Config path errors on Windows with non-ASCII usernames, `.to_string_lossy().to_string()` in path manipulation code.
**Evidence:** [OsStr documentation](https://doc.rust-lang.org/std/ffi/os_str/struct.OsStr.html) explains Windows path encoding. [Community reports](https://users.rust-lang.org/t/path-osstr-and-supporting-non-utf-8-paths-inputs/64826) that "hardly anybody uses OsStr(ing)" but it's necessary for robustness.

### Pitfall 3: Missing Binary Error Messages
**What goes wrong:** "command not found" errors without install instructions leave users confused. They don't know how to install the missing CLI.
**Why it happens:** Discovery functions just propagate `which` errors, don't add user-facing context.
**How to avoid:** When binary discovery fails, error message should include: (1) which binary was searched for, (2) how to install it (e.g., "Install: npm install -g @anthropic-ai/claude-code"), (3) what fallback locations were checked.
**Warning signs:** User confusion in issues, "how do I install X" questions, error messages that are just "NotFound".
**Evidence:** User decision from CONTEXT.md: "When binary not found, error message should include install hint."

### Pitfall 4: Forgetting .exe on Windows Binary Paths
**What goes wrong:** Hardcoded paths to binaries like `~/.cargo/bin/claude` fail on Windows where it's `claude.exe`.
**Why it happens:** Linux developers forget Windows needs .exe, PATH lookup hides this (the `which` crate handles it).
**How to avoid:** Always use `which` crate for discovery, it handles .exe automatically. For fallback paths, use `PathBuf::join("claude")` and let `which` or filesystem check handle the extension.
**Warning signs:** Hardcoded ".exe" in code, path construction with string concatenation.
**Evidence:** The [which crate](https://docs.rs/which) handles .exe extensions automatically on Windows.

### Pitfall 5: Cargo Audit Not in Regular Workflow
**What goes wrong:** Dependencies accumulate vulnerabilities over time, only discovered when building new features or by accident.
**Why it happens:** `cargo audit` isn't run automatically, developers forget to check periodically.
**How to avoid:** Add `cargo audit` as a justfile/Makefile target (not CI per user request), run before releases, block on critical/high CVEs.
**Warning signs:** Outdated dependencies, no recent audit runs, surprise CVE discoveries.
**Evidence:** [cargo-audit documentation](https://crates.io/crates/cargo-audit) recommends regular scanning, integrates with RustSec Advisory Database.

### Pitfall 6: Semver Overpinning or Underpinning
**What goes wrong:** (1) Exact pinning ("=1.2.3") blocks security updates. (2) Wildcard ("*") can pull in breaking changes.
**Why it happens:** Developers either want stability (overpin) or assume semver is perfect (underpin).
**How to avoid:** Use caret requirements (default, "^1.2.3") for most deps. Only pin when integration is fragile. Cargo.lock provides reproducibility without overpinning Cargo.toml.
**Warning signs:** Many exact version pins in Cargo.toml, complaints about "can't update dependency X".
**Evidence:** [Cargo Book on Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) recommends caret as default, warns against exact pins.

## Code Examples

Verified patterns from official sources:

### Cross-Platform Process Termination
```rust
// Source: Research on Windows signal handling + nix documentation
use std::time::Duration;
use tokio::time::timeout;

#[cfg(unix)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    pid: u32,
    grace_period: Duration,
) -> Result<(), Error> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let pid = Pid::from_raw(pid as i32);

    // Send SIGTERM
    signal::kill(pid, Signal::SIGTERM)
        .map_err(|e| Error::SignalFailed(e.to_string()))?;

    // Wait for graceful exit
    if timeout(grace_period, child.wait()).await.is_ok() {
        return Ok(());
    }

    // Force kill with SIGKILL
    signal::kill(pid, Signal::SIGKILL)
        .map_err(|e| Error::SignalFailed(e.to_string()))?;

    child.wait().await?;
    Ok(())
}

#[cfg(windows)]
async fn graceful_shutdown(
    child: &mut tokio::process::Child,
    _pid: u32,
    _grace_period: Duration,
) -> Result<(), Error> {
    // Windows console processes don't have graceful termination mechanism
    // WM_CLOSE only works for GUI apps, GenerateConsoleCtrlEvent is unreliable
    // Best option: immediate TerminateProcess via Child::kill()
    child.kill().await?;
    child.wait().await?;
    Ok(())
}
```

### Binary Discovery with Fallbacks and Install Hints
```rust
// Source: Research on npm/cargo install locations + user requirements
use std::path::PathBuf;
use which::which;

#[cfg(unix)]
fn claude_fallback_locations() -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mut locations = Vec::new();

    if let Some(h) = home {
        // npm global install on Unix
        locations.push(h.join(".npm/bin/claude"));
        // Alternative npm location
        locations.push(h.join(".local/bin/claude"));
    }

    // System-wide npm on Unix
    locations.push(PathBuf::from("/usr/local/bin/claude"));

    locations
}

#[cfg(windows)]
fn claude_fallback_locations() -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mut locations = Vec::new();

    if let Some(h) = home {
        // npm global install on Windows
        locations.push(h.join("AppData/Roaming/npm/claude.exe"));
    }

    // Program Files npm installation
    locations.push(PathBuf::from("C:/Program Files/nodejs/claude.exe"));

    locations
}

pub fn discover_claude(explicit_path: Option<PathBuf>) -> Result<PathBuf, ClaudeError> {
    // 1. Explicit path if provided
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }
        return Err(ClaudeError::ExecutableNotFound(format!(
            "Explicit path does not exist: {}",
            path.display()
        )));
    }

    // 2. Environment variable override
    if let Ok(path_str) = std::env::var("CC_ADAPTER_CLAUDE_BIN") {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    // 3. PATH lookup (which handles .exe on Windows automatically)
    if let Ok(path) = which("claude") {
        return Ok(path);
    }

    // 4. Common install locations
    for location in claude_fallback_locations() {
        if location.exists() {
            return Ok(location);
        }
    }

    // 5. Helpful error with install instructions
    Err(ClaudeError::ExecutableNotFound(
        "claude not found. Install: npm install -g @anthropic-ai/claude-code\n\
         Searched PATH and common install locations.".to_string()
    ))
}
```

### Setup with Cross-Platform Paths
```rust
// Source: dirs crate documentation + current setup.rs
use std::path::PathBuf;

pub fn run_setup(config: &SetupConfig) -> anyhow::Result<()> {
    tracing::info!("Starting Zero-Config self-registration...");

    let exe_path = std::env::current_exe()?;

    // CHANGED: Use dirs crate instead of HOME env var
    let home = dirs::home_dir()
        .context("Could not determine home directory")?;

    // JSON-based configurations (Claude Code, OpenCode)
    let claude_path = home.join(".claude.json");
    setup_json_mcp("Claude Code", &claude_path, &exe_path, "rig-provider", config)?;

    let opencode_path = home.join(".opencode.json");
    setup_json_mcp("OpenCode", &opencode_path, &exe_path, "rig-provider", config)?;

    // TOML-based configurations (Codex)
    let codex_path = home.join(".codex/config.toml");
    setup_codex(&codex_path, &exe_path, "rig-provider", config)?;

    Ok(())
}

fn setup_json_mcp(
    name: &str,
    path: &std::path::Path,
    exe_path: &std::path::Path,
    provider_name: &str,
    config: &SetupConfig,
) -> anyhow::Result<()> {
    println!("Checking {name} config at: {}", path.display());

    // CHANGED: exe_path is now PathBuf, convert to string only for JSON
    let exe_path_str = exe_path.display().to_string();

    // ... rest of function unchanged
    Ok(())
}
```

### Cargo Audit Integration
```bash
# Source: cargo-audit documentation
# Add to justfile or Makefile

# Audit dependencies for vulnerabilities
audit:
    cargo audit

# Update advisory database and audit
audit-update:
    cargo audit update && cargo audit

# Audit with automatic fix attempts (requires fix feature)
audit-fix:
    cargo audit fix

# Check for outdated dependencies
outdated:
    cargo outdated --root-deps-only
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual env var checks (HOME) | dirs crate | dirs 1.0 (2018) | Reliable home dir on all platforms |
| String paths everywhere | OsString/OsStr/PathBuf | Always recommended | Windows non-ASCII support |
| Unix-only signal handling | cfg(target_os) + Windows APIs | Standard practice | Windows compatibility |
| to_string_lossy() for paths | Keep as PathBuf | Best practice | Avoid data loss |
| which 4.x | which 6.x | Major version bump | API changes, better Windows support |
| Manual cargo audit runs | cargo audit in workflow | cargo-audit 0.20+ | Continuous vulnerability monitoring |

**Deprecated/outdated:**
- **HOME environment variable direct access:** Use `dirs::home_dir()` which handles HOME/USERPROFILE/HOMEPATH.
- **nix crate without cfg(unix):** Will fail on Windows. Use conditional compilation.
- **Exact version pins in Cargo.toml:** Use caret requirements unless absolutely necessary.

## Open Questions

Things that couldn't be fully resolved:

1. **Windows graceful shutdown limitation**
   - What we know: Windows console processes don't have a graceful shutdown mechanism like SIGTERM. `TerminateProcess()` is immediate kill.
   - What's unclear: Whether there's a reliable way to signal console processes to exit gracefully on Windows without GUI message loop.
   - Recommendation: Document this as a known limitation. On Windows, timeout just uses immediate `Child::kill()`. Consider warning in documentation that Windows termination is always forceful.

2. **Codex/OpenCode discovery standardization**
   - What we know: User marked as "Claude's discretion" whether to standardize Codex/OpenCode discovery to match Claude's 3-tier pattern.
   - What's unclear: Whether Codex/OpenCode have environment variable conventions or explicit path configs in their ecosystems.
   - Recommendation: Standardize all three adapters to same pattern (explicit path -> env var -> PATH -> fallbacks) for consistency and user experience.

3. **Dependency health thresholds**
   - What we know: User wants audit-only scope, block on critical/high CVEs, warn on medium, ignore low.
   - What's unclear: Exact definition of "well-maintained" for dependencies (last commit date? issue count? maintainer activity?).
   - Recommendation: Define well-maintained as: (1) No critical/high CVEs unpatched for >90 days, (2) Last release within 2 years, (3) Active issue triage (responses within weeks). Flag for manual review if violated.

4. **Semver strategy per dependency type**
   - What we know: Caret requirements are default and recommended.
   - What's unclear: Whether certain dependency types (CLI tools vs libraries) should use different strategies.
   - Recommendation: Use caret for all dependencies. If a specific dep causes breakage, temporarily pin it with a comment explaining why, and revisit quarterly.

## Sources

### Primary (HIGH confidence)
- [tempfile crate](https://context7.com/stebalien/tempfile) - RAII cleanup, cross-platform security
- [dirs crate on crates.io](https://crates.io/crates/dirs) - Home directory resolution across platforms
- [Rust OsStr documentation](https://doc.rust-lang.org/std/ffi/os_str/struct.OsStr.html) - Path encoding on Windows
- [Cargo Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) - Semver best practices
- [cargo-audit on crates.io](https://crates.io/crates/cargo-audit) - Vulnerability scanning
- [nix crate documentation](https://docs.rs/nix) - Unix-only signal handling

### Secondary (MEDIUM confidence)
- [Rust Users Forum: Best cross-platform signal handling](https://users.rust-lang.org/t/best-cross-platform-signal-handling/11022) - Windows signal limitations
- [Rust Users Forum: Path, OsStr, and supporting non-UTF-8 paths](https://users.rust-lang.org/t/path-osstr-and-supporting-non-utf-8-paths-inputs/64826) - OsString practical usage
- [npm folders documentation](https://docs.npmjs.com/cli/v9/configuring-npm/folders/) - npm global binary locations
- [Cargo Book: Installing Binaries](https://doc.rust-lang.org/cargo/commands/cargo-install.html) - Cargo binary install locations
- [which crate documentation](https://docs.rs/which) - PATH binary discovery
- [Rust CLI Book: Signal Handling](https://rust-cli.github.io/book/in-depth/signals.html) - Cross-platform signal patterns

### Tertiary (LOW confidence)
- [GeeksforGeeks: npm path on Windows](https://www.geeksforgeeks.org/how-to-fix-npm-path-in-windows-8-and-10/) - Windows npm paths (educational content, not primary source)
- [Medium: Understanding SemVer Constraints](https://david-garcia.medium.com/understanding-the-semantic-version-semver-constraints-caret-vs-tilde-82c659339637) - Semver explanation (blog post)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - dirs, tempfile, which, cargo-audit are well-documented and standard in Rust ecosystem
- Architecture: HIGH - Platform-specific patterns are standard practice with cfg attributes, verified in stdlib docs
- Pitfalls: HIGH - Nix Unix-only limitation documented, OsString necessity documented, user requirements documented in CONTEXT.md

**Research date:** 2026-02-02
**Valid until:** 60 days (stable ecosystem, crates are mature)
