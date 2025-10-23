//! Unit Tests for Code Manipulation MCP Tools
//!
//! This module contains comprehensive unit tests for all code manipulation tools.
//!
//! Test coverage includes:
//! - Creating functions, structs, and types
//! - Renaming symbols (functions, variables, types)
//! - Extracting code into functions
//! - Adding/removing parameters
//! - Implementing traits/interfaces
//! - AST validation after each manipulation
//! - Token efficiency measurements

pub mod test_create_function;
pub mod test_rename_symbol;
pub mod test_extract_function;
pub mod test_add_parameter;
pub mod test_create_struct;
pub mod test_implement_trait;

pub mod test_helpers;
