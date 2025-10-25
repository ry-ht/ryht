//! Advanced code analysis module.
//!
//! This module provides traits and implementations for advanced AST node analysis,
//! including node classification (checker) and information extraction (getter).
//! These are production-ready implementations integrated from experimental code.
//!
//! # Overview
//!
//! The analysis module provides two main traits:
//!
//! - [`NodeChecker`]: For classifying nodes (comments, functions, closures, etc.)
//! - [`NodeGetter`]: For extracting information (names, space kinds, operator types, etc.)
//!
//! Both traits support multiple programming languages through language-specific
//! implementations.
//!
//! # Examples
//!
//! ## Using NodeChecker
//!
//! ```rust
//! use cortex_code_analysis::{TreeSitterWrapper, Lang};
//! use cortex_code_analysis::analysis::{NodeChecker, DefaultNodeChecker};
//! use cortex_code_analysis::Node;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut parser = TreeSitterWrapper::new(tree_sitter_rust::LANGUAGE.into())?;
//! let code = "// This is a comment\nfn main() {}";
//! let tree = parser.parse(code)?;
//! let root_node = tree.root_node();
//! let root = Node::new(root_node);
//!
//! // Check if nodes are comments, functions, etc.
//! for node in root.children() {
//!     if DefaultNodeChecker::is_comment(&node, Lang::Rust) {
//!         println!("Found a comment");
//!     }
//!     if DefaultNodeChecker::is_func(&node, Lang::Rust) {
//!         println!("Found a function");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Using NodeGetter
//!
//! ```rust
//! use cortex_code_analysis::{TreeSitterWrapper, Lang};
//! use cortex_code_analysis::analysis::{NodeGetter, DefaultNodeGetter, SpaceKind};
//! use cortex_code_analysis::Node;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut parser = TreeSitterWrapper::new(tree_sitter_rust::LANGUAGE.into())?;
//! let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
//! let tree = parser.parse(code)?;
//! let root_node = tree.root_node();
//! let root = Node::new(root_node);
//!
//! for node in root.children() {
//!     let space_kind = DefaultNodeGetter::get_space_kind(&node, Lang::Rust);
//!     if space_kind == SpaceKind::Function {
//!         let name = DefaultNodeGetter::get_func_name(&node, code.as_bytes(), Lang::Rust);
//!         println!("Function: {:?}", name);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod checker;
pub mod getter;
pub mod types;

#[cfg(test)]
mod tests;

pub use checker::{DefaultNodeChecker, NodeChecker};
pub use getter::{DefaultNodeGetter, NodeGetter};
pub use types::{HalsteadType, SpaceKind};
