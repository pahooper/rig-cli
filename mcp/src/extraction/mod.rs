//! Extraction retry and validation loop foundation types.
//!
//! This module provides the core types for implementing retry/validation feedback
//! loops for structured LLM extraction:
//!
//! - [`ExtractionOrchestrator`] - Async retry loop with validation feedback
//! - [`ExtractionError`] - Typed error enum with attempt history
//! - [`ExtractionMetrics`] - Token and timing metrics
//! - [`ExtractionConfig`] - Retry behavior configuration
//! - [`build_validation_feedback`] - Rich validation error formatting

pub mod config;
pub mod error;
pub mod feedback;
pub mod metrics;
pub mod orchestrator;

pub use config::ExtractionConfig;
pub use error::{AttemptRecord, ExtractionError};
pub use feedback::build_validation_feedback;
pub use metrics::{estimate_tokens, ExtractionMetrics};
pub use orchestrator::ExtractionOrchestrator;
