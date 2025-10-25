//! Node checking trait and implementations.
//!
//! This module provides the `NodeChecker` trait for analyzing AST nodes
//! and determining their properties (e.g., is it a comment, function, closure, etc.).
//! Language-specific implementations provide precise classification for each supported language.
//!
//! # Features
//!
//! - **Comment Detection**: Identify comments and useful comments (doc comments, coding declarations)
//! - **Function Detection**: Detect function declarations and definitions with complex heuristics
//! - **Closure Detection**: Identify closures, lambdas, and anonymous functions
//! - **String Literal Detection**: Recognize string literals across all languages
//! - **Control Flow Detection**: Identify else-if patterns and other control structures
//! - **Primitive Type Detection**: Detect primitive type nodes
//!
//! # Performance
//!
//! - Uses `OnceLock` for lazy initialization of regex and Aho-Corasick patterns
//! - Inline function macros for JavaScript-family language function detection
//! - Efficient ancestor counting for complex function/closure heuristics

use std::sync::OnceLock;

use aho_corasick::AhoCorasick;
use regex::bytes::Regex;

use crate::node::Node;
use crate::traits::Search;
use crate::Lang;

// Lazy-initialized pattern matchers for performance
static AHO_CORASICK: OnceLock<AhoCorasick> = OnceLock::new();
static RE: OnceLock<Regex> = OnceLock::new();

/// Node checking trait for classifying AST nodes.
///
/// Provides methods to determine the type and properties of AST nodes
/// in a language-agnostic way. Each language implements this trait
/// to provide accurate node classification.
pub trait NodeChecker {
    /// Check if a node represents a comment.
    fn is_comment(node: &Node, lang: Lang) -> bool;

    /// Check if a node is a useful/significant comment (e.g., doc comments, coding declarations).
    fn is_useful_comment(node: &Node, code: &[u8], lang: Lang) -> bool;

    /// Check if a node represents a function space (function, class, namespace, etc.).
    fn is_func_space(node: &Node, lang: Lang) -> bool;

    /// Check if a node represents a function definition.
    fn is_func(node: &Node, lang: Lang) -> bool;

    /// Check if a node represents a closure/lambda expression.
    fn is_closure(node: &Node, lang: Lang) -> bool;

    /// Check if a node represents a function/method call.
    fn is_call(node: &Node, lang: Lang) -> bool;

    /// Check if a node is a non-argument token (parentheses, commas, etc.).
    fn is_non_arg(node: &Node, lang: Lang) -> bool;

    /// Check if a node represents a string literal.
    fn is_string(node: &Node, lang: Lang) -> bool;

    /// Check if a node represents an else-if statement.
    fn is_else_if(node: &Node, lang: Lang) -> bool;

    /// Check if a kind ID represents a primitive type.
    fn is_primitive(id: u16, lang: Lang) -> bool;

    /// Check if a node contains syntax errors.
    fn is_error(node: &Node) -> bool {
        node.has_error()
    }
}

/// Default implementation of NodeChecker.
pub struct DefaultNodeChecker;

impl NodeChecker for DefaultNodeChecker {
    fn is_comment(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_comment(node),
            Lang::Python => is_python_comment(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_comment(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_comment(node),
            Lang::Java => is_java_comment(node),
            Lang::Cpp => is_cpp_comment(node),
            Lang::Kotlin => false, // Not yet implemented
        }
    }

    fn is_useful_comment(node: &Node, code: &[u8], lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_useful_comment(node, code),
            Lang::Python => is_python_useful_comment(node, code),
            Lang::Cpp => is_cpp_useful_comment(node, code),
            _ => false,
        }
    }

    fn is_func_space(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_func_space(node),
            Lang::Python => is_python_func_space(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_func_space(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_func_space(node),
            Lang::Java => is_java_func_space(node),
            Lang::Cpp => is_cpp_func_space(node),
            Lang::Kotlin => false,
        }
    }

    fn is_func(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_func(node),
            Lang::Python => is_python_func(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_func(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_func(node),
            Lang::Java => is_java_func(node),
            Lang::Cpp => is_cpp_func(node),
            Lang::Kotlin => false,
        }
    }

    fn is_closure(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_closure(node),
            Lang::Python => is_python_closure(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_closure(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_closure(node),
            Lang::Java => is_java_closure(node),
            Lang::Cpp => is_cpp_closure(node),
            Lang::Kotlin => false,
        }
    }

    fn is_call(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_call(node),
            Lang::Python => is_python_call(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_call(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_call(node),
            Lang::Java => is_java_call(node),
            Lang::Cpp => is_cpp_call(node),
            Lang::Kotlin => false,
        }
    }

    fn is_non_arg(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_non_arg(node),
            Lang::Python => is_python_non_arg(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_non_arg(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_non_arg(node),
            Lang::Cpp => is_cpp_non_arg(node),
            _ => false,
        }
    }

    fn is_string(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_string(node),
            Lang::Python => is_python_string(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_string(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_string(node),
            Lang::Java => is_java_string(node),
            Lang::Cpp => is_cpp_string(node),
            Lang::Kotlin => false,
        }
    }

    fn is_else_if(node: &Node, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_else_if(node),
            Lang::TypeScript | Lang::Tsx => is_typescript_else_if(node),
            Lang::JavaScript | Lang::Jsx => is_javascript_else_if(node),
            Lang::Cpp => is_cpp_else_if(node),
            _ => false,
        }
    }

    fn is_primitive(id: u16, lang: Lang) -> bool {
        match lang {
            Lang::Rust => is_rust_primitive(id),
            Lang::TypeScript | Lang::Tsx => is_typescript_primitive(id),
            Lang::Cpp => is_cpp_primitive(id),
            _ => false,
        }
    }
}

// ===== Rust implementations =====

fn is_rust_comment(node: &Node) -> bool {
    let kind = node.kind();
    kind == "line_comment" || kind == "block_comment"
}

fn is_rust_useful_comment(node: &Node, code: &[u8]) -> bool {
    // Check for macro token comments or cbindgen directives
    if let Some(parent) = node.parent() {
        if parent.kind() == "token_tree" {
            return true;
        }
    }
    let start = node.start_byte();
    let end = node.end_byte();
    if end > start && end <= code.len() {
        let slice = &code[start..end];
        slice.starts_with(b"/// cbindgen:")
    } else {
        false
    }
}

fn is_rust_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "source_file" | "function_item" | "impl_item" | "trait_item" | "closure_expression"
    )
}

fn is_rust_func(node: &Node) -> bool {
    node.kind() == "function_item"
}

fn is_rust_closure(node: &Node) -> bool {
    node.kind() == "closure_expression"
}

fn is_rust_call(node: &Node) -> bool {
    node.kind() == "call_expression"
}

fn is_rust_non_arg(node: &Node) -> bool {
    matches!(node.kind(), "(" | "," | ")" | "|" | "attribute_item")
}

fn is_rust_string(node: &Node) -> bool {
    matches!(node.kind(), "string_literal" | "raw_string_literal")
}

fn is_rust_else_if(node: &Node) -> bool {
    if node.kind() != "if_expression" {
        return false;
    }
    node.parent()
        .map(|p| p.kind() == "else_clause")
        .unwrap_or(false)
}

fn is_rust_primitive(id: u16) -> bool {
    // This requires tree-sitter grammar integration
    // For now, we check by string comparison
    // In production, this would use: id == Rust::PrimitiveType
    _ = id;
    false // Placeholder - needs tree-sitter language enum integration
}

// ===== Python implementations =====

fn is_python_comment(node: &Node) -> bool {
    node.kind() == "comment"
}

fn is_python_useful_comment(node: &Node, code: &[u8]) -> bool {
    // Python coding declarations in first two lines
    if node.start_row() > 1 {
        return false;
    }
    let start = node.start_byte();
    let end = node.end_byte();
    if end > start && end <= code.len() {
        let slice = &code[start..end];
        // Check for coding declaration pattern
        RE.get_or_init(|| {
            Regex::new(r"^[ \t\f]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)").unwrap()
        })
        .is_match(slice)
    } else {
        false
    }
}

fn is_python_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "module" | "function_definition" | "class_definition"
    )
}

fn is_python_func(node: &Node) -> bool {
    node.kind() == "function_definition"
}

fn is_python_closure(node: &Node) -> bool {
    node.kind() == "lambda"
}

fn is_python_call(node: &Node) -> bool {
    node.kind() == "call"
}

fn is_python_non_arg(node: &Node) -> bool {
    matches!(node.kind(), "(" | "," | ")")
}

fn is_python_string(node: &Node) -> bool {
    matches!(node.kind(), "string" | "concatenated_string")
}

// ===== TypeScript implementations =====

fn is_typescript_comment(node: &Node) -> bool {
    node.kind() == "comment"
}

fn is_typescript_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "program"
            | "function"
            | "function_expression"
            | "class"
            | "generator_function"
            | "function_declaration"
            | "method_definition"
            | "generator_function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "arrow_function"
    )
}

fn is_typescript_func(node: &Node) -> bool {
    check_if_js_func(node)
}

fn is_typescript_closure(node: &Node) -> bool {
    check_if_js_closure(node)
}

fn is_typescript_call(node: &Node) -> bool {
    node.kind() == "call_expression"
}

fn is_typescript_non_arg(node: &Node) -> bool {
    matches!(node.kind(), "(" | "," | ")")
}

fn is_typescript_string(node: &Node) -> bool {
    matches!(node.kind(), "string" | "template_string")
}

fn is_typescript_else_if(node: &Node) -> bool {
    if node.kind() != "if_statement" {
        return false;
    }
    node.parent()
        .map(|p| p.kind() == "else_clause")
        .unwrap_or(false)
}

fn is_typescript_primitive(id: u16) -> bool {
    // Placeholder - would check: id == Typescript::PredefinedType
    _ = id;
    false
}

// ===== JavaScript implementations =====

fn is_javascript_comment(node: &Node) -> bool {
    node.kind() == "comment"
}

fn is_javascript_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "program"
            | "function_expression"
            | "class"
            | "generator_function"
            | "function_declaration"
            | "method_definition"
            | "generator_function_declaration"
            | "class_declaration"
            | "arrow_function"
    )
}

fn is_javascript_func(node: &Node) -> bool {
    check_if_js_func(node)
}

fn is_javascript_closure(node: &Node) -> bool {
    check_if_js_closure(node)
}

fn is_javascript_call(node: &Node) -> bool {
    node.kind() == "call_expression"
}

fn is_javascript_non_arg(node: &Node) -> bool {
    matches!(node.kind(), "(" | "," | ")")
}

fn is_javascript_string(node: &Node) -> bool {
    matches!(node.kind(), "string" | "template_string")
}

fn is_javascript_else_if(node: &Node) -> bool {
    if node.kind() != "if_statement" {
        return false;
    }
    node.parent()
        .map(|p| p.kind() == "if_statement")
        .unwrap_or(false)
}

// ===== Java implementations =====

fn is_java_comment(node: &Node) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
}

fn is_java_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "program" | "class_declaration" | "interface_declaration"
    )
}

fn is_java_func(node: &Node) -> bool {
    matches!(node.kind(), "method_declaration" | "constructor_declaration")
}

fn is_java_closure(node: &Node) -> bool {
    node.kind() == "lambda_expression"
}

fn is_java_call(node: &Node) -> bool {
    node.kind() == "method_invocation"
}

fn is_java_string(node: &Node) -> bool {
    node.kind() == "string_literal"
}

// ===== C++ implementations =====

fn is_cpp_comment(node: &Node) -> bool {
    node.kind() == "comment"
}

fn is_cpp_useful_comment(node: &Node, code: &[u8]) -> bool {
    // Check for rustbindgen markers
    let start = node.start_byte();
    let end = node.end_byte();
    if end > start && end <= code.len() {
        let slice = &code[start..end];
        AHO_CORASICK
            .get_or_init(|| AhoCorasick::new(["<div rustbindgen"]).unwrap())
            .is_match(slice)
    } else {
        false
    }
}

fn is_cpp_func_space(node: &Node) -> bool {
    matches!(
        node.kind(),
        "translation_unit"
            | "function_definition"
            | "struct_specifier"
            | "class_specifier"
            | "namespace_definition"
    )
}

fn is_cpp_func(node: &Node) -> bool {
    node.kind() == "function_definition"
}

fn is_cpp_closure(node: &Node) -> bool {
    node.kind() == "lambda_expression"
}

fn is_cpp_call(node: &Node) -> bool {
    node.kind() == "call_expression"
}

fn is_cpp_non_arg(node: &Node) -> bool {
    matches!(node.kind(), "(" | "," | ")")
}

fn is_cpp_string(node: &Node) -> bool {
    matches!(
        node.kind(),
        "string_literal" | "concatenated_string" | "raw_string_literal"
    )
}

fn is_cpp_else_if(node: &Node) -> bool {
    if node.kind() != "if_statement" {
        return false;
    }
    node.parent()
        .map(|p| p.kind() == "else_clause")
        .unwrap_or(false)
}

fn is_cpp_primitive(id: u16) -> bool {
    // Placeholder - would check: id == Cpp::PrimitiveType
    _ = id;
    false
}

// ===== JavaScript/TypeScript helper functions =====

/// Complex heuristic to determine if a JavaScript/TypeScript node is a function.
///
/// This implements sophisticated logic to distinguish between:
/// - Function declarations (always functions)
/// - Method definitions (always functions)
/// - Function expressions (check if assigned)
/// - Arrow functions (check if assigned)
///
/// The logic checks ancestor patterns to determine if the function is:
/// - Assigned to a variable
/// - Part of an assignment expression
/// - A labeled statement
/// - An object property
///
/// vs. being:
/// - Inside a statement block (closure/callback)
/// - A return value (closure/callback)
/// - Part of a new expression (closure/callback)
/// - An argument (closure/callback)
fn check_if_js_func(node: &Node) -> bool {
    match node.kind() {
        "function_declaration" | "method_definition" => true,
        "function_expression" => {
            // Check if it's a function assignment
            node.count_specific_ancestors(
                |n| {
                    matches!(
                        n.kind(),
                        "variable_declarator" | "assignment_expression" | "labeled_statement" | "pair"
                    )
                },
                |n| {
                    matches!(
                        n.kind(),
                        "statement_block" | "return_statement" | "new_expression" | "arguments"
                    )
                },
            ) > 0
                || has_child_identifier(node)
        }
        "arrow_function" => {
            node.count_specific_ancestors(
                |n| {
                    matches!(
                        n.kind(),
                        "variable_declarator" | "assignment_expression" | "labeled_statement"
                    )
                },
                |n| {
                    matches!(
                        n.kind(),
                        "statement_block"
                            | "return_statement"
                            | "new_expression"
                            | "call_expression"
                    )
                },
            ) > 0
                || has_sibling_property_identifier(node)
        }
        _ => false,
    }
}

/// Complex heuristic to determine if a JavaScript/TypeScript node is a closure.
///
/// This is essentially the inverse of `check_if_js_func`:
/// - Generator functions are always closures
/// - Function expressions that are NOT assigned are closures
/// - Arrow functions that are NOT assigned are closures
fn check_if_js_closure(node: &Node) -> bool {
    match node.kind() {
        "generator_function" | "generator_function_declaration" => true,
        "function_expression" => !check_if_js_func(node),
        "arrow_function" => !check_if_js_func(node),
        _ => false,
    }
}

/// Check if node has an identifier child
fn has_child_identifier(node: &Node) -> bool {
    node.children().any(|child| child.kind() == "identifier")
}

/// Check if node has a property_identifier sibling
fn has_sibling_property_identifier(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        parent
            .children()
            .any(|child| child.kind() == "property_identifier")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_checker_trait_exists() {
        // Ensure the trait is properly defined
        fn _assert_impl<T: NodeChecker>() {}
        _assert_impl::<DefaultNodeChecker>();
    }

    #[test]
    fn test_regex_lazy_initialization() {
        // Test that regex is lazily initialized
        let re = RE.get_or_init(|| {
            Regex::new(r"^[ \t\f]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)").unwrap()
        });
        assert!(re.is_match(b"# coding: utf-8"));
        assert!(re.is_match(b"# -*- coding: utf-8 -*-"));
        assert!(!re.is_match(b"# regular comment"));
    }

    #[test]
    fn test_aho_corasick_initialization() {
        let ac = AHO_CORASICK.get_or_init(|| AhoCorasick::new(["<div rustbindgen"]).unwrap());
        assert!(ac.is_match(b"<div rustbindgen>"));
        assert!(!ac.is_match(b"regular comment"));
    }
}

// ============================================================================
// Pattern Matching and Lint Rules
// ============================================================================

/// Severity level for lint violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning - potential issue
    Warning,
    /// Error - definite problem
    Error,
}

/// A lint violation found in code
#[derive(Debug, Clone)]
pub struct LintViolation {
    /// Rule that was violated
    pub rule_id: String,
    /// Severity of the violation
    pub severity: Severity,
    /// Description of the violation
    pub message: String,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Optional suggestion for fixing
    pub suggestion: Option<String>,
}

impl LintViolation {
    /// Create a new lint violation
    pub fn new(
        rule_id: String,
        severity: Severity,
        message: String,
        node: &Node,
    ) -> Self {
        Self {
            rule_id,
            severity,
            message,
            start_line: node.start_row() + 1,
            end_line: node.end_row() + 1,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            suggestion: None,
        }
    }

    /// Add a suggestion for fixing the violation
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Trait for implementing custom lint rules
pub trait LintRule {
    /// Get the unique identifier for this rule
    fn id(&self) -> &str;

    /// Get the description of this rule
    fn description(&self) -> &str;

    /// Get the default severity for this rule
    fn severity(&self) -> Severity {
        Severity::Warning
    }

    /// Check a node and return violations if any
    fn check(&self, node: &Node, code: &[u8], lang: Lang) -> Vec<LintViolation>;
}

/// Built-in lint rules

/// Rule: Function too long (> 50 lines)
pub struct FunctionTooLongRule;

impl LintRule for FunctionTooLongRule {
    fn id(&self) -> &str {
        "function-too-long"
    }

    fn description(&self) -> &str {
        "Functions should be shorter than 50 lines"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, node: &Node, _code: &[u8], lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if DefaultNodeChecker::is_func(node, lang) {
            let line_count = node.line_count();
            if line_count > 50 {
                violations.push(
                    LintViolation::new(
                        self.id().to_string(),
                        self.severity(),
                        format!("Function has {} lines, should be < 50", line_count),
                        node,
                    )
                    .with_suggestion("Consider breaking this function into smaller functions".to_string()),
                );
            }
        }

        violations
    }
}

/// Rule: Deeply nested code (> 4 levels)
pub struct DeepNestingRule;

impl LintRule for DeepNestingRule {
    fn id(&self) -> &str {
        "deep-nesting"
    }

    fn description(&self) -> &str {
        "Code should not be nested more than 4 levels deep"
    }

    fn check(&self, node: &Node, _code: &[u8], _lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        let depth = node.depth();
        if depth > 8 {
            // Depth > 8 usually means 4+ levels of nesting in code
            violations.push(
                LintViolation::new(
                    self.id().to_string(),
                    self.severity(),
                    format!("Code is nested {} levels deep", depth / 2),
                    node,
                )
                .with_suggestion("Consider extracting nested logic into separate functions".to_string()),
            );
        }

        violations
    }
}

/// Rule: Missing documentation comment
pub struct MissingDocCommentRule;

impl LintRule for MissingDocCommentRule {
    fn id(&self) -> &str {
        "missing-doc-comment"
    }

    fn description(&self) -> &str {
        "Public functions should have documentation comments"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, node: &Node, code: &[u8], lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if DefaultNodeChecker::is_func(node, lang) {
            // Check if there's a doc comment before this function
            let has_doc = node
                .previous_sibling()
                .map(|prev| {
                    DefaultNodeChecker::is_comment(&prev, lang)
                        && is_doc_comment_text(&prev, code, lang)
                })
                .unwrap_or(false);

            if !has_doc {
                violations.push(LintViolation::new(
                    self.id().to_string(),
                    self.severity(),
                    "Function is missing a documentation comment".to_string(),
                    node,
                ));
            }
        }

        violations
    }
}

fn is_doc_comment_text(node: &Node, code: &[u8], lang: Lang) -> bool {
    if let Some(text) = node.utf8_text(code) {
        match lang {
            Lang::Rust => text.starts_with("///") || text.starts_with("//!"),
            Lang::TypeScript | Lang::JavaScript => text.starts_with("/**"),
            Lang::Java | Lang::Cpp => text.starts_with("/**"),
            _ => false,
        }
    } else {
        false
    }
}

/// Rule: TODO comment found
pub struct TodoCommentRule;

impl LintRule for TodoCommentRule {
    fn id(&self) -> &str {
        "todo-comment"
    }

    fn description(&self) -> &str {
        "TODO comments should be resolved"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, node: &Node, code: &[u8], lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if DefaultNodeChecker::is_comment(node, lang) {
            if let Some(text) = node.utf8_text(code) {
                if text.contains("TODO") || text.contains("FIXME") || text.contains("XXX") {
                    violations.push(LintViolation::new(
                        self.id().to_string(),
                        self.severity(),
                        "TODO comment found".to_string(),
                        node,
                    ));
                }
            }
        }

        violations
    }
}

/// Lint checker that runs multiple rules
pub struct LintChecker {
    rules: Vec<Box<dyn LintRule>>,
}

impl LintChecker {
    /// Create a new lint checker
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a checker with default rules
    pub fn with_default_rules() -> Self {
        let mut checker = Self::new();
        checker.add_rule(Box::new(FunctionTooLongRule));
        checker.add_rule(Box::new(DeepNestingRule));
        checker.add_rule(Box::new(MissingDocCommentRule));
        checker.add_rule(Box::new(TodoCommentRule));
        checker
    }

    /// Add a rule to the checker
    pub fn add_rule(&mut self, rule: Box<dyn LintRule>) {
        self.rules.push(rule);
    }

    /// Check a node against all rules
    pub fn check_node(&self, node: &Node, code: &[u8], lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            violations.extend(rule.check(node, code, lang));
        }

        violations
    }

    /// Check an entire tree
    pub fn check_tree(&self, root: &Node, code: &[u8], lang: Lang) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        root.act_on_node(&mut |node| {
            violations.extend(self.check_node(node, code, lang));
        });

        // Sort by line number
        violations.sort_by_key(|v| (v.start_line, v.start_byte));

        violations
    }
}

impl Default for LintChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Anti-pattern Detection
// ============================================================================

/// Common anti-patterns to detect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiPattern {
    /// Magic numbers (hardcoded constants)
    MagicNumber,
    /// God function (too many responsibilities)
    GodFunction,
    /// Deep nesting
    DeepNesting,
    /// Long parameter list
    LongParameterList,
    /// Duplicate code
    DuplicateCode,
    /// Dead code
    DeadCode,
}

/// Detect anti-patterns in code
pub struct AntiPatternDetector;

impl AntiPatternDetector {
    /// Detect magic numbers (numeric literals in code)
    pub fn detect_magic_numbers<'a>(node: &Node<'a>, code: &[u8]) -> Vec<Node<'a>> {
        let mut numbers = Vec::new();

        node.act_on_node(&mut |n| {
            let kind = n.kind();
            if kind.contains("integer") || kind.contains("float") || kind.contains("number") {
                // Exclude 0, 1, -1 as they're common and not "magic"
                if let Some(text) = n.utf8_text(code) {
                    if text != "0" && text != "1" && text != "-1" && text != "0.0" && text != "1.0" {
                        numbers.push(*n);
                    }
                }
            }
        });

        numbers
    }

    /// Detect functions with too many parameters (> 4)
    pub fn detect_long_parameter_lists<'a>(node: &Node<'a>, lang: Lang) -> Vec<Node<'a>> {
        let mut functions = Vec::new();

        node.act_on_node(&mut |n| {
            if DefaultNodeChecker::is_func(n, lang) {
                let param_count = count_parameters(n);
                if param_count > 4 {
                    functions.push(*n);
                }
            }
        });

        functions
    }
}

fn count_parameters(node: &Node) -> usize {
    // Look for parameters field or count commas in parameter list
    if let Some(params) = node.child_by_field_name("parameters") {
        // Count named children as a rough estimate
        params.named_child_count()
    } else {
        0
    }
}
