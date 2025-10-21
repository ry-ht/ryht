//! Library for Cortex CLI utilities and shared functionality.

pub mod commands;
pub mod config;
pub mod doctor;
pub mod export;
pub mod interactive;
pub mod output;
pub mod testing;
pub mod mcp;

pub use commands::*;
pub use config::*;
pub use doctor::*;
pub use export::*;
pub use interactive::*;
pub use output::*;
pub use testing::*;

// Re-export MCP server types
pub use mcp::{CortexMcpServer, CortexMcpServerBuilder};
