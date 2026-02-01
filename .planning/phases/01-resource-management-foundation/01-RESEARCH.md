# Phase 1: Resource Management Foundation - Research

**Researched:** 2026-02-01
**Domain:** Tokio async subprocess execution, resource lifecycle management
**Confidence:** HIGH

## Summary

Research investigated how to implement robust subprocess execution in Tokio with bounded resources and zero leaks. The standard approach uses bounded mpsc channels for stream communication, explicit JoinHandle tracking for task lifecycle, and careful subprocess cleanup with graceful shutdown sequences. Current implementation has critical flaws: unbounded channels (line 13 in process.rs), `.expect()` calls (lines 30-31), no JoinHandle tracking, and no graceful SIGTERM/SIGKILL sequence.

The Tokio ecosystem provides solid primitives for all requirements. Bounded channels with backpressure are built-in. JoinSet simplifies multi-task tracking with automatic abort-on-drop. Child process cleanup requires explicit `wait()` calls to prevent zombies. Stream draining patterns are well-documented but require careful EOF handling. The nix crate enables SIGTERM before SIGKILL on Unix.

**Primary recommendation:** Replace unbounded channels with bounded (100-1000 capacity), spawn separate tasks for stdout/stderr reading tracked via JoinSet, implement SIGTERM→wait→SIGKILL sequence using nix crate, and propagate all errors via thiserror enums with rich context.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.0+ | Async runtime, process spawning, channels | De facto async runtime for Rust; subprocess support built-in |
| thiserror | 1.0 | Structured error types | Standard for library errors; composable with #[from] and #[source] |
| nix | 0.29+ | Unix signal sending (SIGTERM) | Standard Unix systems programming crate; required for custom signals |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio-stream | 0.1 | Stream utilities (StreamExt) | Already in project; useful for stream combinator patterns |
| futures | 0.3 | Future combinators | Already in project; useful for joining/selecting futures |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio::sync::mpsc | crossbeam-channel | crossbeam not async-aware, requires blocking bridge |
| JoinSet | Manual Vec<JoinHandle> | JoinSet provides abort_all() and automatic cleanup on drop |
| nix::sys::signal::kill | libc::kill directly | nix provides type-safe wrappers around libc |

**Installation:**
```bash
# nix crate with signal feature (others already in dependencies)
cargo add nix --features signal
```

## Architecture Patterns

### Recommended Project Structure
```
claudecode-adapter/src/
├── process.rs          # Subprocess execution, channel setup, task spawning
├── error.rs            # ClaudeError enum with context-rich variants
├── types.rs            # RunConfig, RunResult, StreamEvent
└── cleanup.rs          # (NEW) Graceful shutdown, SIGTERM/SIGKILL logic
```

### Pattern 1: Bounded Channels with Separate Reader Tasks
**What:** Spawn independent tasks for stdout/stderr reading with bounded channels to parent
**When to use:** Always for subprocess stream handling - prevents deadlock and enables concurrent reading
**Example:**
```rust
// Source: Tokio subprocess patterns + bounded channel docs
use tokio::sync::mpsc;
use tokio::io::{AsyncBufReadExt, BufReader};

const CHANNEL_CAPACITY: usize = 100; // Bounded capacity

pub async fn run_with_channels(mut child: Child) -> Result<()> {
    let (stdout_tx, mut stdout_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel(CHANNEL_CAPACITY);

    let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
    let stderr = child.stderr.take().ok_or(Error::NoStderr)?;

    // Spawn reader tasks - tracked via JoinSet
    let mut tasks = JoinSet::new();

    tasks.spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Some(line) = reader.next_line().await? {
            // Backpressure: blocks here if channel full
            stdout_tx.send(line).await?;
        }
        Ok::<_, Error>(())
    });

    tasks.spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Some(line) = reader.next_line().await? {
            stderr_tx.send(line).await?;
        }
        Ok::<_, Error>(())
    });

    // Process streams and wait for child
    // ... (see Pattern 3 for timeout handling)
}
```

### Pattern 2: JoinSet for Task Lifecycle Tracking
**What:** Use tokio::task::JoinSet to track all spawned tasks; automatic abort on drop
**When to use:** When spawning multiple tasks that must be cleaned up together (stdout, stderr, timeout)
**Example:**
```rust
// Source: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html
use tokio::task::JoinSet;

let mut tasks = JoinSet::new();

// Spawn tasks - handles are tracked automatically
tasks.spawn(read_stdout_task);
tasks.spawn(read_stderr_task);

// On timeout or error, dropping tasks aborts all
// Alternatively, explicitly abort:
tasks.abort_all();

// Await all tasks, handling errors
while let Some(result) = tasks.join_next().await {
    match result {
        Ok(Ok(output)) => { /* task succeeded */ },
        Ok(Err(e)) => { /* task returned error */ },
        Err(e) if e.is_cancelled() => { /* task was aborted */ },
        Err(e) if e.is_panic() => { /* task panicked */ },
        Err(e) => { /* other join error */ },
    }
}
```

### Pattern 3: SIGTERM → Grace Period → SIGKILL
**What:** Send SIGTERM, wait for grace period, then SIGKILL if still alive
**When to use:** Always for subprocess cleanup to allow graceful shutdown
**Example:**
```rust
// Source: https://docs.rs/nix/latest/nix/sys/signal/fn.kill.html + tokio Child docs
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use tokio::time::{sleep, Duration};

const GRACE_PERIOD: Duration = Duration::from_secs(5);

async fn graceful_shutdown(child: &mut Child) -> Result<()> {
    let pid = Pid::from_raw(child.id().ok_or(Error::NoPid)? as i32);

    // Send SIGTERM
    kill(pid, Signal::SIGTERM).map_err(|e| Error::SignalFailed(e))?;

    // Wait for grace period with timeout
    match tokio::time::timeout(GRACE_PERIOD, child.wait()).await {
        Ok(Ok(status)) => {
            // Process exited gracefully
            Ok(())
        },
        Ok(Err(e)) => Err(Error::WaitFailed(e)),
        Err(_timeout) => {
            // Grace period exceeded - send SIGKILL
            child.kill().await?; // SIGKILL + wait
            child.wait().await?; // Reap zombie
            Ok(())
        }
    }
}
```

### Pattern 4: Complete Stream Draining with Memory Limit
**What:** Read all data from streams until EOF, but enforce hard limit to prevent OOM
**When to use:** Always when capturing subprocess output
**Example:**
```rust
// Source: https://tokio.rs/tokio/tutorial/io + memory limit best practice
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB

async fn drain_stream_bounded(
    mut reader: BufReader<impl AsyncRead + Unpin>,
    tx: mpsc::Sender<String>
) -> Result<()> {
    let mut total_bytes = 0;
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let line_bytes = line.len();

        if total_bytes + line_bytes > MAX_OUTPUT_BYTES {
            // Hit limit - send truncation warning and stop
            tx.send(format!("[TRUNCATED: output exceeded {} bytes]", MAX_OUTPUT_BYTES))
                .await
                .ok(); // Ignore send failure
            break;
        }

        total_bytes += line_bytes;

        // May block if channel full (backpressure)
        tx.send(line).await?;
    }

    // EOF reached or limit hit
    Ok(())
}
```

### Pattern 5: Rich Context Errors with thiserror
**What:** Structured error enum with pipeline stage, PID, elapsed time, partial output
**When to use:** All error paths - enables caller to handle specific failure modes
**Example:**
```rust
// Source: https://github.com/dtolnay/thiserror + subprocess context patterns
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum SubprocessError {
    #[error("Subprocess timed out after {elapsed:?} (PID: {pid})")]
    Timeout {
        elapsed: Duration,
        pid: u32,
        #[source]
        partial_output: String,
    },

    #[error("Subprocess exited with code {exit_code} (PID: {pid}, elapsed: {elapsed:?})")]
    NonZeroExit {
        exit_code: i32,
        pid: u32,
        elapsed: Duration,
        stdout: String,
        stderr: String,
    },

    #[error("Failed to spawn subprocess: {stage}")]
    SpawnFailed {
        stage: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Stream reader task failed: {stage}")]
    StreamFailed {
        stage: String,
        #[source]
        source: tokio::task::JoinError,
    },

    #[error("Failed to send {signal} to PID {pid}")]
    SignalFailed {
        signal: String,
        pid: u32,
        #[source]
        source: nix::errno::Errno,
    },
}
```

### Anti-Patterns to Avoid
- **Unbounded channels:** Can cause OOM if producer outpaces consumer; always use bounded with explicit capacity
- **Dropping JoinHandle without await:** Task continues running, becomes unkillable leak; use JoinSet for automatic tracking
- **Only SIGKILL for cleanup:** Prevents graceful shutdown; always SIGTERM first with timeout
- **Not checking EOF (Ok(0)):** Creates infinite loop burning CPU; always `break` on zero bytes read
- **`.expect()` in stream handling:** Panics crash entire process; return `Result` and propagate errors
- **Shared mutable state with Arc<Mutex>:** Holding mutex across `.await` causes deadlock; use message passing (channels) instead

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-task tracking | Vec<JoinHandle> + manual abort loop | tokio::task::JoinSet | Automatic abort on drop, handles panics, provides join_next() iterator |
| Subprocess timeout | spawn + timer + kill logic | tokio::time::timeout wrapper | Handles cancellation correctly, integrates with select! |
| Unix signals | Unsafe libc::kill calls | nix::sys::signal::kill | Type-safe Signal enum, proper error handling, Pid wrapper |
| Stream EOF detection | Manual read loop with flags | BufReader::lines() + next_line() | Returns None on EOF, handles partial reads, buffer management |
| Error context chaining | format! + String errors | thiserror with #[source] | Preserves error chain for debugging, enables error downcasting |

**Key insight:** Tokio's ecosystem is mature with battle-tested primitives. Custom implementations miss edge cases (partial writes, spurious wakeups, platform differences). Use provided abstractions.

## Common Pitfalls

### Pitfall 1: Zombie Processes from Dropped Child Handles
**What goes wrong:** Process exits but OS resources not released because parent never called wait()
**Why it happens:** Tokio's best-effort reaping is not guaranteed; dropping Child doesn't immediately reap
**How to avoid:** Always explicitly call `child.wait().await` or `child.kill().await` before dropping
**Warning signs:** `ps aux | grep defunct` shows zombie processes accumulating

### Pitfall 2: Stream Reader Blocking on Full Buffer While Writer Waits
**What goes wrong:** Subprocess writes to stdout, fills pipe buffer, blocks waiting for parent to read. Parent is blocked writing to stdin. Classic deadlock.
**Why it happens:** Sequential operations: write stdin, then read stdout. Subprocess output buffer (typically 64KB on Linux) fills up.
**How to avoid:** Spawn separate tasks for reading stdout/stderr; never read/write sequentially on main task
**Warning signs:** Process hangs indefinitely; strace shows futex waits on both parent and child

### Pitfall 3: Channel Send Deadlock in Single-Threaded Runtime
**What goes wrong:** Reader task does `tx.send(line).await` when channel is full, but receiver loop never runs because runtime has only one thread and it's blocked in sender
**Why it happens:** Bounded channel full, sender awaits capacity, but receiver can't poll because same thread
**How to avoid:** Use multi-threaded runtime (tokio::runtime::Builder::multi_thread) or ensure receiver polls concurrently (spawn receiver in separate task)
**Warning signs:** Process hangs with CPU at 0%; only happens with current_thread runtime

### Pitfall 4: Task Leak from Forgetting to Await JoinHandle
**What goes wrong:** Spawn tasks for readers, drop JoinHandles without awaiting, tasks become detached and unkillable
**Why it happens:** JoinHandle::drop doesn't abort task, just detaches it; task continues in background forever
**How to avoid:** Use JoinSet for automatic tracking; on timeout, call abort_all() before dropping
**Warning signs:** Process count grows over time; tasks visible in tracing but can't be cancelled

### Pitfall 5: UTF-8 Panic from Mid-Character Truncation
**What goes wrong:** Enforcing byte limit on accumulated String, truncate at arbitrary byte, next char straddles boundary, UTF-8 validation panics
**Why it happens:** String::truncate panics if index not on char boundary; CLI might emit multibyte UTF-8
**How to avoid:** Check byte vs. character boundaries (use `is_char_boundary()`) or stop accumulating whole lines once limit approached
**Warning signs:** Panic with "byte index not a char boundary" from random CLI output

### Pitfall 6: Stderr EOF Hang (Tokio Issue #2363)
**What goes wrong:** Reading from stderr with `read().await` blocks for ~15 minutes after subprocess exits, even though stdout EOF detected immediately
**Why it happens:** Platform-specific Tokio reactor bug with certain subprocess stderr patterns; timing-sensitive
**How to avoid:** Use separate tasks for stdout/stderr reading; apply overall timeout to subprocess execution (tokio::time::timeout)
**Warning signs:** Stderr read never returns Ok(0); only stderr hangs, stdout is fine; Linux-specific

### Pitfall 7: Non-Zero Exit Doesn't Mean Failure
**What goes wrong:** Treat any non-zero exit code as error, but some CLIs exit 1 on warnings while still producing valid output
**Why it happens:** Unix convention: 0 = success. But real-world CLIs are inconsistent.
**How to avoid:** Check if expected output received (e.g., valid MCP tool call) BEFORE checking exit code; exit code is secondary signal
**Warning signs:** Valid extractions rejected because CLI printed warning to stderr and exited 1

## Code Examples

Verified patterns from official sources:

### Complete Subprocess Execution with All Patterns
```rust
// Integrates: bounded channels, JoinSet, timeout, graceful shutdown, error propagation
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio::io::{AsyncBufReadExt, BufReader};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

const CHANNEL_CAPACITY: usize = 100;
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10MB
const TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes
const GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn run_subprocess(
    cmd: &str,
    args: Vec<String>,
) -> Result<SubprocessResult, SubprocessError> {
    let start = std::time::Instant::now();

    // Spawn subprocess with piped streams
    let mut child = Command::new(cmd)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| SubprocessError::SpawnFailed {
            stage: "subprocess spawn".into(),
            source: e,
        })?;

    let pid = child.id().ok_or(SubprocessError::NoPid)?;

    // Take streams (or error)
    let stdout = child.stdout.take().ok_or(SubprocessError::NoStdout)?;
    let stderr = child.stderr.take().ok_or(SubprocessError::NoStderr)?;

    // Bounded channels for stream communication
    let (stdout_tx, mut stdout_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (stderr_tx, mut stderr_rx) = mpsc::channel(CHANNEL_CAPACITY);

    // Track tasks with JoinSet
    let mut tasks = JoinSet::new();

    // Spawn stdout reader
    tasks.spawn(async move {
        drain_stream_bounded(BufReader::new(stdout), stdout_tx).await
    });

    // Spawn stderr reader
    tasks.spawn(async move {
        drain_stream_bounded(BufReader::new(stderr), stderr_tx).await
    });

    // Accumulate output with limits
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    // Main execution with timeout
    let execution = async {
        // Read from channels until both EOF
        while !stdout_done || !stderr_done {
            tokio::select! {
                Some(line) = stdout_rx.recv(), if !stdout_done => {
                    stdout_lines.push(line);
                }
                Some(line) = stderr_rx.recv(), if !stderr_done => {
                    stderr_lines.push(line);
                }
                else => break,
            }
        }

        // Wait for child to exit
        let status = child.wait().await
            .map_err(|e| SubprocessError::WaitFailed(e))?;

        // Await reader tasks (should already be done)
        tasks.abort_all(); // Cleanup any stragglers
        while let Some(result) = tasks.join_next().await {
            // Check for task errors
            result.map_err(|e| SubprocessError::StreamFailed {
                stage: "stream reader".into(),
                source: e,
            })??;
        }

        Ok::<_, SubprocessError>((status, stdout_lines, stderr_lines))
    };

    // Apply timeout
    match timeout(TIMEOUT, execution).await {
        Ok(Ok((status, stdout, stderr))) => {
            // Success path
            Ok(SubprocessResult {
                exit_code: status.code().unwrap_or(-1),
                stdout: stdout.join("\n"),
                stderr: stderr.join("\n"),
                duration: start.elapsed(),
            })
        }
        Ok(Err(e)) => Err(e),
        Err(_timeout) => {
            // Timeout path - graceful shutdown
            let pid_nix = Pid::from_raw(pid as i32);

            // Send SIGTERM
            let _ = kill(pid_nix, Signal::SIGTERM);

            // Wait for grace period
            if timeout(GRACE_PERIOD, child.wait()).await.is_err() {
                // Still alive - SIGKILL
                let _ = child.kill().await;
                let _ = child.wait().await;
            }

            // Abort reader tasks
            tasks.abort_all();

            // Return partial output with timeout error
            Err(SubprocessError::Timeout {
                elapsed: start.elapsed(),
                pid,
                partial_output: format!(
                    "stdout: {}\nstderr: {}",
                    stdout_lines.join("\n"),
                    stderr_lines.join("\n")
                ),
            })
        }
    }
}

async fn drain_stream_bounded(
    reader: BufReader<impl AsyncRead + Unpin>,
    tx: mpsc::Sender<String>
) -> Result<(), SubprocessError> {
    let mut total_bytes = 0;
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if total_bytes + line.len() > MAX_OUTPUT_BYTES {
            let _ = tx.send("[TRUNCATED]".into()).await;
            break;
        }

        total_bytes += line.len();

        if tx.send(line).await.is_err() {
            // Receiver dropped - parent cancelled
            break;
        }
    }

    Ok(())
}
```

### Backpressure Strategies for Bounded Channels
```rust
// Source: https://tokio.rs/tokio/tutorial/channels

// Strategy 1: Block (default) - sender waits when full
tx.send(value).await?; // Blocks until capacity available

// Strategy 2: Try-send (non-blocking) - returns error if full
match tx.try_send(value) {
    Ok(()) => {},
    Err(mpsc::error::TrySendError::Full(_)) => {
        // Handle backpressure: drop, log, retry with backoff
    },
    Err(mpsc::error::TrySendError::Closed(_)) => {
        // Receiver gone
    },
}

// Strategy 3: Send with timeout
match timeout(Duration::from_millis(100), tx.send(value)).await {
    Ok(Ok(())) => {},
    Ok(Err(_)) => { /* channel closed */ },
    Err(_) => { /* send timed out - receiver too slow */ },
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| UnboundedSender | Bounded mpsc::Sender | Always standard | Prevents OOM; forces backpressure design |
| Manual JoinHandle tracking | JoinSet with abort_all() | Tokio 1.19 (2022) | Auto-cleanup on drop; simpler error handling |
| SIGKILL only | SIGTERM → grace → SIGKILL | Best practice (2020+) | Allows subprocess cleanup; prevents data loss |
| Panic on errors | Result with thiserror | Rust ecosystem norm | Library never panics; caller handles errors |
| Separate stdout/stderr accumulation | Concurrent with select! | Tokio async idiom | Prevents deadlock; enables real-time processing |

**Deprecated/outdated:**
- `tokio::sync::mpsc::unbounded_channel()`: Still exists but actively discouraged for production; use bounded
- `child.kill()` without `wait()`: Leaves zombies on Unix; always follow with wait() to reap
- `std::sync::Mutex` in async code: Causes deadlock if held across `.await`; use tokio::sync::Mutex or channels

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal Channel Capacity**
   - What we know: Tokio docs suggest 32-100; examples use 100; must be >= 1
   - What's unclear: Performance impact of different sizes for line-based JSONL streams
   - Recommendation: Start with 100 (middle ground); can tune later with benchmarks

2. **Grace Period Duration Between SIGTERM and SIGKILL**
   - What we know: Production examples show 1-60 seconds; depends on CLI cleanup needs
   - What's unclear: How long Claude Code / Codex take to flush state on SIGTERM
   - Recommendation: Start with 5 seconds (conservative); monitor in production

3. **Maximum Output Buffer Size**
   - What we know: Need hard limit to prevent OOM from runaway CLI
   - What's unclear: Typical JSONL protocol message sizes; what's reasonable before truncation
   - Recommendation: 10MB (allows ~100k lines of protocol messages); return truncation warning

4. **Windows SIGTERM Equivalent**
   - What we know: SIGTERM is Unix-specific; Windows has TerminateProcess / GenerateConsoleCtrlEvent
   - What's unclear: How to implement graceful shutdown cross-platform with same semantics
   - Recommendation: Phase 1 implements Unix behavior (nix crate); Phase 6 (Platform) handles Windows

5. **Backpressure Strategy Choice**
   - What we know: Block (default) prevents message loss; drop_oldest prevents sender blocking
   - What's unclear: Whether blocking sender is acceptable or if we need non-blocking try_send
   - Recommendation: Use blocking send (default) - reader tasks should keep up with line-based output

## Sources

### Primary (HIGH confidence)
- [tokio::sync::mpsc::channel](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html) - Bounded channel semantics, backpressure behavior
- [tokio::task::JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) - Abort semantics, drop behavior, tracking patterns
- [tokio::task::JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html) - Multi-task tracking, automatic abort on drop
- [tokio::process::Child](https://docs.rs/tokio/latest/tokio/process/struct.Child.html) - Kill/wait semantics, zombie prevention
- [tokio::io::BufReader](https://docs.rs/tokio/latest/tokio/io/struct.BufReader.html) - Buffer size (8KB), lines() method, memory usage
- [nix::sys::signal::kill](https://docs.rs/nix/latest/nix/sys/signal/fn.kill.html) - Signal sending API, Pid conversion
- [Tokio Channels Tutorial](https://tokio.rs/tokio/tutorial/channels) - Bounded capacity recommendations, backpressure patterns
- [Tokio I/O Tutorial](https://tokio.rs/tokio/tutorial/io) - Stream draining, EOF handling, preventing data loss
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown) - Timeout patterns, cleanup sequences
- [thiserror GitHub](https://github.com/dtolnay/thiserror) - Error enum patterns, #[source] and #[from] usage

### Secondary (MEDIUM confidence)
- [Rust tokio task cancellation patterns](https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/) - Detailed cancellation patterns, gotchas, best practices
- [Tokio subprocess zombie processes issue #2685](https://github.com/tokio-rs/tokio/issues/2685) - Zombie process creation conditions, prevention
- [Tokio subprocess stderr hang issue #2363](https://github.com/tokio-rs/tokio/issues/2363) - Platform-specific stderr blocking bug
- [Mastering Tokio mpsc Channels](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260) - Capacity recommendations, throughput patterns
- [Error Handling in Rust 2025 Guide](https://markaicode.com/rust-error-handling-2025-guide/) - thiserror vs anyhow, library error patterns

### Tertiary (LOW confidence)
- Various Rust forums and Stack Overflow discussions on tokio channels, subprocess handling
- Community blog posts on graceful shutdown patterns (need official docs verification)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries are official Tokio ecosystem crates with extensive documentation
- Architecture: HIGH - Patterns verified against official Tokio docs and source code issues
- Pitfalls: HIGH - Documented in official GitHub issues and Tokio tutorials
- Open questions: MEDIUM - Require empirical testing or CLI-specific knowledge

**Research date:** 2026-02-01
**Valid until:** 2026-04-01 (60 days - stable domain, slow-moving APIs)
