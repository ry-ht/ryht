//! Cognitive Complexity Metric
//!
//! This metric measures the readability and understandability of code
//! by counting structural elements that increase cognitive load.
//!
//! The implementation uses a HashMap-based nesting tracking system to properly
//! account for nested constructs, function depth, and lambda/closure nesting.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

/// Boolean sequence tracker for cognitive complexity
///
/// Tracks sequential boolean operators to properly calculate cognitive complexity.
/// According to cognitive complexity rules, consecutive identical boolean operators
/// (e.g., `a && b && c`) only count once, but changing operators (e.g., `a && b || c`)
/// count separately.
#[derive(Debug, Default, Clone, PartialEq)]
struct BoolSequence {
    boolean_op: Option<u16>,
}

impl BoolSequence {
    /// Resets the boolean sequence tracker
    fn reset(&mut self) {
        self.boolean_op = None;
    }

    /// Records a NOT operator in the sequence
    fn not_operator(&mut self, not_id: u16) {
        self.boolean_op = Some(not_id);
    }

    /// Evaluates the current boolean operator based on the previous one
    ///
    /// Returns the updated structural complexity value:
    /// - If this is the first boolean operator in a sequence, increment by 1
    /// - If the operator is different from the previous one, increment by 1
    /// - If the operator is the same as the previous one, don't increment
    fn eval_based_on_prev(&mut self, bool_id: u16, structural: usize) -> usize {
        if let Some(prev) = self.boolean_op {
            if prev != bool_id {
                // The boolean operator is different from the previous one, so
                // the counter is incremented.
                self.boolean_op = Some(bool_id);
                structural + 1
            } else {
                // The boolean operator is equal to the previous one, so
                // the counter is not incremented.
                structural
            }
        } else {
            // Save the first boolean operator in a sequence of
            // logical operators and increment the counter.
            self.boolean_op = Some(bool_id);
            structural + 1
        }
    }
}

/// Cognitive complexity statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStats {
    structural: usize,
    structural_sum: usize,
    structural_min: usize,
    structural_max: usize,
    nesting: usize,
    total_space_functions: usize,
    #[serde(skip)]
    boolean_seq: BoolSequence,
}

impl Default for CognitiveStats {
    fn default() -> Self {
        Self {
            structural: 0,
            structural_sum: 0,
            structural_min: usize::MAX,
            structural_max: 0,
            nesting: 0,
            total_space_functions: 1,
            boolean_seq: BoolSequence::default(),
        }
    }
}

impl fmt::Display for CognitiveStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.cognitive_sum(),
            self.cognitive_average(),
            self.cognitive_min(),
            self.cognitive_max()
        )
    }
}

impl CognitiveStats {
    /// Creates a new CognitiveStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cognitive complexity value
    pub fn cognitive(&self) -> f64 {
        self.structural as f64
    }

    /// Returns the sum of cognitive complexity
    pub fn cognitive_sum(&self) -> f64 {
        self.structural_sum as f64
    }

    /// Returns the minimum cognitive complexity
    pub fn cognitive_min(&self) -> f64 {
        if self.structural_min == usize::MAX {
            0.0
        } else {
            self.structural_min as f64
        }
    }

    /// Returns the maximum cognitive complexity
    pub fn cognitive_max(&self) -> f64 {
        self.structural_max as f64
    }

    /// Returns the average cognitive complexity
    pub fn cognitive_average(&self) -> f64 {
        if self.total_space_functions == 0 {
            0.0
        } else {
            self.structural_sum as f64 / self.total_space_functions as f64
        }
    }

    /// Increments structural complexity
    pub fn increment(&mut self) {
        self.structural += 1;
    }

    /// Increments with nesting penalty
    pub fn increment_with_nesting(&mut self, nesting_level: usize) {
        self.structural += 1 + nesting_level;
    }

    /// Sets the nesting level
    pub fn set_nesting(&mut self, level: usize) {
        self.nesting = level;
    }

    /// Returns current nesting level
    pub fn nesting(&self) -> usize {
        self.nesting
    }

    /// Computes sum
    pub fn compute_sum(&mut self) {
        self.structural_sum += self.structural;
    }

    /// Computes min/max
    pub fn compute_minmax(&mut self) {
        self.structural_min = self.structural_min.min(self.structural);
        self.structural_max = self.structural_max.max(self.structural);
        self.compute_sum();
    }

    /// Finalizes with total function count
    pub fn finalize(&mut self, total_space_functions: usize) {
        self.total_space_functions = total_space_functions;
    }

    /// Merges another CognitiveStats
    pub fn merge(&mut self, other: &CognitiveStats) {
        self.structural_min = self.structural_min.min(other.structural_min);
        self.structural_max = self.structural_max.max(other.structural_max);
        self.structural_sum += other.structural_sum;
    }

    /// Resets the boolean sequence tracker
    pub fn reset_boolean_seq(&mut self) {
        self.boolean_seq.reset();
    }

    /// Records a NOT operator in the boolean sequence
    pub fn boolean_seq_not_operator(&mut self, not_id: u16) {
        self.boolean_seq.not_operator(not_id);
    }

    /// Evaluates a boolean operator and updates structural complexity
    ///
    /// This method uses the BoolSequence tracker to properly handle
    /// sequential boolean operators according to cognitive complexity rules.
    pub fn eval_boolean_sequence(&mut self, bool_id: u16) {
        self.structural = self.boolean_seq.eval_based_on_prev(bool_id, self.structural);
    }
}

/// Type alias for the nesting map used in cognitive complexity calculation.
/// Maps node ID to (conditional_nesting, function_depth, lambda_nesting).
pub type NestingMap = HashMap<usize, (usize, usize, usize)>;

/// Retrieves nesting information from the nesting map for a given node.
///
/// Returns the nesting tuple (conditional_nesting, function_depth, lambda_nesting)
/// from the parent node, or (0, 0, 0) if no parent or no entry exists.
#[inline(always)]
pub fn get_nesting_from_map(node: &crate::node::Node, nesting_map: &NestingMap) -> (usize, usize, usize) {
    if let Some(parent) = node.parent() {
        nesting_map.get(&parent.id()).copied().unwrap_or((0, 0, 0))
    } else {
        (0, 0, 0)
    }
}

/// Increments the structural complexity with nesting penalty.
///
/// The increment is: structural += nesting + 1
#[inline(always)]
pub fn increment_with_nesting(stats: &mut CognitiveStats, nesting: usize) {
    stats.structural += nesting + 1;
}

/// Increments the structural complexity by one.
#[inline(always)]
pub fn increment_by_one(stats: &mut CognitiveStats) {
    stats.structural += 1;
}

/// Increases nesting level and increments structural complexity with full nesting context.
///
/// This is the core function for handling nesting constructs. It:
/// 1. Sets the current nesting including function depth and lambda nesting
/// 2. Increments structural complexity with the full nesting penalty
/// 3. Increments the conditional nesting level
/// 4. Resets the boolean sequence tracker
#[inline(always)]
pub fn increase_nesting(
    stats: &mut CognitiveStats,
    nesting: &mut usize,
    depth: usize,
    lambda: usize,
) {
    stats.nesting = *nesting + depth + lambda;
    stats.structural += stats.nesting + 1;
    *nesting += 1;
    stats.boolean_seq.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_default() {
        let stats = CognitiveStats::default();
        assert_eq!(stats.cognitive(), 0.0);
    }

    #[test]
    fn test_cognitive_increment() {
        let mut stats = CognitiveStats::default();
        stats.increment();
        assert_eq!(stats.cognitive(), 1.0);
    }

    #[test]
    fn test_cognitive_nesting() {
        let mut stats = CognitiveStats::default();
        stats.increment_with_nesting(2);
        assert_eq!(stats.cognitive(), 3.0); // 1 + 2 nesting levels
    }
}
