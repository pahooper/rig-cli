# Phase 4: Agent Containment - Research

**Researched:** 2026-02-01
**Domain:** CLI agent containment, tool restriction, filesystem sandboxing
**Confidence:** MEDIUM

## Summary

Agent containment in CLI-based AI agents requires a multi-layered approach combining tool restriction, filesystem sandboxing, and working directory isolation. The three target CLIs (Claude Code, Codex, OpenCode) offer different capabilities:

**Claude Code** provides the most comprehensive containment through `--tools ""` (disable all builtins), `--allowed-tools` (MCP-only allowlist), and `--strict-mcp-config` (force MCP-only mode). Filesystem access is controlled via working directory and `--add-dir` flags.

**Codex** provides OS-level sandboxing through the `--sandbox` flag with three modes (read-only, workspace-write, danger-full-access). Landlock on Linux enforces boundaries. Approval policies (`--ask-for-approval`) add a secondary layer.

**OpenCode** offers permission-based restrictions through configuration (external_directory, tool permissions) but lacks explicit CLI flags for containment. Requires Docker-based external sandboxing for production isolation.

The standard approach is **best-effort per-CLI containment**: use each CLI's native flags to their full extent, document limitations honestly, and provide opt-in escape hatches for developers who need broader access. For temp directory sandboxing (CONT-04), Rust's `tempfile` crate provides RAII-based cleanup, but true filesystem isolation requires external containerization (Docker Sandboxes, nsjail).

**Primary recommendation:** Implement tool restriction first (CONT-01, CONT-02, CONT-03), document filesystem sandboxing limitations (CONT-04), and prepare for future Docker Sandbox integration.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tempfile | 3.x | Temp directory management with RAII cleanup | Industry standard for Rust temp resources, automatic cleanup on drop |
| claudecode-adapter | (internal) | Claude Code CLI wrapper | Existing crate with ToolPolicy support |
| codex-adapter | (internal) | Codex CLI wrapper | Existing crate with SandboxMode support |
| opencode-adapter | (internal) | OpenCode CLI wrapper | Existing crate for basic OpenCode integration |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| landlock | Latest | Linux filesystem restriction (5.13+) | When OS-level sandboxing is required (Codex already uses internally) |
| serde_json | 1.x | Permission config serialization | OpenCode permissions require JSON config format |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tempfile TempDir | std::env::temp_dir + manual cleanup | tempfile provides RAII guarantees, std requires manual cleanup prone to leaks |
| Best-effort per-CLI | Refuse unsupported CLIs | Best-effort enables all three adapters, refusal limits user choice |
| CLI-native sandboxing | Docker Sandboxes, nsjail, gVisor | External containers add complexity but provide stronger isolation guarantees |

**Installation:**
```bash
# Already in Cargo.toml
tempfile = "3"
serde_json = "1"
```

## Architecture Patterns

### Recommended Containment Layer Structure
```
rig-provider/src/
├── mcp_agent.rs           # Builder with containment methods
│   ├── McpToolAgentBuilder::builtin_tools()
│   ├── McpToolAgentBuilder::working_dir()
│   └── McpToolAgentBuilder::sandbox_mode()
├── adapters/
│   ├── claude.rs          # Already has ToolPolicy
│   ├── codex.rs           # Already has SandboxMode
│   └── opencode.rs        # Needs permission config
└── containment/           # (New module for Phase 4)
    ├── mod.rs             # Containment types
    ├── tool_policy.rs     # Cross-CLI tool restriction
    └── sandbox.rs         # Temp dir + working dir management
```

### Pattern 1: Tool Restriction (CONT-01, CONT-02)
**What:** Disable builtin tools by default, force MCP-only mode, provide opt-in escape hatches
**When to use:** All three adapters, with adapter-specific implementations
**Example:**
```rust
// Claude Code: --tools "" disables all builtins, --allowed-tools restricts to MCP
let tools = claudecode_adapter::ToolPolicy {
    builtin: BuiltinToolSet::None,           // Disable all builtins
    allowed: Some(allowed_mcp_tools),         // MCP tools only
    disallowed: None,
    disable_slash_commands: true,             // Prevent interactive escapes
};

// Opt-in: Developer explicitly allows Bash
let tools_with_bash = claudecode_adapter::ToolPolicy {
    builtin: BuiltinToolSet::Explicit(vec!["Bash".to_string()]),
    allowed: Some(allowed_mcp_tools),
    disallowed: None,
    disable_slash_commands: true,
};
```

### Pattern 2: Filesystem Sandboxing (Codex-specific, CONT-04)
**What:** Use Codex's `--sandbox` flag for OS-level filesystem restriction
**When to use:** Codex adapter only (Claude Code and OpenCode lack equivalent)
**Example:**
```rust
// Source: Codex CLI research + existing codex-adapter/src/types.rs
let config = CodexConfig {
    sandbox: Some(SandboxMode::WorkspaceWrite), // Default: write only in workspace
    ask_for_approval: Some(ApprovalPolicy::Untrusted), // Require approval for all commands
    cd: Some(temp_dir.path().to_path_buf()),   // Scope to temp directory
    add_dirs: vec![],                           // No additional directories
    full_auto: false,                           // Prevent approval bypass
    ..Default::default()
};
```

### Pattern 3: Temp Directory Working Directory (CONT-04)
**What:** Execute agents in isolated temp directory by default
**When to use:** All adapters via working directory configuration
**Example:**
```rust
// Source: tempfile crate documentation + Rust best practices
use tempfile::TempDir;

// Create temp dir (automatically cleaned on drop)
let temp_dir = TempDir::new()
    .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp dir: {e}")))?;

// Pass to adapter config
let config = claudecode_adapter::RunConfig {
    cwd: Some(temp_dir.path().to_path_buf()),  // Execute in temp dir
    ..Default::default()
};

// Execute agent
let result = cli.run(prompt, &config).await?;

// Temp dir automatically deleted when temp_dir drops
// IMPORTANT: Don't move temp_dir into run(), keep it alive until result is used
```

### Pattern 4: MCP-Only Strict Mode (Claude Code-specific)
**What:** Use `--strict-mcp-config` to enforce MCP-only tool access
**When to use:** Claude Code adapter when maximum containment is required
**Example:**
```rust
// Source: Claude Code CLI reference documentation
let mcp_policy = claudecode_adapter::McpPolicy {
    configs: vec![config_path.to_string_lossy().to_string()],
    strict: true,  // Only use MCP servers from --mcp-config, ignore all other MCP configs
};

let tools = claudecode_adapter::ToolPolicy {
    builtin: BuiltinToolSet::None,  // Disable all builtins
    allowed: Some(allowed_mcp_tools),
    disallowed: None,
    disable_slash_commands: true,
};
```

### Pattern 5: Permission-Based Containment (OpenCode-specific)
**What:** Use OpenCode's permission configuration to restrict tool access
**When to use:** OpenCode adapter (lacks CLI flags, requires config-based approach)
**Example:**
```rust
// Source: OpenCode permissions documentation
let permissions = serde_json::json!({
    "external_directory": "deny",  // Block access outside working directory
    "bash": {
        "*": "deny",               // Block all bash commands by default
        "git *": "allow"           // Allow specific patterns if needed
    },
    "edit": "deny",                // Block file editing
    "read": {
        "~/.*": "ask",             // Ask for approval on home directory reads
        "*": "allow"               // Allow reads in working directory
    }
});

// Note: OpenCode permissions are configured via OPENCODE_PERMISSION env var
// or in mcp_config_path JSON under "permissions" key
```

### Anti-Patterns to Avoid
- **Relying solely on prompts for containment:** Prompts can be jailbroken. Use CLI flags and OS-level sandboxing.
- **Using `--dangerously-skip-permissions` in production:** Bypasses all Codex safety mechanisms. Only use in development with external sandboxing.
- **Moving TempDir into functions:** Causes premature cleanup. Pass references or keep TempDir alive until result is consumed.
- **Assuming read-only mode prevents all writes:** Codex read-only mode allows filesystem reads but may have bypass vulnerabilities (MCP tools bypass reported in Issue #4152).
- **Hardcoding tool allowlists:** Compute allowed tools from ToolSet at runtime to avoid drift between MCP server and allowlist.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directory cleanup | Manual tmpdir management | `tempfile::TempDir` | RAII guarantees prevent resource leaks; manual cleanup prone to errors on early returns/panics |
| OS-level filesystem isolation | Custom chroot/namespace wrapper | Codex's `--sandbox` flag or Docker Sandboxes | Codex already uses Landlock on Linux; Docker Sandboxes provide production-grade isolation with container boundaries |
| Tool name computation | Hardcoded allowlists | Compute from `ToolSet.get_tool_definitions()` | Tool names can change; runtime computation from ToolSet definitions prevents drift |
| MCP config generation | Per-adapter JSON construction | `rig_mcp_server::McpConfig` with adapter-specific serialization | Centralized type with `.to_claude_json()` prevents format errors |
| Permission prompt handling | Custom approval UI | Codex `--ask-for-approval`, Claude `--permission-mode` | CLIs provide built-in approval mechanisms; custom UI adds complexity |

**Key insight:** Sandboxing is hard to get right. Codex's Landlock integration took significant engineering; don't replicate. Use CLI-native mechanisms where available, document limitations where not, and defer strong isolation to external containers (Docker Sandboxes) for production.

## Common Pitfalls

### Pitfall 1: TempDir Dropped Before Result Used
**What goes wrong:** Temp directory is deleted before agent writes output, causing file-not-found errors
**Why it happens:** Rust drops values at end of scope; moving TempDir into a function causes immediate drop
**How to avoid:** Keep TempDir alive until result is fully consumed
**Warning signs:** File-not-found errors when agent tries to write to temp directory, "No such file or directory" in stderr
```rust
// BAD: temp_dir dropped immediately
let result = {
    let temp_dir = TempDir::new()?;
    let config = RunConfig { cwd: Some(temp_dir.path().to_path_buf()), ..Default::default() };
    cli.run(prompt, &config).await?
}; // temp_dir dropped here, before agent finishes writing

// GOOD: temp_dir outlives agent execution
let temp_dir = TempDir::new()?;
let config = RunConfig { cwd: Some(temp_dir.path().to_path_buf()), ..Default::default() };
let result = cli.run(prompt, &config).await?;
// Use result...
// temp_dir dropped at end of scope after result is consumed
```

### Pitfall 2: Allowlist Bypass via Slash Commands
**What goes wrong:** Agent escapes tool restrictions using Claude Code's `/bash`, `/edit` slash commands
**Why it happens:** Slash commands are interactive features that bypass --allowed-tools restrictions
**How to avoid:** Always set `disable_slash_commands: true` in ToolPolicy when restricting tools
**Warning signs:** Agent writes files or runs commands despite --allowed-tools restrictions, stderr mentions "slash command executed"

### Pitfall 3: MCP Tools Bypass Sandbox Restrictions
**What goes wrong:** MCP tools (e.g., edit, bash from MCP server) bypass Codex sandbox modes
**Why it happens:** Known bug (Codex Issue #4152): MCP tools are not subject to sandbox enforcement
**How to avoid:** Document limitation; if strong isolation is required, use Docker Sandboxes
**Warning signs:** Agent writes files outside workspace despite `--sandbox read-only`, MCP tool executions in logs

### Pitfall 4: Assuming `--tools ""` Disables MCP Tools
**What goes wrong:** Developer sets `--tools ""` expecting all tools disabled, but MCP tools still work
**Why it happens:** `--tools` flag controls **builtin** tools only; MCP tools are separate
**How to avoid:** Use `--strict-mcp-config --mcp-config '{}'` to disable MCP, or omit MCP config entirely
**Warning signs:** Agent still calls MCP tools despite `--tools ""`, confusion in logs

### Pitfall 5: Codex Full-Auto Disables Approval Prompts
**What goes wrong:** Developer uses `full_auto: true` for convenience, loses approval safety layer
**Why it happens:** `--full-auto` sets approval to `on-request` and sandbox to `workspace-write`, reducing friction but removing untrusted-mode protections
**How to avoid:** Explicitly set `ask_for_approval: Some(ApprovalPolicy::Untrusted)` when containment is priority, accept user prompts
**Warning signs:** Agent executes commands without approval prompts in restricted environment

### Pitfall 6: OpenCode Lacks CLI-Level Containment Flags
**What goes wrong:** Developer expects `--sandbox` or `--tools` flags in OpenCode, finds none
**Why it happens:** OpenCode containment is permission-based (config-driven), not CLI-flag-based
**How to avoid:** Use permission configuration in MCP config JSON or OPENCODE_PERMISSION env var; for strong isolation, use Docker Sandboxes
**Warning signs:** OpenCode CLI help has no sandbox/tools flags, configuration errors from trying to pass Claude-style flags

## Code Examples

### Example 1: Default Containment Configuration (Claude Code)
```rust
// Source: Claude Code CLI reference + existing claudecode-adapter
use claudecode_adapter::{BuiltinToolSet, ToolPolicy, McpPolicy, RunConfig};
use tempfile::TempDir;

// Create isolated temp directory
let temp_dir = TempDir::new()?;

// Disable all builtin tools, MCP-only mode
let tools = ToolPolicy {
    builtin: BuiltinToolSet::None,           // Disable Bash, Edit, Read, etc.
    allowed: Some(allowed_mcp_tools),         // Only MCP tools from our server
    disallowed: None,
    disable_slash_commands: true,             // Prevent /bash, /edit escapes
};

let mcp = McpPolicy {
    configs: vec![mcp_config_path],
    strict: false,  // Allow other MCP configs (user's own servers)
};

let config = RunConfig {
    cwd: Some(temp_dir.path().to_path_buf()), // Execute in temp dir
    system_prompt: SystemPromptMode::Append(mcp_instruction),
    mcp: Some(mcp),
    tools,
    timeout: Duration::from_secs(300),
    ..Default::default()
};

let result = cli.run(prompt, &config).await?;
// temp_dir automatically cleaned when dropped
```

### Example 2: Opt-In Bash Access (CONT-02)
```rust
// Source: Claude Code CLI reference --tools flag
// Developer explicitly allows Bash tool for specific use case
let tools_with_bash = ToolPolicy {
    builtin: BuiltinToolSet::Explicit(vec!["Bash".to_string()]),
    allowed: Some(allowed_mcp_tools),  // MCP tools + Bash
    disallowed: None,
    disable_slash_commands: true,
};

let config = RunConfig {
    tools: tools_with_bash,
    // ... rest of config
    ..Default::default()
};
```

### Example 3: Maximum Containment (Codex with Sandbox)
```rust
// Source: Codex CLI reference + codex-adapter
use codex_adapter::{SandboxMode, ApprovalPolicy, CodexConfig};
use tempfile::TempDir;

let temp_dir = TempDir::new()?;

let config = CodexConfig {
    sandbox: Some(SandboxMode::ReadOnly),     // OS-level read-only enforcement
    ask_for_approval: Some(ApprovalPolicy::Untrusted), // Approval for all commands
    full_auto: false,                          // Don't bypass approvals
    cd: Some(temp_dir.path().to_path_buf()),  // Scope to temp directory
    add_dirs: vec![],                          // No additional directories
    overrides: vec![/* MCP server config */],
    system_prompt: Some(mcp_instruction),
    timeout: Duration::from_secs(300),
    ..Default::default()
};

let result = cli.run(prompt, &config).await?;
```

### Example 4: OpenCode Permission-Based Containment
```rust
// Source: OpenCode permissions documentation
// Note: OpenCode requires permissions in config file or env var
let permissions = serde_json::json!({
    "external_directory": "deny",  // Block access outside working directory
    "bash": "deny",                // Block shell execution
    "edit": "deny",                // Block file editing
    "read": {
        "/tmp/**": "allow",        // Allow reads in temp directory
        "*": "deny"                // Deny all other reads
    },
    "doom_loop": "ask",            // Require approval for repeated actions
});

// Inject into OpenCode MCP config
let opencode_cfg = serde_json::json!({
    "$schema": "https://opencode.ai/config.json",
    "permissions": permissions,
    "mcp": {
        "rig_mcp": {
            "type": "local",
            "command": [exe_path],
            "environment": {"RIG_MCP_SERVER": "1"}
        }
    }
});

// Write to temp file, pass to OpenCodeConfig::mcp_config_path
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Prompt-only tool restriction | CLI flags (`--tools`, `--allowed-tools`) | Claude Code 2024-Q4 | Agents can't jailbreak out of tool restrictions via prompt manipulation |
| Manual chroot jails | Landlock-based sandboxing | Codex 0.39.0 (2025) | OS-level enforcement with Linux 5.13+ kernel support |
| User approvals for all operations | Fine-grained approval policies | Codex 2025 | `on-request`, `on-failure`, `never`, `untrusted` modes balance safety and friction |
| Container-per-session overhead | Docker Sandboxes with reuse | Docker Sandboxes beta (2026-Q1) | One sandbox per workspace, state persists across sessions, 84% reduction in permission prompts |
| Global MCP config | `--strict-mcp-config` flag | Claude Code 2025-Q4 | Force MCP-only mode, ignore global config |

**Deprecated/outdated:**
- **`--dangerously-bypass-approvals-and-sandbox` (`--yolo`)**: Still exists but strongly discouraged. Use `--full-auto` with explicit sandbox mode instead.
- **Manual cleanup of temp directories**: Use `tempfile::TempDir` RAII instead of `std::env::temp_dir()` + manual deletion.
- **Assuming Codex read-only prevents all writes**: Codex Issue #4152 shows MCP tools can bypass. Document limitation and use external containers for strong isolation.

## Open Questions

### 1. Docker Sandboxes Integration Strategy
- **What we know:** Docker Sandboxes provide production-grade isolation (filesystem, network, process), one sandbox per workspace, state persistence across sessions
- **What's unclear:** Integration API (does rig-cli spawn sandboxes, or does developer manage them externally?)
- **Recommendation:** Phase 4 implements best-effort CLI-native containment, Phase 6 (Platform Hardening) or later phase adds Docker Sandboxes as opt-in strong isolation layer

### 2. OpenCode Containment Hardening Timeline
- **What we know:** OpenCode lacks CLI-level sandbox flags, permission system is config-based, community reports security incidents (goproxy.cn proxy injection)
- **What's unclear:** Whether OpenCode will add `--sandbox` or `--tools` flags, or if config-based permissions are permanent design
- **Recommendation:** Deprioritize OpenCode containment hardening (consistent with STATE.md decision to deprioritize OpenCode for v1.0), document limitations, recommend Docker Sandboxes for production OpenCode use

### 3. MCP Tool Sandbox Bypass Mitigation
- **What we know:** Codex Issue #4152 reports MCP tools bypass sandbox restrictions, issue is known as of late 2025
- **What's unclear:** Timeline for fix, whether workaround exists at MCP server level
- **Recommendation:** Document limitation in CONT-04 implementation, provide example of Docker Sandbox-based mitigation for users requiring strong isolation

### 4. Cross-Platform Temp Directory Behavior
- **What we know:** `tempfile::TempDir` works on Linux and Windows, default permissions differ (Linux: respects umask, Windows: private by default)
- **What's unclear:** Whether temp directory permissions need explicit hardening on Linux (world-readable by default if umask allows)
- **Recommendation:** Test on both platforms during Phase 6 (Platform Hardening), add explicit permission setting if needed

### 5. Builtin Tool Opt-In API Design
- **What we know:** CONT-02 requires developer opt-in for specific builtin tools
- **What's unclear:** API surface (method per tool? `.allow_builtin_bash()`? single `.allow_builtins(vec!["Bash"])`?)
- **Recommendation:** Use single method accepting `Vec<String>` for flexibility, matches Claude Code `--tools` flag format

## Sources

### Primary (HIGH confidence)
- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference) - Official flags documentation for --tools, --allowed-tools, --disallowed-tools, --strict-mcp-config, --disable-slash-commands
- [Codex CLI Reference](https://developers.openai.com/codex/cli/reference/) - Official documentation for --sandbox, --ask-for-approval, --full-auto flags
- [OpenCode Permissions](https://opencode.ai/docs/permissions/) - Official permission system documentation (external_directory, tool permissions)
- [tempfile crate documentation](https://docs.rs/tempfile/latest/tempfile/) - Rust standard for temporary file and directory management

### Secondary (MEDIUM confidence)
- [Codex Sandbox Modes Explained](https://www.vincentschmalbach.com/how-codex-cli-flags-actually-work-full-auto-sandbox-and-bypass/) - Community guide to Codex sandbox implementation (verified against official docs)
- [Claude Code Permissions Guide](https://www.eesel.ai/blog/claude-code-permissions) - Community guide to permission modes (verified against official docs)
- [Docker Sandboxes Overview](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/) - Docker's official announcement of sandbox feature for AI agents
- Existing codebase: claudecode-adapter/src/types.rs (BuiltinToolSet, ToolPolicy), codex-adapter/src/types.rs (SandboxMode, ApprovalPolicy)

### Tertiary (LOW confidence - marked for validation)
- [Codex Issue #4152](https://github.com/openai/codex/issues/4152) - MCP tools bypass sandbox (community-reported bug, unverified fix timeline)
- [Codex Issue #6049](https://github.com/openai/codex/issues/6049) - Feature request for MCP-only mode (community request, unverified implementation status)
- [OpenCode Security Incident](https://taoofmac.com/space/blog/2026/01/12/1830) - goproxy.cn proxy injection (single blog post, not official acknowledgment)
- WebSearch results on best-effort containment patterns (general security practices, not CLI-specific)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - tempfile and existing adapters are verified production dependencies
- Architecture: MEDIUM - Patterns verified with official CLI docs, but Docker Sandboxes integration API unclear
- Pitfalls: MEDIUM - Based on official docs and known issues, but some edge cases require runtime validation
- Tool restriction (CONT-01, CONT-02, CONT-03): HIGH - CLI flags verified in official documentation
- Filesystem sandboxing (CONT-04): MEDIUM - Codex sandbox verified, Claude/OpenCode lack native support, temp directory isolation requires external containers for strong guarantees

**Research date:** 2026-02-01
**Valid until:** 30 days (2026-03-03) - CLI tools update frequently, flags may change
