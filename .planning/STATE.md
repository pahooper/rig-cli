# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-01)

**Core value:** When a developer passes a struct and data to a CLI agent, they get validated typed output back reliably — the agent is forced through MCP tool constraints to submit conforming JSON rather than freeform text.
**Current focus:** Phase 7 complete. Rig integration polished with two execution paths (agent(), mcp_agent()).

## Current Position

Phase: 7 of 11 (Rig Integration Polish)
Plan: 7 of 7 in current phase - PHASE COMPLETE
Status: Phase 7 complete (Complete: 07-01, 07-02, 07-03, 07-04, 07-05, 07-06, 07-07)
Last activity: 2026-02-03 — Completed 07-07-PLAN.md (Payload Wiring and Dead Code Cleanup)

Progress: [█████████████████] 27/27 plans complete (Phase 1: 5/5, Phase 2: 2/2, Phase 2.1: 3/3, Phase 3: 2/2, Phase 4: 2/2, Phase 5: 2/2, Phase 6: 4/4, Phase 7: 7/7)

## Performance Metrics

**Velocity:**
- Total plans completed: 27
- Average duration: 2.7 min
- Total execution time: 1.6 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-resource-management-foundation | 5 | 23min | 5min |
| 02-retry-validation-loop | 2 | 6min | 3min |
| 02.1-transparent-mcp-tool-agent | 3 | 8min | 3min |
| 03-payload-instruction-system | 2 | 4.5min | 2.25min |
| 04-agent-containment | 2 | 4.4min | 2.2min |
| 05-observability-infrastructure | 2 | 5.5min | 2.75min |
| 06-platform-hardening | 4 | 8.7min | 2.2min |
| 07-rig-integration-polish | 7 | 31.5min | 4.5min |

**Recent Trend:**
- Last 5 plans: 07-07 (3.2min), 07-06 (6min), 07-05 (8min), 07-04 (3.4min), 07-03 (5.2min)
- Trend: Phase 7 COMPLETE - all Rig integration polish tasks done

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Force structured output via MCP tools rather than prompt-only (gives schema enforcement at protocol level)
- Three-tool pattern (submit/validate/example) for workflow guidance
- Adapter-per-CLI crate structure (clean separation of concerns)
- Best-effort containment per CLI (document limitations rather than refuse to support)
- Deprioritize OpenCode for v1.0 (focus on getting two adapters rock solid)
- Apply resource management fixes to opencode-adapter despite deprioritization (infrastructure-level stability concern)
- Use same bounded channel architecture across all adapters for consistency (01-01, 01-02, 01-03)
- Standardize on 100-message channel capacity, 10MB output limit, 5s grace period across all adapters
- Use pid: 0 placeholder in rig-provider NonZeroExit since RunResult doesn't carry PID (01-04)
- Match claudecode-adapter's graceful_shutdown pattern exactly across all adapters (01-05)
- Use chars().count() not len() for token estimation to handle UTF-8 correctly (02-01)
- ExtractionError::MaxRetriesExceeded holds complete history, raw output, and metrics (02-01)
- Validation feedback includes schema, submission echo, all errors, and attempt counter (02-01)
- Orchestrator not generic over T - works with serde_json::Value, caller deserializes (02-02)
- Conversation continuation strategy: append feedback to prompt for retry context (02-02)
- Parse failures count against same retry budget as validation failures (02-02)
- McpToolAgent uses free functions (not &self methods) for per-adapter execution to avoid partial-move issues (02.1-02)
- Env var detection (RIG_MCP_SERVER=1) for server mode instead of --server CLI flag (02.1-03)
- Codex and OpenCode lack --system-prompt flag; prepend system prompt to user prompt instead (E2E testing)
- Each adapter manages its own MCP config delivery: Claude uses file path, Codex uses -c overrides (CodexConfig.overrides), OpenCode uses env var + file with different JSON format (E2E testing)
- OpenCode uses opencode/big-pickle model for MCP agent execution (E2E testing)
- Default to MCP-only mode: disable all builtin tools unless developer explicitly opts in (04-01)
- Temp directory by default: agents execute in isolated temp dir with RAII cleanup (04-01)
- Codex full_auto: false preserves sandbox and approval safety layers (04-01)
- Claude Code strict: true forces MCP-only config, ignores global MCP configs (04-01)
- Best-effort per-CLI containment: use each CLI's native flags to full extent, document limitations (04-01)
- Unit tests use windows(2) pattern to find adjacent flag-value pairs in CLI args (04-02)
- Default config tests verify full_auto absence to ensure containment posture (04-02)
- Codex MCP sandbox bypass limitation documented inline as known external issue (04-02)
- Codex CLI v0.91.0 dropped --ask-for-approval flag; removed ApprovalPolicy enum and ask_for_approval field from codex-adapter (E2E testing)
- Codex requires --skip-git-repo-check for non-git temp directory containment; added skip_git_repo_check field to CodexConfig (E2E testing)
- OpenCode adapter now has 6 unit tests for CLI arg generation in cmd.rs (E2E testing)
- Use #[tracing::instrument] with skip_all to avoid logging closures and prompts (security-first observability) (05-01)
- Emit flat events with attempt=N field instead of nested per-attempt spans (avoids async Span::enter() pitfalls) (05-01)
- Log only character counts (prompt_chars, output_chars), never prompt or response content at any level (05-01)
- Event message strings match event field values for machine-parseable grep/filter (05-01)
- Version requirements are hardcoded const functions per adapter, not developer-configurable (05-02)
- Version detection warns and continues on mismatch, never blocks execution (fail-open policy) (05-02)
- Distinct warning events: version_unsupported (below min) vs version_untested (above max_tested) (05-02)
- Version detection is stateless, runs once per agent execution (no caching) (05-02)
- Use cfg(unix)/cfg(windows) conditional compilation for platform-specific code, not runtime detection (06-01)
- Windows graceful shutdown uses immediate Child::kill() (TerminateProcess) - documented platform limitation (06-01)
- Platform-neutral error types use String descriptions instead of Unix-specific errno types (06-01)
- Nix crate imports moved inside cfg(unix) function bodies, gated behind [target.'cfg(unix)'.dependencies] (06-01)
- Include cargo audit in check recipe for continuous security validation (06-04)
- Provide standalone audit, audit-update, and outdated targets for developer convenience (06-04)
- cargo-outdated is optional tooling, target defined but installation not required (06-04)
- All adapters follow 5-step discovery: explicit path, env var, PATH, fallback locations, helpful error (06-02)
- Use dirs::home_dir() for cross-platform home directory resolution instead of HOME env var (06-02, 06-03)
- Platform-specific fallback locations use cfg(unix)/cfg(windows) compilation flags (06-02)
- Windows npm installs use .cmd wrappers, Go binaries use .exe extension in fallback paths (06-02)
- Keep paths as PathBuf/&Path as long as possible, convert to String only at serialization boundaries (06-03)
- Use display().to_string() over to_string_lossy() for idiomatic path-to-string conversion (06-03)
- Document all to_string_lossy() usage with inline comments explaining why conversion is acceptable (06-03)
- Feature flags control module compilation not dependency compilation - adapters always compile, user picks which to use (07-01)
- Error enum wraps ProviderError with #[from] for automatic conversion while providing actionable Display messages (07-01)
- ClientConfig defaults: 300s timeout, 100 message channel capacity, auto-discovery for binary path (07-01)
- Workspace facade pattern preserves existing adapter separation and provides clean public API (07-01)
- CliResponse is rig-cli-owned type, not adapter-internal RunResult - shared across all providers (07-02, 07-03)
- Tool routing: direct CLI for simple prompts (backward compatible), MCP path prepared for extractor pattern (07-02)
- For v1, completion_with_mcp falls back to direct CLI since ToolDefinition -> Tool trait object conversion is complex (07-02)
- MCP enforcement via extractor pattern (Plan 07-04) rather than completion() interception (07-02)
- Client wraps ClaudeCli directly, not ClaudeModel (facade, not delegation) (07-02)
- All three providers (Claude, Codex, OpenCode) follow identical implementation pattern for API consistency (07-03)
- Codex and OpenCode StreamEvent enums have fewer variants (Text/Error/Unknown only, no ToolCall/ToolResult) (07-03)
- Payload field stored but unused in Model pending future MCP integration (07-03)
- Prelude exports minimal set: Client types (feature-gated), Error, Rig traits (Prompt, Chat), MCP types (07-04)
- Escape hatches return CLI handle directly (not internal Model) for adapter-specific access (07-04)
- debug-output feature is opt-in only, not in default features (production safety) (07-04)
- Re-export rig crate at lib root for user access to Rig types via rig_cli::rig::... (07-04)
- MCP types re-exported through extraction and tools modules for discoverability (07-04)
- McpStreamEvent enum unified across all adapters (Claude has ToolCall/ToolResult, Codex/OpenCode have Text/Error only) (07-05)
- Streaming helpers spawn tokio tasks for background execution with channel-based event forwarding (07-05)
- CliAgent prompt() and chat() methods consume self since ToolSet lacks Clone (07-05)
- CliAgent provides methods, not Rig Prompt/Chat trait implementations (trait signatures incompatible) (07-05)
- mcp_agent() uses CliAgent::builder() factory method not CliAgentBuilder::new() (private constructor) (07-06)
- mcp_agent() conceptual examples marked as 'ignore' since they require user-supplied ToolSet (07-06)
- Removed completion_with_mcp/completion_without_mcp routing - clean separation via agent() vs mcp_agent() (07-06)
- CompletionClient trait added to prelude for consistent agent() access in doctests (07-06)
- Payload wraps prompts with XML <context>/<task> structure for instruction/data separation (07-07)
- Codex uses system_prompt field, OpenCode uses prompt field for preamble (adapter-specific naming) (07-07)
- Remove duplicate CliResponse from claude.rs, use shared type from response.rs (07-07)
- model_name field allowed as dead_code with justification (API consistency, CLI agents don't use per-request model selection) (07-07)

### Pending Todos

None.

### Roadmap Evolution

- Phase 2.1 added (INSERTED): Transparent MCP Tool Agent — McpToolAgent builder that auto-spawns MCP server, generates config, and wires Claude CLI. Inserted between Phase 2 and Phase 3. COMPLETE: 3 plans executed, verified 6/6 must-haves.

### Blockers/Concerns

- Pre-existing ~265 missing-docs clippy warnings across adapter crates (not blocking, future documentation pass)
- Codex Issue #4152: MCP tools bypass Landlock sandbox restrictions (known external limitation, documented inline)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 002 | Save Phase 2.1 plan files to GSD planning system | 2026-02-01 | abd49bc | [002-save-phase-2-1-plans-to-gsd](./quick/002-save-phase-2-1-plans-to-gsd/) |
| 003 | Update planning docs with E2E testing findings | 2026-02-02 | 0616a58 | [003-update-planning-docs-for-e2e-testing-f](./quick/003-update-planning-docs-for-e2e-testing-f/) |
| 004 | Document E2E testing findings and adapter fixes from Phase 4 | 2026-02-02 | d9198b2 | [004-document-e2e-testing-findings-and-adapte](./quick/004-document-e2e-testing-findings-and-adapte/) |

## Session Continuity

Last session: 2026-02-03
Stopped at: Completed 07-07-PLAN.md (Payload Wiring and Dead Code Cleanup) - Phase 7 complete
Resume file: None
