//! Halstead Complexity Metrics
//!
//! This module implements Maurice Halstead's software metrics suite,
//! which provide measurements about program complexity based on
//! operators and operands.
//!
//! This implementation includes advanced operator/operand tracking that maintains
//! frequency maps for detailed analysis and most frequent element tracking.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

/// Advanced Halstead maps for tracking operators and operands
///
/// This structure maintains frequency maps for both operators (by kind_id)
/// and operands (by content), enabling detailed analysis of code complexity patterns.
#[derive(Debug, Default, Clone)]
pub struct HalsteadMaps<'a> {
    /// Map of operator kind IDs to their frequency counts
    pub operators: HashMap<u16, u64>,
    /// Map of operand content to their frequency counts
    pub operands: HashMap<&'a [u8], u64>,
}

impl<'a> HalsteadMaps<'a> {
    /// Creates a new empty HalsteadMaps
    pub fn new() -> Self {
        HalsteadMaps {
            operators: HashMap::default(),
            operands: HashMap::default(),
        }
    }

    /// Merges another HalsteadMaps into this one
    pub fn merge(&mut self, other: &HalsteadMaps<'a>) {
        for (k, v) in other.operators.iter() {
            *self.operators.entry(*k).or_insert(0) += v;
        }
        for (k, v) in other.operands.iter() {
            *self.operands.entry(*k).or_insert(0) += v;
        }
    }

    /// Finalizes the maps and populates the provided HalsteadStats
    pub fn finalize(&self, stats: &mut HalsteadStats) {
        stats.u_operators = self.operators.len() as u64;
        stats.operators = self.operators.values().sum::<u64>();
        stats.u_operands = self.operands.len() as u64;
        stats.operands = self.operands.values().sum::<u64>();
    }

    /// Returns the most frequent operators with their counts
    pub fn most_frequent_operators(&self, limit: usize) -> Vec<(u16, u64)> {
        let mut items: Vec<(u16, u64)> = self.operators.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Returns the most frequent operands with their counts
    pub fn most_frequent_operands(&self, limit: usize) -> Vec<(&'a [u8], u64)> {
        let mut items: Vec<(&'a [u8], u64)> = self.operands.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Returns the number of unique operators
    pub fn unique_operator_count(&self) -> usize {
        self.operators.len()
    }

    /// Returns the number of unique operands
    pub fn unique_operand_count(&self) -> usize {
        self.operands.len()
    }
}

/// Halstead metrics statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HalsteadStats {
    /// Number of distinct operators (η1)
    u_operators: u64,
    /// Total number of operators (N1)
    operators: u64,
    /// Number of distinct operands (η2)
    u_operands: u64,
    /// Total number of operands (N2)
    operands: u64,
}

impl Default for HalsteadStats {
    fn default() -> Self {
        Self {
            u_operators: 0,
            operators: 0,
            u_operands: 0,
            operands: 0,
        }
    }
}

impl fmt::Display for HalsteadStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "n1: {}, N1: {}, n2: {}, N2: {}, length: {}, vocabulary: {}, volume: {}, difficulty: {}, effort: {}, time: {}, bugs: {}",
            self.u_operators(),
            self.operators(),
            self.u_operands(),
            self.operands(),
            self.length(),
            self.vocabulary(),
            self.volume(),
            self.difficulty(),
            self.effort(),
            self.time(),
            self.bugs(),
        )
    }
}

impl HalsteadStats {
    /// Creates a new HalsteadStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates HalsteadStats from operator and operand counts
    pub fn from_counts(
        u_operators: u64,
        operators: u64,
        u_operands: u64,
        operands: u64,
    ) -> Self {
        Self {
            u_operators,
            operators,
            u_operands,
            operands,
        }
    }

    /// Returns η1, the number of distinct operators
    pub fn u_operators(&self) -> f64 {
        self.u_operators as f64
    }

    /// Returns N1, the number of total operators
    pub fn operators(&self) -> f64 {
        self.operators as f64
    }

    /// Returns η2, the number of distinct operands
    pub fn u_operands(&self) -> f64 {
        self.u_operands as f64
    }

    /// Returns N2, the number of total operands
    pub fn operands(&self) -> f64 {
        self.operands as f64
    }

    /// Returns the program length (N = N1 + N2)
    pub fn length(&self) -> f64 {
        self.operands() + self.operators()
    }

    /// Returns the calculated estimated program length
    pub fn estimated_program_length(&self) -> f64 {
        if self.u_operators() == 0.0 || self.u_operands() == 0.0 {
            return 0.0;
        }
        self.u_operators() * self.u_operators().log2()
            + self.u_operands() * self.u_operands().log2()
    }

    /// Returns the purity ratio
    pub fn purity_ratio(&self) -> f64 {
        let length = self.length();
        if length == 0.0 {
            return 0.0;
        }
        self.estimated_program_length() / length
    }

    /// Returns the program vocabulary (η = η1 + η2)
    pub fn vocabulary(&self) -> f64 {
        self.u_operands() + self.u_operators()
    }

    /// Returns the program volume (V = N * log2(η))
    ///
    /// Unit of measurement: bits
    pub fn volume(&self) -> f64 {
        let vocab = self.vocabulary();
        if vocab == 0.0 {
            return 0.0;
        }
        self.length() * vocab.log2()
    }

    /// Returns the estimated difficulty required to program (D = (η1/2) * (N2/η2))
    pub fn difficulty(&self) -> f64 {
        if self.u_operands() == 0.0 {
            return 0.0;
        }
        self.u_operators() / 2.0 * self.operands() / self.u_operands()
    }

    /// Returns the estimated level of difficulty required to program (L = 1/D)
    pub fn level(&self) -> f64 {
        let diff = self.difficulty();
        if diff == 0.0 {
            return 0.0;
        }
        1.0 / diff
    }

    /// Returns the estimated effort required to program (E = D * V)
    pub fn effort(&self) -> f64 {
        self.difficulty() * self.volume()
    }

    /// Returns the estimated time required to program (T = E/18)
    ///
    /// Unit of measurement: seconds
    /// The Stroud number (18) represents moments per second
    pub fn time(&self) -> f64 {
        self.effort() / 18.0
    }

    /// Returns the estimated number of delivered bugs (B = E^(2/3) / 3000)
    pub fn bugs(&self) -> f64 {
        self.effort().powf(2.0 / 3.0) / 3000.0
    }

    /// Merges another HalsteadStats into this one
    pub fn merge(&mut self, _other: &HalsteadStats) {
        // Halstead metrics are typically computed per-function,
        // merging doesn't make semantic sense for these metrics
    }
}

/// Helper structure for collecting operators and operands (string-based)
///
/// This is the high-level collector that uses string keys for compatibility
/// with existing code. For advanced AST-based analysis, use HalsteadMaps directly.
#[derive(Debug, Default)]
pub struct HalsteadCollector<'a> {
    operators: HashMap<&'a str, u64>,
    operands: HashMap<&'a str, u64>,
}

impl<'a> HalsteadCollector<'a> {
    /// Creates a new collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an operator
    pub fn add_operator(&mut self, op: &'a str) {
        *self.operators.entry(op).or_insert(0) += 1;
    }

    /// Adds an operand
    pub fn add_operand(&mut self, operand: &'a str) {
        *self.operands.entry(operand).or_insert(0) += 1;
    }

    /// Finalizes and returns the Halstead statistics
    pub fn finalize(&self) -> HalsteadStats {
        HalsteadStats {
            u_operators: self.operators.len() as u64,
            operators: self.operators.values().sum::<u64>(),
            u_operands: self.operands.len() as u64,
            operands: self.operands.values().sum::<u64>(),
        }
    }

    /// Merges another collector into this one
    pub fn merge(&mut self, other: &HalsteadCollector<'a>) {
        for (k, v) in other.operators.iter() {
            *self.operators.entry(*k).or_insert(0) += v;
        }
        for (k, v) in other.operands.iter() {
            *self.operands.entry(*k).or_insert(0) += v;
        }
    }

    /// Returns the most frequent operators with their counts
    pub fn most_frequent_operators(&self, limit: usize) -> Vec<(&'a str, u64)> {
        let mut items: Vec<(&'a str, u64)> = self.operators.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Returns the most frequent operands with their counts
    pub fn most_frequent_operands(&self, limit: usize) -> Vec<(&'a str, u64)> {
        let mut items: Vec<(&'a str, u64)> = self.operands.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.into_iter().take(limit).collect()
    }

    /// Returns the number of unique operators
    pub fn unique_operator_count(&self) -> usize {
        self.operators.len()
    }

    /// Returns the number of unique operands
    pub fn unique_operand_count(&self) -> usize {
        self.operands.len()
    }

    /// Returns a reference to the operator frequency map
    pub fn operators_map(&self) -> &HashMap<&'a str, u64> {
        &self.operators
    }

    /// Returns a reference to the operand frequency map
    pub fn operands_map(&self) -> &HashMap<&'a str, u64> {
        &self.operands
    }
}

/// Common operators for various languages
pub const RUST_OPERATORS: &[&str] = &[
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=",
    "&&", "||", "!", "&", "|", "^", "<<", ">>",
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
    ".", "::", "->", "=>", "?", "as", "match", "if", "else", "for",
    "while", "loop", "return", "break", "continue", "fn", "let", "mut",
];

pub const TYPESCRIPT_OPERATORS: &[&str] = &[
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=",
    "&&", "||", "!", "&", "|", "^", "<<", ">>",
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
    ".", "?.", "=>", "?", ":", "typeof", "instanceof", "new",
    "if", "else", "for", "while", "do", "switch", "case", "return",
    "break", "continue", "function", "const", "let", "var",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halstead_default() {
        let stats = HalsteadStats::default();
        assert_eq!(stats.length(), 0.0);
        assert_eq!(stats.vocabulary(), 0.0);
    }

    #[test]
    fn test_halstead_calculations() {
        let stats = HalsteadStats::from_counts(5, 10, 3, 8);
        assert_eq!(stats.length(), 18.0);
        assert_eq!(stats.vocabulary(), 8.0);
        assert!(stats.volume() > 0.0);
        assert!(stats.difficulty() > 0.0);
    }

    #[test]
    fn test_halstead_collector() {
        let mut collector = HalsteadCollector::new();
        collector.add_operator("+");
        collector.add_operator("+");
        collector.add_operator("-");
        collector.add_operand("a");
        collector.add_operand("b");
        collector.add_operand("a");

        let stats = collector.finalize();
        assert_eq!(stats.u_operators(), 2.0);
        assert_eq!(stats.operators(), 3.0);
        assert_eq!(stats.u_operands(), 2.0);
        assert_eq!(stats.operands(), 3.0);
    }

    #[test]
    fn test_halstead_edge_cases() {
        let stats = HalsteadStats::default();
        assert_eq!(stats.volume(), 0.0);
        assert_eq!(stats.difficulty(), 0.0);
        assert_eq!(stats.level(), 0.0);
    }

    #[test]
    fn test_halstead_maps_basic() {
        let mut maps = HalsteadMaps::new();

        // Add operators (using kind IDs)
        *maps.operators.entry(1).or_insert(0) += 1;  // operator kind 1
        *maps.operators.entry(1).or_insert(0) += 1;  // operator kind 1 again
        *maps.operators.entry(2).or_insert(0) += 1;  // operator kind 2

        // Add operands (using byte slices)
        *maps.operands.entry(b"var_a").or_insert(0) += 1;
        *maps.operands.entry(b"var_b").or_insert(0) += 1;
        *maps.operands.entry(b"var_a").or_insert(0) += 1;

        let mut stats = HalsteadStats::default();
        maps.finalize(&mut stats);

        assert_eq!(stats.u_operators(), 2.0);  // 2 unique operators
        assert_eq!(stats.operators(), 3.0);     // 3 total operators
        assert_eq!(stats.u_operands(), 2.0);    // 2 unique operands
        assert_eq!(stats.operands(), 3.0);      // 3 total operands
    }

    #[test]
    fn test_halstead_maps_merge() {
        let mut maps1 = HalsteadMaps::new();
        *maps1.operators.entry(1).or_insert(0) += 2;
        *maps1.operands.entry(b"a").or_insert(0) += 1;

        let mut maps2 = HalsteadMaps::new();
        *maps2.operators.entry(1).or_insert(0) += 1;
        *maps2.operators.entry(2).or_insert(0) += 1;
        *maps2.operands.entry(b"a").or_insert(0) += 1;
        *maps2.operands.entry(b"b").or_insert(0) += 1;

        maps1.merge(&maps2);

        assert_eq!(maps1.operators.get(&1), Some(&3));  // 2 + 1
        assert_eq!(maps1.operators.get(&2), Some(&1));  // 0 + 1
        assert_eq!(maps1.operands.get(&b"a"[..]), Some(&2));  // 1 + 1
        assert_eq!(maps1.operands.get(&b"b"[..]), Some(&1));  // 0 + 1
    }

    #[test]
    fn test_halstead_maps_most_frequent_operators() {
        let mut maps = HalsteadMaps::new();
        *maps.operators.entry(1).or_insert(0) += 5;
        *maps.operators.entry(2).or_insert(0) += 3;
        *maps.operators.entry(3).or_insert(0) += 8;
        *maps.operators.entry(4).or_insert(0) += 1;

        let top = maps.most_frequent_operators(2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0], (3, 8));  // kind 3 with count 8
        assert_eq!(top[1], (1, 5));  // kind 1 with count 5
    }

    #[test]
    fn test_halstead_maps_most_frequent_operands() {
        let mut maps = HalsteadMaps::new();
        *maps.operands.entry(b"foo").or_insert(0) += 5;
        *maps.operands.entry(b"bar").or_insert(0) += 3;
        *maps.operands.entry(b"baz").or_insert(0) += 8;
        *maps.operands.entry(b"qux").or_insert(0) += 1;

        let top = maps.most_frequent_operands(2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0], (&b"baz"[..], 8));
        assert_eq!(top[1], (&b"foo"[..], 5));
    }

    #[test]
    fn test_halstead_maps_unique_counts() {
        let mut maps = HalsteadMaps::new();
        *maps.operators.entry(1).or_insert(0) += 5;
        *maps.operators.entry(2).or_insert(0) += 3;
        *maps.operators.entry(3).or_insert(0) += 8;

        *maps.operands.entry(b"a").or_insert(0) += 10;
        *maps.operands.entry(b"b").or_insert(0) += 5;

        assert_eq!(maps.unique_operator_count(), 3);
        assert_eq!(maps.unique_operand_count(), 2);
    }

    #[test]
    fn test_halstead_collector_most_frequent() {
        let mut collector = HalsteadCollector::new();

        collector.add_operator("+");
        collector.add_operator("+");
        collector.add_operator("+");
        collector.add_operator("-");
        collector.add_operator("*");
        collector.add_operator("*");

        collector.add_operand("x");
        collector.add_operand("x");
        collector.add_operand("x");
        collector.add_operand("x");
        collector.add_operand("y");
        collector.add_operand("y");
        collector.add_operand("z");

        let top_ops = collector.most_frequent_operators(2);
        assert_eq!(top_ops.len(), 2);
        assert_eq!(top_ops[0], ("+", 3));
        assert_eq!(top_ops[1], ("*", 2));

        let top_operands = collector.most_frequent_operands(2);
        assert_eq!(top_operands.len(), 2);
        assert_eq!(top_operands[0], ("x", 4));
        assert_eq!(top_operands[1], ("y", 2));
    }

    #[test]
    fn test_halstead_collector_frequency_maps() {
        let mut collector = HalsteadCollector::new();
        collector.add_operator("+");
        collector.add_operator("-");
        collector.add_operand("a");

        assert_eq!(collector.operators_map().len(), 2);
        assert_eq!(collector.operands_map().len(), 1);
        assert_eq!(collector.operators_map().get("+"), Some(&1));
        assert_eq!(collector.operands_map().get("a"), Some(&1));
    }

    #[test]
    fn test_halstead_division_by_zero_safety() {
        // Test with zero operands
        let stats = HalsteadStats::from_counts(5, 10, 0, 0);
        assert_eq!(stats.difficulty(), 0.0);

        // Test with zero operators and operands
        let stats = HalsteadStats::from_counts(0, 0, 0, 0);
        assert_eq!(stats.estimated_program_length(), 0.0);
        assert_eq!(stats.purity_ratio(), 0.0);
        assert_eq!(stats.volume(), 0.0);
        assert_eq!(stats.difficulty(), 0.0);
        assert_eq!(stats.level(), 0.0);
    }

    #[test]
    fn test_halstead_maps_empty() {
        let maps = HalsteadMaps::new();
        assert_eq!(maps.unique_operator_count(), 0);
        assert_eq!(maps.unique_operand_count(), 0);

        let top_ops = maps.most_frequent_operators(10);
        assert_eq!(top_ops.len(), 0);

        let top_operands = maps.most_frequent_operands(10);
        assert_eq!(top_operands.len(), 0);
    }
}
