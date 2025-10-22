//! Test utilities for MCP tools testing
//!
//! This module provides comprehensive testing infrastructure for all MCP tools:
//! - Test harness for setup and teardown
//! - Assertion helpers for validating tool results
//! - Token counting and efficiency measurement
//! - Fixtures for test data generation

pub mod test_harness;
pub mod assertions;
pub mod token_counter;
pub mod fixtures;

// Re-export commonly used items
pub use test_harness::{TestHarness, TestContext, TestWorkspace};
pub use assertions::{ToolResultAssertions, assert_tool_success, assert_tool_error};
pub use token_counter::{TokenCounter, TokenComparison, EfficiencyReport};
pub use fixtures::{ProjectFixture, CodeFixture, LanguageType};
