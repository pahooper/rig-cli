# Phase 6: Platform Hardening - Context

**Gathered:** 2026-02-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Cross-platform reliability for Linux and Windows. Subprocess spawning, temp directories, config paths, and CLI binary discovery work identically on Pop!_OS and Windows. All external crate dependencies are verified as well-maintained and stable.

</domain>

<decisions>
## Implementation Decisions

### Binary discovery
- Add common install location fallbacks after PATH lookup (e.g., ~/.npm/bin, ~/.cargo/bin, platform-specific locations)
- When binary not found, error message should include install hint (e.g., "claude not found. Install: npm install -g @anthropic-ai/claude-code")
- Claude's discretion: whether to standardize Codex/OpenCode discovery to match Claude's 3-tier pattern (explicit path -> env var -> PATH)

### Path & temp dir strategy
- Use `dirs` crate for home directory resolution (handles HOME, USERPROFILE, HOMEPATH across platforms)
- Config paths should match each CLI tool's actual default location on each platform (not hardcoded Unix conventions)
- Use `OsString`/`OsStr` where possible instead of `to_string_lossy()` for path-to-string conversion (handles non-ASCII usernames on Windows)
- tempfile crate current RAII pattern is solid, no changes needed for temp dirs

### Windows support scope
- All three adapters (Claude Code, Codex, OpenCode) get Windows support
- Full parity — all features work identically on Windows, blocking bugs fixed before release
- Test on a Windows Docker instance for verification
- Support both cmd.exe and PowerShell for subprocess shell needs

### Dependency audit
- Run `cargo audit` for vulnerability scanning
- Add as a justfile/Makefile target (no GitHub Actions)
- Block on critical/high CVEs, warn on medium, ignore low
- Audit-only scope — verify current deps are healthy, only replace if critically broken or vulnerable

### Claude's Discretion
- Exact common install locations to check per platform per adapter
- Semver range vs exact pinning strategy for dependencies
- What constitutes "well-maintained" threshold for dependency health
- Whether to use `Command::new()` directly everywhere or add shell wrapper detection logic for cmd.exe/PowerShell

</decisions>

<specifics>
## Specific Ideas

- User explicitly wants no GitHub/GitHub Actions in the CI pipeline — justfile targets only
- Windows Docker instance for cross-platform verification
- Current codebase uses HOME env var directly in setup.rs — needs `dirs` crate replacement
- Current `to_string_lossy()` usage in mcp_agent.rs for config paths needs OsString migration

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-platform-hardening*
*Context gathered: 2026-02-02*
