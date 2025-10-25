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

/// Java-specific ABC helper: Inspects parenthesized expressions and NOT operators to find unary conditions
///
/// According to ABC metric definition for Java, unary conditional expressions are implicit conditions
/// that use no relational operators. Examples:
/// - `if (x)` - variable used as boolean
/// - `if (!x)` - NOT operator on variable
/// - `if (m())` - method invocation returning boolean
/// - `if (!(m()))` - NOT operator on method call
///
/// This function recursively traverses parenthesized expressions and NOT operators to determine
/// if the contained expression is actually a boolean condition.
fn java_inspect_container(node: &crate::node::Node, conditions: &mut f64) {
    let mut current = *node;
    let mut current_kind = current.kind();

    // Flag is true if container is known to hold boolean value
    let mut has_boolean_content = if let Some(parent) = node.parent() {
        match parent.kind() {
            "binary_expression" | "if_statement" | "while_statement" | "do_statement" | "for_statement" => true,
            "ternary_expression" => {
                // Only count if this is the condition part (not the result expressions)
                node.previous_sibling()
                    .map(|prev| !matches!(prev.kind(), "?" | ":"))
                    .unwrap_or(true)
            }
            _ => false,
        }
    } else {
        false
    };

    // Traverse through parentheses and NOT operators
    loop {
        let is_parenthesized = current_kind == "parenthesized_expression";
        let is_not_operator = current_kind == "unary_expression"
            && current.child(0)
                .map(|c| c.kind() == "!")
                .unwrap_or(false);

        if !is_parenthesized && !is_not_operator {
            break;
        }

        // NOT operator proves boolean content
        if !has_boolean_content && is_not_operator {
            has_boolean_content = true;
        }

        // Both parenthesized and NOT operators store expression at child index 1
        if let Some(child) = current.child(1) {
            current = child;
            current_kind = current.kind();

            // Found the actual content
            if matches!(current_kind, "method_invocation" | "identifier" | "true" | "false") {
                if has_boolean_content {
                    *conditions += 1.0;
                }
                break;
            }
        } else {
            break;
        }
    }
}

/// Java-specific ABC helper: Counts unary conditions in element lists
///
/// Scans through children of a list node (e.g., BinaryExpression, ArgumentList)
/// and counts any unary conditional expressions found.
fn java_count_unary_conditions(node: &crate::node::Node, conditions: &mut f64) {
    let list_kind = node.kind();

    for child in node.children() {
        let child_kind = child.kind();

        // Direct unary conditions in binary expressions
        if matches!(child_kind, "method_invocation" | "identifier" | "true" | "false")
            && list_kind == "binary_expression"
            && list_kind != "argument_list"
        {
            *conditions += 1.0;
        } else {
            // Check for container nodes that might hold unary conditions
            java_inspect_container(&child, conditions);
        }
    }
}

/// Computes ABC metrics for Java code
///
/// Implements the comprehensive ABC metric computation for Java as defined in:
/// Fitzpatrick, Jerry (1997). "Applying the ABC metric to C, C++ and Java". C++ Report.
/// https://www.softwarerenovation.com/Articles.aspx
///
/// This function performs a complete traversal of the AST, tracking assignments,
/// branches, and conditions with special handling for Java-specific constructs like:
/// - Constant declarations (final fields) which are NOT counted as assignments
/// - Unary conditional expressions in various contexts
/// - Generic type parameters (< and > are NOT counted as conditions)
pub fn compute_java_abc(node: &crate::node::Node, stats: &mut AbcStats) {
    let node_kind = node.kind();

    match node_kind {
        // Assignment operators
        "*=" | "/=" | "%=" | "-=" | "+=" | "<<=" | ">>=" | "&=" | "|=" | "^=" | ">>>=" | "++" | "--" => {
            stats.add_assignment();
        }

        // Declaration tracking
        "field_declaration" | "local_variable_declaration" => {
            stats.start_var_declaration();
        }
        "final" => {
            stats.promote_to_const();
        }
        ";" => {
            stats.clear_declaration();
        }

        // Assignment with context (respects const declarations)
        "=" => {
            stats.add_assignment_with_context();
        }

        // Branches
        "method_invocation" | "new" => {
            stats.add_branch();
        }

        // Conditions
        ">=" | "<=" | "==" | "!=" | "else" | "case" | "default" | "?" | "try" | "catch" => {
            stats.add_condition();
        }

        // < and > (excluding generic types)
        ">" | "<" => {
            if let Some(parent) = node.parent() {
                if parent.kind() != "type_arguments" {
                    stats.add_condition();
                }
            }
        }

        // Unary conditions in binary expressions with && or ||
        "&&" | "||" => {
            if let Some(parent) = node.parent() {
                java_count_unary_conditions(&parent, &mut stats.conditions);
            }
        }

        // Unary conditions in argument lists
        "argument_list" => {
            java_count_unary_conditions(node, &mut stats.conditions);
        }

        // Unary conditions in assignments
        "variable_declarator" | "assignment_expression" => {
            // The child node of index 2 contains the right operand of an assignment operation
            if let Some(right_operand) = node.child(2) {
                if matches!(right_operand.kind(), "parenthesized_expression" | "unary_expression") {
                    java_inspect_container(&right_operand, &mut stats.conditions);
                }
            }
        }

        // Unary conditions in if and while statements
        "if_statement" | "while_statement" => {
            // The child node of index 1 contains the condition
            if let Some(condition) = node.child(1) {
                if condition.kind() == "parenthesized_expression" {
                    java_inspect_container(&condition, &mut stats.conditions);
                }
            }
        }

        // Unary conditions in do-while statements
        "do_statement" => {
            // The child node of index 3 contains the condition
            if let Some(condition) = node.child(3) {
                if condition.kind() == "parenthesized_expression" {
                    java_inspect_container(&condition, &mut stats.conditions);
                }
            }
        }

        // Unary conditions in for statements
        "for_statement" => {
            // The child node of index 3 contains the condition when
            // the initialization expression is a variable declaration
            if let Some(condition) = node.child(3) {
                match condition.kind() {
                    ";" => {
                        // The child node of index 4 contains the condition when
                        // the initialization expression is not a variable declaration
                        if let Some(cond) = node.child(4) {
                            match cond.kind() {
                                "method_invocation" | "identifier" | "true" | "false" | ";" | ")" => {
                                    stats.add_condition();
                                }
                                "parenthesized_expression" | "unary_expression" => {
                                    java_inspect_container(&cond, &mut stats.conditions);
                                }
                                _ => {}
                            }
                        }
                    }
                    "method_invocation" | "identifier" | "true" | "false" => {
                        stats.add_condition();
                    }
                    "parenthesized_expression" | "unary_expression" => {
                        java_inspect_container(&condition, &mut stats.conditions);
                    }
                    _ => {}
                }
            }
        }

        // Unary conditions in return statements
        "return_statement" => {
            // The child node of index 1 contains the return value
            if let Some(value) = node.child(1) {
                if matches!(value.kind(), "parenthesized_expression" | "unary_expression") {
                    java_inspect_container(&value, &mut stats.conditions);
                }
            }
        }

        // Unary conditions in lambda expressions (implicit return)
        "lambda_expression" => {
            // The child node of index 2 contains the return value
            if let Some(value) = node.child(2) {
                if matches!(value.kind(), "parenthesized_expression" | "unary_expression") {
                    java_inspect_container(&value, &mut stats.conditions);
                }
            }
        }

        // Unary conditions in ternary expressions
        "ternary_expression" => {
            // The child node of index 0 contains the condition
            if let Some(condition) = node.child(0) {
                match condition.kind() {
                    "method_invocation" | "identifier" | "true" | "false" => {
                        stats.add_condition();
                    }
                    "parenthesized_expression" | "unary_expression" => {
                        java_inspect_container(&condition, &mut stats.conditions);
                    }
                    _ => {}
                }
            }
            // The child node of index 2 contains the first expression
            if let Some(expression) = node.child(2) {
                if matches!(expression.kind(), "parenthesized_expression" | "unary_expression") {
                    java_inspect_container(&expression, &mut stats.conditions);
                }
            }
            // The child node of index 4 contains the second expression
            if let Some(expression) = node.child(4) {
                if matches!(expression.kind(), "parenthesized_expression" | "unary_expression") {
                    java_inspect_container(&expression, &mut stats.conditions);
                }
            }
        }

        _ => {}
    }

    // Recursively process children
    for child in node.children() {
        compute_java_abc(&child, stats);
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

    // Helper function to parse Java code and compute ABC metrics
    #[cfg(test)]
    fn compute_java_abc_for_code(source: &str) -> AbcStats {
        use crate::TreeSitterWrapper;

        let mut parser = TreeSitterWrapper::new(tree_sitter_java::LANGUAGE.into())
            .expect("Failed to create parser");
        let tree = parser.parse(source).expect("Failed to parse Java code");
        let root = crate::node::Node::new(tree.root_node());

        let mut stats = AbcStats::new();
        compute_java_abc(&root, &mut stats);
        stats.compute_minmax();
        stats
    }

    // Java ABC Tests - migrated from experimental codebase

    // Constant declarations are not counted as assignments
    #[test]
    fn java_constant_declarations() {
        let source = r#"
class A {
    private final int X1 = 0, Y1 = 0;
    public final float PI = 3.14f;
    final static String HELLO = "Hello,";
    protected String world = " world!";   // +1a
    public float e = 2.718f;                // +1a
    private int x2 = 1, y2 = 2;             // +2a

    void m() {
        final int Z1 = 0, Z2 = 0, Z3 = 0;
        final float T = 0.0f;
        int z1 = 1, z2 = 2, z3 = 3;         // +3a
        float t = 60.0f;                    // +1a
    }
}
"#;
        let stats = compute_java_abc_for_code(source);

        // Expected: 8 assignments (excluding final declarations)
        assert_eq!(stats.assignments_sum(), 8.0);
        assert_eq!(stats.branches_sum(), 0.0);
        assert_eq!(stats.conditions_sum(), 0.0);
    }

    #[test]
    fn java_declarations_with_conditions() {
        let source = r#"
boolean a = (1 > 2);            // +1a +1c
boolean b = 3 > 4;              // +1a +1c
boolean c = (1 > 2) && 3 > 4;   // +1a +2c
boolean d = b && (x > 5) || c;  // +1a +3c
boolean e = !d;                 // +1a +1c
boolean f = ((!false));         // +1a +1c
boolean g = !(!(true));         // +1a +1c
boolean h = true;               // +1a
boolean i = (false);            // +1a
boolean j = (((((true)))));     // +1a
boolean k = (((((m())))));      // +1a +1b
boolean l = (((((!m())))));     // +1a +1b +1c
boolean m = (!(!((m()))));      // +1a +1b +1c
List<String> n = null;          // +1a (< and > used for generic types are not counted as conditions)
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 14.0);
        assert_eq!(stats.branches_sum(), 3.0);
        assert_eq!(stats.conditions_sum(), 12.0);
    }

    #[test]
    fn java_assignments_with_conditions() {
        let source = r#"
a = 2 < 1;                  // +1a +1c
b = (4 >= 3) && 2 <= 1;     // +1a +2c
c = a || (x != 10) && b;    // +1a +3c
d = !false;                 // +1a +1c
e = (!false);               // +1a +1c
f = !(false);               // +1a +1c
g = (!(((true))));          // +1a +1c
h = ((true));               // +1a
i = !m();                   // +1a +1b +1c
j = !((m()));               // +1a +1b +1c
k = (!(m()));               // +1a +1b +1c
l = ((!(m())));             // +1a +1b +1c
m = !B.<Integer>m(2);       // +1a +1b +1c
n = !((B.<Integer>m(4)));   // +1a +1b +1c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 14.0);
        assert_eq!(stats.branches_sum(), 6.0);
        assert_eq!(stats.conditions_sum(), 16.0);
    }

    #[test]
    fn java_methods_arguments_with_conditions() {
        let source = r#"
m1(a);                                  // +1b
m2(a, b);                               // +1b
m3(true, (false), (((true))));          // +1b
m3(m1(false), m1(true), m1(false));     // +4b
m1(!a);                                 // +1b +1c
m2((((a))), (!b));                      // +1b +1c
m3(!(a), b, !!!c);                      // +1b +2c
m3(a, !b, m2(!a, !m2(!b, !m1(!c))));    // +4b +6c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 14.0);
        assert_eq!(stats.conditions_sum(), 10.0);
    }

    #[test]
    fn java_if_single_conditions() {
        let source = r#"
if ( a < 0 ) {}             // +1c
if ( ((a != 0)) ) {}        // +1c
if ( !(a > 0) ) {}          // +1c
if ( !(((a == 0))) ) {}     // +1c
if ( b.m1() ) {}            // +1b +1c
if ( !b.m1() ) {}           // +1b +1c
if ( !!b.m2() ) {}          // +1b +1c
if ( (!(b.m1())) ) {}       // +1b +1c
if ( (!(!b.m1())) ) {}      // +1b +1c
if ( ((b.m2())) ) {}        // +1b +1c
if ( ((b.m().m1())) ) {}    // +2b +1c
if ( c ) {}                 // +1c
if ( !c ) {}                // +1c
if ( !!!!!!!!!!c ) {}       // +1c
if ( (((c))) ) {}           // +1c
if ( (((!c))) ) {}          // +1c
if ( ((!(c))) ) {}          // +1c
if ( true ) {}              // +1c
if ( !true ) {}             // +1c
if ( ((false)) ) {}         // +1c
if ( !(!(false)) ) {}       // +1c
if ( !!!false ) {}          // +1c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 8.0);
        assert_eq!(stats.conditions_sum(), 22.0);
    }

    #[test]
    fn java_if_multiple_conditions() {
        let source = r#"
if ( a || b || c || d ) {}              // +4c
if ( a || b && c && d ) {}              // +4c
if ( x < y && a == b ) {}               // +2c
if ( ((z < (x + y))) ) {}               // +1c
if ( a || ((((b))) && c) ) {}           // +3c
if ( a && ((((a == b))) && c) ) {}      // +3c
if ( a || ((((a == b))) || ((c))) ) {}  // +3c
if ( x < y && B.m() ) {}                // +1b +2c
if ( x < y && !(((B.m()))) ) {}         // +1b +2c
if ( !(x < y) && !B.m() ) {}            // +1b +2c
if ( !!!(!!!(a)) && B.m() ||            // +1b +2c
     !B.m() && (((x > 4))) ) {}         // +1b +2c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 5.0);
        assert_eq!(stats.conditions_sum(), 30.0);
    }

    #[test]
    fn java_while_and_do_while_conditions() {
        let source = r#"
while ( (!(!(!(a)))) ) {}                   // +1c
while ( b || 1 > 2 ) {}                     // +2c
while ( x.m() && (((c))) ) {}               // +1b +2c
do {} while ( !!!(((!!!a))) );              // +1c
do {} while ( a || (b && c) );              // +3c
do {} while ( !x.m() && 1 > 2 || !true );   // +1b +3c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 2.0);
        assert_eq!(stats.conditions_sum(), 12.0);
    }

    #[test]
    fn java_return_with_conditions() {
        let source = r#"
class A {
    boolean m1() {
        return !(z >= 0);       // +1c
    }
    boolean m2() {
        return (((!x)));        // +1c
    }
    boolean m3() {
        return x && y;          // +2c
    }
    boolean m4() {
        return y || (z < 0);    // +2c
    }
    boolean m5() {
        return x || y ?         // +3c (two unary conditions and one ?)
            true : false;
    }
}
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 0.0);
        assert_eq!(stats.conditions_sum(), 9.0);
    }

    #[test]
    fn java_return_without_conditions() {
        let source = r#"
class A {
    boolean m1() {
        return x;
    }
    boolean m2() {
        return (x);
    }
    boolean m3() {
        return y.m();   // +1b
    }
    boolean m4() {
        return false;
    }
    void m5() {
        return;
    }
}
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 0.0);
        assert_eq!(stats.branches_sum(), 1.0);
        assert_eq!(stats.conditions_sum(), 0.0);
    }

    #[test]
    fn java_lambda_expressions_return_with_conditions() {
        let source = r#"
Predicate<Boolean> p1 = a -> a;                         // +1a
Predicate<Boolean> p2 = b -> true;                      // +1a
Predicate<Boolean> p3 = c -> m();                       // +1a +1b
Predicate<Integer> p4 = d -> d > 10;                    // +1a +1c
Predicate<Boolean> p5 = (e) -> !e;                      // +1a +1c
Predicate<Boolean> p6 = (f) -> !((!f));                 // +1a +1c
Predicate<Boolean> p7 = (g) -> !g && true;              // +1a +2c
BiPredicate<Boolean, Boolean> bp1 = (h, i) -> !h && !i; // +1a +2c
BiPredicate<Boolean, Boolean> bp2 = (j, k) -> {
    return j || k;                                      // +2c
};
"#;
        let stats = compute_java_abc_for_code(source);

        // 9 lambda assignments (note: last one has braces so no implicit assignment on lambda itself)
        assert_eq!(stats.assignments_sum(), 9.0);
        assert_eq!(stats.branches_sum(), 1.0);
        assert_eq!(stats.conditions_sum(), 9.0);
    }

    #[test]
    fn java_for_with_variable_declaration() {
        let source = r#"
for ( int i1 = 0; !(!(!(!a))); i1++ ) {}                // +2a +1c
for ( int i2 = 0; !B.m(); i2++ ) {}                     // +2a +1b +1c
for ( int i3 = 0; a || false; i3++ ) {}                 // +2a +2c
for ( int i4 = 0; a && B.m() ? true : false; i4++ ) {}  // +2a +1b +3c
for ( int i5 = 0; true; i5++ ) {}                       // +2a +1c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 10.0);
        assert_eq!(stats.branches_sum(), 2.0);
        assert_eq!(stats.conditions_sum(), 8.0);
    }

    #[test]
    fn java_for_without_variable_declaration() {
        let source = r#"
class A{
    void m1() {
        for (i = 0; x < y; i++) {}          // +2a +1c
        for (i = 0; ((x < y)); i++) {}      // +2a +1c
        for (i = 0; !(!(x < y)); i++) {}    // +2a +1c
        for (i = 0; true; i++) {}           // +2a +1c
    }
    void m2() {
        for ( ; true; ) {}  // +1c
    }
    void m3() {
        for ( ; ; ) {}      // +1c (one implicit unary condition set to true)
    }
}
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 8.0);
        assert_eq!(stats.branches_sum(), 0.0);
        assert_eq!(stats.conditions_sum(), 6.0);
    }

    #[test]
    fn java_ternary_conditions() {
        let source = r#"
a = true;                                   // +1a
b = a ? true : false;                       // +1a +2c
c = ((((a)))) ? !false : !b;                // +1a +4c
d = !this.m() ? !!a : (false);              // +1a +1b +3c
e = !(a) && b ? ((c)) : !d;                 // +1a +4c
if ( this.m() ? a : !this.m() ) {}          // +2b +3c
if ( x > 0 ? !(false) : this.m() ) {}       // +1b +3c
if ( x > 0 && x != 3 ? !(a) : (!(b)) ) {}   // +5c
"#;
        let stats = compute_java_abc_for_code(source);

        assert_eq!(stats.assignments_sum(), 5.0);
        assert_eq!(stats.branches_sum(), 4.0);
        assert_eq!(stats.conditions_sum(), 24.0);
    }
}
