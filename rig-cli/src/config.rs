//! Shared client configuration for CLI-based providers.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for CLI-based provider clients.
///
/// This configuration is shared across all agents created from a client,
/// providing CLI-specific settings like binary path, execution timeout,
/// and channel capacity for streaming.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Override CLI binary location (None = auto-discover).
    ///
    /// When set to `Some(path)`, the client will use the specified binary
    /// instead of searching PATH and standard installation locations.
    pub binary_path: Option<PathBuf>,

    /// Maximum execution time for CLI operations.
    ///
    /// Default: 300 seconds (5 minutes)
    pub timeout: Duration,

    /// Bounded channel size for streaming responses.
    ///
    /// Controls how many messages can be buffered when streaming
    /// CLI output. Default: 100 messages.
    pub channel_capacity: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            timeout: Duration::from_secs(300),
            channel_capacity: 100,
        }
    }
}

impl ClientConfig {
    /// Create a new `ClientConfig` with default settings.
    ///
    /// Equivalent to `ClientConfig::default()`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
