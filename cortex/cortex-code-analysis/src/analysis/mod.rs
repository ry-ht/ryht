//! Advanced code analysis module.
//!
//! This module provides a comprehensive suite of advanced code analysis tools:
//!
//! ## Core Analysis
//! - [`NodeChecker`]: Node classification (comments, functions, closures, etc.)
//! - [`NodeGetter`]: Information extraction (names, space kinds, operator types, etc.)
//!
//! ## Advanced Features
//! - [`find`]: High-performance AST search and navigation
//! - [`count`]: Efficient node counting with statistics
//! - [`alterator`]: AST transformation and mutation
//! - [`tools`]: Utility functions for file I/O and language detection
//! - [`cache`]: LRU caching for parsed ASTs and computed metrics
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
//! ## Using Advanced Search
//!
//! ```rust
//! use cortex_code_analysis::analysis::find::{AstFinder, FindConfig, NodeFilter};
//! use cortex_code_analysis::{Parser, Lang};
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut parser = Parser::new(Lang::Rust)?;
//! let source = "fn main() {} fn test() {}";
//! parser.parse(source.as_bytes(), None)?;
//!
//! let config = FindConfig::builder()
//!     .add_filter(NodeFilter::Kind("function_item".to_string()))
//!     .build();
//!
//! let finder = AstFinder::new(&parser);
//! let results = finder.find(&config)?;
//! println!("Found {} functions", results.nodes.len());
//! # Ok(())
//! # }
//! ```

pub mod alterator;
pub mod cache;
pub mod checker;
pub mod count;
pub mod find;
pub mod getter;
pub mod tools;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export core types
pub use alterator::{Alterator, TransformConfig, TransformConfigBuilder};
pub use cache::{
    AstCache, Cache, CacheBuilder, CacheManager, CachedAst, CachedMetrics, CachedSearch,
    MetricsCache, SearchCache, SearchKey, SourceKey,
};
pub use checker::{DefaultNodeChecker, NodeChecker};
pub use count::{
    AstCounter, ConcurrentCounter, CountConfig, CountConfigBuilder, CountFilter, CountStats,
};
pub use find::{
    AstFinder, FindConfig, FindConfigBuilder, FindResult, NodeFilter,
};
pub use getter::{DefaultNodeGetter, NodeGetter};
pub use types::{HalsteadType, SpaceKind};
