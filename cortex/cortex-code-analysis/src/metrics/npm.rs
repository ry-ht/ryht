//! Number of Public Methods (NPM) Metric
//!
//! This metric counts the number of public methods in a class.

use serde::{Serialize, Deserialize};
use std::fmt;

/// NPM statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpmStats {
    npm: usize,
    npm_sum: usize,
    npm_min: usize,
    npm_max: usize,
    space_count: usize,
}

impl Default for NpmStats {
    fn default() -> Self {
        Self {
            npm: 0,
            npm_sum: 0,
            npm_min: usize::MAX,
            npm_max: 0,
            space_count: 1,
        }
    }
}

impl fmt::Display for NpmStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "npm: {}, average: {}, min: {}, max: {}",
            self.npm_sum(),
            self.npm_average(),
            self.npm_min(),
            self.npm_max()
        )
    }
}

impl NpmStats {
    /// Creates a new NpmStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the NPM value
    pub fn npm(&self) -> f64 {
        self.npm as f64
    }

    /// Returns the sum of NPM
    pub fn npm_sum(&self) -> f64 {
        self.npm_sum as f64
    }

    /// Returns the average NPM
    pub fn npm_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.npm_sum as f64 / self.space_count as f64
        }
    }

    /// Returns the minimum NPM
    pub fn npm_min(&self) -> f64 {
        if self.npm_min == usize::MAX {
            0.0
        } else {
            self.npm_min as f64
        }
    }

    /// Returns the maximum NPM
    pub fn npm_max(&self) -> f64 {
        self.npm_max as f64
    }

    /// Increments the public method count
    pub fn add_public_method(&mut self) {
        self.npm += 1;
    }

    /// Computes sum and min/max
    pub fn compute_minmax(&mut self) {
        self.npm_min = self.npm_min.min(self.npm);
        self.npm_max = self.npm_max.max(self.npm);
        self.npm_sum += self.npm;
    }

    /// Merges another NpmStats
    pub fn merge(&mut self, other: &NpmStats) {
        self.npm_min = self.npm_min.min(other.npm_min);
        self.npm_max = self.npm_max.max(other.npm_max);
        self.npm_sum += other.npm_sum;
        self.space_count += other.space_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_default() {
        let stats = NpmStats::default();
        assert_eq!(stats.npm(), 0.0);
    }

    #[test]
    fn test_npm_increment() {
        let mut stats = NpmStats::default();
        stats.add_public_method();
        stats.add_public_method();
        assert_eq!(stats.npm(), 2.0);
    }
}
