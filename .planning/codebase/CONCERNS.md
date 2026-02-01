# Codebase Concerns

**Analysis Date:** 2026-02-01

## Tech Debt

**Hardcoded MCP Configuration Paths:**
- Issue: The serve mode hardcodes `~/.claude.json` as the only MCP config to load, limiting flexibility for other adapters and configurations.
- Files: `rig-provider/src/main.rs` (line 66)
- Impact: Multi-adapter environments may not be properly initialized; configuration flexibility is severely limited; setup process requires administrative changes to work correctly.
- Fix approach: Accept MCP config paths as environment variables or command-line arguments; support dynamic discovery of active MCP servers; detect installed CLIs at runtime rather than hardcoding paths.

**Panicking on Stream Handling:**
- Issue: `opencode-adapter/src/process.rs` uses `.expect()` calls on stdout/stderr stream acquisition (lines 26-27), which will panic if stream setup fails rather than gracefully handling the error.
- Files: `opencode-adapter/src/process.rs` (lines 26, 27)
- Impact: Server crashes on stream acquisition failure; no graceful degradation for subprocess communication issues; affects reliability of process execution.
- Fix approach: Replace `.expect()` with proper error propagation using `?` operator; ensure error types properly wrap stream capture failures.

## Known Bugs

**Possible Task Leak in Async Stream Handling:**
- Symptoms: Long-running OpenCode/Claude processes with streaming output may leak background tokio tasks if the main process fails
- Files: `opencode-adapter/src/process.rs` (lines 33-54, 57-64), `rig-provider/src/adapters/opencode.rs` (line 83), `rig-provider/src/adapters/claude.rs` (line 89)
- Trigger: Spawning unbounded child tasks without task tracking; if the receiving end is dropped, spawned tasks continue running until completion
- Workaround: Tasks will eventually terminate when the underlying process exits; monitor resource usage under high concurrency

**Silent JSON Parsing Failures in Stream Processing:**
- Symptoms: Non-JSON stream lines from CLI tools silently degrade to text events instead of failing explicitly
- Files: `opencode-adapter/src/process.rs` (lines 39-48)
- Trigger: When OpenCode outputs non-JSON lines, parsing failures are caught and ignored with empty else branch
- Workaround: Inspect stderr output for actual errors; fallback to Text events masks potential command failures

## Security Considerations

**Unvalidated File Path Construction in Setup:**
- Risk: Setup process constructs and writes to user config files without validating symlinks or checking for path traversal attacks
- Files: `rig-provider/src/setup.rs` (lines 24, 33, 43, 101, 138)
- Current mitigation: Uses standard `PathBuf` operations which prevent directory traversal, but no symlink resolution check
- Recommendations: Use `fs::canonicalize()` to resolve and validate target paths; implement symlink detection; verify file ownership before writing

**Command Injection Risk in Process Spawning:**
- Risk: While arguments are properly separated via `build_args()`, the message content is passed directly to CLI tools without sanitization
- Files: `opencode-adapter/src/cmd.rs` (line 34), `rig-provider/src/adapters/opencode.rs` (line 155)
- Current mitigation: Arguments are passed as separate `OsString` items, not shell-interpreted; users are expected to supply safe input
- Recommendations: Document input sanitization expectations; consider adding content validation layer; log executed commands for audit trails

**Temporary Directory Permissions:**
- Risk: `SessionManager` uses `tempfile::tempdir()` which creates directories with default umask permissions
- Files: `rig-provider/src/sessions.rs` (line 30)
- Current mitigation: Temporary directories are created with secure system defaults (typically 0700)
- Recommendations: Explicitly set directory permissions after creation; document permission model; add validation that isolation is working

**Environment Variable Handling:**
- Risk: `setup.rs` requires `HOME` environment variable without fallback; setup fails without meaningful error message if HOME is unset
- Files: `rig-provider/src/setup.rs` (line 21)
- Current mitigation: Uses `.context()` which provides error message for missing HOME
- Recommendations: Add fallback to `users` crate for better portability; validate home directory exists and is writable; test on systems with unusual environment setups

## Performance Bottlenecks

**Unbounded Channel for Stream Events:**
- Problem: OpenCode and Claude adapters use unbounded mpsc channels for streaming output, potentially consuming unlimited memory
- Files: `rig-provider/src/adapters/opencode.rs` (line 77), `rig-provider/src/adapters/claude.rs` (line 77)
- Cause: `tokio::sync::mpsc::unbounded_channel()` has no backpressure mechanism for slow consumers
- Improvement path: Replace with bounded channels (e.g., `channel(1000)`); add metrics for queue depth; implement flow control in stream handlers

**Synchronous JSON Parsing on Every Stream Line:**
- Problem: Each output line from subprocesses is JSON-parsed without buffering or batching
- Files: `opencode-adapter/src/process.rs` (lines 39-48)
- Cause: Line-by-line JSON parsing creates parsing overhead for every line of output
- Improvement path: Batch parse JSON objects; use streaming JSON parser; implement line buffering for better throughput

**Blocking String Accumulation in Stream Capture:**
- Problem: Entire stdout/stderr is accumulated in `String` using string mutations under mutex lock
- Files: `opencode-adapter/src/process.rs` (lines 29-30, 51-52), `rig-provider/src/adapters/opencode.rs` (line 155)
- Cause: Each line acquires and releases mutex; no allocation strategy for large outputs
- Improvement path: Use `Vec<u8>` for binary data; implement chunked writing; consider ring buffer for bounded memory

## Fragile Areas

**Process Stream Termination Race Condition:**
- Files: `opencode-adapter/src/process.rs` (lines 73-91)
- Why fragile: Timeout handler kills child process but spawned stdout/stderr tasks may still be reading; tasks are not cancelled explicitly
- Safe modification: Call `task.abort()` on spawned tasks before/after `child.kill()`; use `select!` to ensure proper cleanup order
- Test coverage: Missing integration tests for timeout behavior; no tests for stream task cleanup

**Setup Configuration Parsing Edge Cases:**
- Files: `rig-provider/src/setup.rs` (lines 64-76)
- Why fragile: JSON parsing uses `.unwrap_or()` fallback which silently creates new config on parse failure; Codex TOML parsing concatenates without validation
- Safe modification: Validate parsed JSON structure; implement TOML parsing with error logging; test with malformed configs
- Test coverage: No tests for corrupted or partially written config files; missing edge case coverage for missing mcpServers key

**Adapter Initialization Order Dependencies:**
- Files: `rig-provider/src/main.rs` (lines 68-78)
- Why fragile: All three adapters must initialize successfully; first failure prevents server startup entirely
- Safe modification: Implement graceful degradation where missing adapters log warnings but server continues; add retry logic for discovery failures
- Test coverage: Missing tests for partial adapter initialization; no tests for CLI discovery failures

## Scaling Limits

**Single-Threaded Session Directory Cleanup:**
- Current capacity: Temporary directories accumulate indefinitely in `SessionManager`; no cleanup mechanism
- Limit: Long-running servers will accumulate session directories consuming unbounded disk space
- Scaling path: Implement automatic cleanup; add TTL-based expiration for inactive sessions; provide manual cleanup commands

**Unbounded Session Storage in Memory:**
- Current capacity: `SessionManager` stores all session `TempDir` references in HashMap indefinitely
- Limit: Memory grows linearly with number of unique session IDs; no eviction policy
- Scaling path: Add LRU eviction; implement session timeout; add configurable max session count

**No Rate Limiting on Tool Calls:**
- Current capacity: MCP server accepts unlimited concurrent tool invocations
- Limit: Resource exhaustion possible with many concurrent subprocesses
- Scaling path: Add request queue with max depth; implement per-tool concurrency limits; add metrics and alerting

## Dependencies at Risk

**No Dependency Version Pinning:**
- Risk: `rig-core` is pinned to `0.29.0` but dependency tree is not locked to predictable versions
- Impact: Transitive dependency updates could introduce breaking changes; reproducibility issues in production builds
- Migration plan: Commit `Cargo.lock` to repository; use `cargo update` explicitly; implement CI checks for dependency updates

**Tokio Feature Completeness:**
- Risk: `rig-provider` uses `tokio` with `features = ["full"]`, pulling in unnecessary dependencies
- Impact: Increased binary size; broader attack surface; slower compilation
- Migration plan: Audit actual tokio features used; replace with minimal feature set (likely `rt`, `sync`, `time`, `process`, `io-util`)

## Missing Critical Features

**No Logging Instrumentation in Adapters:**
- Problem: Process execution, error handling, and stream parsing lack detailed logging for debugging
- Blocks: Difficult to diagnose failures in production; stream parsing failures silently become text events
- Priority: Medium - Critical for production support and debugging

**No Health Checks for Adapter CLI Availability:**
- Problem: CLI discovery happens at startup, but tools don't validate CLI remains available or detect version changes at runtime
- Blocks: Server may continue running with unavailable adapters; version mismatches undetected
- Priority: Medium - Affects reliability in long-running servers

**No Configuration File Validation:**
- Problem: Setup writes MCP configs without schema validation or syntax checking
- Blocks: Silent corruption possible; invalid JSON/TOML could be written
- Priority: Medium - Could lead to user configuration issues

**Missing Metrics and Observability:**
- Problem: No collection of tool call counts, latencies, error rates, or resource usage
- Blocks: Impossible to diagnose performance issues or scaling requirements
- Priority: High - Essential for production deployment

## Test Coverage Gaps

**Process Execution Timeout Handling:**
- What's not tested: Timeout path in `run_opencode()`; process killing and task cleanup on timeout
- Files: `opencode-adapter/src/process.rs` (lines 73-91)
- Risk: Timeout code path could have uncaught panics or resource leaks
- Priority: High

**Stream Parsing Failure Modes:**
- What's not tested: JSON parsing errors; malformed stream events; partial JSON lines
- Files: `opencode-adapter/src/process.rs` (lines 39-48)
- Risk: Silent failures mask actual command errors
- Priority: High

**Setup Configuration Overwrites:**
- What's not tested: Behavior when config files already exist; concurrent setup invocations; config file permission errors
- Files: `rig-provider/src/setup.rs` (all setup functions)
- Risk: Data loss or corruption during updates
- Priority: Medium

**Session Isolation:**
- What's not tested: Directory cleanup; session ID uniqueness; concurrent session access
- Files: `rig-provider/src/sessions.rs`
- Risk: Session isolation assumptions could break under load
- Priority: Medium

**Adapter Error Propagation:**
- What's not tested: Adapter initialization failures; discovery failures; health check failures
- Files: `rig-provider/src/main.rs` (lines 68-78)
- Risk: Unhandled errors could panic the server
- Priority: Medium

---

*Concerns audit: 2026-02-01*
