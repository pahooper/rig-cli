# Coding Conventions

**Analysis Date:** 2026-02-01

## Naming Patterns

**Files:**
- Snake case for Rust modules: `lib.rs`, `main.rs`, `error.rs`, `types.rs`, `process.rs`, `discovery.rs`
- Adapter packages use hyphenated names: `claudecode-adapter`, `codex-adapter`, `opencode-adapter`
- Integration test files: `integration.rs` in `tests/` directory

**Functions:**
- Snake case for all functions: `build_args()`, `format_chat_history()`, `get_session_dir()`, `discover_opencode()`
- Public functions in module files prefixed with namespace (implicit): `run_opencode()`, `init()`, `discover_claude()`
- Async functions marked with `pub async fn`: `check_health()`, `run()`, `stream()`, `completion()`

**Variables:**
- Snake case for local variables and parameters: `session_id`, `stdout_cap`, `final_stdout`, `exit_code`, `duration_ms`
- Use descriptive names for Arc/Mutex wrapped data: `captured_stdout`, `captured_stderr`, `sessions`
- Path variables use standard naming: `path`, `cwd`, `dir`

**Types:**
- PascalCase for structs: `OpenCodeCli`, `RunResult`, `StreamEvent`, `OpenCodeConfig`, `SessionManager`, `ClaudeModel`, `ProviderError`
- PascalCase for enums: `OpenCodeError`, `StreamEvent`, `ClaudeError`, `ProviderError`
- Enum variants use PascalCase: `Text`, `Error`, `Unknown`, `ExecutableNotFound`, `NonZeroExit`, `Timeout`
- Trailing `Config` suffix for configuration structs: `OpenCodeConfig`, `RunConfig`
- Trailing `Error` suffix for error enums: `OpenCodeError`, `ClaudeError`, `ProviderError`

## Code Style

**Formatting:**
- Standard Rust formatting (rustfmt)
- 4-space indentation (Rust default)
- Lines follow standard Rust conventions

**Linting:**
- Workspace-wide clippy rules defined in root `Cargo.toml`
- `#![warn(clippy::pedantic)]` applied to adapter crates: `opencode-adapter`, `claudecode-adapter`
- Workspace lints include:
  - `unsafe_code = "deny"`
  - `missing_docs = "warn"`
  - `unwrap_used = "warn"`
  - `expect_used = "warn"`
  - `panic = "warn"`
  - `todo = "warn"`
  - `unimplemented = "warn"`
  - `dbg_macro = "warn"`
  - Clippy levels: `pedantic`, `nursery`, `perf`, `cargo` set to `warn`

## Documentation

**Module Documentation:**
- Module-level doc comments with `//!` format describing purpose:
  - `//! The Rig Provider crate acts as the central integration point for AI CLI adapters.`
  - `//! This crate provides a bridge between the Rig toolset and the Model Context Protocol (MCP).`

**Function Documentation:**
- Triple-slash `///` doc comments for public functions describing what they do
- `# Errors` section for functions that return `Result` types
- `#[must_use]` attribute for functions returning values that should not be ignored
- Example from sessions: `/// Gets or creates a temporary directory for the given session ID.`

**Field Documentation:**
- Doc comments for enum variants and struct fields using `///`
- Examples from `StreamEvent`:
  ```rust
  /// A chunk of text content.
  text: String
  /// The error message.
  message: String
  ```

**Attribute Usage:**
- `#![deny(missing_docs)]` on crate roots requiring documentation (e.g., `rig-provider/src/lib.rs`)
- `#![warn(clippy::pedantic)]` on adapter crates
- Attribute at file level for lint configuration

## Import Organization

**Order:**
1. External crate imports (e.g., `use tokio::process::Command`)
2. Standard library imports (e.g., `use std::path::PathBuf`)
3. Internal module imports (e.g., `use crate::error::OpenCodeError`)
4. Re-exports at module level

**Path Aliases:**
- No path aliases configured (uses standard module paths)
- Crate dependencies use `path = "../module"` in monorepo workspace

**Module Structure:**
- Public modules exposed via `pub mod` declarations in `lib.rs`
- Re-exports of key types/functions at module level for ergonomic access
- Prelude modules for common imports (e.g., `rig_mcp_server::prelude::*`)

## Error Handling

**Patterns:**
- Use `thiserror::Error` for error enum definitions with `#[derive(Debug, Error)]`
- Error enums implement Display via `#[error("message")]` attributes on variants
- Source error context via `#[from]` attributes on enum variants:
  ```rust
  #[error("Failed to spawn process: {0}")]
  SpawnFailed(#[from] std::io::Error),
  ```
- Detailed error info using named fields in variants:
  ```rust
  #[error("Process exited with non-zero status: {exit_code}\nSTDOUT: {stdout}\nSTDERR: {stderr}")]
  NonZeroExit {
      exit_code: i32,
      stdout: String,
      stderr: String,
  }
  ```
- Return `Result<T, ErrorType>` for fallible functions
- Use `anyhow::Result<T>` for propagating errors in some contexts (e.g., setup functions)
- Use `?` operator for error propagation from `Result` types

## Type Definitions

**Generic Types:**
- Use standard Rust generics for reusable components
- Arc + Mutex for shared mutable state: `Arc<Mutex<HashMap<String, Arc<TempDir>>>>`
- Option for optional values: `Option<String>`, `Option<PathBuf>`, `Option<u16>`

**Derive Macros:**
- `#[derive(Debug, Clone, Serialize, Deserialize)]` for data structures
- `#[derive(JsonSchema)]` for API schema generation (used with `schemars`)
- `#[derive(Debug, Error)]` for error types
- `#[derive(Clone, Default)]` for manager types

## Struct Design

**Patterns:**
- Public fields when they are simple data containers (not encapsulation)
- Example: `OpenCodeCli { pub path: PathBuf }`
- Methods implemented in separate `impl` blocks
- Builder patterns not used (prefer constructor functions)

## Comments

**When to Comment:**
- Explain WHY, not WHAT (code is what, comments explain rationale)
- Mark fallback/error handling paths: `// Fallback`, `// Raw text line`
- Document non-obvious behavior: `// JSON parse attempt, fallback to text`
- Implementation notes for complex async patterns

**Inline Comments:**
- Single-line comments for brief explanations
- No commented-out code in main modules

## Async/Await Patterns

**Async Functions:**
- Mark async functions with `pub async fn` or `async fn`
- Use `#[tokio::main]` for main function in binaries
- Use `tokio::spawn()` for concurrent task execution
- Use `timeout()` from `tokio::time` for time-limited operations
- Use `tokio::sync::mpsc::UnboundedSender` for channel communication

**Await Points:**
- Call `.await` on all futures
- Use `let _ = future.await;` when result is explicitly ignored

## Module Organization

**Patterns:**
- Each adapter (`claudecode-adapter`, `opencode-adapter`) follows consistent structure:
  - `lib.rs` - Main module with `pub struct Cli`
  - `cmd.rs` - Command building utilities
  - `error.rs` - Error type definitions
  - `types.rs` - Data structures
  - `process.rs` - Execution logic
  - `discovery.rs` - Binary discovery logic
- Provider crate organizes by concern:
  - `adapters/` - Integration with specific tools
  - `errors.rs` - Error aggregation
  - `sessions.rs` - Session management
  - `utils.rs` - Shared utilities
  - `setup.rs` - Initialization logic

## Serialization

**Patterns:**
- Implement `Serialize` and `Deserialize` from `serde` for config/data types
- Use `serde_json` for JSON serialization
- Derive implementations: `#[derive(Serialize, Deserialize)]`
- Use qualified imports: `use serde::{Deserialize, Serialize};`

## Testing Naming

**Test Functions:**
- Use descriptive names: `test_mcp_config_formats()`, `test_toolkit_and_submit_callback()`
- Mark with `#[tokio::test]` for async tests or `#[test]` for sync tests
- Start with `test_` prefix

---

*Convention analysis: 2026-02-01*
