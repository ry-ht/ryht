//! ABC Software Metric
//!
//! The ABC metric measures the size of source code by counting:
//! - Assignments (A)
//! - Branches (B)
//! - Conditions (C)
//!
//! The ABC score can be represented by its components or by the magnitude:
//! |<A,B,C>| = sqrt(A^2 + B^2 + C^2)

use serde::{Serialize, Deserialize};
use std::fmt;

/// ABC metric statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbcStats {
    assignments: f64,
    assignments_sum: f64,
    assignments_min: f64,
    assignments_max: f64,
    branches: f64,
    branches_sum: f64,
    branches_min: f64,
    branches_max: f64,
    conditions: f64,
    conditions_sum: f64,
    conditions_min: f64,
    conditions_max: f64,
    space_count: usize,
}

impl Default for AbcStats {
    fn default() -> Self {
        Self {
            assignments: 0.0,
            assignments_sum: 0.0,
            assignments_min: f64::MAX,
            assignments_max: 0.0,
            branches: 0.0,
            branches_sum: 0.0,
            branches_min: f64::MAX,
            branches_max: 0.0,
            conditions: 0.0,
            conditions_sum: 0.0,
            conditions_min: f64::MAX,
            conditions_max: 0.0,
            space_count: 1,
        }
    }
}

impl fmt::Display for AbcStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "assignments: {}, branches: {}, conditions: {}, magnitude: {}",
            self.assignments_sum(),
            self.branches_sum(),
            self.conditions_sum(),
            self.magnitude_sum()
        )
    }
}

impl AbcStats {
    /// Creates a new AbcStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the assignments value
    pub fn assignments(&self) -> f64 {
        self.assignments
    }

    /// Returns the sum of assignments
    pub fn assignments_sum(&self) -> f64 {
        self.assignments_sum
    }

    /// Returns the average assignments
    pub fn assignments_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.assignments_sum / self.space_count as f64
        }
    }

    /// Returns the minimum assignments
    pub fn assignments_min(&self) -> f64 {
        if self.assignments_min == f64::MAX {
            0.0
        } else {
            self.assignments_min
        }
    }

    /// Returns the maximum assignments
    pub fn assignments_max(&self) -> f64 {
        self.assignments_max
    }

    /// Returns the branches value
    pub fn branches(&self) -> f64 {
        self.branches
    }

    /// Returns the sum of branches
    pub fn branches_sum(&self) -> f64 {
        self.branches_sum
    }

    /// Returns the average branches
    pub fn branches_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.branches_sum / self.space_count as f64
        }
    }

    /// Returns the minimum branches
    pub fn branches_min(&self) -> f64 {
        if self.branches_min == f64::MAX {
            0.0
        } else {
            self.branches_min
        }
    }

    /// Returns the maximum branches
    pub fn branches_max(&self) -> f64 {
        self.branches_max
    }

    /// Returns the conditions value
    pub fn conditions(&self) -> f64 {
        self.conditions
    }

    /// Returns the sum of conditions
    pub fn conditions_sum(&self) -> f64 {
        self.conditions_sum
    }

    /// Returns the average conditions
    pub fn conditions_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.conditions_sum / self.space_count as f64
        }
    }

    /// Returns the minimum conditions
    pub fn conditions_min(&self) -> f64 {
        if self.conditions_min == f64::MAX {
            0.0
        } else {
            self.conditions_min
        }
    }

    /// Returns the maximum conditions
    pub fn conditions_max(&self) -> f64 {
        self.conditions_max
    }

    /// Returns the magnitude of the ABC vector
    pub fn magnitude(&self) -> f64 {
        (self.assignments.powi(2) + self.branches.powi(2) + self.conditions.powi(2)).sqrt()
    }

    /// Returns the magnitude sum
    pub fn magnitude_sum(&self) -> f64 {
        (self.assignments_sum.powi(2) + self.branches_sum.powi(2) + self.conditions_sum.powi(2)).sqrt()
    }

    /// Increments assignments count
    pub fn add_assignment(&mut self) {
        self.assignments += 1.0;
    }

    /// Increments branches count
    pub fn add_branch(&mut self) {
        self.branches += 1.0;
    }

    /// Increments conditions count
    pub fn add_condition(&mut self) {
        self.conditions += 1.0;
    }

    /// Computes sum and min/max
    pub fn compute_minmax(&mut self) {
        self.assignments_min = self.assignments_min.min(self.assignments);
        self.assignments_max = self.assignments_max.max(self.assignments);
        self.branches_min = self.branches_min.min(self.branches);
        self.branches_max = self.branches_max.max(self.branches);
        self.conditions_min = self.conditions_min.min(self.conditions);
        self.conditions_max = self.conditions_max.max(self.conditions);

        self.assignments_sum += self.assignments;
        self.branches_sum += self.branches;
        self.conditions_sum += self.conditions;
    }

    /// Merges another AbcStats
    pub fn merge(&mut self, other: &AbcStats) {
        self.assignments_min = self.assignments_min.min(other.assignments_min);
        self.assignments_max = self.assignments_max.max(other.assignments_max);
        self.branches_min = self.branches_min.min(other.branches_min);
        self.branches_max = self.branches_max.max(other.branches_max);
        self.conditions_min = self.conditions_min.min(other.conditions_min);
        self.conditions_max = self.conditions_max.max(other.conditions_max);

        self.assignments_sum += other.assignments_sum;
        self.branches_sum += other.branches_sum;
        self.conditions_sum += other.conditions_sum;

        self.space_count += other.space_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abc_default() {
        let stats = AbcStats::default();
        assert_eq!(stats.assignments(), 0.0);
        assert_eq!(stats.branches(), 0.0);
        assert_eq!(stats.conditions(), 0.0);
    }

    #[test]
    fn test_abc_increment() {
        let mut stats = AbcStats::default();
        stats.add_assignment();
        stats.add_branch();
        stats.add_condition();
        assert_eq!(stats.assignments(), 1.0);
        assert_eq!(stats.branches(), 1.0);
        assert_eq!(stats.conditions(), 1.0);
    }

    #[test]
    fn test_abc_magnitude() {
        let mut stats = AbcStats::default();
        stats.assignments = 3.0;
        stats.branches = 4.0;
        stats.conditions = 0.0;
        assert_eq!(stats.magnitude(), 5.0); // 3-4-5 triangle
    }
}
