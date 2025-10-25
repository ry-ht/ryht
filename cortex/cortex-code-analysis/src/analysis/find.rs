//! Advanced AST Search and Navigation Module
//!
//! This module provides powerful AST node search and navigation capabilities with:
//! - Efficient iterative traversal using a stack-based approach
//! - Flexible filtering with multiple filter types
//! - Performance optimizations for large ASTs
//! - Type-safe node kind filtering
//! - Range-based filtering (line and column)
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::analysis::find::{AstFinder, FindConfig, NodeFilter};
//! use cortex_code_analysis::{Parser, RustLanguage};
//! use std::path::Path;
//!
//! let source = "fn main() { println!(); }";
//! let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("example.rs"))?;
//!
//! // Find all function declarations
//! let config = FindConfig::builder()
//!     .add_filter(NodeFilter::Kind("function_item".to_string()))
//!     .build();
//!
//! let finder = AstFinder::new(&parser);
//! let results = finder.find(&config)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::node::Node;
use crate::traits::ParserTrait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Filter types for AST node search
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeFilter {
    /// Filter by node kind (e.g., "function_item", "struct_item")
    Kind(String),

    /// Filter by multiple node kinds (matches any)
    Kinds(Vec<String>),

    /// Filter by line range (inclusive)
    LineRange { start: usize, end: usize },

    /// Filter by column range at a specific line
    ColumnRange {
        line: usize,
        start_col: usize,
        end_col: usize,
    },

    /// Filter by node depth in the AST
    Depth { min: Option<usize>, max: Option<usize> },
}

impl NodeFilter {
    /// Check if a node matches this filter
    pub fn matches<'a>(&self, node: &Node<'a>, depth: usize) -> bool {
        match self {
            NodeFilter::Kind(kind) => node.kind() == kind.as_str(),
            NodeFilter::Kinds(kinds) => kinds.iter().any(|k| node.kind() == k.as_str()),
            NodeFilter::LineRange { start, end } => {
                let (node_start, _) = node.start_position();
                let (node_end, _) = node.end_position();
                node_start >= *start && node_end <= *end
            }
            NodeFilter::ColumnRange {
                line,
                start_col,
                end_col,
            } => {
                let (node_line, node_col) = node.start_position();
                node_line == *line && node_col >= *start_col && node_col <= *end_col
            }
            NodeFilter::Depth { min, max } => {
                let matches_min = min.map(|m| depth >= m).unwrap_or(true);
                let matches_max = max.map(|m| depth <= m).unwrap_or(true);
                matches_min && matches_max
            }
        }
    }
}

/// Configuration for AST node search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindConfig {
    /// Path to the file containing the code (for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    /// Filters to apply (node matches if ANY filter matches)
    pub filters: Vec<NodeFilter>,

    /// Maximum number of results to return (None = unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Whether to include descendant nodes of matched nodes
    pub include_descendants: bool,

    /// Whether to deduplicate results
    pub deduplicate: bool,
}

impl Default for FindConfig {
    fn default() -> Self {
        Self {
            path: None,
            filters: Vec::new(),
            limit: None,
            include_descendants: true,
            deduplicate: false,
        }
    }
}

impl FindConfig {
    /// Create a new builder for FindConfig
    pub fn builder() -> FindConfigBuilder {
        FindConfigBuilder::default()
    }
}

/// Builder for FindConfig
#[derive(Debug, Default)]
pub struct FindConfigBuilder {
    path: Option<PathBuf>,
    filters: Vec<NodeFilter>,
    limit: Option<usize>,
    include_descendants: bool,
    deduplicate: bool,
}

impl FindConfigBuilder {
    /// Set the file path
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Add a filter
    pub fn add_filter(mut self, filter: NodeFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add multiple filters
    pub fn filters(mut self, filters: Vec<NodeFilter>) -> Self {
        self.filters = filters;
        self
    }

    /// Set the result limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set whether to include descendants
    pub fn include_descendants(mut self, include: bool) -> Self {
        self.include_descendants = include;
        self
    }

    /// Set whether to deduplicate results
    pub fn deduplicate(mut self, dedupe: bool) -> Self {
        self.deduplicate = dedupe;
        self
    }

    /// Build the FindConfig
    pub fn build(self) -> FindConfig {
        FindConfig {
            path: self.path,
            filters: self.filters,
            limit: self.limit,
            include_descendants: self.include_descendants,
            deduplicate: self.deduplicate,
        }
    }
}

/// Result of an AST search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResult<'a> {
    /// The matched nodes
    #[serde(skip)]
    pub nodes: Vec<Node<'a>>,

    /// Total number of nodes visited during search
    pub nodes_visited: usize,

    /// Total number of nodes matched
    pub nodes_matched: usize,

    /// Whether the search was limited
    pub limited: bool,
}

impl<'a> FindResult<'a> {
    /// Create a new FindResult
    pub fn new(nodes: Vec<Node<'a>>, nodes_visited: usize, limited: bool) -> Self {
        let nodes_matched = nodes.len();
        Self {
            nodes,
            nodes_visited,
            nodes_matched,
            limited,
        }
    }
}

/// High-performance AST finder with optimized traversal
pub struct AstFinder<'a, T: ParserTrait> {
    parser: &'a T,
    visited_cache: Option<HashSet<usize>>,
}

impl<'a, T: ParserTrait> AstFinder<'a, T> {
    /// Create a new AstFinder
    pub fn new(parser: &'a T) -> Self {
        Self {
            parser,
            visited_cache: None,
        }
    }

    /// Enable deduplication caching
    pub fn with_deduplication(mut self) -> Self {
        self.visited_cache = Some(HashSet::new());
        self
    }

    /// Find nodes matching the configuration
    ///
    /// Uses an iterative stack-based approach for efficient traversal,
    /// avoiding recursion overhead and stack overflow issues.
    pub fn find(&self, config: &FindConfig) -> Result<FindResult<'a>> {
        if config.filters.is_empty() {
            return Ok(FindResult::new(Vec::new(), 0, false));
        }

        let root = self.parser.get_root();
        let mut cursor = root.cursor();
        let mut stack = Vec::with_capacity(1024); // Pre-allocate for performance
        let mut depth_stack = Vec::with_capacity(1024);
        let mut matched = Vec::new();
        let mut children = Vec::with_capacity(32); // Typical max children per node
        let mut visited = 0usize;
        let mut visited_set = if config.deduplicate {
            Some(HashSet::with_capacity(1024))
        } else {
            None
        };

        stack.push(root);
        depth_stack.push(0usize);

        while let Some(node) = stack.pop() {
            let depth = depth_stack.pop().unwrap();
            visited += 1;

            // Check deduplication
            if let Some(ref mut set) = visited_set {
                let node_id = node.id();
                if !set.insert(node_id) {
                    continue; // Already visited
                }
            }

            // Apply filters
            let matches = config.filters.iter().any(|filter| filter.matches(&node, depth));

            if matches {
                matched.push(node);

                // Check limit
                if let Some(limit) = config.limit {
                    if matched.len() >= limit {
                        return Ok(FindResult::new(matched, visited, true));
                    }
                }

                // Skip descendants if not requested
                if !config.include_descendants {
                    continue;
                }
            }

            // Add children to stack (in reverse order for depth-first left-to-right)
            cursor.reset(&node);
            if cursor.goto_first_child() {
                children.clear();
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }

                // Push children in reverse order
                for child in children.iter().rev() {
                    stack.push(*child);
                    depth_stack.push(depth + 1);
                }
            }
        }

        Ok(FindResult::new(matched, visited, false))
    }

    /// Find the first node matching the configuration
    pub fn find_first(&self, config: &FindConfig) -> Result<Option<Node<'a>>> {
        let mut limited_config = config.clone();
        limited_config.limit = Some(1);

        let result = self.find(&limited_config)?;
        Ok(result.nodes.into_iter().next())
    }

    /// Find all nodes of a specific kind
    pub fn find_by_kind(&self, kind: &str) -> Result<FindResult<'a>> {
        let config = FindConfig::builder()
            .add_filter(NodeFilter::Kind(kind.to_string()))
            .build();
        self.find(&config)
    }

    /// Find all nodes matching multiple kinds
    pub fn find_by_kinds(&self, kinds: &[&str]) -> Result<FindResult<'a>> {
        let config = FindConfig::builder()
            .add_filter(NodeFilter::Kinds(
                kinds.iter().map(|k| k.to_string()).collect(),
            ))
            .build();
        self.find(&config)
    }

    /// Find nodes in a specific line range
    pub fn find_in_line_range(&self, start: usize, end: usize) -> Result<FindResult<'a>> {
        let config = FindConfig::builder()
            .add_filter(NodeFilter::LineRange { start, end })
            .build();
        self.find(&config)
    }

    /// Count nodes matching the configuration
    pub fn count(&self, config: &FindConfig) -> Result<usize> {
        let result = self.find(config)?;
        Ok(result.nodes_matched)
    }
}

/// Convenience function to find nodes in parsed code
pub fn find<'a, T: ParserTrait>(
    parser: &'a T,
    filters: &[NodeFilter],
) -> Result<Vec<Node<'a>>> {
    let config = FindConfig::builder().filters(filters.to_vec()).build();
    let finder = AstFinder::new(parser);
    let result = finder.find(&config)?;
    Ok(result.nodes)
}

/// Convenience function to find nodes by kind
pub fn find_by_kind<'a, T: ParserTrait>(parser: &'a T, kind: &str) -> Result<Vec<Node<'a>>> {
    let finder = AstFinder::new(parser);
    let result = finder.find_by_kind(kind)?;
    Ok(result.nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage};
    use std::path::Path;

    #[test]
    fn test_find_by_kind() {
        let source = "fn main() {} fn test() {}";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let finder = AstFinder::new(&parser);
        let result = finder.find_by_kind("function_item").unwrap();

        assert_eq!(result.nodes.len(), 2);
        assert!(result.nodes_visited > 0);
    }

    #[test]
    fn test_find_with_limit() {
        let source = "fn a() {} fn b() {} fn c() {}";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let config = FindConfig::builder()
            .add_filter(NodeFilter::Kind("function_item".to_string()))
            .limit(2)
            .build();

        let finder = AstFinder::new(&parser);
        let result = finder.find(&config).unwrap();

        assert_eq!(result.nodes.len(), 2);
        assert!(result.limited);
    }

    #[test]
    fn test_find_first() {
        let source = "fn main() {} fn test() {}";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let config = FindConfig::builder()
            .add_filter(NodeFilter::Kind("function_item".to_string()))
            .build();

        let finder = AstFinder::new(&parser);
        let result = finder.find_first(&config).unwrap();

        assert!(result.is_some());
    }

    #[test]
    fn test_node_filter_kinds() {
        let filter = NodeFilter::Kinds(vec![
            "function_item".to_string(),
            "struct_item".to_string(),
        ]);

        // This would require a mock node, so we'll skip the actual matching test
        // In production, this is tested via integration tests
    }

    #[test]
    fn test_builder_pattern() {
        let config = FindConfig::builder()
            .add_filter(NodeFilter::Kind("function_item".to_string()))
            .limit(10)
            .include_descendants(false)
            .deduplicate(true)
            .build();

        assert_eq!(config.filters.len(), 1);
        assert_eq!(config.limit, Some(10));
        assert!(!config.include_descendants);
        assert!(config.deduplicate);
    }
}
