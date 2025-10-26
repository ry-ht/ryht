//! Unit tests for workspace MCP tools
//!
//! This test file imports and runs comprehensive unit tests for workspace management tools.
//! Tests are organized in the unit/workspace/ directory.
//!
//! Test coverage:
//! - workspace.create: 24 tests
//! - workspace.get: 23 tests
//! - workspace.list: 22 tests
//! - workspace.activate: 24 tests
//!
//! Total: 93 unit tests covering:
//! - Successful operations
//! - Error cases (invalid paths, missing workspace, etc.)
//! - Edge cases (empty workspace, large workspace, special characters)
//! - Performance benchmarks
//! - Token efficiency comparisons

#[path = "unit/workspace/mod.rs"]
mod workspace;
