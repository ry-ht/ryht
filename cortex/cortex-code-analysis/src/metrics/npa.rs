//! Number of Public Attributes (NPA) Metric
//!
//! This metric counts the number of public attributes/fields in a class.

use serde::{Serialize, Deserialize};
use std::fmt;

/// NPA statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpaStats {
    npa: usize,
    npa_sum: usize,
    npa_min: usize,
    npa_max: usize,
    space_count: usize,
}

impl Default for NpaStats {
    fn default() -> Self {
        Self {
            npa: 0,
            npa_sum: 0,
            npa_min: usize::MAX,
            npa_max: 0,
            space_count: 1,
        }
    }
}

impl fmt::Display for NpaStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "npa: {}, average: {}, min: {}, max: {}",
            self.npa_sum(),
            self.npa_average(),
            self.npa_min(),
            self.npa_max()
        )
    }
}

impl NpaStats {
    /// Creates a new NpaStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the NPA value
    pub fn npa(&self) -> f64 {
        self.npa as f64
    }

    /// Returns the sum of NPA
    pub fn npa_sum(&self) -> f64 {
        self.npa_sum as f64
    }

    /// Returns the average NPA
    pub fn npa_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.npa_sum as f64 / self.space_count as f64
        }
    }

    /// Returns the minimum NPA
    pub fn npa_min(&self) -> f64 {
        if self.npa_min == usize::MAX {
            0.0
        } else {
            self.npa_min as f64
        }
    }

    /// Returns the maximum NPA
    pub fn npa_max(&self) -> f64 {
        self.npa_max as f64
    }

    /// Increments the public attribute count
    pub fn add_public_attribute(&mut self) {
        self.npa += 1;
    }

    /// Computes sum and min/max
    pub fn compute_minmax(&mut self) {
        self.npa_min = self.npa_min.min(self.npa);
        self.npa_max = self.npa_max.max(self.npa);
        self.npa_sum += self.npa;
    }

    /// Merges another NpaStats
    pub fn merge(&mut self, other: &NpaStats) {
        self.npa_min = self.npa_min.min(other.npa_min);
        self.npa_max = self.npa_max.max(other.npa_max);
        self.npa_sum += other.npa_sum;
        self.space_count += other.space_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npa_default() {
        let stats = NpaStats::default();
        assert_eq!(stats.npa(), 0.0);
    }

    #[test]
    fn test_npa_increment() {
        let mut stats = NpaStats::default();
        stats.add_public_attribute();
        stats.add_public_attribute();
        assert_eq!(stats.npa(), 2.0);
    }
}
