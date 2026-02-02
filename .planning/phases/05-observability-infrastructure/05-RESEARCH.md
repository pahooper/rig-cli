# Phase 5: Observability Infrastructure - Research

**Researched:** 2026-02-02
**Domain:** Structured tracing, version detection, extraction workflow debugging
**Confidence:** HIGH

## Summary

The Rust ecosystem has a mature, standardized observability stack centered on the `tracing` crate (part of the Tokio project). This is not logging-as-text but structured, span-based tracing designed for async systems. The codebase already uses `tracing` and `tracing-subscriber`, but lacks instrumentation at key decision points (prompt sent, validation result, retry decisions) and version validation infrastructure.

For Phase 5's goals—full extraction workflow traceability and CLI tool version awareness—the standard approach is:
1. **Structured tracing**: Instrument the extraction orchestrator with spans and events using `#[tracing::instrument]` and explicit span construction
2. **Version detection**: Use the `semver` crate for parsing and comparing CLI tool versions
3. **Startup validation**: Extend the existing `InitReport` pattern to include version requirement checking with clear warnings

The ecosystem strongly prefers JSON-formatted logs for production (machine-readable, queryable) and offers zero-cost abstractions (disabled spans compile to no-ops). The codebase's existing adapter-per-CLI architecture and `InitReport` structure align perfectly with standard observability patterns.

**Primary recommendation:** Add `#[tracing::instrument]` to extraction orchestrator methods, emit structured events at each extraction stage, and extend `InitReport` with semver-based version requirement checking. Use JSON formatting for production deployments via `tracing-subscriber` features.

## Standard Stack

The established libraries/tools for Rust observability:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tracing | 0.1 | Application-level instrumentation | Official Tokio project, Rust's de facto standard for structured tracing |
| tracing-subscriber | 0.3 | Subscriber implementations and utilities | Official companion crate, provides fmt layer, JSON output, env filtering |
| semver | 1.0.27 | Semantic version parsing/comparison | Cargo's official semver implementation, maintained by Rust project |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing-appender | 0.2 | Non-blocking file appenders with rotation | Production deployments needing persistent logs |
| opentelemetry-tracing | (optional) | OpenTelemetry integration | Distributed tracing across services (not needed for single-process CLI) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tracing | slog | More complex API, lower adoption; slog docs now recommend tracing |
| tracing | log crate | Unstructured text logging only, no span concept |
| semver | version_check | Build-time only, lacks runtime comparison features |
| JSON output | Pretty formatting | Human-readable but not machine-queryable for analysis tools |

**Installation:**
```bash
# Already in Cargo.toml, add features for JSON and filtering
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
semver = "1.0"
```

## Architecture Patterns

### Recommended Instrumentation Structure

The extraction orchestrator should be instrumented at three levels:

1. **Span per extraction session** - Top-level span tracking the entire extract call
2. **Span per attempt** - Nested span for each retry iteration
3. **Events at decision points** - Discrete events for prompt sent, response received, validation result, retry decision

```
extraction_session (span)
├── attempt_1 (span)
│   ├── prompt_sent (event)
│   ├── agent_response (event)
│   ├── validation_result (event)
│   └── retry_decision (event)
├── attempt_2 (span)
│   └── ...
└── extraction_complete (event)
```

### Pattern 1: Instrument Extraction Orchestrator

**What:** Add `#[tracing::instrument]` to async methods and emit structured events at decision points
**When to use:** When tracing async workflows with retry loops
**Example:**
```rust
// Source: https://tokio.rs/tokio/topics/tracing
use tracing::{info, warn, instrument, Span};

#[instrument(
    skip(agent_fn),
    fields(
        max_attempts = %self.config.max_attempts,
        schema_type = "extraction"
    )
)]
pub async fn extract<F, Fut>(
    &self,
    agent_fn: F,
    initial_prompt: String,
) -> Result<(Value, ExtractionMetrics), ExtractionError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let start = Instant::now();

    for attempt in 1..=self.config.max_attempts {
        let attempt_span = tracing::info_span!(
            "extraction_attempt",
            attempt = attempt,
            is_last = (attempt == self.config.max_attempts)
        );
        let _guard = attempt_span.enter();

        info!(
            prompt_length = current_prompt.len(),
            "sending prompt to agent"
        );

        let agent_output = agent_fn(current_prompt.clone()).await
            .map_err(ExtractionError::AgentError)?;

        info!(
            output_length = agent_output.len(),
            "received agent response"
        );

        // Parse and validate...
        match serde_json::from_str::<Value>(&agent_output) {
            Ok(parsed) => {
                let errors = collect_validation_errors(&self.schema, &parsed);

                if errors.is_empty() {
                    info!("validation passed, extraction successful");
                    return Ok((parsed, metrics));
                }

                warn!(
                    error_count = errors.len(),
                    errors = ?errors,
                    "validation failed"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "JSON parse failed"
                );
            }
        }

        if attempt < self.config.max_attempts {
            info!("retrying with feedback");
        }
    }

    Err(ExtractionError::MaxRetriesExceeded { ... })
}
```

### Pattern 2: Version Validation at Startup

**What:** Parse CLI tool version strings, compare against requirements, emit warnings
**When to use:** During adapter initialization (existing `init()` functions)
**Example:**
```rust
// Source: https://docs.rs/semver
use semver::{Version, VersionReq};
use tracing::{info, warn};

pub struct VersionRequirement {
    pub min_supported: Version,
    pub max_tested: Option<Version>,
}

impl InitReport {
    pub fn validate_version(&self, req: &VersionRequirement) -> VersionStatus {
        // Parse version string like "claude-code v1.2.3"
        let version_str = self.version
            .split_whitespace()
            .last()
            .unwrap_or(&self.version)
            .trim_start_matches('v');

        match Version::parse(version_str) {
            Ok(version) => {
                if version < req.min_supported {
                    warn!(
                        detected = %version,
                        required = %req.min_supported,
                        cli_path = %self.claude_path.display(),
                        "CLI version is below minimum supported"
                    );
                    VersionStatus::Unsupported { version }
                } else if let Some(max) = &req.max_tested {
                    if version > *max {
                        warn!(
                            detected = %version,
                            max_tested = %max,
                            "CLI version is newer than tested versions"
                        );
                        VersionStatus::Untested { version }
                    } else {
                        info!(
                            version = %version,
                            "CLI version validated"
                        );
                        VersionStatus::Supported { version }
                    }
                } else {
                    VersionStatus::Supported { version }
                }
            }
            Err(e) => {
                warn!(
                    version_string = %self.version,
                    error = %e,
                    "Failed to parse CLI version"
                );
                VersionStatus::Unknown
            }
        }
    }
}

pub enum VersionStatus {
    Supported { version: Version },
    Unsupported { version: Version },
    Untested { version: Version },
    Unknown,
}
```

### Pattern 3: JSON Formatting for Production

**What:** Configure `tracing-subscriber` to output structured JSON logs
**When to use:** Production deployments, CI/CD pipelines, any machine-readable log analysis
**Example:**
```rust
// Source: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/format/struct.Json.html
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing_json() {
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)  // Merge event fields into root JSON object
        .with_current_span(true)  // Include current span context
        .with_span_list(true)  // Include full span hierarchy
        .with_env_filter(EnvFilter::from_default_env())  // RUST_LOG env var
        .init();
}

// Output format:
// {"timestamp":"2026-02-02T18:47:10.821315Z","level":"INFO","fields":{"message":"sending prompt to agent","prompt_length":1234,"attempt":1},"target":"rig_mcp_server::extraction","span":{"name":"extraction_attempt","attempt":1}}
```

### Anti-Patterns to Avoid

- **Don't use `Span::enter()` across `.await` points:** The guard will be held during suspensions, producing incorrect traces. Use `#[instrument]` or `Instrument` trait instead.
- **Don't log sensitive data in structured fields:** Use `skip` or `skip_all` in `#[instrument]` to exclude sensitive arguments. Don't log full prompts containing user data.
- **Don't use string formatting in events:** Prefer structured fields. Bad: `info!("attempt {} failed", n)`. Good: `info!(attempt = n, "attempt failed")`.
- **Don't create spans without entering them:** If you construct a span manually, either use `_enter()` or `Instrument` trait. Unused spans don't appear in traces.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Version parsing | String splitting, regex | `semver` crate | Handles pre-release tags, build metadata, comparisons, edge cases (v prefix, whitespace) |
| Async span tracking | Manual context tracking | `#[tracing::instrument]` macro | Correctly handles span entry/exit across await points, automatic field capture |
| Correlation IDs | Thread-local UUID generation | `tracing::Span::current()` | Spans automatically provide hierarchical context; span IDs serve as correlation IDs |
| Log rotation | Custom file writing logic | `tracing-appender::rolling` | Thread-safe, non-blocking, handles midnight rollover, prevents log loss on shutdown |
| Filtering by module | String matching on log statements | `EnvFilter` with `RUST_LOG` | Supports per-module levels, dynamic enabling without recompilation |

**Key insight:** The tracing ecosystem is built for async Rust and handles the subtle lifetime issues around spans/futures. Custom solutions will miss edge cases (span enters across await, premature guard drops) that are already solved.

## Common Pitfalls

### Pitfall 1: Holding Span::enter() Guard Across Await

**What goes wrong:** In async code, using `span.enter()` directly and holding the guard across an `.await` produces incorrect traces. The span appears to stay entered even when the task is suspended.

**Why it happens:** The returned guard uses RAII to exit the span on drop. When a future suspends at `.await`, the guard remains on the stack, but the logical execution has left that span's context.

**How to avoid:**
- Use `#[tracing::instrument]` on async functions (it handles this correctly)
- Or use the `Instrument` trait: `async_fn().instrument(span).await`
- Never manually call `.enter()` in async contexts

**Warning signs:**
- Spans in traces showing unrealistic durations (hours instead of milliseconds)
- Multiple concurrent requests appearing to share the same span
- Span hierarchy inverting (children appearing to contain parents)

### Pitfall 2: Forgetting to Retain WorkerGuard

**What goes wrong:** When using `tracing_appender::non_blocking`, the returned `WorkerGuard` MUST be kept alive for the program's lifetime. If dropped, buffered logs are lost on shutdown.

**Why it happens:** The guard's Drop implementation triggers flush and thread join. Assigning to `_` in Rust still drops immediately at statement end, not scope end.

**How to avoid:**
```rust
// BAD - guard dropped immediately
let _ = tracing_appender::non_blocking(file);

// GOOD - guard lives until program exit
let (_non_blocking, _guard) = tracing_appender::non_blocking(file);
// Store _guard in a long-lived struct or let it live in main()
```

**Warning signs:**
- Last few log entries missing after normal shutdown
- Works fine in debug mode but loses logs in release
- Logs appear complete when process is killed but not on clean exit

### Pitfall 3: Over-Instrumentation Performance Impact

**What goes wrong:** Adding `#[instrument]` to every small function creates excessive span overhead. In tight loops or frequently-called functions, this degrades performance.

**Why it happens:** Each span has allocation and formatting costs. While cheap, these add up in hot paths.

**How to avoid:**
- Instrument at the request/operation level, not every function
- Use `info_span!` manually for selective instrumentation
- Profile before instrumenting tight loops
- Consider using `Level::DEBUG` or `Level::TRACE` for fine-grained spans
- Test with `RUST_LOG=info` (production setting) not `trace` (dev setting)

**Warning signs:**
- Benchmarks show >10% slowdown after adding tracing
- Span count in traces exceeds 1000s per request
- GC/allocation pressure increases significantly

### Pitfall 4: Version String Parsing Assumptions

**What goes wrong:** CLI tools output version strings in different formats. Naive parsing breaks on pre-release versions, build metadata, or non-standard formats.

**Why it happens:** Assuming `--version` always outputs just `X.Y.Z`, but real tools output `tool-name v1.2.3-beta+build123` or `version 1.2.3 (commit abc123)`.

**How to avoid:**
- Extract the version substring first (split on whitespace, remove 'v' prefix)
- Use `semver::Version::parse()` which handles `1.2.3-alpha.1+build.123`
- Have a fallback for unparseable versions (log warning, continue with unknown status)
- Test against multiple version formats from each CLI tool

**Warning signs:**
- Version validation fails on pre-release CLI builds
- Parsing breaks when CLI tool adds commit hashes to version output
- Tests pass but real-world CLIs fail validation

## Code Examples

Verified patterns from official sources:

### Extracting Version String from CLI Output

```rust
// Source: Combining patterns from codebase + semver docs
use semver::Version;
use tracing::{info, warn};

/// Parse version from CLI tool's --version output
/// Handles formats like:
/// - "claude v1.2.3"
/// - "codex version 0.5.1-beta"
/// - "1.2.3"
pub fn parse_cli_version(version_output: &str) -> Result<Version, semver::Error> {
    // Extract the actual version number:
    // 1. Take last whitespace-separated token (handles "tool-name version X.Y.Z")
    // 2. Remove 'v' prefix if present
    let version_str = version_output
        .split_whitespace()
        .last()
        .unwrap_or(version_output)
        .trim_start_matches('v');

    Version::parse(version_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_version() {
        assert_eq!(
            parse_cli_version("claude v1.2.3").unwrap(),
            Version::new(1, 2, 3)
        );

        assert_eq!(
            parse_cli_version("version 0.5.1-beta").unwrap(),
            Version::parse("0.5.1-beta").unwrap()
        );

        assert_eq!(
            parse_cli_version("1.2.3").unwrap(),
            Version::new(1, 2, 3)
        );
    }
}
```

### Structured Event Emission in Retry Loop

```rust
// Source: https://tokio.rs/tokio/topics/tracing + codebase orchestrator.rs
use tracing::{info, warn, error, info_span};
use serde_json::Value;

// Inside ExtractionOrchestrator::extract()
for attempt in 1..=self.config.max_attempts {
    // Create span for this attempt with structured fields
    let attempt_span = info_span!(
        "extraction_attempt",
        attempt = attempt,
        max_attempts = self.config.max_attempts,
        is_retry = (attempt > 1)
    );

    // Enter span for the duration of this attempt
    let _guard = attempt_span.enter();

    // Event: Prompt sent
    info!(
        prompt_chars = current_prompt.len(),
        estimated_tokens = estimate_tokens(&current_prompt),
        "prompt_sent_to_agent"
    );

    // Call agent
    let agent_output = agent_fn(current_prompt.clone()).await
        .map_err(|e| {
            error!(error = %e, "agent_execution_failed");
            ExtractionError::AgentError(e)
        })?;

    // Event: Response received
    info!(
        output_chars = agent_output.len(),
        estimated_tokens = estimate_tokens(&agent_output),
        "agent_response_received"
    );

    // Parse JSON
    let parsed = match serde_json::from_str::<Value>(&agent_output) {
        Ok(value) => value,
        Err(e) => {
            // Event: Parse failure
            warn!(
                error = %e,
                output_preview = %&agent_output[..agent_output.len().min(100)],
                "json_parse_failed"
            );

            attempt_history.push(AttemptRecord {
                attempt_number: attempt,
                submitted_json: Value::Null,
                validation_errors: vec![format!("JSON parse error: {e}")],
                raw_agent_output: agent_output.clone(),
                elapsed: start.elapsed(),
            });

            // Retry decision
            if attempt < self.config.max_attempts {
                info!("retry_decision: will_retry");
                let feedback = build_parse_error_feedback(...);
                current_prompt = format!("{current_prompt}\n\n{feedback}");
                continue;
            } else {
                error!("retry_decision: max_attempts_reached");
                break;
            }
        }
    };

    // Validate
    let errors = collect_validation_errors(&self.schema, &parsed);

    if errors.is_empty() {
        // Event: Success
        info!(
            elapsed_ms = start.elapsed().as_millis(),
            total_attempts = attempt,
            "validation_passed"
        );
        return Ok((parsed, metrics));
    }

    // Event: Validation failure
    warn!(
        error_count = errors.len(),
        errors = ?errors,  // Debug formatting for Vec<String>
        "validation_failed"
    );

    // Retry decision
    if attempt < self.config.max_attempts {
        info!(
            attempts_remaining = self.config.max_attempts - attempt,
            "retry_decision: will_retry_with_feedback"
        );
    } else {
        error!("retry_decision: max_attempts_exhausted");
    }
}
```

### Configurable Tracing Subscriber Initialization

```rust
// Source: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

pub enum LogFormat {
    Pretty,   // Human-readable, colored (development)
    Compact,  // Single-line text (basic production)
    Json,     // Structured JSON (production with analysis tools)
}

pub fn init_tracing(format: LogFormat) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match format {
        LogFormat::Pretty => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .init();
        }
        LogFormat::Compact => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .with_target(false)
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(true)
                .with_env_filter(env_filter)
                .init();
        }
    }

    Ok(())
}

// Usage:
// Development: init_tracing(LogFormat::Pretty)?;
// Production: init_tracing(LogFormat::Json)?;
// Via env var: RUST_LOG=debug cargo run
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `log` crate for everything | `tracing` for instrumentation | 2019-2020 | Structured fields, span hierarchy, async-aware |
| `env_logger` subscriber | `tracing-subscriber` with layers | 2020-2021 | Composable subscribers, JSON output, filtering |
| Manual correlation IDs | Span hierarchy as context | 2020-2021 | Automatic parent-child relationships, no manual threading |
| `slog` for structured logging | `tracing` recommended even by slog | 2022+ | Simpler API, better async support, Tokio ecosystem |
| String version parsing | `semver` crate | Always standard | Handles pre-release, build metadata, comparisons |

**Deprecated/outdated:**
- **`log` crate for new projects**: Still maintained for compatibility but `tracing` subsumes its functionality with `tracing::log` compat layer
- **`env_logger`**: Superseded by `tracing-subscriber`'s `EnvFilter` which supports the same `RUST_LOG` syntax
- **Custom span/correlation ID management**: Built into tracing's span model
- **Manual version string regex**: `semver` is the standard, handles all edge cases

## Open Questions

Things that couldn't be fully resolved:

1. **Version requirement specification location**
   - What we know: Should be per-adapter, checked at init time
   - What's unclear: Should requirements be hardcoded in adapter code, or configurable (e.g., in TOML)?
   - Recommendation: Start with hardcoded constants in each adapter module (simple, version-controlled). Make configurable only if users request it. Example: `const MIN_CLAUDE_VERSION: &str = "1.2.0";`

2. **Log output destination in production**
   - What we know: Current setup logs to stdout/stderr via `tracing_subscriber::fmt()`
   - What's unclear: Should Phase 5 add file output, or leave that to deployment configuration (systemd, Docker logs)?
   - Recommendation: Keep stdout/stderr as default (12-factor app pattern). Document how to add `tracing-appender` if users want file rotation. Don't impose file logging by default.

3. **Trace sampling/filtering in high-volume scenarios**
   - What we know: Each extraction session is traced; `RUST_LOG` env var controls level filtering
   - What's unclear: If orchestrator is called 1000s of times, do we need sampling (trace every Nth request)?
   - Recommendation: Not needed for Phase 5. CLI tool is interactive/batch, not high-throughput server. Document `RUST_LOG=error` for minimal logging if needed.

## Sources

### Primary (HIGH confidence)
- [Tokio tracing getting started](https://tokio.rs/tokio/topics/tracing) - Official Tokio documentation
- [tracing-subscriber fmt JSON formatter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/format/struct.Json.html) - Official API docs
- [semver crate documentation](https://docs.rs/semver) - Official semver API
- [tracing-appender docs](https://docs.rs/tracing-appender/latest/tracing_appender/) - Official appender API
- [tracing span documentation](https://docs.rs/tracing/latest/tracing/span/index.html) - Official span guide

### Secondary (MEDIUM confidence)
- [How to Structure Logs Properly in Rust with tracing and OpenTelemetry (Jan 2026)](https://oneuptime.com/blog/post/2026-01-07-rust-tracing-structured-logs/view) - Recent tutorial showing current best practices
- [Tracing in Rust: A Comprehensive Guide](https://www.hamzak.xyz/blog-posts/tracing-in-rust-a-comprehensive-guide) - Community guide covering spans, events, instrumentation patterns
- [Getting Started with Tracing in Rust | Shuttle](https://www.shuttle.dev/blog/2024/01/09/getting-started-tracing-rust) - Tutorial on tracing basics

### Tertiary (LOW confidence)
- GitHub discussions on tracing performance overhead - Community anecdotes, not benchmarked
- Blog posts on alternative tracing solutions (fastrace) - Niche solutions, not standard stack

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Official Tokio project documentation, widely adopted, already in codebase
- Architecture: HIGH - Patterns documented in official Tokio guides, verified in production systems
- Pitfalls: HIGH - Documented in official tracing docs (span.enter() warning), verified through community experience
- Version detection: HIGH - semver is Cargo's official implementation, well-documented
- Production config: MEDIUM - JSON formatting and filtering are documented, but deployment-specific concerns (file vs stdout) vary by environment

**Research date:** 2026-02-02
**Valid until:** 30 days (tracing ecosystem is stable; minimal API changes expected)
