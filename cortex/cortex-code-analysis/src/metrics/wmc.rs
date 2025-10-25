//! Weighted Methods per Class (WMC) Metric
//!
//! This metric measures the complexity of a class by summing
//! the cyclomatic complexity of all its methods.

use serde::{Serialize, Deserialize};
use std::fmt;

use super::cyclomatic::CyclomaticStats;

/// WMC statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WmcStats {
    wmc: f64,
    wmc_sum: f64,
    wmc_min: f64,
    wmc_max: f64,
    space_count: usize,
}

impl Default for WmcStats {
    fn default() -> Self {
        Self {
            wmc: 0.0,
            wmc_sum: 0.0,
            wmc_min: f64::MAX,
            wmc_max: 0.0,
            space_count: 1,
        }
    }
}

impl fmt::Display for WmcStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "wmc: {}, average: {}, min: {}, max: {}",
            self.wmc_sum(),
            self.wmc_average(),
            self.wmc_min(),
            self.wmc_max()
        )
    }
}

impl WmcStats {
    /// Creates a new WmcStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates WmcStats from cyclomatic complexity
    pub fn from_cyclomatic(cyclomatic: &CyclomaticStats) -> Self {
        let mut stats = Self::default();
        stats.wmc = cyclomatic.cyclomatic_sum();
        stats
    }

    /// Returns the WMC value
    pub fn wmc(&self) -> f64 {
        self.wmc
    }

    /// Returns the sum of WMC
    pub fn wmc_sum(&self) -> f64 {
        self.wmc_sum
    }

    /// Returns the average WMC
    pub fn wmc_average(&self) -> f64 {
        if self.space_count == 0 {
            0.0
        } else {
            self.wmc_sum / self.space_count as f64
        }
    }

    /// Returns the minimum WMC
    pub fn wmc_min(&self) -> f64 {
        if self.wmc_min == f64::MAX {
            0.0
        } else {
            self.wmc_min
        }
    }

    /// Returns the maximum WMC
    pub fn wmc_max(&self) -> f64 {
        self.wmc_max
    }

    /// Computes sum and min/max
    pub fn compute_minmax(&mut self) {
        self.wmc_min = self.wmc_min.min(self.wmc);
        self.wmc_max = self.wmc_max.max(self.wmc);
        self.wmc_sum += self.wmc;
    }

    /// Merges another WmcStats
    pub fn merge(&mut self, other: &WmcStats) {
        self.wmc_min = self.wmc_min.min(other.wmc_min);
        self.wmc_max = self.wmc_max.max(other.wmc_max);
        self.wmc_sum += other.wmc_sum;
        self.space_count += other.space_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wmc_default() {
        let stats = WmcStats::default();
        assert_eq!(stats.wmc(), 0.0);
    }

    #[test]
    fn test_wmc_from_cyclomatic() {
        let mut cyc = CyclomaticStats::default();
        cyc.increment();
        cyc.compute_sum();

        let wmc = WmcStats::from_cyclomatic(&cyc);
        assert!(wmc.wmc() > 0.0);
    }
}
