//! Exit Points (NExit) Metric
//!
//! This metric counts the number of possible exit points
//! from a function or method.

use serde::{Serialize, Deserialize};
use std::fmt;

/// Exit points statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExitStats {
    exit: usize,
    exit_sum: usize,
    total_space_functions: usize,
    exit_min: usize,
    exit_max: usize,
}

impl Default for ExitStats {
    fn default() -> Self {
        Self {
            exit: 0,
            exit_sum: 0,
            total_space_functions: 1,
            exit_min: usize::MAX,
            exit_max: 0,
        }
    }
}

impl fmt::Display for ExitStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.exit_sum(),
            self.exit_average(),
            self.exit_min(),
            self.exit_max()
        )
    }
}

impl ExitStats {
    /// Creates a new ExitStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the exit count
    pub fn exit(&self) -> f64 {
        self.exit as f64
    }

    /// Returns the sum of exits
    pub fn exit_sum(&self) -> f64 {
        self.exit_sum as f64
    }

    /// Returns the minimum exits
    pub fn exit_min(&self) -> f64 {
        if self.exit_min == usize::MAX {
            0.0
        } else {
            self.exit_min as f64
        }
    }

    /// Returns the maximum exits
    pub fn exit_max(&self) -> f64 {
        self.exit_max as f64
    }

    /// Returns the average exits
    pub fn exit_average(&self) -> f64 {
        if self.total_space_functions == 0 {
            0.0
        } else {
            self.exit_sum as f64 / self.total_space_functions as f64
        }
    }

    /// Increments the exit count
    pub fn increment(&mut self) {
        self.exit += 1;
    }

    /// Computes sum
    pub fn compute_sum(&mut self) {
        self.exit_sum += self.exit;
    }

    /// Computes min/max
    pub fn compute_minmax(&mut self) {
        self.exit_max = self.exit_max.max(self.exit);
        self.exit_min = self.exit_min.min(self.exit);
        self.compute_sum();
    }

    /// Merges another ExitStats
    pub fn merge(&mut self, other: &ExitStats) {
        self.exit_max = self.exit_max.max(other.exit_max);
        self.exit_min = self.exit_min.min(other.exit_min);
        self.exit_sum += other.exit_sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_default() {
        let stats = ExitStats::default();
        assert_eq!(stats.exit(), 0.0);
        assert_eq!(stats.exit_sum(), 0.0);
    }

    #[test]
    fn test_exit_increment() {
        let mut stats = ExitStats::default();
        stats.increment();
        stats.increment();
        assert_eq!(stats.exit(), 2.0);
    }
}
