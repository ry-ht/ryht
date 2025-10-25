//! ABC Software Metric
//!
//! The ABC metric measures the size of source code by counting:
//! - Assignments (A)
//! - Branches (B)
//! - Conditions (C)
//!
//! The ABC score can be represented by its components or by the magnitude:
//! |<A,B,C>| = sqrt(A^2 + B^2 + C^2)
//!
//! ## Advanced Declaration Tracking
//!
//! This implementation includes advanced declaration tracking that distinguishes between:
//! - Variable declarations (counted as assignments)
//! - Constant declarations (NOT counted as assignments)
//!
//! ### Usage Example
//!
//! ```rust
//! use cortex_code_analysis::metrics::abc::AbcStats;
//!
//! let mut stats = AbcStats::new();
//!
//! // Example 1: Variable declaration with initialization
//! // Equivalent to: let x = 5;
//! stats.start_var_declaration();
//! stats.add_assignment_with_context(); // Counts as assignment
//! stats.clear_declaration();
//! assert_eq!(stats.assignments(), 1.0);
//!
//! // Example 2: Constant declaration with initialization
//! // Equivalent to: const X = 5;
//! let mut stats = AbcStats::new();
//! stats.start_const_declaration();
//! stats.add_assignment_with_context(); // Does NOT count as assignment
//! stats.clear_declaration();
//! assert_eq!(stats.assignments(), 0.0);
//!
//! // Example 3: Java final modifier (promotes var to const)
//! // Equivalent to: final int X = 5;
//! let mut stats = AbcStats::new();
//! stats.start_var_declaration();
//! stats.promote_to_const(); // Promotes to constant
//! stats.add_assignment_with_context(); // Does NOT count as assignment
//! stats.clear_declaration();
//! assert_eq!(stats.assignments(), 0.0);
//! ```
//!
//! ### Language-Specific Implementation Guide
//!
//! When implementing language-specific ABC metrics, use the declaration tracking methods:
//!
//! **For Java:**
//! - Call `start_var_declaration()` on `FieldDeclaration` or `LocalVariableDeclaration`
//! - Call `promote_to_const()` when encountering `Final` keyword
//! - Call `add_assignment_with_context()` on assignment operators (`=`)
//! - Call `clear_declaration()` on statement terminators (`;`)
//!
//! **For JavaScript/TypeScript:**
//! - Call `start_var_declaration()` on `var` or `let` declarations
//! - Call `start_const_declaration()` on `const` declarations
//! - Call `add_assignment_with_context()` on initialization assignments
//! - Call `clear_declaration()` after processing the declaration
//!
//! **For Rust:**
//! - Call `start_var_declaration()` on `let mut` bindings
//! - Call `start_const_declaration()` on `let` bindings (immutable) or `const` items
//! - Call `add_assignment_with_context()` on initialization
//! - Call `clear_declaration()` after the binding statement

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
    #[serde(skip)]
    declaration: Vec<DeclKind>,
}

/// Declaration kind tracking for advanced assignment detection
#[derive(Debug, Clone, PartialEq)]
enum DeclKind {
    /// Variable declaration
    Var,
    /// Constant declaration
    Const,
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
            declaration: Vec::new(),
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

    /// Marks the start of a variable declaration
    pub fn start_var_declaration(&mut self) {
        self.declaration.push(DeclKind::Var);
    }

    /// Marks the start of a constant declaration
    pub fn start_const_declaration(&mut self) {
        self.declaration.push(DeclKind::Const);
    }

    /// Promotes the last variable declaration to a constant declaration
    pub fn promote_to_const(&mut self) {
        if matches!(self.declaration.last(), Some(DeclKind::Var)) {
            self.declaration.push(DeclKind::Const);
        }
    }

    /// Clears declaration tracking (e.g., at end of statement)
    pub fn clear_declaration(&mut self) {
        self.declaration.clear();
    }

    /// Adds an assignment, respecting declaration context
    /// Constant declarations are not counted as assignments
    pub fn add_assignment_with_context(&mut self) {
        match self.declaration.last() {
            Some(DeclKind::Const) => {
                // Constant declarations are not counted as assignments
            }
            Some(DeclKind::Var) => {
                self.assignments += 1.0;
            }
            None => {
                // Regular assignment outside of declaration context
                self.assignments += 1.0;
            }
        }
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

    #[test]
    fn test_declaration_tracking_var() {
        let mut stats = AbcStats::default();

        // Simulate: let x = 5;
        stats.start_var_declaration();
        stats.add_assignment_with_context(); // Should count as assignment
        stats.clear_declaration();

        assert_eq!(stats.assignments(), 1.0);
    }

    #[test]
    fn test_declaration_tracking_const() {
        let mut stats = AbcStats::default();

        // Simulate: const X = 5;
        stats.start_const_declaration();
        stats.add_assignment_with_context(); // Should NOT count as assignment
        stats.clear_declaration();

        assert_eq!(stats.assignments(), 0.0);
    }

    #[test]
    fn test_declaration_tracking_promote() {
        let mut stats = AbcStats::default();

        // Simulate: final int X = 5; (Java)
        // Start as var, then promote to const
        stats.start_var_declaration();
        stats.promote_to_const();
        stats.add_assignment_with_context(); // Should NOT count as assignment
        stats.clear_declaration();

        assert_eq!(stats.assignments(), 0.0);
    }

    #[test]
    fn test_declaration_tracking_multiple_vars() {
        let mut stats = AbcStats::default();

        // Simulate: let x = 1, y = 2;
        stats.start_var_declaration();
        stats.add_assignment_with_context(); // x = 1
        stats.add_assignment_with_context(); // y = 2
        stats.clear_declaration();

        assert_eq!(stats.assignments(), 2.0);
    }

    #[test]
    fn test_declaration_tracking_regular_assignment() {
        let mut stats = AbcStats::default();

        // Regular assignment without declaration context
        stats.add_assignment_with_context();

        assert_eq!(stats.assignments(), 1.0);
    }

    #[test]
    fn test_declaration_context_isolation() {
        let mut stats = AbcStats::default();

        // const X = 1; (should not count)
        stats.start_const_declaration();
        stats.add_assignment_with_context();
        stats.clear_declaration();

        // Regular assignment after declaration cleared
        stats.add_assignment_with_context(); // Should count

        assert_eq!(stats.assignments(), 1.0);
    }
}
