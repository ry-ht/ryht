//! Halstead Complexity Metrics
//!
//! This module implements Maurice Halstead's software metrics suite,
//! which provide measurements about program complexity based on
//! operators and operands.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

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

/// Helper structure for collecting operators and operands
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
}
