//! Cyclomatic Complexity Metric
//!
//! This module implements McCabe's cyclomatic complexity metric, which measures
//! the number of linearly independent paths through a program's source code.

use serde::{Serialize, Deserialize};
use std::fmt;
use tree_sitter::Node;

/// Cyclomatic complexity statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CyclomaticStats {
    /// Sum of cyclomatic complexity values
    cyclomatic_sum: f64,
    /// Current cyclomatic complexity value
    cyclomatic: f64,
    /// Number of spaces (functions/methods)
    n: usize,
    /// Maximum cyclomatic complexity
    cyclomatic_max: f64,
    /// Minimum cyclomatic complexity
    cyclomatic_min: f64,
}

impl Default for CyclomaticStats {
    fn default() -> Self {
        Self {
            cyclomatic_sum: 0.0,
            cyclomatic: 1.0,
            n: 1,
            cyclomatic_max: 0.0,
            cyclomatic_min: f64::MAX,
        }
    }
}

impl fmt::Display for CyclomaticStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.cyclomatic_sum(),
            self.cyclomatic_average(),
            self.cyclomatic_min(),
            self.cyclomatic_max()
        )
    }
}

impl CyclomaticStats {
    /// Creates a new CyclomaticStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges another CyclomaticStats into this one
    pub fn merge(&mut self, other: &CyclomaticStats) {
        self.cyclomatic_max = self.cyclomatic_max.max(other.cyclomatic_max);
        self.cyclomatic_min = self.cyclomatic_min.min(other.cyclomatic_min);
        self.cyclomatic_sum += other.cyclomatic_sum;
        self.n += other.n;
    }

    /// Returns the current cyclomatic complexity value
    pub fn cyclomatic(&self) -> f64 {
        self.cyclomatic
    }

    /// Returns the sum of cyclomatic complexity values
    pub fn cyclomatic_sum(&self) -> f64 {
        self.cyclomatic_sum
    }

    /// Returns the average cyclomatic complexity
    pub fn cyclomatic_average(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.cyclomatic_sum / self.n as f64
        }
    }

    /// Returns the maximum cyclomatic complexity
    pub fn cyclomatic_max(&self) -> f64 {
        self.cyclomatic_max
    }

    /// Returns the minimum cyclomatic complexity
    pub fn cyclomatic_min(&self) -> f64 {
        if self.cyclomatic_min == f64::MAX {
            0.0
        } else {
            self.cyclomatic_min
        }
    }

    /// Increments the cyclomatic complexity
    pub fn increment(&mut self) {
        self.cyclomatic += 1.0;
    }

    /// Computes sum and min/max
    pub fn compute_sum(&mut self) {
        self.cyclomatic_sum += self.cyclomatic;
    }

    /// Computes min/max and sum
    pub fn compute_minmax(&mut self) {
        self.cyclomatic_max = self.cyclomatic_max.max(self.cyclomatic);
        self.cyclomatic_min = self.cyclomatic_min.min(self.cyclomatic);
        self.compute_sum();
    }
}

/// Computes cyclomatic complexity for a tree-sitter node
pub fn compute_cyclomatic(node: Node, source: &[u8]) -> CyclomaticStats {
    let mut stats = CyclomaticStats::new();
    compute_cyclomatic_recursive(node, source, &mut stats);
    stats.compute_minmax();
    stats
}

fn compute_cyclomatic_recursive(node: Node, source: &[u8], stats: &mut CyclomaticStats) {
    let kind = node.kind();

    // Count decision points based on node kind
    match kind {
        // Rust control flow
        "if_expression" | "match_arm" | "while_expression"
        | "for_expression" | "loop_expression" => {
            stats.increment();
        }
        // Boolean operators
        "&&" | "||" => {
            stats.increment();
        }
        // TypeScript/JavaScript control flow
        "if_statement" | "for_statement" | "while_statement"
        | "switch_case" | "catch_clause" | "ternary_expression" => {
            stats.increment();
        }
        _ => {}
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        compute_cyclomatic_recursive(child, source, stats);
    }
}

/// Computes cyclomatic complexity for Rust code
pub fn compute_rust_cyclomatic(node: Node, source: &[u8]) -> CyclomaticStats {
    let mut stats = CyclomaticStats::new();
    compute_rust_cyclomatic_recursive(node, source, &mut stats);
    stats.compute_minmax();
    stats
}

fn compute_rust_cyclomatic_recursive(node: Node, source: &[u8], stats: &mut CyclomaticStats) {
    let kind = node.kind();

    match kind {
        "if_expression" | "match_arm" | "while_expression"
        | "for_expression" | "loop_expression" | "try_expression" => {
            stats.increment();
        }
        "binary_expression" => {
            // Check for && or ||
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "&&" || child.kind() == "||" {
                    stats.increment();
                }
            }
        }
        _ => {}
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        compute_rust_cyclomatic_recursive(child, source, stats);
    }
}

/// Computes cyclomatic complexity for TypeScript/JavaScript code
pub fn compute_typescript_cyclomatic(node: Node, source: &[u8]) -> CyclomaticStats {
    let mut stats = CyclomaticStats::new();
    compute_typescript_cyclomatic_recursive(node, source, &mut stats);
    stats.compute_minmax();
    stats
}

fn compute_typescript_cyclomatic_recursive(node: Node, source: &[u8], stats: &mut CyclomaticStats) {
    let kind = node.kind();

    match kind {
        "if_statement" | "for_statement" | "while_statement"
        | "switch_case" | "catch_clause" | "ternary_expression" => {
            stats.increment();
        }
        "binary_expression" => {
            // Check for && or ||
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "&&" || child.kind() == "||" {
                    stats.increment();
                }
            }
        }
        _ => {}
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        compute_typescript_cyclomatic_recursive(child, source, stats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclomatic_default() {
        let stats = CyclomaticStats::default();
        assert_eq!(stats.cyclomatic(), 1.0);
        assert_eq!(stats.cyclomatic_sum(), 0.0);
    }

    #[test]
    fn test_cyclomatic_increment() {
        let mut stats = CyclomaticStats::default();
        stats.increment();
        assert_eq!(stats.cyclomatic(), 2.0);
    }

    #[test]
    fn test_cyclomatic_merge() {
        let mut stats1 = CyclomaticStats::default();
        stats1.cyclomatic = 5.0;
        stats1.compute_minmax();

        let mut stats2 = CyclomaticStats::default();
        stats2.cyclomatic = 3.0;
        stats2.compute_minmax();

        stats1.merge(&stats2);
        assert_eq!(stats1.cyclomatic_max(), 5.0);
        assert_eq!(stats1.cyclomatic_min(), 3.0);
    }
}
