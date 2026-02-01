//! Library for the Rig MCP server.
//!
//! This crate provides a bridge between the Rig toolset and the Model Context Protocol (MCP).

pub mod extraction;
pub mod server;
pub mod tools;

/// Common traits and types for ergonomic usage of the Rig MCP server.
pub mod prelude {
    pub use crate::extraction::{
        ExtractionConfig, ExtractionError, ExtractionMetrics, ExtractionOrchestrator,
    };
    pub use crate::server::{McpConfig, RigMcpHandler, ToolSetExt};
    pub use crate::tools::JsonSchemaToolkit;
}
