//! Advanced Code Metrics Counting Module
//!
//! This module provides efficient counting and statistics collection for AST nodes with:
//! - High-performance iterative traversal
//! - Flexible filtering for targeted counting
//! - Support for concurrent counting operations
//! - Real-time statistics accumulation
//! - Memory-efficient operations
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::analysis::count::{AstCounter, CountConfig, CountFilter};
//! use cortex_code_analysis::{Parser, RustLanguage};
//! use std::path::Path;
//!
//! let source = "fn main() { println!(); }";
//! let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("example.rs"))?;
//!
//! // Count all function declarations
//! let config = CountConfig::builder()
//!     .add_filter(CountFilter::Kind("function_item".to_string()))
//!     .build();
//!
//! let counter = AstCounter::new(&parser);
//! let stats = counter.count(&config)?;
//! println!("Found {} functions out of {} total nodes", stats.matched, stats.total);
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::node::Node;
use crate::traits::ParserTrait;
use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Filter types for counting operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountFilter {
    /// Count nodes of a specific kind
    Kind(String),

    /// Count nodes matching any of these kinds
    Kinds(Vec<String>),

    /// Count nodes at a specific depth
    AtDepth(usize),

    /// Count nodes within a depth range
    DepthRange { min: usize, max: usize },

    /// Count leaf nodes only
    LeafNodesOnly,

    /// Count nodes with children
    HasChildren,

    /// Count nodes matching a pattern in their text
    TextContains(String),
}

impl CountFilter {
    /// Check if a node matches this filter
    pub fn matches<'a>(&self, node: &Node<'a>, depth: usize, code: &[u8]) -> bool {
        match self {
            CountFilter::Kind(kind) => node.kind() == kind.as_str(),
            CountFilter::Kinds(kinds) => kinds.iter().any(|k| node.kind() == k.as_str()),
            CountFilter::AtDepth(d) => depth == *d,
            CountFilter::DepthRange { min, max } => depth >= *min && depth <= *max,
            CountFilter::LeafNodesOnly => node.child_count() == 0,
            CountFilter::HasChildren => node.child_count() > 0,
            CountFilter::TextContains(pattern) => {
                let text = &code[node.start_byte()..node.end_byte()];
                String::from_utf8_lossy(text).contains(pattern)
            }
        }
    }
}

/// Configuration for counting operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountConfig {
    /// Filters to apply (node counts if ANY filter matches)
    pub filters: Vec<CountFilter>,

    /// Whether to collect per-kind statistics
    pub collect_per_kind: bool,

    /// Whether to collect depth statistics
    pub collect_depth_stats: bool,

    /// Maximum depth to traverse (None = unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
}

impl Default for CountConfig {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            collect_per_kind: false,
            collect_depth_stats: false,
            max_depth: None,
        }
    }
}

impl CountConfig {
    /// Create a new builder for CountConfig
    pub fn builder() -> CountConfigBuilder {
        CountConfigBuilder::default()
    }
}

/// Builder for CountConfig
#[derive(Debug, Default)]
pub struct CountConfigBuilder {
    filters: Vec<CountFilter>,
    collect_per_kind: bool,
    collect_depth_stats: bool,
    max_depth: Option<usize>,
}

impl CountConfigBuilder {
    /// Add a filter
    pub fn add_filter(mut self, filter: CountFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add multiple filters
    pub fn filters(mut self, filters: Vec<CountFilter>) -> Self {
        self.filters = filters;
        self
    }

    /// Enable per-kind statistics collection
    pub fn collect_per_kind(mut self, collect: bool) -> Self {
        self.collect_per_kind = collect;
        self
    }

    /// Enable depth statistics collection
    pub fn collect_depth_stats(mut self, collect: bool) -> Self {
        self.collect_depth_stats = collect;
        self
    }

    /// Set maximum traversal depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Build the CountConfig
    pub fn build(self) -> CountConfig {
        CountConfig {
            filters: self.filters,
            collect_per_kind: self.collect_per_kind,
            collect_depth_stats: self.collect_depth_stats,
            max_depth: self.max_depth,
        }
    }
}

/// Statistics from counting operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CountStats {
    /// Total number of nodes visited
    pub total: usize,

    /// Number of nodes that matched filters
    pub matched: usize,

    /// Per-kind node counts (if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_kind: Option<HashMap<String, usize>>,

    /// Per-depth node counts (if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_depth: Option<HashMap<usize, usize>>,

    /// Maximum depth reached
    pub max_depth_reached: usize,

    /// Average node depth
    pub avg_depth: f64,
}

impl CountStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with optional per-kind and per-depth tracking
    pub fn with_tracking(per_kind: bool, per_depth: bool) -> Self {
        Self {
            total: 0,
            matched: 0,
            per_kind: if per_kind {
                Some(HashMap::new())
            } else {
                None
            },
            per_depth: if per_depth {
                Some(HashMap::new())
            } else {
                None
            },
            max_depth_reached: 0,
            avg_depth: 0.0,
        }
    }

    /// Merge another CountStats into this one
    pub fn merge(&mut self, other: &CountStats) {
        self.total += other.total;
        self.matched += other.matched;

        if let Some(ref mut this_per_kind) = self.per_kind {
            if let Some(ref other_per_kind) = other.per_kind {
                for (kind, count) in other_per_kind {
                    *this_per_kind.entry(kind.clone()).or_insert(0) += count;
                }
            }
        }

        if let Some(ref mut this_per_depth) = self.per_depth {
            if let Some(ref other_per_depth) = other.per_depth {
                for (depth, count) in other_per_depth {
                    *this_per_depth.entry(*depth).or_insert(0) += count;
                }
            }
        }

        self.max_depth_reached = self.max_depth_reached.max(other.max_depth_reached);

        // Recalculate average depth
        if self.total > 0 {
            let total_depth = (self.avg_depth * (self.total - other.total) as f64)
                + (other.avg_depth * other.total as f64);
            self.avg_depth = total_depth / self.total as f64;
        }
    }

    /// Get the percentage of matched nodes
    pub fn match_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.matched as f64 / self.total as f64) * 100.0
        }
    }

    /// Get the most common node kind
    pub fn most_common_kind(&self) -> Option<(&String, &usize)> {
        self.per_kind
            .as_ref()
            .and_then(|map| map.iter().max_by_key(|(_, count)| *count))
    }
}

impl fmt::Display for CountStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Total nodes: {}",
            self.total.to_formatted_string(&Locale::en)
        )?;
        writeln!(
            f,
            "Matched nodes: {}",
            self.matched.to_formatted_string(&Locale::en)
        )?;
        writeln!(f, "Match percentage: {:.2}%", self.match_percentage())?;
        writeln!(f, "Max depth: {}", self.max_depth_reached)?;
        writeln!(f, "Average depth: {:.2}", self.avg_depth)?;

        if let Some(ref per_kind) = self.per_kind {
            writeln!(f, "\nNode counts by kind:")?;
            let mut kinds: Vec<_> = per_kind.iter().collect();
            kinds.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (kind, count) in kinds.iter().take(10) {
                writeln!(
                    f,
                    "  {}: {}",
                    kind,
                    count.to_formatted_string(&Locale::en)
                )?;
            }
            if kinds.len() > 10 {
                writeln!(f, "  ... and {} more", kinds.len() - 10)?;
            }
        }

        if let Some(ref per_depth) = self.per_depth {
            writeln!(f, "\nNode counts by depth:")?;
            let mut depths: Vec<_> = per_depth.iter().collect();
            depths.sort_by_key(|(depth, _)| **depth);
            for (depth, count) in depths.iter().take(10) {
                writeln!(
                    f,
                    "  Depth {}: {}",
                    depth,
                    count.to_formatted_string(&Locale::en)
                )?;
            }
        }

        Ok(())
    }
}

/// High-performance AST counter
pub struct AstCounter<'a, T: ParserTrait> {
    parser: &'a T,
}

impl<'a, T: ParserTrait> AstCounter<'a, T> {
    /// Create a new AstCounter
    pub fn new(parser: &'a T) -> Self {
        Self { parser }
    }

    /// Count nodes matching the configuration
    pub fn count(&self, config: &CountConfig) -> Result<CountStats> {
        let root = self.parser.get_root();
        let code = self.parser.get_code();
        let mut cursor = root.cursor();
        let mut stack = Vec::with_capacity(1024);
        let mut depth_stack = Vec::with_capacity(1024);
        let mut stats = CountStats::with_tracking(
            config.collect_per_kind,
            config.collect_depth_stats,
        );
        let mut children = Vec::with_capacity(32);
        let mut total_depth = 0usize;

        stack.push(root);
        depth_stack.push(0usize);

        while let Some(node) = stack.pop() {
            let depth = depth_stack.pop().unwrap();

            // Check max depth limit
            if let Some(max_depth) = config.max_depth {
                if depth > max_depth {
                    continue;
                }
            }

            stats.total += 1;
            total_depth += depth;
            stats.max_depth_reached = stats.max_depth_reached.max(depth);

            // Update per-kind statistics
            if let Some(ref mut per_kind) = stats.per_kind {
                *per_kind.entry(node.kind().to_string()).or_insert(0) += 1;
            }

            // Update per-depth statistics
            if let Some(ref mut per_depth) = stats.per_depth {
                *per_depth.entry(depth).or_insert(0) += 1;
            }

            // Apply filters
            if !config.filters.is_empty() {
                let matches = config
                    .filters
                    .iter()
                    .any(|filter| filter.matches(&node, depth, code));

                if matches {
                    stats.matched += 1;
                }
            }

            // Add children to stack
            cursor.reset(&node);
            if cursor.goto_first_child() {
                children.clear();
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }

                for child in children.iter().rev() {
                    stack.push(*child);
                    depth_stack.push(depth + 1);
                }
            }
        }

        // Calculate average depth
        if stats.total > 0 {
            stats.avg_depth = total_depth as f64 / stats.total as f64;
        }

        Ok(stats)
    }

    /// Count all nodes (no filtering)
    pub fn count_all(&self) -> Result<CountStats> {
        let config = CountConfig::builder()
            .collect_per_kind(true)
            .collect_depth_stats(true)
            .build();
        self.count(&config)
    }

    /// Count nodes of a specific kind
    pub fn count_by_kind(&self, kind: &str) -> Result<usize> {
        let config = CountConfig::builder()
            .add_filter(CountFilter::Kind(kind.to_string()))
            .build();
        let stats = self.count(&config)?;
        Ok(stats.matched)
    }

    /// Count nodes matching multiple kinds
    pub fn count_by_kinds(&self, kinds: &[&str]) -> Result<usize> {
        let config = CountConfig::builder()
            .add_filter(CountFilter::Kinds(
                kinds.iter().map(|k| k.to_string()).collect(),
            ))
            .build();
        let stats = self.count(&config)?;
        Ok(stats.matched)
    }

    /// Count leaf nodes
    pub fn count_leaf_nodes(&self) -> Result<usize> {
        let config = CountConfig::builder()
            .add_filter(CountFilter::LeafNodesOnly)
            .build();
        let stats = self.count(&config)?;
        Ok(stats.matched)
    }
}

/// Thread-safe counter for concurrent operations
#[derive(Debug)]
pub struct ConcurrentCounter {
    stats: Arc<Mutex<CountStats>>,
}

impl ConcurrentCounter {
    /// Create a new concurrent counter
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(CountStats::new())),
        }
    }

    /// Create with tracking options
    pub fn with_tracking(per_kind: bool, per_depth: bool) -> Self {
        Self {
            stats: Arc::new(Mutex::new(CountStats::with_tracking(per_kind, per_depth))),
        }
    }

    /// Get a handle to the stats for concurrent updates
    pub fn stats_handle(&self) -> Arc<Mutex<CountStats>> {
        Arc::clone(&self.stats)
    }

    /// Merge stats from another counter
    pub fn merge(&self, other_stats: &CountStats) {
        let mut stats = self.stats.lock().unwrap();
        stats.merge(other_stats);
    }

    /// Get the final statistics
    pub fn finalize(self) -> CountStats {
        Arc::try_unwrap(self.stats)
            .ok()
            .and_then(|mutex| mutex.into_inner().ok())
            .unwrap_or_default()
    }
}

impl Default for ConcurrentCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to count nodes
pub fn count<T: ParserTrait>(parser: &T, filters: &[CountFilter]) -> Result<(usize, usize)> {
    let config = CountConfig::builder().filters(filters.to_vec()).build();
    let counter = AstCounter::new(parser);
    let stats = counter.count(&config)?;
    Ok((stats.matched, stats.total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage};
    use std::path::Path;

    #[test]
    fn test_count_by_kind() {
        let source = "fn main() {} fn test() {}";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let counter = AstCounter::new(&parser);
        let count = counter.count_by_kind("function_item").unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_all() {
        let source = "fn main() { let x = 1; }";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let counter = AstCounter::new(&parser);
        let stats = counter.count_all().unwrap();

        assert!(stats.total > 0);
        assert!(stats.per_kind.is_some());
        assert!(stats.per_depth.is_some());
    }

    #[test]
    fn test_count_leaf_nodes() {
        let source = "fn main() {}";
        let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();

        let counter = AstCounter::new(&parser);
        let count = counter.count_leaf_nodes().unwrap();

        assert!(count > 0);
    }

    #[test]
    fn test_stats_merge() {
        let mut stats1 = CountStats::new();
        stats1.total = 100;
        stats1.matched = 50;

        let mut stats2 = CountStats::new();
        stats2.total = 50;
        stats2.matched = 25;

        stats1.merge(&stats2);

        assert_eq!(stats1.total, 150);
        assert_eq!(stats1.matched, 75);
    }

    #[test]
    fn test_concurrent_counter() {
        let counter = ConcurrentCounter::new();

        let mut stats = CountStats::new();
        stats.total = 100;
        stats.matched = 50;

        counter.merge(&stats);

        let final_stats = counter.finalize();
        assert_eq!(final_stats.total, 100);
        assert_eq!(final_stats.matched, 50);
    }

    #[test]
    fn test_builder_pattern() {
        let config = CountConfig::builder()
            .add_filter(CountFilter::Kind("function_item".to_string()))
            .collect_per_kind(true)
            .collect_depth_stats(true)
            .max_depth(10)
            .build();

        assert_eq!(config.filters.len(), 1);
        assert!(config.collect_per_kind);
        assert!(config.collect_depth_stats);
        assert_eq!(config.max_depth, Some(10));
    }
}
