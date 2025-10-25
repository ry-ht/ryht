//! Number of Arguments (NArgs) Metric
//!
//! This metric counts the number of arguments in functions and closures.

use serde::{Serialize, Deserialize};
use std::fmt;

/// Number of arguments statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NargsStats {
    fn_nargs: usize,
    closure_nargs: usize,
    fn_nargs_sum: usize,
    closure_nargs_sum: usize,
    fn_nargs_min: usize,
    closure_nargs_min: usize,
    fn_nargs_max: usize,
    closure_nargs_max: usize,
    total_functions: usize,
    total_closures: usize,
}

impl Default for NargsStats {
    fn default() -> Self {
        Self {
            fn_nargs: 0,
            closure_nargs: 0,
            fn_nargs_sum: 0,
            closure_nargs_sum: 0,
            fn_nargs_min: usize::MAX,
            closure_nargs_min: usize::MAX,
            fn_nargs_max: 0,
            closure_nargs_max: 0,
            total_functions: 0,
            total_closures: 0,
        }
    }
}

impl fmt::Display for NargsStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "fn_args: {}, closure_args: {}, total: {}, average: {}",
            self.fn_args_sum(),
            self.closure_args_sum(),
            self.nargs_total(),
            self.nargs_average()
        )
    }
}

impl NargsStats {
    /// Creates a new NargsStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns function argument count
    pub fn fn_args(&self) -> f64 {
        self.fn_nargs as f64
    }

    /// Returns closure argument count
    pub fn closure_args(&self) -> f64 {
        self.closure_nargs as f64
    }

    /// Returns sum of function arguments
    pub fn fn_args_sum(&self) -> f64 {
        self.fn_nargs_sum as f64
    }

    /// Returns sum of closure arguments
    pub fn closure_args_sum(&self) -> f64 {
        self.closure_nargs_sum as f64
    }

    /// Returns average function arguments
    pub fn fn_args_average(&self) -> f64 {
        if self.total_functions == 0 {
            0.0
        } else {
            self.fn_nargs_sum as f64 / self.total_functions as f64
        }
    }

    /// Returns average closure arguments
    pub fn closure_args_average(&self) -> f64 {
        if self.total_closures == 0 {
            0.0
        } else {
            self.closure_nargs_sum as f64 / self.total_closures as f64
        }
    }

    /// Returns minimum function arguments
    pub fn fn_args_min(&self) -> f64 {
        if self.fn_nargs_min == usize::MAX {
            0.0
        } else {
            self.fn_nargs_min as f64
        }
    }

    /// Returns maximum function arguments
    pub fn fn_args_max(&self) -> f64 {
        self.fn_nargs_max as f64
    }

    /// Returns minimum closure arguments
    pub fn closure_args_min(&self) -> f64 {
        if self.closure_nargs_min == usize::MAX {
            0.0
        } else {
            self.closure_nargs_min as f64
        }
    }

    /// Returns maximum closure arguments
    pub fn closure_args_max(&self) -> f64 {
        self.closure_nargs_max as f64
    }

    /// Returns total argument count
    pub fn nargs_total(&self) -> f64 {
        (self.fn_nargs_sum + self.closure_nargs_sum) as f64
    }

    /// Returns average argument count
    pub fn nargs_average(&self) -> f64 {
        let total = self.total_functions + self.total_closures;
        if total == 0 {
            0.0
        } else {
            self.nargs_total() / total as f64
        }
    }

    /// Sets function argument count
    pub fn set_fn_args(&mut self, count: usize) {
        self.fn_nargs = count;
        self.total_functions += 1;
    }

    /// Sets closure argument count
    pub fn set_closure_args(&mut self, count: usize) {
        self.closure_nargs = count;
        self.total_closures += 1;
    }

    /// Computes sum
    pub fn compute_sum(&mut self) {
        self.fn_nargs_sum += self.fn_nargs;
        self.closure_nargs_sum += self.closure_nargs;
    }

    /// Computes min/max
    pub fn compute_minmax(&mut self) {
        self.fn_nargs_min = self.fn_nargs_min.min(self.fn_nargs);
        self.fn_nargs_max = self.fn_nargs_max.max(self.fn_nargs);
        self.closure_nargs_min = self.closure_nargs_min.min(self.closure_nargs);
        self.closure_nargs_max = self.closure_nargs_max.max(self.closure_nargs);
        self.compute_sum();
    }

    /// Merges another NargsStats
    pub fn merge(&mut self, other: &NargsStats) {
        self.closure_nargs_min = self.closure_nargs_min.min(other.closure_nargs_min);
        self.closure_nargs_max = self.closure_nargs_max.max(other.closure_nargs_max);
        self.fn_nargs_min = self.fn_nargs_min.min(other.fn_nargs_min);
        self.fn_nargs_max = self.fn_nargs_max.max(other.fn_nargs_max);
        self.fn_nargs_sum += other.fn_nargs_sum;
        self.closure_nargs_sum += other.closure_nargs_sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nargs_default() {
        let stats = NargsStats::default();
        assert_eq!(stats.fn_args(), 0.0);
        assert_eq!(stats.closure_args(), 0.0);
    }

    #[test]
    fn test_nargs_set() {
        let mut stats = NargsStats::default();
        stats.set_fn_args(3);
        assert_eq!(stats.fn_args(), 3.0);
    }
}
