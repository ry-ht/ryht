pub mod code_indexer;
pub mod parser;
pub mod pattern_matcher;
pub mod search;
pub mod tree_sitter_parser;
pub mod vector;
pub mod watcher;
pub mod delta_indexer;
pub mod monorepo;
pub mod ignore;

use crate::types::{CodeSymbol, Query, QueryResult};
use anyhow::Result;
use std::path::Path;

pub use code_indexer::{CodeIndexer, DependencyDirection, DependencyGraph};
pub use parser::MonorepoParser;
pub use pattern_matcher::{
    CompiledPattern, GoPatternMatcher, JavaScriptPatternMatcher, PatternMatch, PatternMatcher,
    PatternSearchEngine, PythonPatternMatcher, RustPatternMatcher, TypeScriptPatternMatcher,
};
pub use search::SearchEngine;
pub use tree_sitter_parser::TreeSitterParser;
pub use vector::{HnswConfig, HnswIndex, VectorIndex, VECTOR_DIM};
pub use watcher::{FileWatcher, WatcherConfig, FileChangeEvent, FileChangeKind};
pub use delta_indexer::{DeltaIndexer, WatchStatus, ApplyResult};
pub use monorepo::{MonorepoConfig, MonorepoType};
pub use ignore::IgnoreMatcher;

/// Main indexer interface
#[async_trait::async_trait]
pub trait Indexer: Send + Sync {
    /// Index a project
    async fn index_project(&mut self, path: &Path, force: bool) -> Result<()>;

    /// Search symbols
    async fn search_symbols(&self, query: &Query) -> Result<QueryResult>;

    /// Get symbol by ID
    async fn get_symbol(&self, id: &str) -> Result<Option<CodeSymbol>>;

    /// Update a file in the index
    async fn update_file(&mut self, path: &Path) -> Result<()>;
}
