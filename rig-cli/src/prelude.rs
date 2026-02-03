//! Commonly used types and traits.
//!
//! This prelude module provides convenient re-exports of frequently used types.
//! Import with `use rig_cli::prelude::*;` to bring common items into scope.

// Re-export error type (always available)
pub use crate::errors::Error;

// Re-export config type (always available)
pub use crate::config::ClientConfig;
