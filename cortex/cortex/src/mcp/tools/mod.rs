//! Cortex MCP Tools
//!
//! This module provides 175+ MCP tools for Cortex, organized by category:
//! - Workspace Management (8 tools)
//! - Virtual Filesystem (17 tools)
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

/// Helper function to convert a Tool implementation to a ToolDefinition
fn tool_to_definition<T: Tool>(tool: &T) -> ToolDefinition {
    ToolDefinition {
        name: tool.name().to_string(),
        description: tool.description().map(|s| s.to_string()),
        input_schema: tool.input_schema(),
        output_schema: None, // Tools don't currently expose output_schema
    }
}

/// Returns all available tool definitions for the MCP server
///
/// This function creates instances of all implemented tools and converts them
/// to ToolDefinition for registration with the MCP server. While this approach
/// requires instantiating tools, it provides a centralized registry that can
/// be used for tool discovery and validation.
///
/// Note: Tool instances are created with default/placeholder contexts as we're
/// only extracting metadata (name, description, schema). For actual execution,
/// tools should be properly initialized with their required dependencies.
pub fn get_tools() -> Vec<ToolDefinition> {
    // Note: This implementation creates dummy instances of each tool just to extract
    // their metadata. In production, consider using a macro-based approach to generate
    // ToolDefinitions at compile time, or implement a registration pattern where tools
    // self-register their definitions without requiring instantiation.
    //
    // For now, we return an empty vector since instantiating all 170+ tools requires
    // complex context setup (storage, VFS, parsers, etc.) that isn't available at
    // the module level. Tools should be registered directly with the MCP server
    // when it's initialized with proper context.
    //
    // Future improvement: Implement a proc macro like #[derive(ToolDefinition)]
    // that generates the definition at compile time without needing instantiation.
    Vec::new()
}
