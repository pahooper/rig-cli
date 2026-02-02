---
phase: 04-agent-containment
verified: 2026-02-01T20:30:00Z
status: passed
score: 7/7 must-haves verified
---

# Phase 4: Agent Containment Verification Report

**Phase Goal:** Agents are locked to MCP tools only, no builtin tool escape
**Verified:** 2026-02-01T20:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | McpToolAgentBuilder defaults to all builtins disabled (MCP-only mode) | ✓ VERIFIED | `builtin_tools: None` at line 98, translates to `BuiltinToolSet::None` at line 378-379 |
| 2 | Developer can opt-in to specific builtin tools via .allow_builtins() | ✓ VERIFIED | Public method at line 183, sets `builtin_tools: Some(tools)`, propagates to `BuiltinToolSet::Explicit` |
| 3 | Agent executes in a temp directory by default, not host filesystem | ✓ VERIFIED | `working_dir: None` default at line 100, TempDir RAII at lines 263-271, `_temp_dir` kept alive |
| 4 | Developer can override working directory via .working_dir() | ✓ VERIFIED | Public method at line 204, overrides temp dir when `Some(path)` |
| 5 | Claude Code runs with --tools '' and --disable-slash-commands by default | ✓ VERIFIED | `disable_slash_commands: true` at line 393, `strict: true` at line 387, `BuiltinToolSet::None` generates `--tools ""` verified by unit test |
| 6 | Codex runs with --sandbox read-only by default | ✓ VERIFIED | `sandbox_mode: Some(SandboxMode::ReadOnly)` at line 99, propagated to CodexConfig at line 443, test verifies CLI arg generation |
| 7 | Containment flags verified via unit tests (12 total tests passing) | ✓ VERIFIED | 6 tests in claudecode-adapter/src/cmd.rs + 6 tests in codex-adapter/src/cmd.rs, all passing |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `rig-provider/src/mcp_agent.rs` | Containment-first builder with opt-in escape hatches | ✓ VERIFIED | Fields: builtin_tools (line 82), sandbox_mode (line 83), working_dir (line 84). Methods: allow_builtins (line 183), sandbox_mode (line 193), working_dir (line 204). All substantive and documented. |
| `claudecode-adapter/src/cmd.rs` | Unit tests for containment flag CLI arg generation | ✓ VERIFIED | 6 tests (lines 94-254): BuiltinToolSet::None → --tools "", Explicit → --tools "Bash", disable_slash_commands, strict MCP, allowed tools, full containment. All passing. |
| `codex-adapter/src/cmd.rs` | Unit tests for containment flag CLI arg generation | ✓ VERIFIED | 6 tests (lines 70-193): SandboxMode::ReadOnly → --sandbox read-only, WorkspaceWrite, ApprovalPolicy::Never, cd flag, full_auto absence, full containment. All passing. Codex Issue #4152 documented (line 76-79). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| McpToolAgentBuilder builtin_tools | run_claude_code | ToolPolicy | ✓ WIRED | Lines 377-380: `match builtin_tools` → `BuiltinToolSet::None` or `Explicit(tools)`. Propagates to `config.tools.builtin` at line 390. |
| McpToolAgentBuilder sandbox_mode | run_codex | CodexConfig | ✓ WIRED | Line 260: unwrap_or default to ReadOnly. Line 443: propagated to `CodexConfig.sandbox`. |
| McpToolAgentBuilder working_dir | TempDir RAII | effective_cwd | ✓ WIRED | Lines 263-271: TempDir created when None, path extracted, `_temp_dir` kept alive until run() completes. Passed to all 3 adapters (lines 396, 445, 503). |
| BuiltinToolSet::None | CLI args | build_args | ✓ WIRED | claudecode-adapter/src/cmd.rs lines 52-54: `BuiltinToolSet::None` generates `--tools ""`. Verified by test at line 113-117. |
| SandboxMode::ReadOnly | CLI args | build_args | ✓ WIRED | codex-adapter/src/cmd.rs lines 18-24: `SandboxMode::ReadOnly` generates `--sandbox read-only`. Verified by test at line 91-95. |
| disable_slash_commands: true | CLI args | build_args | ✓ WIRED | claudecode-adapter/src/cmd.rs lines 72-74: flag present when true. Set at mcp_agent.rs line 393. Test at cmd.rs line 156-161. |
| strict: true | CLI args | build_args | ✓ WIRED | claudecode-adapter/src/cmd.rs lines 45-47: `--strict-mcp-config` when strict=true. Set at mcp_agent.rs line 387. Test at cmd.rs line 182-186. |

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| CONT-01: Default posture disables agent builtin tools | ✓ SATISFIED | `builtin_tools: None` default (line 98) → `BuiltinToolSet::None` → `--tools ""`. Test coverage line 99-118. |
| CONT-02: Developer can opt-in to specific builtin tools | ✓ SATISFIED | `.allow_builtins(vec!["Bash"])` method (line 183) → `BuiltinToolSet::Explicit` → `--tools "Bash"`. Test coverage line 120-140. |
| CONT-03: Per-CLI flags audited and applied | ✓ SATISFIED | Claude Code: --tools "", --disable-slash-commands, --strict-mcp-config. Codex: --sandbox read-only, --ask-for-approval never, --cd. 12 unit tests verify correct CLI arg generation. |
| CONT-04: Agent execution sandboxed to temp directory | ✓ SATISFIED | `working_dir: None` default (line 100) → TempDir::new() (line 266) → cwd set on all adapters. RAII cleanup via `_temp_dir` binding (line 263). |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | - | - | - | - |

**No blocker or warning anti-patterns found.**

The only occurrence of "placeholder" is in documentation (line 14) describing the `{allowed_tools}` template variable, not a stub implementation.

### Human Verification Required

None. All containment claims are verifiable programmatically:

1. Default configuration: Checked via builder defaults (lines 98-100)
2. CLI flag generation: Verified via 12 passing unit tests
3. Temp directory isolation: Verified via TempDir RAII pattern inspection
4. Opt-in escape hatches: Verified via public builder methods and test coverage

The implementation is fully deterministic and testable without human interaction.

---

## Detailed Verification

### Truth 1: McpToolAgentBuilder defaults to MCP-only mode

**Check 1: Builder defaults**
```bash
$ grep -n "builtin_tools:" rig-provider/src/mcp_agent.rs
98:            builtin_tools: None,
```
✓ PASS: Defaults to None (all builtins disabled)

**Check 2: Propagation to ToolPolicy**
```bash
$ grep -A 3 "let builtin_set = match builtin_tools" rig-provider/src/mcp_agent.rs
    let builtin_set = match builtin_tools {
        None => claudecode_adapter::BuiltinToolSet::None,
        Some(tools) => claudecode_adapter::BuiltinToolSet::Explicit(tools.clone()),
    };
```
✓ PASS: None translates to BuiltinToolSet::None

**Check 3: CLI arg generation**
```bash
$ cargo test -p claudecode-adapter test_builtin_none_generates_empty_tools_flag
test cmd::tests::test_builtin_none_generates_empty_tools_flag ... ok
```
✓ PASS: BuiltinToolSet::None generates `--tools ""`

### Truth 2: Developer can opt-in via .allow_builtins()

**Check 1: Public method exists**
```bash
$ grep -A 4 "pub fn allow_builtins" rig-provider/src/mcp_agent.rs
    pub fn allow_builtins(mut self, tools: Vec<String>) -> Self {
        self.builtin_tools = Some(tools);
        self
    }
```
✓ PASS: Method is public and sets builtin_tools to Some(tools)

**Check 2: Propagates to Explicit**
Already verified in Truth 1 Check 2 - Some(tools) translates to BuiltinToolSet::Explicit(tools)

**Check 3: CLI arg generation**
```bash
$ cargo test -p claudecode-adapter test_builtin_explicit_generates_tools_flag
test cmd::tests::test_builtin_explicit_generates_tools_flag ... ok
```
✓ PASS: BuiltinToolSet::Explicit(["Bash"]) generates `--tools "Bash"`

### Truth 3: Agent executes in temp directory by default

**Check 1: Default field**
```bash
$ grep -n "working_dir:" rig-provider/src/mcp_agent.rs
84:    working_dir: Option<std::path::PathBuf>,
100:            working_dir: None,
```
✓ PASS: Defaults to None

**Check 2: TempDir RAII pattern**
```bash
$ grep -B 2 -A 8 "_temp_dir, effective_cwd" rig-provider/src/mcp_agent.rs
        // Create temp dir if working_dir not provided (CONT-04)
        let (_temp_dir, effective_cwd) = match self.working_dir {
            Some(dir) => (None, dir),
            None => {
                let td = tempfile::TempDir::new()
                    .map_err(|e| ProviderError::McpToolAgent(format!("Failed to create temp dir: {e}")))?;
                let path = td.path().to_path_buf();
                (Some(td), path)
            }
        };
```
✓ PASS: TempDir created when working_dir is None, kept alive via `_temp_dir` binding until run() completes

**Check 3: Propagated to all adapters**
```bash
$ grep -n "cwd: Some(cwd.to_path_buf())" rig-provider/src/mcp_agent.rs
396:        cwd: Some(cwd.to_path_buf()),  # Claude Code
503:        cwd: Some(cwd.to_path_buf()),  # OpenCode

$ grep -n "cd: Some(cwd.to_path_buf())" rig-provider/src/mcp_agent.rs
445:        cd: Some(cwd.to_path_buf()),   # Codex
```
✓ PASS: All three adapters receive effective_cwd

### Truth 4: Developer can override via .working_dir()

**Check 1: Public method exists**
```bash
$ grep -A 4 "pub fn working_dir" rig-provider/src/mcp_agent.rs
    pub fn working_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }
```
✓ PASS: Method is public and sets working_dir to Some(path)

**Check 2: Overrides temp dir**
Verified in Truth 3 Check 2 - when working_dir is Some(dir), temp dir is NOT created and dir is used instead

### Truth 5: Claude Code containment flags applied

**Check 1: disable_slash_commands**
```bash
$ grep -n "disable_slash_commands: true" rig-provider/src/mcp_agent.rs
393:            disable_slash_commands: true,

$ cargo test -p claudecode-adapter test_disable_slash_commands_flag
test cmd::tests::test_disable_slash_commands_flag ... ok
```
✓ PASS: Always set to true, generates `--disable-slash-commands`

**Check 2: strict MCP**
```bash
$ grep -n "strict: true" rig-provider/src/mcp_agent.rs
387:            strict: true,

$ cargo test -p claudecode-adapter test_strict_mcp_config_flag
test cmd::tests::test_strict_mcp_config_flag ... ok
```
✓ PASS: Always set to true, generates `--strict-mcp-config`

**Check 3: --tools ''**
Already verified in Truth 1

### Truth 6: Codex sandbox mode applied

**Check 1: Default to ReadOnly**
```bash
$ grep -n "sandbox_mode:" rig-provider/src/mcp_agent.rs
83:    sandbox_mode: Option<codex_adapter::SandboxMode>,
99:            sandbox_mode: Some(codex_adapter::SandboxMode::ReadOnly),
260:        let sandbox_mode = self.sandbox_mode.unwrap_or(codex_adapter::SandboxMode::ReadOnly);
```
✓ PASS: Defaults to Some(ReadOnly), unwrap_or ensures ReadOnly even if None

**Check 2: Propagated to CodexConfig**
```bash
$ grep -A 5 "let config = codex_adapter::CodexConfig" rig-provider/src/mcp_agent.rs
    let config = codex_adapter::CodexConfig {
        full_auto: false,
        sandbox: Some(sandbox_mode.clone()),
        ask_for_approval: Some(codex_adapter::ApprovalPolicy::Never),
        cd: Some(cwd.to_path_buf()),
```
✓ PASS: sandbox_mode cloned into config

**Check 3: CLI arg generation**
```bash
$ cargo test -p codex-adapter test_sandbox_readonly_flag
test cmd::tests::test_sandbox_readonly_flag ... ok
```
✓ PASS: SandboxMode::ReadOnly generates `--sandbox read-only`

**Check 4: full_auto disabled**
```bash
$ grep -n "full_auto: false" rig-provider/src/mcp_agent.rs
442:        full_auto: false,

$ cargo test -p codex-adapter test_full_auto_not_set_by_default
test cmd::tests::test_full_auto_not_set_by_default ... ok
```
✓ PASS: full_auto is false, test confirms --full-auto flag is absent

### Truth 7: Unit tests verify containment flags

**Check 1: Claude Code tests**
```bash
$ cargo test -p claudecode-adapter --lib 2>&1 | grep "test result"
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
```
✓ PASS: 6 tests covering BuiltinToolSet::None, Explicit, disable_slash_commands, strict MCP, allowed tools, full containment

**Check 2: Codex tests**
```bash
$ cargo test -p codex-adapter --lib 2>&1 | grep "test result"
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
```
✓ PASS: 6 tests covering SandboxMode::ReadOnly, WorkspaceWrite, ApprovalPolicy::Never, cd, full_auto absence, full containment

**Check 3: Workspace compilation**
```bash
$ cargo check --workspace 2>&1 | tail -1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```
✓ PASS: Workspace compiles cleanly

---

_Verified: 2026-02-01T20:30:00Z_
_Verifier: Claude (gsd-verifier)_
