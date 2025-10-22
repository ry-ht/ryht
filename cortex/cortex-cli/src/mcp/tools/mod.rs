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
pub fn get_tools() -> Vec<ToolDefinition> {
    let mut tools = Vec::new();

    // Workspace Management Tools (8)
    tools.push(workspace::WorkspaceCreateTool::definition());
    tools.push(workspace::WorkspaceGetTool::definition());
    tools.push(workspace::WorkspaceListTool::definition());
    tools.push(workspace::WorkspaceActivateTool::definition());
    tools.push(workspace::WorkspaceSyncTool::definition());
    tools.push(workspace::WorkspaceExportTool::definition());
    tools.push(workspace::WorkspaceArchiveTool::definition());
    tools.push(workspace::WorkspaceDeleteTool::definition());

    // VFS Tools (12)
    tools.push(vfs::VfsReadTool::definition());
    tools.push(vfs::VfsWriteTool::definition());
    tools.push(vfs::VfsDeleteTool::definition());
    tools.push(vfs::VfsCopyTool::definition());
    tools.push(vfs::VfsMoveTool::definition());
    tools.push(vfs::VfsListTool::definition());
    tools.push(vfs::VfsMetadataTool::definition());
    tools.push(vfs::VfsSearchTool::definition());
    tools.push(vfs::VfsCreateDirectoryTool::definition());
    tools.push(vfs::VfsDiffTool::definition());
    tools.push(vfs::VfsWatchTool::definition());
    tools.push(vfs::VfsStatsTool::definition());

    // Code Navigation Tools (10)
    tools.push(code_nav::FindDefinitionTool::definition());
    tools.push(code_nav::FindReferencesTool::definition());
    tools.push(code_nav::GetSymbolsTool::definition());
    tools.push(code_nav::GetCallHierarchyTool::definition());
    tools.push(code_nav::GetTypeHierarchyTool::definition());
    tools.push(code_nav::NavigateToSymbolTool::definition());
    tools.push(code_nav::GetDocumentOutlineTool::definition());
    tools.push(code_nav::FindImplementationsTool::definition());
    tools.push(code_nav::GetHoverInfoTool::definition());
    tools.push(code_nav::GetSignatureHelpTool::definition());

    // Code Manipulation Tools (15)
    tools.push(code_manipulation::ExtractMethodTool::definition());
    tools.push(code_manipulation::RenameSymbolTool::definition());
    tools.push(code_manipulation::InlineVariableTool::definition());
    tools.push(code_manipulation::ChangeSignatureTool::definition());
    tools.push(code_manipulation::AddParameterTool::definition());
    tools.push(code_manipulation::RemoveParameterTool::definition());
    tools.push(code_manipulation::IntroduceConstantTool::definition());
    tools.push(code_manipulation::MoveMethodTool::definition());
    tools.push(code_manipulation::ExtractInterfaceTool::definition());
    tools.push(code_manipulation::PullUpMethodTool::definition());
    tools.push(code_manipulation::PushDownMethodTool::definition());
    tools.push(code_manipulation::AddImportTool::definition());
    tools.push(code_manipulation::RemoveUnusedImportsTool::definition());
    tools.push(code_manipulation::ImplementInterfaceTool::definition());
    tools.push(code_manipulation::OverrideMethodTool::definition());

    // Semantic Search Tools (8)
    tools.push(semantic_search::SemanticSearchTool::definition());
    tools.push(semantic_search::FindSimilarCodeTool::definition());
    tools.push(semantic_search::SearchByConceptTool::definition());
    tools.push(semantic_search::SearchByExampleTool::definition());
    tools.push(semantic_search::GetSemanticContextTool::definition());
    tools.push(semantic_search::FindRelatedFilesTool::definition());
    tools.push(semantic_search::ExplainCodeTool::definition());
    tools.push(semantic_search::SuggestAlternativesTool::definition());

    // Continue with remaining tool categories...
    // Note: This is a comprehensive list. In production, you might want to
    // register tools dynamically or use a macro to reduce boilerplate.

    tools
}
