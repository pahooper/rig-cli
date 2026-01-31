# Rig Provider Ecosystem

This workspace contains a collection of adapters and a centralized provider designed to integrate various AI CLIs (Claude Code, Codex, OpenCode) as high-level tools within the [Rig](https://github.com/0xPlayground/rig) ecosystem.

## Workspace Structure

- **[rig-provider](file:///home/pnod/dev/projects/rig-cli/rig-provider)**: The main MCP server that orchestrates the adapters and provides a "Zero-Config" setup experience.
- **[claudecode-adapter](file:///home/pnod/dev/projects/rig-cli/claudecode-adapter)**: Adapter for Anthropic's Claude Code CLI.
- **[codex-adapter](file:///home/pnod/dev/projects/rig-cli/codex-adapter)**: Adapter for the Codex CLI.
- **[opencode-adapter](file:///home/pnod/dev/projects/rig-cli/opencode-adapter)**: Adapter for the OpenCode CLI.
- **[mcp](file:///home/pnod/dev/projects/rig-cli/mcp)**: The core `rig-mcp-server` library used by the provider.

## Key Features

- **Zero-Config Setup**: Run `rig-provider setup` to automatically register the provider across all supported AI CLIs.
- **Per-Session Sandboxing**: Each agent session operates in an isolated, persistent temporary directory.
- **Idiomatic Rig Experience**: Native support for one-shot prompts, chat, and structured data extraction.
- **Observability**: Built-in telemetry aligned with Rig's internal tracing style.

## Examples

See the `examples/` directory for idiomatic usage patterns:

- **[One-Shot Usage](file:///home/pnod/dev/projects/rig-cli/examples/one_shot.rs)**: Simple, direct tool execution.
- **[Agent Workflow](file:///home/pnod/dev/projects/rig-cli/examples/agent_workflow.rs)**: Integrating provider tools into complex AI agents.
- **[Structured Data Extraction](file:///home/pnod/dev/projects/rig-cli/examples/data_extraction.rs)**: Leveraging the built-in JSON extraction magic.
- **[Session Isolation](file:///home/pnod/dev/projects/rig-cli/examples/session_isolation.rs)**: Demonstrating persistence across multiple tool calls.

## Documentation Artifacts

For detailed information on the project's evolution, design, and verification, see the following internal artifacts:

1.  **[Walkthrough](file:///home/pnod/.gemini/antigravity/brain/8c9cd30d-45e4-419d-a617-679ab85a5b7e/walkthrough.md)**: A comprehensive summary of implemented features and proof-of-work.
2.  **[User Stories](file:///home/pnod/.gemini/antigravity/brain/8c9cd30d-45e4-419d-a617-679ab85a5b7e/rig_user_stories.md)**: Detailed mapping of Rig user stories to provider capabilities.
3.  **[Implementation Plan](file:///home/pnod/.gemini/antigravity/brain/8c9cd30d-45e4-419d-a617-679ab85a5b7e/implementation_plan.md)**: The technical roadmap for the multi-crate architecture.
4.  **[Task Checklist](file:///home/pnod/.gemini/antigravity/brain/8c9cd30d-45e4-419d-a617-679ab85a5b7e/task.md)**: The living checklist of all completed items.

## Quick Start

```bash
# Register the provider (Dry run)
cargo run -p rig-provider -- setup --dry-run

# Start the MCP server
cargo run -p rig-provider
```
