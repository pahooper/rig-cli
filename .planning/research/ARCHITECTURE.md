# Architecture Research: Production Rig CLI Provider

**Domain:** Rust CLI subprocess wrapper with structured extraction and streaming
**Researched:** 2026-02-01
**Confidence:** MEDIUM-HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   MCP Client Layer (External)                    │
│              Claude Code / Codex / OpenCode                      │
└────────────────────────┬────────────────────────────────────────┘
                         │ stdio (MCP protocol)
┌────────────────────────┴────────────────────────────────────────┐
│                     MCP Server Layer                             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │   RigMcpHandler (rmcp ServerHandler impl)                │   │
│  │   - list_tools → tool definitions                        │   │
│  │   - call_tool → dispatch to ToolSet                      │   │
│  └───────────────────┬──────────────────────────────────────┘   │
├──────────────────────┴──────────────────────────────────────────┤
│                  Tool Orchestration Layer                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ ClaudeTool  │  │  CodexTool   │  │ OpenCodeTool │           │
│  │ (Rig Tool)  │  │  (Rig Tool)  │  │  (Rig Tool)  │           │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                │                 │                    │
├─────────┴────────────────┴─────────────────┴────────────────────┤
│              CompletionModel Implementation Layer                │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ClaudeModel  │  │  CodexModel  │  │OpenCodeModel │           │
│  │ (Rig trait) │  │  (Rig trait) │  │  (Rig trait) │           │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                │                 │                    │
├─────────┴────────────────┴─────────────────┴────────────────────┤
│                   CLI Adapter Layer                              │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ ClaudeCli   │  │  CodexCli    │  │ OpenCodeCli  │           │
│  │ - run()     │  │  - run()     │  │  - run()     │           │
│  │ - stream()  │  │  - stream()  │  │  - stream()  │           │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                │                 │                    │
└─────────┴────────────────┴─────────────────┴────────────────────┘
          │                │                 │
     ┌────┴────┐      ┌────┴────┐      ┌────┴────┐
     │ claude  │      │  codex  │      │opencode │
     │  (CLI)  │      │  (CLI)  │      │  (CLI)  │
     └─────────┘      └─────────┘      └─────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **MCP Server** | Protocol translation between RMCP and Rig ToolSet | `RigMcpHandler` implementing `rmcp::ServerHandler` |
| **Provider Adapters** | Implement Rig's `CompletionModel` trait for CLI tools | Async functions wrapping CLI execution, stream parsing |
| **CLI Adapters** | Subprocess lifecycle, argument construction, output parsing | Tokio process spawning, line-by-line stream reading |
| **Session Manager** | Isolated temporary directories per session ID | HashMap of Arc<TempDir> with lazy creation |
| **Error Aggregation** | Hierarchical error mapping across layers | `thiserror` enums with `#[from]` conversions |

## Production-Hardened Architecture

### Extraction Loop Pattern

**Current State (Prototype):**
```rust
// claudecode-adapter/src/process.rs (lines 41-59)
tokio::spawn(async move {
    let mut reader = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if format == Some(OutputFormat::StreamJson) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                // Parse succeeded, emit event
                if let Ok(event) = serde_json::from_value::<StreamEvent>(val) {
                    let _ = tx.send(event);
                }
            }
            // Silent failure if JSON parse fails
        }
        s.push_str(&line);
    }
});
```

**Production Pattern (Recommended):**
```rust
// extraction_loop.rs (hypothetical)
struct ExtractionLoop {
    retry_policy: ExponentialBackoff,
    validator: Box<dyn Fn(&str) -> Result<StreamEvent, ValidationError>>,
    metrics: Arc<Metrics>,
}

impl ExtractionLoop {
    async fn run(&self, stdout: ChildStdout, tx: Sender<StreamEvent>) -> Result<(), LoopError> {
        let mut reader = BufReader::new(stdout).lines();
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;

        while let Some(line) = reader.next_line().await? {
            match self.parse_with_retry(&line).await {
                Ok(event) => {
                    consecutive_errors = 0;
                    if tx.send(event).await.is_err() {
                        return Err(LoopError::ReceiverClosed);
                    }
                    self.metrics.record_success();
                }
                Err(e) => {
                    consecutive_errors += 1;
                    self.metrics.record_parse_error(&e);
                    tracing::warn!("Parse failed: {}, line: {}", e, line);

                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        return Err(LoopError::TooManyErrors(consecutive_errors));
                    }
                }
            }
        }
        Ok(())
    }

    async fn parse_with_retry(&self, line: &str) -> Result<StreamEvent, ParseError> {
        self.retry_policy.retry(|| {
            serde_json::from_str::<serde_json::Value>(line)
                .map_err(ParseError::Json)
                .and_then(|val| {
                    serde_json::from_value::<StreamEvent>(val.clone())
                        .map_err(ParseError::Schema)
                })
                .and_then(|event| {
                    (self.validator)(&serde_json::to_string(&event)?)
                        .map_err(ParseError::Validation)
                })
        }).await
    }
}
```

**Key Improvements:**
1. **Explicit error budget:** Fail fast after N consecutive parse errors
2. **Observability:** Metrics on success rate, parse failures, validation failures
3. **Validation stage:** Separate parsing from semantic validation
4. **Bounded retry:** Exponential backoff for transient errors
5. **Clear termination:** Distinguish receiver-closed from parse-failed from process-exit

### Error Propagation Patterns

**Layered Error Strategy:**

```rust
// Layer 1: CLI Adapter Errors (adapter-specific)
#[derive(Debug, Error)]
pub enum ClaudeError {
    #[error("Executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Process timed out after {0:?}")]
    Timeout(Duration),

    #[error("Process exited with code {exit_code}: {stderr}")]
    NonZeroExit { exit_code: i32, stdout: String, stderr: String },

    #[error("Stream parsing failed after {attempt} attempts: {cause}")]
    StreamParsing { attempt: u32, cause: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Layer 2: Provider Errors (cross-adapter aggregation)
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Claude adapter: {0}")]
    Claude(#[from] ClaudeError),

    #[error("Codex adapter: {0}")]
    Codex(#[from] CodexError),

    #[error("OpenCode adapter: {0}")]
    OpenCode(#[from] OpenCodeError),

    #[error("Session management: {0}")]
    Session(String),
}

// Layer 3: Rig Integration Errors (CompletionModel trait boundary)
impl From<ClaudeError> for CompletionError {
    fn from(e: ClaudeError) -> Self {
        match e {
            ClaudeError::Timeout(d) => {
                CompletionError::ProviderError(format!("Timeout after {:?}", d))
            }
            ClaudeError::NonZeroExit { exit_code, stderr, .. } => {
                CompletionError::ProviderError(format!("Exit {}: {}", exit_code, stderr))
            }
            _ => CompletionError::ProviderError(e.to_string()),
        }
    }
}
```

**Propagation Guidelines:**
- **Transient errors:** Retry at extraction loop layer (network, parse)
- **Configuration errors:** Fail fast at initialization (missing executable)
- **User errors:** Return to caller immediately (invalid arguments)
- **System errors:** Log + metric + return (OOM, file descriptor exhaustion)

### Rig 0.29 CompletionModel Integration

**Trait Requirements:**

Based on rig-core usage in the codebase and [official Rig documentation](https://docs.rs/rig-core/latest/rig/):

```rust
pub trait CompletionModel {
    type Response;               // RunResult in current implementation
    type StreamingResponse;      // () in current, should carry metadata
    type Client;                 // ClaudeCli, CodexCli, etc.

    fn make(client: &Self::Client, model: impl Into<String>) -> Self;

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError>;

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>;
}
```

**Current Implementation Pattern:**
```rust
// rig-provider/src/adapters/claude.rs (lines 46-69)
async fn completion(&self, request: CompletionRequest) -> Result<...> {
    let prompt_str = format_chat_history(&request);

    let mut config = RunConfig::default();
    if !request.tools.is_empty() {
        config.tools.allowed = Some(request.tools.iter().map(|t| t.name.clone()).collect());
    }

    let result = self.cli.run(&prompt_str, &config).await
        .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

    Ok(CompletionResponse {
        choice: OneOrMany::one(AssistantContent::text(result.stdout.clone())),
        usage: Default::default(),  // No token usage from CLI
        raw_response: result,
    })
}
```

**Production Improvements:**

1. **Structured Output Extraction:**
```rust
async fn completion(&self, request: CompletionRequest) -> Result<...> {
    // 1. Configure structured output if schema present
    let mut config = RunConfig::default();
    if let Some(schema) = request.extensions.get("json_schema") {
        config.json_schema = JsonSchema::JsonValue(schema.clone());
        config.output_format = Some(OutputFormat::Json);
    }

    // 2. Execute with retry policy
    let result = self.retry_policy.retry(|| {
        self.cli.run(&prompt_str, &config)
    }).await?;

    // 3. Validate exit code
    if result.exit_code != 0 {
        return Err(CompletionError::ProviderError(
            format!("Non-zero exit: {} - {}", result.exit_code, result.stderr)
        ));
    }

    // 4. Extract structured output
    let content = if let Some(json) = result.structured_output {
        AssistantContent::with_json(json)
    } else {
        AssistantContent::text(result.stdout)
    };

    Ok(CompletionResponse {
        choice: OneOrMany::one(content),
        usage: extract_usage(&result),  // Parse from stderr or metadata
        raw_response: result,
    })
}
```

2. **Streaming with Backpressure:**
```rust
async fn stream(&self, request: CompletionRequest) -> Result<...> {
    // Use BOUNDED channel instead of unbounded
    let (tx, rx) = tokio::sync::mpsc::channel(1000);  // Buffer 1000 events max

    let cli = self.cli.clone();
    let mut config = RunConfig::default();
    config.output_format = Some(OutputFormat::StreamJson);

    // Spawn with abort handle for cleanup
    let handle = tokio::spawn(async move {
        match cli.stream(&prompt_str, &config, tx).await {
            Ok(result) => tracing::info!("Stream completed: {:?}", result.duration_ms),
            Err(e) => tracing::error!("Stream failed: {}", e),
        }
    });

    // Attach abort handle to stream for cleanup on drop
    let stream = ReceiverStream::new(rx)
        .map(|event| match event {
            StreamEvent::Text { text } => Ok(RawStreamingChoice::Message(text)),
            StreamEvent::ToolCall { name, input } => {
                Ok(RawStreamingChoice::ToolCall(
                    RawStreamingToolCall::new(Uuid::new_v4().to_string(), name, input)
                ))
            }
            StreamEvent::Error { message } => Err(CompletionError::ProviderError(message)),
            _ => Ok(RawStreamingChoice::Message(String::new())),
        })
        .chain(futures::stream::once(async move {
            // Ensure spawned task completes
            let _ = handle.await;
            Ok(RawStreamingChoice::Message(String::new()))
        }));

    Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
}
```

### Subprocess Lifecycle Management

**Current Pattern (Fragile):**
```rust
// claudecode-adapter/src/process.rs (lines 72-91)
match timeout(config.timeout, wait_task).await {
    Ok(res) => { /* process completed */ }
    Err(_) => {
        let _ = child.kill().await;  // Kill child, but tasks may still run
        Err(ClaudeError::Timeout(config.timeout))
    }
}
```

**Production Pattern:**

Based on [Tokio process documentation](https://docs.rs/tokio/latest/tokio/process/struct.Command.html) and [TokioConf 2026 guidance](https://tokio.rs/blog/2025-09-26-announcing-tokio-conf-cfp) on lifecycle management:

```rust
struct ProcessHandle {
    child: Child,
    stdout_task: JoinHandle<Result<String, io::Error>>,
    stderr_task: JoinHandle<Result<String, io::Error>>,
}

impl ProcessHandle {
    async fn wait_with_timeout(mut self, timeout: Duration) -> Result<RunResult, ClaudeError> {
        let wait_future = async {
            // Wait for process exit
            let status = self.child.wait().await?;

            // Then wait for output tasks (they'll finish after EOF)
            let stdout = self.stdout_task.await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
            let stderr = self.stderr_task.await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;

            Ok((status, stdout, stderr))
        };

        match tokio::time::timeout(timeout, wait_future).await {
            Ok(Ok((status, stdout, stderr))) => {
                Ok(RunResult {
                    stdout,
                    stderr,
                    exit_code: status.code().unwrap_or(-1),
                    ..Default::default()
                })
            }
            Ok(Err(e)) => Err(ClaudeError::Io(e)),
            Err(_) => {
                // Timeout: kill child AND abort tasks
                let _ = self.child.kill().await;
                self.stdout_task.abort();
                self.stderr_task.abort();

                // Wait briefly for cleanup
                tokio::time::sleep(Duration::from_millis(100)).await;

                Err(ClaudeError::Timeout(timeout))
            }
        }
    }
}
```

**Key Lifecycle Principles:**
1. **Task Tracking:** Store JoinHandles, don't fire-and-forget
2. **Ordered Cleanup:** Kill child → abort tasks → brief grace period
3. **Cancellation Safety:** Tasks must handle abort gracefully
4. **Resource Bounds:** Use bounded channels to prevent memory exhaustion

### Channel Strategy (Bounded vs Unbounded)

**Current Issue (from CONCERNS.md):**
> Problem: OpenCode and Claude adapters use unbounded mpsc channels for streaming output, potentially consuming unlimited memory

**Production Recommendation:**

Based on [Tokio channels documentation](https://tokio.rs/tokio/tutorial/channels) and [production channel patterns](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260):

```rust
// BOUNDED channel with backpressure
let (tx, rx) = tokio::sync::mpsc::channel::<StreamEvent>(1000);

// Producer side (extraction loop)
match tx.send(event).await {
    Ok(_) => { /* continue */ }
    Err(_) => {
        // Receiver dropped, stop producing
        tracing::warn!("Receiver closed, stopping stream");
        return Ok(());
    }
}

// Consumer side (stream adapter)
let stream = ReceiverStream::new(rx)
    .map(|event| { /* transform */ });
```

**When to Use Bounded vs Unbounded:**

| Scenario | Channel Type | Reason |
|----------|--------------|--------|
| CLI stdout streaming | Bounded (1000-10000) | Backpressure prevents OOM if consumer is slow |
| Internal control signals | Unbounded | Small message volume, need guaranteed delivery |
| Tool call dispatch | Bounded with semaphore | Limit concurrent subprocesses |
| Metrics collection | Bounded with overflow drop | Can lose samples under load |

### Payload Support Architecture

**Current State:** No payload concept

**Recommended Pattern (Idiomatic Rust):**

```rust
// 1. Define payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayloadType {
    FileContent { path: PathBuf, content: String },
    BinaryData { mime_type: String, data: Vec<u8> },
    JsonData { schema: Option<JsonSchema>, value: serde_json::Value },
}

// 2. Extend RunConfig
pub struct RunConfig {
    // ... existing fields ...
    pub payloads: Vec<PayloadType>,
}

// 3. Argument construction
impl RunConfig {
    fn build_args(&self, prompt: &str) -> Vec<OsString> {
        let mut args = vec![];

        // Add payload arguments
        for payload in &self.payloads {
            match payload {
                PayloadType::FileContent { path, content } => {
                    // Write to temp file, pass path to CLI
                    args.push("--file".into());
                    args.push(path.as_os_str().to_owned());
                }
                PayloadType::JsonData { value, .. } => {
                    // Pass as --json argument
                    args.push("--json".into());
                    args.push(serde_json::to_string(value).unwrap().into());
                }
                _ => {}
            }
        }

        args
    }
}

// 4. CompletionRequest extension
pub trait CompletionRequestExt {
    fn with_payload(self, payload: PayloadType) -> Self;
}

impl CompletionRequestExt for CompletionRequest {
    fn with_payload(mut self, payload: PayloadType) -> Self {
        self.extensions
            .entry("payloads".to_string())
            .or_insert_with(|| serde_json::Value::Array(vec![]))
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(payload).unwrap());
        self
    }
}
```

**Integration Points:**
1. **CompletionModel::completion()**: Extract payloads from `request.extensions`, add to RunConfig
2. **CLI Argument Builder**: Translate payloads to CLI-specific flags
3. **Temporary File Management**: Create temp files for file payloads, clean up after execution
4. **Result Extraction**: Parse structured output from payload-enhanced responses

## Recommended Project Structure

```
rig-cli/
├── claudecode-adapter/          # CLI subprocess wrapper
│   ├── src/
│   │   ├── lib.rs               # Public API (ClaudeCli)
│   │   ├── process.rs           # Subprocess execution + extraction loop
│   │   ├── cmd.rs               # Argument construction
│   │   ├── types.rs             # RunConfig, StreamEvent, etc.
│   │   ├── error.rs             # ClaudeError enum
│   │   ├── discovery.rs         # Executable path resolution
│   │   └── retry.rs             # NEW: Retry policies for extraction
│   └── Cargo.toml
├── codex-adapter/               # (same structure)
├── opencode-adapter/            # (same structure)
├── rig-provider/                # Provider orchestration
│   ├── src/
│   │   ├── lib.rs               # Re-exports
│   │   ├── main.rs              # CLI entry point
│   │   ├── adapters/
│   │   │   ├── mod.rs
│   │   │   ├── claude.rs        # ClaudeModel (CompletionModel impl)
│   │   │   ├── codex.rs
│   │   │   └── opencode.rs
│   │   ├── sessions.rs          # SessionManager
│   │   ├── setup.rs             # Config registration
│   │   ├── errors.rs            # ProviderError aggregation
│   │   ├── metrics.rs           # NEW: Prometheus metrics
│   │   └── utils.rs             # Chat history formatting
│   ├── examples/                # Usage examples
│   └── Cargo.toml
└── mcp/                         # MCP server bridge
    ├── src/
    │   ├── lib.rs
    │   ├── server.rs            # RigMcpHandler
    │   └── tools.rs             # JsonSchemaToolkit
    └── Cargo.toml
```

### Structure Rationale

- **Adapter isolation:** Each CLI adapter is independent crate, reusable outside Rig context
- **Provider orchestration:** `rig-provider` depends on adapters, not vice versa
- **MCP layer separation:** Generic bridge between Rig and RMCP, not adapter-specific
- **Shared concerns:** Retry logic, metrics, validation in adapter crates (not provider)

## Architectural Patterns

### Pattern 1: Adapter with Retry Policy

**What:** CLI adapters encapsulate retry logic for transient failures

**When to use:** Subprocess execution, stream parsing, structured output validation

**Trade-offs:**
- **Pro:** Failures handled close to failure point, easier debugging
- **Con:** Each adapter reimplements retry, potential inconsistency
- **Mitigation:** Extract shared `retry` module to workspace-level crate

**Example:**
```rust
// claudecode-adapter/src/retry.rs
pub struct RetryPolicy {
    max_attempts: u32,
    backoff: ExponentialBackoff,
}

impl ClaudeCli {
    pub async fn run_with_retry(
        &self,
        prompt: &str,
        config: &RunConfig,
    ) -> Result<RunResult, ClaudeError> {
        let mut attempts = 0;
        loop {
            match self.run(prompt, config).await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempts < self.retry_policy.max_attempts => {
                    attempts += 1;
                    let delay = self.retry_policy.backoff.next_delay();
                    tracing::warn!("Retry {} after {:?}: {}", attempts, delay, e);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### Pattern 2: Extraction Loop with Validation Pipeline

**What:** Multi-stage validation (parse → schema → semantic) with explicit error budgets

**When to use:** Streaming JSON output from CLI subprocesses

**Trade-offs:**
- **Pro:** Clear separation of concerns, easy to add validation stages
- **Con:** More complex than simple parse-and-forward
- **Mitigation:** Provide default validator that passes everything

**Example:**
```rust
pub struct ValidationPipeline {
    stages: Vec<Box<dyn Fn(&str) -> Result<(), ValidationError>>>,
}

impl ValidationPipeline {
    pub fn validate(&self, line: &str) -> Result<(), ValidationError> {
        for stage in &self.stages {
            stage(line)?;
        }
        Ok(())
    }
}

// Usage
let pipeline = ValidationPipeline {
    stages: vec![
        Box::new(|line| {
            serde_json::from_str::<serde_json::Value>(line)?;
            Ok(())
        }),
        Box::new(|line| {
            let event: StreamEvent = serde_json::from_str(line)?;
            if event.is_valid() { Ok(()) } else { Err(ValidationError::Semantic) }
        }),
    ],
};
```

### Pattern 3: Bounded Resource Pool

**What:** Limit concurrent subprocesses using semaphore

**When to use:** Multi-tool MCP server under heavy load

**Trade-offs:**
- **Pro:** Prevents resource exhaustion, predictable memory usage
- **Con:** Requests may queue/timeout under load
- **Mitigation:** Expose queue depth metrics, tune pool size

**Example:**
```rust
pub struct AdapterPool {
    semaphore: Arc<Semaphore>,
    cli: ClaudeCli,
}

impl AdapterPool {
    pub fn new(cli: ClaudeCli, max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            cli,
        }
    }

    pub async fn execute(&self, request: CompletionRequest) -> Result<...> {
        let _permit = self.semaphore.acquire().await?;
        self.cli.run(&format_request(&request), &RunConfig::default()).await
    }
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Unbounded Channels for Stream Events

**What people do:** Use `tokio::sync::mpsc::unbounded_channel()` for subprocess output

**Why it's wrong:** Producer (subprocess) can outrun consumer (stream handler), causing OOM

**Do this instead:**
```rust
// BAD
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

// GOOD
let (tx, rx) = tokio::sync::mpsc::channel(1000);  // Bounded with backpressure
```

**Source:** [Tokio channels documentation](https://tokio.rs/tokio/tutorial/channels) emphasizes "unbounded queues will eventually fill up all available memory."

### Anti-Pattern 2: Fire-and-Forget Tokio Tasks

**What people do:** `tokio::spawn()` without storing JoinHandle

**Why it's wrong:** Task leaks on timeout, no way to cancel or wait for cleanup

**Do this instead:**
```rust
// BAD
tokio::spawn(async move {
    cli.stream(&prompt, &config, tx).await
});

// GOOD
let handle = tokio::spawn(async move {
    cli.stream(&prompt, &config, tx).await
});
// Later: handle.abort() or handle.await
```

**Source:** Current codebase has this issue (CONCERNS.md, "Possible Task Leak in Async Stream Handling")

### Anti-Pattern 3: Silent JSON Parse Failures

**What people do:** Ignore parse errors, fall back to empty events

**Why it's wrong:** Masks real errors (CLI crashed, corrupted output), makes debugging impossible

**Do this instead:**
```rust
// BAD
if let Ok(event) = serde_json::from_str(&line) {
    tx.send(event);
}
// Parse failure silently ignored

// GOOD
match serde_json::from_str::<StreamEvent>(&line) {
    Ok(event) => tx.send(event).await?,
    Err(e) => {
        tracing::warn!("Parse failed: {}, line: {}", e, line);
        consecutive_errors += 1;
        if consecutive_errors >= MAX_ERRORS {
            return Err(LoopError::TooManyParseErrors);
        }
    }
}
```

### Anti-Pattern 4: Panicking on Stream Acquisition

**What people do:** `.expect("Failed to open stdout")` on child.stdout.take()

**Why it's wrong:** Crashes entire server on subprocess setup failure

**Do this instead:**
```rust
// BAD
let stdout = child.stdout.take().expect("Failed to open stdout");

// GOOD
let stdout = child.stdout.take()
    .ok_or_else(|| ClaudeError::Other("Failed to capture stdout".into()))?;
```

**Source:** Current codebase has this issue (CONCERNS.md, "Panicking on Stream Handling")

## Data Flow Patterns

### Completion Request Flow

```
CompletionRequest (Rig)
    ↓
format_chat_history() → prompt string
    ↓
RunConfig construction (tools, schema, payloads)
    ↓
build_args() → Vec<OsString>
    ↓
tokio::process::Command::new(cli_path).args(args).spawn()
    ↓
stdout/stderr capture (BufReader::lines())
    ↓
Extraction loop (parse → validate → emit)
    ↓
StreamEvent → RawStreamingChoice
    ↓
StreamingCompletionResponse (Rig)
```

### Error Propagation Flow

```
ClaudeError::Timeout
    ↓ (From impl)
ProviderError::Claude(ClaudeError::Timeout)
    ↓ (From impl)
CompletionError::ProviderError("Timeout after 300s")
    ↓ (MCP layer)
ErrorData { code: -1, message: "..." }
    ↓ (RMCP protocol)
JSON-RPC error response to client
```

### Session State Flow

```
Tool call with session_id: "abc123"
    ↓
SessionManager::get_session_dir("abc123")
    ↓ (HashMap lookup)
Entry exists? → Return existing path
Entry missing? → tempfile::tempdir() → store Arc<TempDir> → return path
    ↓
RunConfig { cwd: Some(session_path) }
    ↓
CLI subprocess spawned in session directory
    ↓
State files (.git, .env, etc.) persist across calls
```

## Build Order and Dependencies

### Dependency Graph

```
mcp ←─────────────┐
  │               │
  └→ rig-provider ←┼── claudecode-adapter
                   ├── codex-adapter
                   └── opencode-adapter
```

### Suggested Build Order

1. **Phase 1: Harden CLI Adapters**
   - Replace unbounded channels with bounded (backpressure)
   - Add retry policies to `claudecode-adapter`
   - Implement extraction loop with error budget
   - Add metrics to `process.rs` (parse success rate, timeout count)

   **Why first:** Foundation for all higher layers, isolated testing

2. **Phase 2: Improve Subprocess Lifecycle**
   - Track JoinHandles, implement graceful cleanup
   - Add `ProcessHandle` abstraction
   - Test timeout scenarios, task cancellation

   **Why second:** Prevents resource leaks, enables reliable streaming

3. **Phase 3: Structured Output Validation**
   - Add payload support to RunConfig
   - Implement validation pipeline
   - Extract structured output from JSON format

   **Why third:** Depends on reliable subprocess execution

4. **Phase 4: Provider Layer Hardening**
   - Add bounded resource pool (semaphore)
   - Implement session cleanup (TTL, LRU)
   - Add observability (tracing, metrics)

   **Why fourth:** Depends on hardened adapters

5. **Phase 5: Production Deployment**
   - Add health checks
   - Implement graceful shutdown
   - Production metrics/alerting

   **Why last:** Requires all layers stable

## Integration Patterns

### Rig ToolSet Integration

**Current Pattern:**
```rust
// rig-provider/src/main.rs (lines 68-78)
let toolset = rig::tool::ToolSet::builder()
    .tool(claude_tool)
    .tool(codex_tool)
    .tool(opencode_tool)
    .tool(JsonSchemaToolkit::submit())
    .tool(JsonSchemaToolkit::validate())
    .build();

let handler = RigMcpHandler::from_toolset(toolset).await?;
handler.serve_stdio().await?;
```

**Production Pattern:**
```rust
let toolset = rig::tool::ToolSet::builder()
    .tool(AdapterPool::new(claude_tool, 10))  // Max 10 concurrent
    .tool(AdapterPool::new(codex_tool, 10))
    .tool(AdapterPool::new(opencode_tool, 10))
    .tool(JsonSchemaToolkit::submit())
    .tool(JsonSchemaToolkit::validate())
    .build();

let handler = RigMcpHandler::builder()
    .toolset(toolset)
    .name("rig-mcp-server")
    .with_metrics(metrics_registry)  // Prometheus metrics
    .with_health_check(health_checker)  // Liveness probe
    .build()
    .await?;

handler.serve_stdio_with_shutdown(shutdown_signal).await?;
```

### MCP Protocol Integration

**Tool Definition Translation:**
```rust
// Rig ToolDefinition → RMCP Tool
pub fn definition_to_mcp(definition: ToolDefinition) -> McpTool {
    McpTool {
        name: Cow::Owned(definition.name.clone()),
        description: Some(Cow::Owned(definition.description)),
        input_schema: Arc::new(definition.parameters.as_object().unwrap().clone()),
        // ... other fields
    }
}
```

**Call Routing:**
```rust
// RMCP CallToolRequestParams → Rig ToolSet::call
async fn call_tool(
    &self,
    request: CallToolRequestParams,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, ErrorData> {
    let args_str = request.arguments
        .as_ref()
        .map_or_else(String::new, |a| Value::Object(a.clone()).to_string());

    match self.toolset.call(&request.name, args_str).await {
        Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
        Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
    }
}
```

## Sources

**Rig Framework Architecture:**
- [Rig Documentation](https://docs.rig.rs/) - Overview and core concepts
- [Rig API Docs](https://docs.rs/rig-core/latest/rig/) - CompletionModel trait, streaming module
- [Rig GitHub Repository](https://github.com/0xPlaygrounds/rig) - Source code and examples

**Tokio Async Patterns:**
- [Tokio Process Documentation](https://docs.rs/tokio/latest/tokio/process/index.html) - Async subprocess management
- [Tokio Channels Tutorial](https://tokio.rs/tokio/tutorial/channels) - Bounded vs unbounded channels
- [TokioConf 2026 Focus](https://tokio.rs/blog/2025-09-26-announcing-tokio-conf-cfp) - Production patterns, lifecycle management

**Retry and Error Handling:**
- [Rust Retry with Exponential Backoff (Jan 2026)](https://oneuptime.com/blog/post/2026-01-07-rust-retry-exponential-backoff/view) - Production retry patterns
- [BackON Retry Crate](https://xuanwo.io/2024/08-backon-reaches-v1/) - User-friendly retry API design
- [Async Error Handling Patterns](https://calmops.com/programming/rust/rust-async-error-handling-patterns/) - Transient vs permanent errors

**Structured Output:**
- [Claude Structured Outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs) - Validation and retry mechanisms
- [OpenAI Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs) - Schema matching and streaming

**Channel Patterns:**
- [Mastering Tokio mpsc Channels](https://medium.com/@CodeWithPurpose/mastering-tokio-building-mpsc-channels-for-maximum-throughput-afb15ca64260) - Throughput optimization

---

*Architecture research for: Rig CLI Provider (production hardening milestone)*
*Researched: 2026-02-01*
*Confidence: MEDIUM-HIGH (Rig 0.29 patterns verified from codebase, Tokio patterns from official docs, some extrapolation for production patterns)*
