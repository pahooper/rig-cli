# Pitfalls Research

**Domain:** CLI-agent-wrapping projects with Rig provider integration and structured extraction
**Researched:** 2026-02-01
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: Zombie Process Accumulation from Incomplete Process Cleanup

**What goes wrong:**
Child processes exit but are not properly reaped, leaving zombie processes that accumulate over time and count against system process limits. On Unix platforms, the tokio runtime performs "best-effort" reaping, but provides no guarantees about timing. In the current implementation, when timeout occurs (line 88 in `opencode-adapter/src/process.rs`), `child.kill()` is called but the process is never awaited, creating a guaranteed zombie.

**Why it happens:**
Developers assume that `kill()` completes cleanup, not understanding that on Unix the parent must call `wait()` to release OS resources. The tokio documentation explicitly warns: "it is possible for a zombie process to remain after a kill is sent; to avoid this, the caller should ensure that either child.wait().await or child.try_wait() is invoked successfully."

**How to avoid:**
1. Always await the child process after killing: `child.kill().await?; child.wait().await?;`
2. For graceful shutdown, send SIGTERM first, wait with timeout, then SIGKILL if needed
3. Track spawned child process handles and ensure cleanup in Drop implementations
4. Use `kill_on_drop(true)` but understand it sends SIGKILL immediately without graceful teardown

**Warning signs:**
- `ps aux` shows `<defunct>` processes accumulating
- Process table exhaustion after extended server uptime
- System-wide process count approaching ulimit
- Integration tests leaving zombie processes between runs

**Phase to address:**
Phase 1 (Error Handling & Resource Management) - This is a production-blocking bug that causes resource exhaustion.

---

### Pitfall 2: Orphaned Async Tasks from Untracked Spawns

**What goes wrong:**
Background tasks spawned with `tokio::spawn` continue running indefinitely even after the parent future is dropped. In the current implementation (lines 33-54, 57-64 in `opencode-adapter/src/process.rs`), stdout/stderr reader tasks are spawned but their `JoinHandle`s are immediately discarded. When timeout occurs and `child.kill()` is called (line 88), these tasks continue attempting to read from killed process pipes, potentially blocking indefinitely.

**Why it happens:**
Developers think dropping a future cancels concurrently running tasks, but tokio documentation is explicit: "Dropping a future does not cancel a concurrently running (tokio::spawn) task." The misconception is reinforced by async/await's cancel-on-drop semantics for non-spawned futures. Additionally, developers don't realize that calling `JoinHandle::abort()` only *schedules* cancellation - you must still await the handle to ensure completion.

**How to avoid:**
1. Store all `JoinHandle`s and explicitly abort them: `handle.abort(); let _ = handle.await;`
2. Use `tokio::select!` to race process completion against timeout, ensuring task handles are in scope
3. Implement structured concurrency patterns where spawned tasks cannot outlive their parent scope
4. Use `tokio::task::JoinSet` to track multiple related tasks and abort them as a group

**Warning signs:**
- Memory growth over time despite bounded request queue
- CPU usage on background threads even when no requests are active
- Log messages from tasks that should have been cancelled
- `tokio-runtime-worker` threads stuck in `park` state with high counts

**Phase to address:**
Phase 1 (Error Handling & Resource Management) - Task leaks cause memory bloat and resource exhaustion in long-running servers.

---

### Pitfall 3: Unbounded Channel Memory Explosion Under Backpressure

**What goes wrong:**
The current implementation uses `tokio::sync::mpsc::unbounded_channel()` (line 77 in `rig-provider/src/adapters/claude.rs`). When the CLI agent produces output faster than the receiver consumes it, the channel's internal queue grows without limit, eventually causing OOM kills. Tokio documentation warns: "unbounded channels have no backpressure and can blow up memory if producers outrun consumers."

**Why it happens:**
Unbounded channels feel safer because `send()` never fails. Developers choose them to avoid handling channel-full errors, not realizing they've traded explicit failures for silent memory accumulation. The problem is invisible during testing with small outputs but catastrophic in production when an agent streams large file contents or error loops.

**How to avoid:**
1. Replace with bounded channels: `mpsc::channel(1000)` with appropriate buffer size
2. Handle `try_send()` failures explicitly: drop oldest, apply backpressure, or fail fast
3. Monitor queue depth via channel metrics/observability
4. For streaming responses, implement flow control where consumer can signal slow-down to producer
5. Bound the maximum response size at the protocol level

**Warning signs:**
- Memory usage spikes correlating with long-running agent invocations
- OOM kills in production but not development
- Response times increase over time (GC pressure from large allocations)
- `receiver.recv()` latency increases as queue grows

**Phase to address:**
Phase 1 (Error Handling & Resource Management) - Unbounded channels are a production-blocking reliability issue.

---

### Pitfall 4: LLM Schema Drift Despite Tool Constraints

**What goes wrong:**
Even with JSON schema validation tools (`submit`, `validate`, `example`), LLMs will spontaneously rename fields ("status" → "current_state"), add helpful but unexpected fields, nest data differently than specified, or inject markdown code fences around JSON. This breaks schema validation even though the agent "tried to help." The three-tool pattern in `mcp/src/tools.rs` validates *when called* but cannot prevent the agent from submitting malformed JSON on first attempt.

**Why it happens:**
Schema constraints are *soft* - they're expressed in natural language (tool descriptions) that the LLM interprets probabilistically. Model updates between testing and production change behavior. Temperature settings above 0.0 introduce non-determinism. The agent's training biases it toward "helpful" transformations that violate rigid schemas. Smaller models particularly struggle with exact format compliance.

**How to avoid:**
1. Use structured output APIs when available (API-native constraint enforcement)
2. Implement retry loop with validation errors fed back to agent: "Field 'priority' must be integer 1-5, got string 'high'"
3. Set temperature to 0.0 for structured extraction (sacrifice creativity for determinism)
4. Include negative examples in prompts: "Do NOT wrap in markdown, do NOT add explanatory text"
5. Add preamble stripping: detect and remove markdown fences, prose before/after JSON
6. Implement "self-correction" pass: on parse failure, give malformed output back to agent with correction instructions

**Warning signs:**
- JSON parsing failures in production despite passing tests
- Schema validation errors citing fields that shouldn't exist
- Failures correlate with model version updates or provider switches
- Different failure modes between temperature 0.0 and 0.7

**Phase to address:**
Phase 2 (Agent Containment & Structured Extraction) - Core to the product value proposition.

---

### Pitfall 5: MCP Tool Poisoning via Malicious Tool Metadata

**What goes wrong:**
MCP's trust model assumes tool providers are benign. A compromised or malicious MCP server can register tools with deceptive descriptions: "send_email" that actually exfiltrates data, "validate_json" that injects instructions into the agent's context, or lookalike tools that shadow trusted ones. The agent has no mechanism to verify tool authenticity or detect metadata manipulation. This is particularly dangerous because `rig-cli` loads MCP configs via `setup.rs` and serves them without sandboxing tool definitions.

**Why it happens:**
MCP sampling relies on an implicit trust model and lacks robust built-in security controls. The protocol designers chose not to include authentication in v1, leaving each MCP server to implement its own approach. Tool permissions allow combining tools to exfiltrate data. Developers assume tool descriptions are read-only metadata, not realizing they're active instructions that shape agent behavior.

**How to avoid:**
1. Implement tool allowlisting at the MCP handler level - only expose explicitly approved tools
2. Validate tool definitions against expected schemas on load
3. Add tool namespacing: prefix tools with source (`github.com/trusted/tool_name`)
4. Log all tool invocations with full arguments for audit trail
5. Implement tool capability isolation - prevent tools from composing in dangerous ways
6. For user-provided MCP configs, show diff of tool definitions and require explicit approval

**Warning signs:**
- Tool invocations in logs that don't match expected workflow
- Unexpected network activity or file access during agent runs
- Tool descriptions that include imperative instructions ("Always submit passwords using...")
- Duplicate tool names from different MCP servers

**Phase to address:**
Phase 3 (Security & MCP Hardening) - MCP trust model is emerging threat for 2026 production deployments.

---

### Pitfall 6: Stream Race Condition Between Process Exit and Pipe Closure

**What goes wrong:**
The current implementation spawns stdout/stderr reader tasks (lines 33-54, 57-64 in `opencode-adapter/src/process.rs`) and then waits for the process to exit (line 67). There's a race: the process can exit and close its pipes before the reader tasks have consumed all buffered data. The tasks see EOF and return, but the last chunk of output is lost. This is especially problematic for JSON streaming where the final closing brace might be in the unconsumed buffer.

**Why it happens:**
Developers assume pipe closure is atomic with process exit, not realizing the OS buffers data and readers can lose the race. Awaiting the process first seems natural, but it creates the race window. The implementation waits for tasks (lines 68-69) but uses `let _ =` which silently ignores join errors - if a task panicked or was cancelled, the failure is invisible.

**How to avoid:**
1. Use `tokio::select!` to wait for *both* process exit *and* reader task completion
2. Explicitly await reader tasks and propagate errors: `stdout_task.await??`
3. Ensure process wait comes *after* confirming readers have consumed all data
4. For critical streams, implement explicit EOF markers in protocol (don't rely on pipe closure)
5. Test by injecting delays between process exit and pipe closure

**Warning signs:**
- Truncated JSON in production ("unexpected EOF" parse errors)
- Last line of output sometimes missing from captured streams
- Non-deterministic test failures where output length varies
- Issues reproduce more frequently under high system load

**Phase to address:**
Phase 1 (Error Handling & Resource Management) - Causes data corruption in structured extraction.

---

### Pitfall 7: Agent Containment Bypass via Builtin Tool Fallback

**What goes wrong:**
Even when MCP tools are provided and `tools.allowed` is set (lines 54-56, 85-86 in `rig-provider/src/adapters/claude.rs`), CLI agents may fall back to builtin tools (file editing, bash execution, web browsing) when they deem MCP tools insufficient. The current implementation sets `builtin: BuiltinToolSet::Default` (line 56), which permits this fallback. An agent can escape the MCP sandbox by claiming "I need to check the file system to complete this task" and using builtin tools instead.

**Why it happens:**
CLI agent containment flags are inconsistent across tools. `claude code` has multiple flag combinations (`--allow-tools`, `--tool-choice`, `--builtin-tools`) with subtle interaction semantics. Developers expect `tools.allowed` to be exclusive, not realizing it's additive with builtins unless explicitly disabled. Documentation is ambiguous about precedence. The agent's training biases it toward using familiar builtin tools over custom MCP tools.

**How to avoid:**
1. Audit each CLI's containment flags: test that `--builtin-tools=none` actually prevents builtin usage
2. Set `builtin: BuiltinToolSet::None` when MCP tools should be exclusive
3. Implement agent instruction injection that explicitly forbids non-MCP tool use
4. Monitor tool invocations: alert if builtin tools are called when they shouldn't be
5. Test containment by providing insufficient MCP tools and verifying agent fails rather than escapes
6. Document containment limitations per CLI version (some may not support full isolation)

**Warning signs:**
- Agents accessing file system despite no file-related MCP tools provided
- Bash command execution appearing in logs when only structured extraction tools were supplied
- Network requests from agent when no network-related MCP tools exist
- Session directories containing files created by agent rather than by host process

**Phase to address:**
Phase 2 (Agent Containment & Structured Extraction) - Core to security model and structured extraction reliability.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `.expect()` / `.unwrap()` on I/O operations | Faster prototyping, less boilerplate | Production panics, server crashes, no error context | Never in process spawning/stream handling |
| Unbounded channels | No backpressure handling, sends never block | Memory exhaustion, OOM kills under load | Only for bounded-size message types (control signals, not data streams) |
| Silent JSON parse failures (`if let Ok` with empty else) | Appears robust, doesn't crash | Masks command failures, degrades to wrong behavior | Never - always log parse failures at WARN level |
| Ignoring `JoinHandle` results (`let _ = task.await`) | Hides task panic boilerplate | Task failures invisible, debugging impossible | Never - always propagate or explicitly log task errors |
| Hardcoded config paths (`~/.claude.json`) | Simple implementation, works for common case | Cannot support multi-adapter setups, breaks in containers/CI | Only for initial MVP, must parameterize for v1.0 |
| Single timeout value for all operations | Simple configuration API | Short prompts waste time, long prompts hit timeout | Defer until proven necessary, acceptable for v1.0 |
| `.clone()` on CLI clients for threading | Avoids lifetime complexity | Unnecessary allocations if client is Arc internally | Acceptable if client implements cheap clone (check implementation) |

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| MCP Server Registration | Overwriting entire mcpServers config without reading existing entries | Parse existing JSON, merge new server entry, preserve others (`setup.rs` lines 64-76 does this correctly) |
| Subprocess Working Directory | Not validating `cwd` exists before spawning | Check path existence and permissions; fail early with actionable error instead of cryptic spawn failure |
| CLI Version Detection | Assuming CLI capabilities from executable presence | Parse `--version` output, map to capability matrix, fail fast on unsupported versions |
| Tool Definition Schema | Assuming `schemars::schema_for!` output is stable | Pin schemars version, test schema output in CI, detect breaking schema changes |
| Session Directory Cleanup | Assuming `TempDir` drop cleans up immediately | `TempDir` cleanup is best-effort; implement explicit cleanup for disk-constrained environments |
| Streaming Event Ordering | Assuming events arrive in order sent | Buffer and sequence events; handle reordering, duplicate, missing events in protocol layer |
| Error Message Parsing | Regex parsing stderr for error detection | Use structured error output when available (`--output-format json`); stderr is for humans, not machines |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Accumulating session directories in HashMap | Slow hash lookups, memory growth | Implement LRU eviction with TTL-based expiration | >10K unique sessions |
| No rate limiting on tool calls | Resource exhaustion, process table overflow | Add bounded request queue, per-tool concurrency limits | >100 concurrent requests |
| Synchronous JSON parsing on every stream line | High CPU on JSON parse, increased latency | Batch parse, use streaming JSON parser (serde_json::StreamDeserializer) | >1MB/s output throughput |
| String accumulation under mutex | Lock contention, allocation churn | Use lock-free ring buffer or mpsc channel for line aggregation | >1000 lines/sec per stream |
| Unbounded retry loops | Cost explosion, API quota exhaustion | Cap retry attempts (3-5), exponential backoff with max delay (60s) | First production incident |
| No subprocess pooling | Process spawn overhead dominates latency | Reuse long-lived agent processes with session reset protocol | >10 req/sec sustained |

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Unsanitized prompt injection into CLI args | Command injection if prompt contains `--flag=evil` sequences | Validate prompt doesn't start with `-`; use `--` delimiter to terminate flags; escape shell metacharacters |
| Passing user data to agent without context isolation | Cross-user data leakage via agent context window | Use session sandboxing; clear context between users; never reuse agent process across security boundaries |
| Trusting LLM-generated file paths | Path traversal attacks (`../../etc/passwd`) | Canonicalize paths, validate they're within allowed directory tree, reject symlinks |
| Exposing internal tools via MCP without authz | Privilege escalation (user calls admin-only tools) | Implement tool-level authorization; check caller identity before exposing sensitive tools |
| Logging full prompts with PII | Data breach via log aggregation systems | Redact sensitive fields; use structured logging with PII markers; implement log retention limits |
| Subprocess inheriting parent environment | Credentials leak to untrusted agent code | Explicitly clear `env_clear()` and set only required variables; audit what agent can access |
| No validation of extracted data before downstream use | SQL injection, XSS in extracted content | Treat LLM output as untrusted user input; sanitize before database/UI use |

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Generic error messages ("Agent failed") | User has no actionable information to retry | Include validation errors, cost incurred, partial output, retry suggestion |
| No progress indication for long operations | User assumes system hung, force-kills process | Stream progress events: "Validating schema...", "Processing file 3/10..." |
| Retrying silently without user visibility | Unexpected latency, cost uncertainty | Log retry attempts at INFO level, expose retry count in response metadata |
| No cost estimation before execution | Bill shock when agent uses expensive model | Pre-flight cost estimate based on prompt size, warn if >threshold |
| Truncating error output | Critical error details lost ("stderr: Process fail...") | Return full stderr (bounded), provide link to complete logs |
| No schema validation feedback loop | User gets "invalid JSON" without knowing why | Return detailed validation errors with line numbers, field paths, expected vs actual |
| Requiring perfect JSON on first attempt | Agent fails, user must manually retry | Implement automatic retry with validation feedback to agent |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Subprocess Management:** Often missing graceful shutdown (SIGTERM before SIGKILL) — verify timeout path kills process AND awaits cleanup
- [ ] **Agent Containment:** Often missing builtin tool restrictions — verify `--builtin-tools=none` flag is passed and agent cannot escape to file system
- [ ] **Structured Extraction:** Often missing retry loop — verify validation failures feed back to agent with specific errors
- [ ] **Stream Parsing:** Often missing EOF/truncation handling — verify last line of output is captured even if process exits immediately
- [ ] **Channel Cleanup:** Often missing receiver drop detection — verify sender handles `Err(SendError)` when receiver is gone
- [ ] **Error Propagation:** Often missing `.await??` on JoinHandles — verify task panics surface as errors, not silent failures
- [ ] **Session Isolation:** Often missing cross-session validation — verify sessions cannot read each other's directories
- [ ] **Cost Control:** Often missing retry limit — verify agent cannot spiral into infinite retry loop burning API quota
- [ ] **Tool Allowlisting:** Often missing "only these tools" enforcement — verify tool invocations reject tools not on allowlist
- [ ] **Schema Validation:** Often missing version detection — verify schema changes between versions are detected and fail loudly

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Zombie processes accumulated | LOW | Run `ps aux \| grep defunct` to identify zombies; kill parent process to release; add zombie cleanup to startup script |
| Orphaned async tasks leaking memory | MEDIUM | Restart server to reclaim resources; add metrics to detect leak earlier; implement graceful shutdown with task abort |
| Unbounded channel OOM | HIGH | Immediate restart to restore service; review logs for triggering prompt; add channel depth metric and alerting |
| LLM schema drift after model update | LOW | Downgrade to previous model version; add schema validation tests to CI; contact provider about regression |
| MCP tool poisoning | MEDIUM | Remove malicious MCP server from config; audit tool invocation logs for data exfiltration; rotate secrets |
| Stream race condition data loss | LOW | Retry operation; add delay between process exit and stream close for affected CLI version; test with high concurrency |
| Agent containment bypass | MEDIUM | Review session directory for unauthorized modifications; strengthen builtin tool restrictions; add tool invocation monitoring |
| Session directory disk exhaustion | LOW | Implement manual cleanup script; add cron job for TTL-based removal; monitor disk usage |
| Retry loop cost spiral | MEDIUM | Kill runaway process; set retry limit in config; add cost budget per request with hard cutoff |
| Subprocess table exhaustion | LOW | Restart server; add ulimit monitoring; implement subprocess pooling to reduce spawning |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Zombie processes from incomplete cleanup | Phase 1: Error Handling & Resource Management | Integration test spawns 100 processes with timeouts, verifies no zombies remain |
| Orphaned async tasks | Phase 1: Error Handling & Resource Management | Abort test partway through stream, verify all spawned tasks are cancelled |
| Unbounded channel memory explosion | Phase 1: Error Handling & Resource Management | Benchmark with slow consumer, verify memory stays bounded |
| LLM schema drift | Phase 2: Agent Containment & Structured Extraction | Test suite with deliberate schema violations, verify retry loop corrects them |
| MCP tool poisoning | Phase 3: Security & MCP Hardening | Malicious MCP server test, verify tool is rejected or sandboxed |
| Stream race condition | Phase 1: Error Handling & Resource Management | Stress test with 1000 concurrent short-lived processes, verify no truncation |
| Agent containment bypass | Phase 2: Agent Containment & Structured Extraction | Provide only MCP tools, verify agent cannot use file system or bash |
| Silent JSON parse failures | Phase 1: Error Handling & Resource Management | Inject malformed JSON in stream, verify error is logged and surfaced |
| Session directory accumulation | Phase 4: Observability & Production Hardening | Run server for 24h with rotating sessions, verify cleanup occurs |
| Retry loop cost spiral | Phase 2: Agent Containment & Structured Extraction | Configure max retries=3, verify agent stops after 3 attempts |

## Sources

### Subprocess Management
- [tokio::process - Rust](https://docs.rs/tokio/latest/tokio/process/)
- [tokio::process::Command leaves zombies when child future is dropped · Issue #2685](https://github.com/tokio-rs/tokio/issues/2685)
- [Managing Processes in Rust | DanielMSchmidt.de](https://danielmschmidt.de/posts/2023-03-23-managing-processes-in-rust/)
- [Interrupting an (Interactive)Process should attempt to kill the process gracefully · Issue #13230](https://github.com/pantsbuild/pants/issues/13230)

### Async Task Management
- [Spawning | Tokio - An asynchronous Rust runtime](https://tokio.rs/tokio/tutorial/spawning)
- [spawn in tokio::task - Rust](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html)
- [Cancellation in rust async](https://news.ycombinator.com/item?id=45475580)
- [A practical guide to async in Rust - LogRocket Blog](https://blog.logrocket.com/a-practical-guide-to-async-in-rust/)

### Channel Backpressure
- [Channels | Tokio - An asynchronous Rust runtime](https://tokio.rs/tokio/tutorial/channels)
- [Unbounded MPSC does not free large amounts of memory · Issue #4321](https://github.com/tokio-rs/tokio/issues/4321)
- [Mastering Tokio: Building mpsc Channels for Maximum Throughput](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260)
- [Improve "Backpressure and bounded channels" · Issue #449](https://github.com/tokio-rs/website/issues/449)

### LLM Structured Output
- [LLM Output Parsing and Structured Generation Guide](https://tetrate.io/learn/ai/llm-output-parsing-structured-generation)
- [The guide to structured outputs and function calling with LLMs](https://agenta.ai/blog/the-guide-to-structured-outputs-and-function-calling-with-llms)
- [Handle Invalid JSON Output for Small Size LLM](https://watchsound.medium.com/handle-invalid-json-output-for-small-size-llm-a2dc455993bd)
- [Structured Output Generation in LLMs: JSON Schema and Grammar-Based Decoding](https://medium.com/@emrekaratas-ai/structured-output-generation-in-llms-json-schema-and-grammar-based-decoding-6a5c58b698a6)

### MCP Security
- [Model Context Protocol (MCP): Understanding security risks and controls](https://www.redhat.com/en/blog/model-context-protocol-mcp-understanding-security-risks-and-controls)
- [New Prompt Injection Attack Vectors Through MCP Sampling](https://unit42.paloaltonetworks.com/model-context-protocol-attack-vectors/)
- [6 challenges of using the Model Context Protocol (MCP)](https://www.merge.dev/blog/mcp-challenges)
- [Everything Wrong with MCP](https://blog.sshh.io/p/everything-wrong-with-mcp)
- [The Hidden Dangers of MCP: Emerging Threats for the Novel Protocol](https://www.jit.io/resources/app-security/the-hidden-dangers-of-mcp-emerging-threats-for-the-novel-protocol)

### Agent Containment & Sandboxing
- [SandCell: Sandboxing Rust Beyond Unsafe Code](https://arxiv.org/abs/2509.24032)
- [wasm_sandbox - Rust](https://docs.rs/wasm-sandbox)
- [Hydravisor Dev Diary: Wrestling AI to Build Secure Rust Sandboxes](https://dev.to/trippingkelsea/hydravisor-dev-diary-wrestling-ai-to-build-secure-rust-sandboxes-2npm)

### Retry Logic & Error Recovery
- [Implementing Retry Mechanisms for LLM Calls](https://apxml.com/courses/prompt-engineering-llm-application-development/chapter-7-output-parsing-validation-reliability/implementing-retry-mechanisms)
- [Retries, fallbacks, and circuit breakers in LLM apps: what to use when](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)
- [Mastering Retry Logic Agents: A Deep Dive into 2025 Best Practices](https://sparkco.ai/blog/mastering-retry-logic-agents-a-deep-dive-into-2025-best-practices)
- [Backoff and Retry Strategies for LLM Failures](https://palospublishing.com/backoff-and-retry-strategies-for-llm-failures/)

### Stream Parsing & Data Corruption
- [A case about parsing errors](https://traffaillac.github.io/content/parsing.html)
- [fix: ILP parser on string partial parsing edge cases · Pull Request #1409](https://github.com/questdb/questdb/pull/1409)
- [Preventing and Fixing Bad Data in Event Streams — Part 2](https://medium.com/confluent/preventing-and-fixing-bad-data-in-event-streams-part-2-526e459c7c6f)

### Testing & Mocking
- [A guide to testing and mocking in Rust](https://danielbunte.medium.com/a-guide-to-testing-and-mocking-in-rust-a73d022b4075)
- [Mastering Asynchronous Testing in Rust](https://moldstud.com/articles/p-mastering-asynchronous-testing-in-rust-strategies-and-frameworks-with-tokio)
- [GitHub - AltSysrq/rusty-fork: Run Rust tests in isolated subprocesses](https://github.com/altsysrq/rusty-fork)

### Rig Framework
- [Rig - Build Powerful LLM Applications in Rust](https://rig.rs/)
- [rig - Rust](https://docs.rs/rig-core/latest/rig/)
- [GitHub - 0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig)

### Existing Codebase Analysis
- `/home/pnod/dev/projects/rig-cli/.planning/codebase/CONCERNS.md` - Tech debt and known bugs inventory
- `/home/pnod/dev/projects/rig-cli/.planning/codebase/ARCHITECTURE.md` - System architecture patterns
- `/home/pnod/dev/projects/rig-cli/.planning/PROJECT.md` - Project context and requirements

---
*Pitfalls research for: CLI-agent-wrapping projects with Rig provider integration*
*Researched: 2026-02-01*
