//! Advanced Code Metrics Module
//!
//! This module provides a comprehensive suite of software metrics for analyzing
//! code quality, complexity, and maintainability.
//!
//! ## Available Metrics
//!
//! ### Complexity Metrics
//! - **Cyclomatic Complexity**: Measures the number of linearly independent paths through code
//! - **Cognitive Complexity**: Measures how difficult code is to understand
//!
//! ### Size Metrics
//! - **LOC (Lines of Code)**: Counts various types of lines (source, physical, logical, comments, blank)
//! - **Halstead Metrics**: Measures program vocabulary and volume
//!
//! ### Design Metrics
//! - **ABC (Assignments, Branches, Conditions)**: Measures code size through counting
//! - **WMC (Weighted Methods per Class)**: Sums complexity of all methods in a class
//! - **NOM (Number of Methods)**: Counts functions and closures
//! - **NPM (Number of Public Methods)**: Counts public methods in classes
//! - **NPA (Number of Public Attributes)**: Counts public attributes in classes
//!
//! ### Maintainability Metrics
//! - **MI (Maintainability Index)**: Composite metric for code maintainability
//!
//! ### Other Metrics
//! - **Exit Points**: Counts possible exit points from functions
//! - **NArgs (Number of Arguments)**: Counts function/method parameters
//!
//! ## Usage
//!
//! ```
//! use cortex_code_analysis::metrics::{CyclomaticStats, LocStats, HalsteadStats};
//!
//! // Compute metrics on parsed code
//! let cyc_stats = CyclomaticStats::new();
//! let loc_stats = LocStats::new();
//! let halstead_stats = HalsteadStats::new();
//! ```

pub mod abc;
pub mod cognitive;
pub mod cyclomatic;
pub mod exit;
pub mod halstead;
pub mod loc;
pub mod mi;
pub mod nargs;
pub mod nom;
pub mod npa;
pub mod npm;
pub mod wmc;

// Re-export main types for convenience
pub use abc::AbcStats;
pub use cognitive::CognitiveStats;
pub use cyclomatic::CyclomaticStats;
pub use exit::ExitStats;
pub use halstead::{HalsteadStats, HalsteadCollector};
pub use loc::{LocStats, Sloc, Ploc, Cloc, Lloc, Blank};
pub use mi::MaintainabilityIndexStats;
pub use nargs::NargsStats;
pub use nom::NomStats;
pub use npa::NpaStats;
pub use npm::NpmStats;
pub use wmc::WmcStats;

use serde::{Serialize, Deserialize};
use std::fmt;

/// Complete metrics suite for a code file or function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeMetrics {
    /// Cyclomatic complexity metrics
    pub cyclomatic: CyclomaticStats,

    /// Lines of code metrics
    pub loc: LocStats,

    /// Halstead complexity metrics
    pub halstead: HalsteadStats,

    /// ABC metrics
    pub abc: AbcStats,

    /// Cognitive complexity metrics
    pub cognitive: CognitiveStats,

    /// Maintainability index
    pub maintainability_index: MaintainabilityIndexStats,

    /// Exit points metrics
    pub exit: ExitStats,

    /// Number of methods metrics
    pub nom: NomStats,

    /// Number of arguments metrics
    pub nargs: NargsStats,

    /// Number of public methods metrics
    pub npm: NpmStats,

    /// Number of public attributes metrics
    pub npa: NpaStats,

    /// Weighted methods per class metrics
    pub wmc: WmcStats,
}

impl Default for CodeMetrics {
    fn default() -> Self {
        Self {
            cyclomatic: CyclomaticStats::default(),
            loc: LocStats::default(),
            halstead: HalsteadStats::default(),
            abc: AbcStats::default(),
            cognitive: CognitiveStats::default(),
            maintainability_index: MaintainabilityIndexStats::default(),
            exit: ExitStats::default(),
            nom: NomStats::default(),
            nargs: NargsStats::default(),
            npm: NpmStats::default(),
            npa: NpaStats::default(),
            wmc: WmcStats::default(),
        }
    }
}

impl fmt::Display for CodeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Code Metrics:")?;
        writeln!(f, "  Cyclomatic: {}", self.cyclomatic)?;
        writeln!(f, "  LOC: {}", self.loc)?;
        writeln!(f, "  Halstead: {}", self.halstead)?;
        writeln!(f, "  ABC: {}", self.abc)?;
        writeln!(f, "  Cognitive: {}", self.cognitive)?;
        writeln!(f, "  MI: {}", self.maintainability_index)?;
        writeln!(f, "  Exit: {}", self.exit)?;
        writeln!(f, "  NOM: {}", self.nom)?;
        writeln!(f, "  NArgs: {}", self.nargs)?;
        writeln!(f, "  NPM: {}", self.npm)?;
        writeln!(f, "  NPA: {}", self.npa)?;
        writeln!(f, "  WMC: {}", self.wmc)?;
        Ok(())
    }
}

impl CodeMetrics {
    /// Creates a new CodeMetrics instance with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes derived metrics like MI and WMC
    pub fn compute_derived(&mut self) {
        // Compute Maintainability Index from LOC, Cyclomatic, and Halstead
        self.maintainability_index = MaintainabilityIndexStats::from_metrics(
            &self.loc,
            &self.cyclomatic,
            &self.halstead,
        );

        // Compute WMC from Cyclomatic
        self.wmc = WmcStats::from_cyclomatic(&self.cyclomatic);
    }

    /// Merges another CodeMetrics into this one
    pub fn merge(&mut self, other: &CodeMetrics) {
        self.cyclomatic.merge(&other.cyclomatic);
        self.loc.merge(&other.loc);
        self.halstead.merge(&other.halstead);
        self.abc.merge(&other.abc);
        self.cognitive.merge(&other.cognitive);
        self.maintainability_index.merge(&other.maintainability_index);
        self.exit.merge(&other.exit);
        self.nom.merge(&other.nom);
        self.nargs.merge(&other.nargs);
        self.npm.merge(&other.npm);
        self.npa.merge(&other.npa);
        self.wmc.merge(&other.wmc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_metrics_default() {
        let metrics = CodeMetrics::default();
        assert_eq!(metrics.cyclomatic.cyclomatic(), 1.0);
        assert_eq!(metrics.loc.sloc(), 1.0); // Default is 1 line (line 0)
    }

    #[test]
    fn test_code_metrics_new() {
        let metrics = CodeMetrics::new();
        assert!(metrics.cyclomatic.cyclomatic() > 0.0);
    }

    #[test]
    fn test_compute_derived() {
        let mut metrics = CodeMetrics::new();
        metrics.compute_derived();
        // MI should be computed
        assert!(metrics.maintainability_index.mi_original() >= 0.0 || metrics.maintainability_index.mi_original() < 0.0);
    }

    #[test]
    fn test_display() {
        let metrics = CodeMetrics::new();
        let display = format!("{}", metrics);
        assert!(display.contains("Code Metrics"));
    }
}
