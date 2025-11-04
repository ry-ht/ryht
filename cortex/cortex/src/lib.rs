//! Library for Cortex CLI utilities and shared functionality.

pub mod commands;
pub mod db_manager;
pub mod doctor;
pub mod export;
pub mod interactive;
pub mod output;
pub mod testing;
pub mod mcp;
pub mod api;
pub mod server_manager;
pub mod qdrant_commands;
pub mod services;
pub mod conversions;
pub mod cortex_bridge;

pub use commands::*;
pub use doctor::*;
pub use export::*;
pub use interactive::*;
pub use output::*;
pub use testing::*;

// Re-export MCP server types
pub use mcp::{CortexMcpServer, CortexMcpServerBuilder};

// Re-export REST API server types
pub use api::{RestApiServer, ApiResponse, ApiMetadata, ApiError, ApiResult};

// Re-export service layer types
pub use services::{WorkspaceService, VfsService, SearchService, MemoryService};

// Re-export cortex_bridge types for MCP tools
pub use cortex_bridge::{
    AgentId, SessionId, WorkspaceId, AgentType, AgentStatus,
    CortexBridge, Episode, EpisodeOutcome, EpisodeType, Pattern, PatternType,
    SearchFilters, UnitFilters, TokenUsage, MergeStrategy, SessionScope,
};
