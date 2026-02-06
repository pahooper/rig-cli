//! Unified CLI binary discovery.
//!
//! Re-exports individual adapter discovery functions and provides a unified
//! [`discover_all`] that probes every adapter in parallel, returning a status
//! for each. Consumers (e.g. Tauri apps) should use this module instead of
//! rolling their own `which` / `--version` checks.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() {
//! let statuses = rig_cli::discovery::discover_all().await;
//! for s in &statuses {
//!     if let Some(ref path) = s.path {
//!         println!("{}: {} ({})", s.adapter, path.display(),
//!             s.version.as_deref().unwrap_or("unknown"));
//!     }
//! }
//! # }
//! ```

use rig_cli_provider::mcp_agent::CliAdapter;
use std::path::PathBuf;

// ── Re-exports of per-adapter discover functions ────────────────────
#[cfg(feature = "claude")]
pub use rig_cli_claude::discover_claude;
#[cfg(feature = "codex")]
pub use rig_cli_codex::discover_codex;
#[cfg(feature = "opencode")]
pub use rig_cli_opencode::discover_opencode;

/// Status of a single adapter after discovery.
#[derive(Debug, Clone)]
pub struct AdapterStatus {
    /// Which adapter this status describes.
    pub adapter: CliAdapter,
    /// Resolved path to the CLI binary, or `None` if not found.
    pub path: Option<PathBuf>,
    /// Version string (from `<binary> --version`), if retrievable.
    pub version: Option<String>,
}

impl AdapterStatus {
    /// Returns `true` when the adapter's CLI binary was found on this system.
    #[must_use]
    pub const fn is_installed(&self) -> bool {
        self.path.is_some()
    }
}

/// Discover all enabled adapters and return their installation status.
///
/// Each adapter is probed independently — a missing Claude binary does not
/// prevent Codex or `OpenCode` from being discovered.
///
/// Version detection runs `<binary> --version` only for binaries that were
/// successfully discovered.
pub async fn discover_all() -> Vec<AdapterStatus> {
    let mut statuses = Vec::new();

    #[cfg(feature = "claude")]
    {
        let status = discover_adapter(CliAdapter::ClaudeCode, || {
            rig_cli_claude::discover_claude(None).ok()
        })
        .await;
        statuses.push(status);
    }

    #[cfg(feature = "codex")]
    {
        let status = discover_adapter(CliAdapter::Codex, || {
            rig_cli_codex::discover_codex(None).ok()
        })
        .await;
        statuses.push(status);
    }

    #[cfg(feature = "opencode")]
    {
        let status = discover_adapter(CliAdapter::OpenCode, || {
            rig_cli_opencode::discover_opencode(None).ok()
        })
        .await;
        statuses.push(status);
    }

    statuses
}

/// Probe a single adapter: resolve binary, then optionally fetch version.
async fn discover_adapter(
    adapter: CliAdapter,
    discover_fn: impl FnOnce() -> Option<PathBuf>,
) -> AdapterStatus {
    let path = discover_fn();
    let version = match &path {
        Some(p) => get_version(p).await,
        None => None,
    };
    AdapterStatus {
        adapter,
        path,
        version,
    }
}

/// Run `<binary> --version` and return the first line of stdout.
async fn get_version(binary: &std::path::Path) -> Option<String> {
    let output = tokio::process::Command::new(binary)
        .arg("--version")
        .output()
        .await
        .ok()?;

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(|s| s.trim().to_owned())
    } else {
        None
    }
}
