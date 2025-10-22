//! Integration Tests for MCP Tools
//!
//! These tests verify complete workflows across multiple tool categories:
//! - Workspace + VFS integration
//! - VFS + Parser integration
//! - Parser + Semantic graph integration
//! - Semantic + Search integration
//! - Navigation + Manipulation integration
//!
//! Each test demonstrates real-world usage patterns and validates
//! that data flows correctly between components.

mod workspace_vfs_integration;
mod vfs_parser_integration;
mod parser_semantic_integration;
mod semantic_search_integration;
mod code_nav_manipulation_integration;
