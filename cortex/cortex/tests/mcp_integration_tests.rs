//! Comprehensive Integration Tests for MCP Tools
//!
//! This test module imports and runs all integration tests that verify
//! complete workflows across multiple tool categories.
//!
//! Test Categories:
//! - Workspace + VFS Integration
//! - VFS + Parser Integration
//! - Parser + Semantic Graph Integration
//! - Semantic + Search Integration
//! - Code Navigation + Manipulation Integration
//!
//! Each category tests real-world scenarios with multiple tools working together.

#[path = "mcp/utils/mod.rs"]
mod utils;

#[path = "mcp/integration/mod.rs"]
mod integration;
