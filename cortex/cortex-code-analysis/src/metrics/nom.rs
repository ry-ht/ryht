//! Number of Methods (NOM) Metric
//!
//! This metric counts the number of functions and closures in a scope.

use serde::{Serialize, Deserialize};
use std::fmt;

/// Number of methods statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NomStats {
    functions: usize,
    closures: usize,
    functions_sum: usize,
    closures_sum: usize,
    functions_min: usize,
    functions_max: usize,
    closures_min: usize,
    closures_max: usize,
    space_count: usize,
}

impl Default for NomStats {
    fn default() -> Self {
        Self {
            functions: 0,
            closures: 0,
            functions_sum: 0,
            closures_sum: 0,
            functions_min: usize::MAX,
            functions_max: 0,
            closures_min: usize::MAX,
            closures_max: 0,
            space_count: 1,
        }
    }
}

impl fmt::Display for NomStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "functions: {}, closures: {}, total: {}, average: {}",
            self.functions_sum(),
            self.closures_sum(),
            self.total(),
            self.average()
        )
    }
}

impl NomStats {
    /// Creates a new NomStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of functions
    pub fn functions(&self) -> f64 {
        self.functions as f64
    }

    /// Returns the number of closures
    pub fn closures(&self) -> f64 {
        self.closures as f64
    }

    /// Returns the sum of functions
    pub fn functions_sum(&self) -> f64 {
        self.functions_sum as f64
    }

    /// Returns the sum of closures
    pub fn closures_sum(&self) -> f64 {
        self.closures_sum as f64
    }

    /// Returns the average number of functions
    pub fn functions_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.functions_sum as f64 / self.space_count as f64
        }
    }

    /// Returns the average number of closures
    pub fn closures_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.closures_sum as f64 / self.space_count as f64
        }
    }

    /// Returns the minimum functions
    pub fn functions_min(&self) -> f64 {
        if self.functions_min == usize::MAX {
            0.0
        } else {
            self.functions_min as f64
        }
    }

    /// Returns the maximum functions
    pub fn functions_max(&self) -> f64 {
        self.functions_max as f64
    }

    /// Returns the minimum closures
    pub fn closures_min(&self) -> f64 {
        if self.closures_min == usize::MAX {
            0.0
        } else {
            self.closures_min as f64
        }
    }

    /// Returns the maximum closures
    pub fn closures_max(&self) -> f64 {
        self.closures_max as f64
    }

    /// Returns the total count
    pub fn total(&self) -> f64 {
        (self.functions_sum + self.closures_sum) as f64
    }

    /// Returns the average
    pub fn average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.total() / self.space_count as f64
        }
    }

    /// Increments function count
    pub fn add_function(&mut self) {
        self.functions += 1;
    }

    /// Increments closure count
    pub fn add_closure(&mut self) {
        self.closures += 1;
    }

    /// Computes sum
    pub fn compute_sum(&mut self) {
        self.functions_sum += self.functions;
        self.closures_sum += self.closures;
    }

    /// Computes min/max
    pub fn compute_minmax(&mut self) {
        self.functions_min = self.functions_min.min(self.functions);
        self.functions_max = self.functions_max.max(self.functions);
        self.closures_min = self.closures_min.min(self.closures);
        self.closures_max = self.closures_max.max(self.closures);
        self.compute_sum();
    }

    /// Merges another NomStats
    pub fn merge(&mut self, other: &NomStats) {
        self.functions_min = self.functions_min.min(other.functions_min);
        self.functions_max = self.functions_max.max(other.functions_max);
        self.closures_min = self.closures_min.min(other.closures_min);
        self.closures_max = self.closures_max.max(other.closures_max);
        self.functions_sum += other.functions_sum;
        self.closures_sum += other.closures_sum;
        self.space_count += other.space_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nom_default() {
        let stats = NomStats::default();
        assert_eq!(stats.functions(), 0.0);
        assert_eq!(stats.closures(), 0.0);
    }

    #[test]
    fn test_nom_increment() {
        let mut stats = NomStats::default();
        stats.add_function();
        stats.add_function();
        stats.add_closure();
        assert_eq!(stats.functions(), 2.0);
        assert_eq!(stats.closures(), 1.0);
    }
}
