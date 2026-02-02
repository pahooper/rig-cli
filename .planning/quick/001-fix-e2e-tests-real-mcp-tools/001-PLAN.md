---
phase: quick
plan: 001
type: execute
wave: 1
depends_on: []
files_modified:
  - rig-provider/examples/mcp_extraction_e2e.rs
  - rig-provider/examples/extraction_e2e.rs
  - rig-provider/examples/extraction_retry_e2e.rs
  - rig-provider/examples/extraction_summarize_e2e.rs
autonomous: true

must_haves:
  truths:
    - "The E2E test launches Claude Code CLI with an MCP config pointing to itself as server"
    - "In server mode, the binary serves submit/validate_json/json_example tools over stdio MCP"
    - "Claude Code discovers the MCP tools, calls them autonomously, and submits structured JSON"
    - "The on_submit callback writes the result to a temp file that the client verifies"
    - "The three broken examples are deleted and replaced with the single working one"
  artifacts:
    - path: "rig-provider/examples/mcp_extraction_e2e.rs"
      provides: "Single canonical E2E test of the full MCP extraction pipeline"
      min_lines: 150
  key_links:
    - from: "mcp_extraction_e2e.rs (client mode)"
      to: "claude CLI --mcp-config"
      via: "McpPolicy in RunConfig with temp config file path"
      pattern: "McpPolicy"
    - from: "mcp_extraction_e2e.rs (server mode)"
      to: "JsonSchemaToolkit -> ToolSet -> serve_stdio"
      via: "ToolSetExt::into_handler().serve_stdio()"
      pattern: "serve_stdio"
    - from: "MCP config JSON"
      to: "Self binary with --server flag"
      via: "std::env::current_exe() in args array"
      pattern: "current_exe"
---

<objective>
Fix the E2E tests so they exercise the REAL MCP tool pipeline: agent discovers tools via MCP protocol, agent autonomously calls json_example/validate_json/submit, and the on_submit callback captures the result.

Purpose: The current three E2E examples are broken — two bypass MCP entirely (agent outputs raw JSON text, orchestrator validates client-side) and the third times out. This defeats the core value proposition of rig-cli: forcing the agent through MCP tool constraints. We need one working E2E test that proves the full pipeline works.

Output: A single `mcp_extraction_e2e.rs` example that replaces all three broken examples.
</objective>

<execution_context>
@/home/pnod/.claude/get-shit-done/workflows/execute-plan.md
@/home/pnod/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
Key files to understand the system:

@rig-provider/examples/extraction_summarize_e2e.rs  (closest attempt - times out, use as starting point)
@rig-provider/examples/claudecode_mcp.rs             (working MCP server pattern to reference)
@mcp/src/server.rs                                    (RigMcpHandler, serve_stdio, McpConfig)
@mcp/src/tools.rs                                     (JsonSchemaToolkit, SubmitTool, ValidateJsonTool, JsonExampleTool)
@claudecode-adapter/src/cmd.rs                        (how --mcp-config and --allowed-tools flags are built)
@claudecode-adapter/src/types.rs                      (McpPolicy, RunConfig, ToolPolicy, BuiltinToolSet)
@claudecode-adapter/src/process.rs                    (run_claude subprocess execution)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Debug timeout root cause and create working MCP E2E test</name>
  <files>rig-provider/examples/mcp_extraction_e2e.rs</files>
  <action>
Create a new file `rig-provider/examples/mcp_extraction_e2e.rs` that implements the full MCP extraction pipeline. Use `extraction_summarize_e2e.rs` as the starting point but fix the issues that cause the timeout.

**Root cause analysis of the timeout (fix ALL of these):**

1. **Missing `--allowed-tools` for MCP tools.** Claude Code in `--print` (non-interactive) mode needs explicit tool permission. Without `--allowed-tools`, the agent may skip MCP tools or hang waiting for approval. MCP tools in Claude Code are namespaced as `mcp__<server-name>__<tool-name>` (double underscores). The server name comes from the key in the mcpServers config object. So for a server named `"rig-extraction"` with tools `submit`, `validate_json`, `json_example`, the allowed tools list must include:
   - `mcp__rig-extraction__submit`
   - `mcp__rig-extraction__validate_json`
   - `mcp__rig-extraction__json_example`
   Set `config.tools.allowed = Some(vec![...])` with these namespaced tool names.

2. **Builtin tools not disabled.** Set `config.tools.builtin = BuiltinToolSet::None` so the agent cannot fall back to file system / bash tools instead of using MCP tools. This is critical — Pitfall 7 from research confirms agents prefer builtins over MCP tools when builtins are available.

3. **System prompt should strongly direct tool usage.** Use `SystemPromptMode::Append(...)` to add a directive like: "You have access to MCP tools: json_example, validate_json, and submit. You MUST use these tools to complete the task. Do NOT output raw JSON text. Call json_example first, then validate_json with your data, then submit."

4. **Timeout may be too short.** The MCP handshake (Claude Code spawning the server subprocess, initializing stdio transport, listing tools) adds overhead. Set timeout to 600 seconds (10 min) for E2E tests. The default 300s may be borderline.

**Implementation structure (dual-mode binary pattern):**

The binary runs in two modes controlled by `--server` flag (use `clap::Parser`):

**Server mode (`--server --output-path <path>`):**
- Build `JsonSchemaToolkit::<TextSummary>::builder()` with an `.on_submit()` callback that writes the deserialized result to `output_path` as JSON
- Provide a good `.example()` instance
- Add all three tools to a `ToolSet`
- Call `toolset.into_handler().await?.serve_stdio().await?` (NOT `run_stdio()` which prints banners to stderr that could interfere)
- Do NOT print anything to stdout — stdout is the MCP transport channel. Only use stderr for debug output if needed.

**Client mode (default):**
- Step 1: `init(None).await?` to discover Claude CLI
- Step 2: `std::env::current_exe()?` to get this binary's path
- Step 3: Create temp file for result output, create temp file for MCP config
- Step 4: Write MCP config JSON: `{"mcpServers": {"rig-extraction": {"command": "<exe>", "args": ["--server", "--output-path", "<result_path>"]}}}`
- Step 5: Build prompt telling agent to summarize a text passage using the tools
- Step 6: Build `RunConfig` with:
  - `mcp: Some(McpPolicy { configs: vec![mcp_config_path], strict: false })`
  - `tools.builtin: BuiltinToolSet::None`
  - `tools.allowed: Some(vec!["mcp__rig-extraction__submit".into(), "mcp__rig-extraction__validate_json".into(), "mcp__rig-extraction__json_example".into()])`
  - `system_prompt: SystemPromptMode::Append("You MUST use the MCP tools (json_example, validate_json, submit) to complete this task. Do not output raw text.")`
  - `timeout: Duration::from_secs(600)`
  - `output_format: Some(OutputFormat::Text)`
- Step 7: `cli.run(&prompt, &config).await?`
- Step 8: Read result file, deserialize to `TextSummary`, assert fields are non-empty and valid

**Use the TextSummary struct from extraction_summarize_e2e.rs** (title, key_points, sentiment, word_count, entities) and the same SOURCE_TEXT passage about Rust adoption.

**Important details:**
- The MCP server name in the config JSON key MUST match the `mcp__<name>__` prefix in allowed-tools. Use `"rig-extraction"` consistently.
- Use `tempfile::NamedTempFile` for both the result file and MCP config file. Keep them alive (don't let them drop) until after the CLI run completes.
- Print progress messages to stdout prefixed with step numbers like `[1/5]` for debuggability.
- On failure, print both stdout and stderr from the CLI run to help diagnose issues.
  </action>
  <verify>
Run `cargo check --example mcp_extraction_e2e` from the `rig-provider` directory to verify the code compiles without errors. The actual E2E run requires Claude Code CLI to be installed and authenticated, so compilation check is the automated verification. Manually run `cargo run --example mcp_extraction_e2e` to verify the full pipeline (requires Claude Code CLI).
  </verify>
  <done>
`mcp_extraction_e2e.rs` compiles cleanly and implements the dual-mode binary pattern with all timeout root causes addressed (allowed-tools with MCP namespacing, builtins disabled, system prompt directive, adequate timeout).
  </done>
</task>

<task type="auto">
  <name>Task 2: Delete the three broken E2E examples</name>
  <files>
    rig-provider/examples/extraction_e2e.rs
    rig-provider/examples/extraction_retry_e2e.rs
    rig-provider/examples/extraction_summarize_e2e.rs
  </files>
  <action>
Delete all three broken example files:
- `rig-provider/examples/extraction_e2e.rs` — bypasses MCP, agent outputs raw JSON, orchestrator validates client-side
- `rig-provider/examples/extraction_retry_e2e.rs` — same bypass pattern with strict schema
- `rig-provider/examples/extraction_summarize_e2e.rs` — attempted MCP but times out due to missing allowed-tools/builtin-disable

Use `git rm` for each file so they are properly staged for removal.

After deletion, run `cargo check --examples` from the `rig-provider` directory to verify no other code depends on these deleted files (they are standalone examples, so nothing should break).
  </action>
  <verify>
`git status` shows the three files deleted. `cargo check --examples` in `rig-provider/` passes without errors (no broken references to deleted examples).
  </verify>
  <done>
The three broken E2E examples are removed from the repository and the remaining examples (including the new `mcp_extraction_e2e.rs`) compile cleanly.
  </done>
</task>

</tasks>

<verification>
1. `cargo check --example mcp_extraction_e2e` compiles without errors
2. `cargo check --examples` (all examples in rig-provider) compiles without errors
3. The three broken example files no longer exist
4. The new example follows the dual-mode binary pattern (--server flag for MCP server mode)
5. RunConfig includes: McpPolicy, BuiltinToolSet::None, allowed tools with mcp__ prefix, SystemPromptMode::Append, 600s timeout
6. Server mode uses serve_stdio() (not run_stdio()) and writes nothing to stdout except MCP protocol
</verification>

<success_criteria>
- Single `mcp_extraction_e2e.rs` file exists that exercises the full MCP pipeline
- Agent is forced through MCP tools (json_example -> validate_json -> submit)
- All three timeout root causes are addressed in the RunConfig
- The three broken examples are deleted
- Full workspace compiles: `cargo check --workspace --examples`
</success_criteria>

<output>
After completion, create `.planning/quick/001-fix-e2e-tests-real-mcp-tools/001-SUMMARY.md`
</output>
