---
status: resolved
trigger: "Investigate issue: mcp-server-not-spawned-by-claude-cli"
created: 2026-02-01T00:00:00Z
updated: 2026-02-01T00:15:00Z
---

## Current Focus

hypothesis: MCP server connects successfully but reports capabilities.tools = false in initialize response, causing Claude CLI to ignore it
test: Check what capabilities rmcp ServerHandler reports in initialize response. Fix it to report tools capability.
expecting: Need to set capabilities.tools = true in the initialize response
next_action: Check rmcp ServerHandler implementation and RigMcpHandler to see how capabilities are reported

## Symptoms

expected: Claude CLI reads `--mcp-config /path/to/config.json`, spawns the MCP server process defined in the JSON, performs MCP handshake, and makes the server's tools available to the agent as `mcp__<server-name>__<tool-name>`.
actual: Claude CLI completely ignores the `--mcp-config` flag. A sniffer server placed at the config path is never spawned (no log file created, no stdin received). The agent only lists built-in tools (Task, Bash, Read, etc.) and globally-configured MCP tools (context7, github, playwright) — never the custom server's tools.
errors: No error messages at all. Silent failure.
reproduction:
  1. Create config at /tmp/test_sniffer_config.json: `{"mcpServers":{"sniffer":{"command":"python3","args":["/tmp/mcp_sniffer.py"]}}}`
  2. Run: `claude --print --mcp-config /tmp/test_sniffer_config.json --output-format text -- "hi"`
  3. The sniffer server is never started (no log file at /tmp/mcp_sniffer_log.txt)
  4. Claude's response lists only built-in + globally-configured tools, not sniffer tools
started: First time testing this feature. The working example uses ~/.claude.json directly, not --mcp-config flag.

## Eliminated

## Evidence

- timestamp: 2026-02-01T00:01:00Z
  checked: Claude CLI help output for --mcp-config
  found: "--mcp-config <configs...> Load MCP servers from JSON files or strings (space-separated)"
  implication: The flag expects file paths, and the cmd.rs implementation passes config paths correctly (line 42-43). Format looks correct.

- timestamp: 2026-02-01T00:02:00Z
  checked: claudecode-adapter/src/cmd.rs build_args function
  found: Lines 40-48 handle MCP config - for each config path in mcp.configs, it adds "--mcp-config" followed by the path as separate args
  implication: Argument building looks correct. Each config file gets its own --mcp-config flag.

- timestamp: 2026-02-01T00:03:00Z
  checked: mcp_extraction_e2e.rs client mode setup
  found: Lines 156-169 create MCP config JSON with structure {"mcpServers":{"rig_extraction":{"command":"...","args":[...]}}}. Lines 212-214 pass config path via McpPolicy::configs. Line 213 sets strict:false.
  implication: JSON structure matches ~/.claude.json format that works. Config is passed to RunConfig.mcp correctly.

- timestamp: 2026-02-01T00:04:00Z
  checked: Ran Claude CLI with test MCP config: claude --print --mcp-config /tmp/test_mcp_config.json --output-format text -- "List all available tools"
  found: Output lists built-in tools + context7 + github + playwright MCP tools. NO tools from test_server. File /tmp/test_output.json does NOT exist.
  implication: Server was never spawned. Claude CLI completely ignored the --mcp-config file.

- timestamp: 2026-02-01T00:05:00Z
  checked: claudecode_mcp.rs (the working example)
  found: Lines 164-201 register_self_as_mcp() writes directly to ~/.claude.json under mcpServers key. It does NOT use --mcp-config flag at all. The client mode (lines 122-161) just runs the agent - MCP tools are available because they're in ~/.claude.json globally.
  implication: The working example doesn't actually test --mcp-config. It modifies global config instead. This explains why mcp_extraction_e2e (which uses --mcp-config) doesn't work.

- timestamp: 2026-02-01T00:06:00Z
  checked: Tested inline JSON string: --mcp-config '{...json...}'
  found: Still no custom tools listed. Same behavior as file path.
  implication: Format (file vs inline) is not the issue.

- timestamp: 2026-02-01T00:07:00Z
  checked: Tested --strict-mcp-config with file path
  found: With --strict, ALL MCP tools disappeared (no context7, github, playwright). Only built-in tools remain. Still no custom test_server tools.
  implication: --strict-mcp-config DOES affect MCP loading (removes globals), but the config file still isn't being read. This suggests --mcp-config flag is parsed but the config content is not processed.

- timestamp: 2026-02-01T00:08:00Z
  checked: Added server via "claude mcp add test_cli_server -- /path/to/binary --server --output-path /tmp/test.json"
  found: Command succeeded: "Added stdio MCP server... to local config". Server shows as "✓ Connected" in claude mcp list. BUT: tools still not available in --print mode.
  implication: Server is healthy and responds correctly to MCP handshake, but tools don't appear in --print mode.

- timestamp: 2026-02-01T00:09:00Z
  checked: Manually tested server with full MCP handshake (initialize -> initialized notification -> tools/list)
  found: Server responds perfectly: returns 3 tools (json_example, validate_json, submit) with complete schemas. No errors. Uses newline-delimited JSON correctly.
  implication: The MCP server implementation is working correctly. The problem is 100% in Claude CLI not spawning or connecting to custom stdio servers.

- timestamp: 2026-02-01T00:10:00Z
  checked: Claude CLI debug logs at ~/.claude/debug/*.txt
  found: "MCP server \"test_cli_server\": Connection established with capabilities: {\"hasTools\":false,\"hasPrompts\":false,\"hasResources\":false,\"serverVersion\":{\"name\":\"rmcp\",\"version\":\"0.14.0\"}}"
  implication: ROOT CAUSE FOUND! The server connects successfully but reports hasTools=false in its capabilities. Claude CLI sees no tools available, so it doesn't expose them to the agent. The server's initialize response must not include the tools capability flag.

## Resolution

root_cause: RigMcpHandler implements ServerHandler but does NOT override get_info() to report server capabilities. The default rmcp implementation returns capabilities with hasTools=false, causing Claude CLI to connect successfully but ignore all tools. The fix requires implementing get_info() to return ServerInfo with ServerCapabilities::builder().enable_tools().build().

fix: Added get_info() and initialize() methods to ServerHandler impl for RigMcpHandler in mcp/src/server.rs. The get_info() method returns ServerInfo with protocol version V_2024_11_05, capabilities with enable_tools(), and server implementation details.

verification:
- Rebuilt example binary and added to claude mcp list
- Tools now appear in --print mode: mcp__test_fixed_server__validate_json, mcp__test_fixed_server__json_example, mcp__test_fixed_server__submit
- Full e2e test passes: agent discovers tools, calls them in sequence, submits valid structured JSON
- All assertions pass

files_changed: ["mcp/src/server.rs"]
