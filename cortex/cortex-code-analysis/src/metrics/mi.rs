//! Maintainability Index (MI) Metric
//!
//! The Maintainability Index is a composite metric that combines
//! Halstead Volume, Cyclomatic Complexity, and Lines of Code to
//! assess code maintainability.

use serde::{Serialize, Deserialize};
use std::fmt;

use super::cyclomatic::CyclomaticStats;
use super::halstead::HalsteadStats;
use super::loc::LocStats;

/// Maintainability Index statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaintainabilityIndexStats {
    halstead_length: f64,
    halstead_vocabulary: f64,
    halstead_volume: f64,
    cyclomatic: f64,
    sloc: f64,
    comments_percentage: f64,
}

impl Default for MaintainabilityIndexStats {
    fn default() -> Self {
        Self {
            halstead_length: 0.0,
            halstead_vocabulary: 0.0,
            halstead_volume: 0.0,
            cyclomatic: 0.0,
            sloc: 0.0,
            comments_percentage: 0.0,
        }
    }
}

impl fmt::Display for MaintainabilityIndexStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "mi_original: {}, mi_sei: {}, mi_visual_studio: {}",
            self.mi_original(),
            self.mi_sei(),
            self.mi_visual_studio()
        )
    }
}

impl MaintainabilityIndexStats {
    /// Creates a new MaintainabilityIndexStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes MI from component metrics
    pub fn from_metrics(
        loc: &LocStats,
        cyclomatic: &CyclomaticStats,
        halstead: &HalsteadStats,
    ) -> Self {
        let sloc = loc.sloc();
        let cloc = loc.cloc();
        let comments_percentage = if sloc > 0.0 { cloc / sloc } else { 0.0 };

        Self {
            halstead_length: halstead.length(),
            halstead_vocabulary: halstead.vocabulary(),
            halstead_volume: halstead.volume(),
            cyclomatic: cyclomatic.cyclomatic_sum(),
            sloc,
            comments_percentage,
        }
    }

    /// Returns the MI calculated using the original formula
    ///
    /// Formula: 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(SLOC)
    /// where V = Halstead Volume, G = Cyclomatic Complexity
    ///
    /// Note: This value can be negative
    pub fn mi_original(&self) -> f64 {
        if self.halstead_volume == 0.0 || self.sloc == 0.0 {
            return 0.0;
        }
        171.0 - 5.2 * self.halstead_volume.ln()
            - 0.23 * self.cyclomatic
            - 16.2 * self.sloc.ln()
    }

    /// Returns the MI calculated using the SEI formula
    ///
    /// Formula: 171 - 5.2 * log2(V) - 0.23 * G - 16.2 * log2(SLOC) + 50 * sin(sqrt(2.4 * perCM))
    /// where perCM = percentage of comment lines
    ///
    /// Note: This value can be negative
    pub fn mi_sei(&self) -> f64 {
        if self.halstead_volume == 0.0 || self.sloc == 0.0 {
            return 0.0;
        }
        171.0 - 5.2 * self.halstead_volume.log2()
            - 0.23 * self.cyclomatic
            - 16.2 * self.sloc.log2()
            + 50.0 * (self.comments_percentage * 2.4).sqrt().sin()
    }

    /// Returns the MI calculated using the Microsoft Visual Studio formula
    ///
    /// Formula: max(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(SLOC)) * 100 / 171)
    ///
    /// This formula normalizes the result to 0-100 range
    pub fn mi_visual_studio(&self) -> f64 {
        if self.halstead_volume == 0.0 || self.sloc == 0.0 {
            return 0.0;
        }
        let formula = 171.0 - 5.2 * self.halstead_volume.ln()
            - 0.23 * self.cyclomatic
            - 16.2 * self.sloc.ln();
        (formula * 100.0 / 171.0).max(0.0)
    }

    /// Merges another MaintainabilityIndexStats (no-op for MI)
    pub fn merge(&mut self, _other: &MaintainabilityIndexStats) {
        // MI is typically computed per-function
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mi_default() {
        let stats = MaintainabilityIndexStats::default();
        assert_eq!(stats.mi_original(), 0.0);
        assert_eq!(stats.mi_sei(), 0.0);
        assert_eq!(stats.mi_visual_studio(), 0.0);
    }

    #[test]
    fn test_mi_calculation() {
        let loc = LocStats::default();
        let cyclomatic = CyclomaticStats::default();
        let halstead = HalsteadStats::default();

        let mi = MaintainabilityIndexStats::from_metrics(&loc, &cyclomatic, &halstead);
        // With default values, MI should be 0
        assert_eq!(mi.mi_original(), 0.0);
    }

    #[test]
    fn test_mi_visual_studio_non_negative() {
        let mut stats = MaintainabilityIndexStats::default();
        stats.halstead_volume = 100.0;
        stats.sloc = 50.0;
        stats.cyclomatic = 5.0;

        // Visual Studio MI should never be negative
        assert!(stats.mi_visual_studio() >= 0.0);
    }
}
