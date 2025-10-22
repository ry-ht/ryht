//! Cortex MCP Tools
//!
//! This module provides 170+ MCP tools for Cortex, organized by category:
//! - Workspace Management (8 tools)
//! - Virtual Filesystem (12 tools)
//! - Code Navigation (10 tools)
//! - Code Manipulation (15 tools)
//! - Semantic Search (8 tools)
//! - Dependency Analysis (10 tools)
//! - Code Quality (8 tools)
//! - Version Control (10 tools)
//! - Cognitive Memory (12 tools)
//! - Multi-Agent Coordination (10 tools)
//! - Materialization (8 tools)
//! - Testing & Validation (10 tools)
//! - Documentation (8 tools)
//! - Build & Execution (8 tools)
//! - Monitoring & Analytics (10 tools)
//! - Security Analysis (4 tools) - NEW
//! - Type Analysis (4 tools) - NEW
//! - AI-Assisted Development (6 tools) - NEW
//! - Advanced Testing (6 tools) - NEW
//! - Architecture Analysis (5 tools) - NEW

pub mod workspace;
pub mod vfs;
pub mod code_nav;
pub mod code_manipulation;
pub mod semantic_search;
pub mod dependency_analysis;
pub mod code_quality;
pub mod version_control;
pub mod cognitive_memory;
pub mod multi_agent;
pub mod materialization;
pub mod testing;
pub mod documentation;
pub mod build_execution;
pub mod monitoring;
pub mod security_analysis;
pub mod type_analysis;
pub mod ai_assisted;
pub mod advanced_testing;
pub mod architecture_analysis;

// Re-export all tools
pub use workspace::*;
pub use vfs::*;
pub use code_nav::*;
pub use code_manipulation::*;
pub use semantic_search::*;
pub use dependency_analysis::*;
pub use code_quality::*;
pub use version_control::*;
pub use cognitive_memory::*;
pub use multi_agent::*;
pub use materialization::*;
pub use testing::*;
pub use documentation::*;
pub use build_execution::*;
pub use monitoring::*;
pub use security_analysis::*;
pub use type_analysis::*;
pub use ai_assisted::*;
pub use advanced_testing::*;
pub use architecture_analysis::*;

use mcp_sdk::tool::ToolDefinition;
use mcp_sdk::Tool;

/// Returns all available tool definitions for the MCP server
///
/// Note: This is a placeholder implementation that returns empty definitions.
/// In production, tool definitions should be generated from actual tool instances
/// or through a macro system. The tools are properly implemented and can be
/// registered with the MCP server directly.
pub fn get_tools() -> Vec<ToolDefinition> {
    // TODO: Implement proper tool definition generation
    // For now, return an empty vector. Tools should be registered
    // directly with the MCP server using the Tool trait implementation.
    Vec::new()
}
