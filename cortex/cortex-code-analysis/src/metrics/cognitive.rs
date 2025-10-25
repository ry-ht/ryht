//! Cognitive Complexity Metric
//!
//! This metric measures the readability and understandability of code
//! by counting structural elements that increase cognitive load.

use serde::{Serialize, Deserialize};
use std::fmt;

/// Cognitive complexity statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CognitiveStats {
    structural: usize,
    structural_sum: usize,
    structural_min: usize,
    structural_max: usize,
    nesting: usize,
    total_space_functions: usize,
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
