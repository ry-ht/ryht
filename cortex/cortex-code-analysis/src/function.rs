//! Function detection and boundary analysis.
//!
//! This module provides functionality to detect functions in source code and determine
//! their boundaries (start and end lines). It supports all languages in the cortex ecosystem.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{detect_functions, Lang};
//!
//! # fn main() -> anyhow::Result<()> {
//! let code = r#"
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! fn multiply(x: i32, y: i32) -> i32 {
//!     x * y
//! }
//! "#;
//!
//! let functions = detect_functions(code, Lang::Rust)?;
//! assert_eq!(functions.len(), 2);
//! assert_eq!(functions[0].name, "add");
//! assert_eq!(functions[1].name, "multiply");
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::lang::Lang;
use crate::node::Node;
use crate::parser::Parser;
use crate::traits::{LanguageInfo, ParserTrait, Search};
use crate::languages::{
    RustLanguage, TypeScriptLanguage, JavaScriptLanguage, PythonLanguage,
};

/// Represents the span of a function in source code.
///
/// Contains the function name and its line boundaries. Lines are 1-indexed
/// to match typical editor conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSpan {
    /// The name of the function.
    ///
    /// For anonymous functions, this will be `"<anonymous>"`.
    pub name: String,

    /// The first line of the function (1-indexed).
    pub start_line: usize,

    /// The last line of the function (1-indexed).
    pub end_line: usize,
}

impl FunctionSpan {
    /// Create a new function span.
    pub fn new(name: String, start_line: usize, end_line: usize) -> Self {
        Self {
            name,
            start_line,
            end_line,
        }
    }

    /// Get the number of lines in this function.
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Check if a given line number is within this function's span.
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// Detect all functions in the given code for the specified language.
///
/// This function parses the code and identifies all function definitions,
/// returning their names and line boundaries.
///
/// # Arguments
///
/// * `code` - The source code to analyze
/// * `lang` - The programming language of the code
///
/// # Returns
///
/// A vector of [`FunctionSpan`] objects, one for each detected function.
///
/// # Errors
///
/// Returns an error if:
/// - The language is not supported
/// - The code cannot be parsed
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{detect_functions, Lang};
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn hello() { println!(\"Hello!\"); }";
/// let functions = detect_functions(code, Lang::Rust)?;
/// assert_eq!(functions.len(), 1);
/// assert_eq!(functions[0].name, "hello");
/// # Ok(())
/// # }
/// ```
pub fn detect_functions(code: &str, lang: Lang) -> Result<Vec<FunctionSpan>> {
    match lang {
        Lang::Rust => detect_functions_impl::<RustLanguage>(code),
        Lang::TypeScript | Lang::Tsx => detect_functions_impl::<TypeScriptLanguage>(code),
        Lang::JavaScript | Lang::Jsx => detect_functions_impl::<JavaScriptLanguage>(code),
        Lang::Python => detect_functions_impl::<PythonLanguage>(code),
        _ => anyhow::bail!("Function detection not yet implemented for {:?}", lang),
    }
}

/// Internal implementation of function detection for a specific language.
fn detect_functions_impl<T: LanguageInfo>(code: &str) -> Result<Vec<FunctionSpan>> {
    let path = std::path::Path::new("dummy.rs");
    let parser = Parser::<T>::new(code.as_bytes().to_vec(), path)
        .context("Failed to parse code")?;

    Ok(extract_functions::<T>(&parser))
}

/// Extract functions from a parsed AST.
fn extract_functions<T: LanguageInfo>(parser: &Parser<T>) -> Vec<FunctionSpan> {
    let root = parser.get_root();
    let code = parser.get_code();
    let mut spans = Vec::new();

    root.act_on_node(&mut |node| {
        if is_function::<T>(node) {
            let start_line = node.start_row() + 1;
            let end_line = node.end_row() + 1;

            if let Some(name) = get_function_name::<T>(node, code) {
                spans.push(FunctionSpan {
                    name: name.to_string(),
                    start_line,
                    end_line,
                });
            } else {
                // Function without a clear name - mark as anonymous
                spans.push(FunctionSpan {
                    name: "<anonymous>".to_string(),
                    start_line,
                    end_line,
                });
            }
        }
    });

    spans
}

/// Check if a node represents a function in the given language.
fn is_function<T: LanguageInfo>(node: &Node) -> bool {
    let lang = T::get_lang();
    let kind = node.kind();

    match lang {
        Lang::Rust => {
            kind == "function_item"
        }
        Lang::TypeScript | Lang::Tsx => {
            matches!(
                kind,
                "function_declaration"
                    | "method_definition"
                    | "function_expression"
                    | "arrow_function"
                    | "generator_function"
                    | "generator_function_declaration"
            ) && is_named_function_typescript(node)
        }
        Lang::JavaScript | Lang::Jsx => {
            matches!(
                kind,
                "function_declaration"
                    | "method_definition"
                    | "function_expression"
                    | "arrow_function"
                    | "generator_function"
                    | "generator_function_declaration"
            ) && is_named_function_javascript(node)
        }
        Lang::Python => {
            kind == "function_definition"
        }
        Lang::Java => {
            kind == "method_declaration" || kind == "constructor_declaration"
        }
        Lang::Cpp => {
            kind == "function_definition"
        }
        _ => false,
    }
}

/// Check if a TypeScript/JavaScript node is a named function (not a closure/callback).
fn is_named_function_typescript(node: &Node) -> bool {
    let kind = node.kind();

    match kind {
        "function_declaration" | "method_definition" => true,
        "function_expression" => {
            // Check if this is assigned to a variable or property
            count_specific_ancestors(
                node,
                |n| matches!(n.kind(), "variable_declarator" | "assignment_expression" | "labeled_statement" | "pair"),
                |n| matches!(n.kind(), "statement_block" | "return_statement" | "new_expression" | "arguments"),
            ) > 0 || node.is_child_of_kind("identifier")
        }
        "arrow_function" => {
            // Arrow functions are functions if they're assigned to a variable
            count_specific_ancestors(
                node,
                |n| matches!(n.kind(), "variable_declarator" | "assignment_expression" | "labeled_statement"),
                |n| matches!(n.kind(), "statement_block" | "return_statement" | "new_expression" | "call_expression"),
            ) > 0 || has_sibling_of_kind(node, "property_identifier")
        }
        _ => false,
    }
}

/// Check if a JavaScript node is a named function (not a closure/callback).
fn is_named_function_javascript(node: &Node) -> bool {
    // For now, use the same logic as TypeScript
    is_named_function_typescript(node)
}

/// Helper to count specific ancestors matching a predicate.
fn count_specific_ancestors(
    node: &Node,
    check: fn(&Node) -> bool,
    stop: fn(&Node) -> bool,
) -> usize {
    let mut count = 0;
    let mut current = *node;

    while let Some(parent) = current.parent() {
        if stop(&parent) {
            break;
        }
        if check(&parent) {
            count += 1;
        }
        current = parent;
    }

    count
}

/// Check if node has a sibling of the given kind.
fn has_sibling_of_kind(node: &Node, kind: &str) -> bool {
    if let Some(parent) = node.parent() {
        for child in parent.children() {
            if child.kind() == kind {
                return true;
            }
        }
    }
    false
}

impl Node<'_> {
    /// Helper method to check if node has children of a specific kind.
    fn is_child_of_kind(&self, kind: &str) -> bool {
        for child in self.children() {
            if child.kind() == kind {
                return true;
            }
        }
        false
    }
}

/// Extract the function name from a node.
fn get_function_name<'a, T: LanguageInfo>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    let lang = T::get_lang();

    match lang {
        Lang::Rust => {
            // Rust functions have a "name" field
            if let Some(name_node) = node.child_by_field_name("name") {
                let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                return std::str::from_utf8(name_bytes).ok();
            }
            None
        }
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
            // Try to get the name field first
            if let Some(name_node) = node.child_by_field_name("name") {
                let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                return std::str::from_utf8(name_bytes).ok();
            }

            // For function expressions, check if in a variable declarator or pair
            if let Some(parent) = node.parent() {
                match parent.kind() {
                    "pair" => {
                        // Object method: { foo: function() {} }
                        if let Some(key) = parent.child_by_field_name("key") {
                            let name_bytes = &code[key.start_byte()..key.end_byte()];
                            return std::str::from_utf8(name_bytes).ok();
                        }
                    }
                    "variable_declarator" => {
                        // Variable assignment: const foo = function() {}
                        if let Some(name_node) = parent.child_by_field_name("name") {
                            let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                            return std::str::from_utf8(name_bytes).ok();
                        }
                    }
                    _ => {}
                }
            }

            None
        }
        Lang::Python => {
            // Python functions have a "name" field
            if let Some(name_node) = node.child_by_field_name("name") {
                let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                return std::str::from_utf8(name_bytes).ok();
            }
            None
        }
        Lang::Java => {
            // Java methods have a "name" field
            if let Some(name_node) = node.child_by_field_name("name") {
                let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                return std::str::from_utf8(name_bytes).ok();
            }
            None
        }
        Lang::Cpp => {
            // C++ function names are in the declarator
            get_cpp_function_name(node, code)
        }
        _ => None,
    }
}

/// Extract function name from C++ function definition.
fn get_cpp_function_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    // C++ functions have a declarator field
    if let Some(declarator) = node.child_by_field_name("declarator") {
        // Look for function_declarator within the declarator
        if let Some(func_decl) = find_first_of_kind(&declarator, "function_declarator") {
            // The first child of function_declarator is usually the name
            if let Some(first_child) = func_decl.child(0) {
                match first_child.kind() {
                    "identifier" | "field_identifier" | "qualified_identifier"
                    | "destructor_name" | "operator_name" => {
                        let name_bytes = &code[first_child.start_byte()..first_child.end_byte()];
                        return std::str::from_utf8(name_bytes).ok();
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Find the first descendant node with the given kind.
fn find_first_of_kind<'a>(node: &Node<'a>, target_kind: &str) -> Option<Node<'a>> {
    if node.kind() == target_kind {
        return Some(*node);
    }

    for child in node.children() {
        if let Some(found) = find_first_of_kind(&child, target_kind) {
            return Some(found);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_function_detection() {
        let code = r#"
fn main() {
    println!("Hello, world!");
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

async fn fetch_data() -> Result<String> {
    Ok("data".to_string())
}
"#;

        let functions = detect_functions(code, Lang::Rust).unwrap();
        assert_eq!(functions.len(), 3);

        assert_eq!(functions[0].name, "main");
        assert_eq!(functions[0].start_line, 2);

        assert_eq!(functions[1].name, "add");
        assert_eq!(functions[2].name, "fetch_data");
    }

    #[test]
    fn test_typescript_function_detection() {
        let code = r#"
function greet(name: string): void {
    console.log(`Hello, ${name}!`);
}

const add = (a: number, b: number): number => {
    return a + b;
};

class Calculator {
    multiply(x: number, y: number): number {
        return x * y;
    }
}
"#;

        let functions = detect_functions(code, Lang::TypeScript).unwrap();
        assert!(functions.len() >= 2, "Expected at least 2 functions, got {}", functions.len());

        assert_eq!(functions[0].name, "greet");

        // Check that we found the add function
        let has_add = functions.iter().any(|f| f.name == "add");
        assert!(has_add, "Expected to find 'add' function");
    }

    #[test]
    fn test_python_function_detection() {
        let code = r#"
def hello():
    print("Hello, world!")

def add(a, b):
    return a + b

class Calculator:
    def multiply(self, x, y):
        return x * y
"#;

        let functions = detect_functions(code, Lang::Python).unwrap();
        assert_eq!(functions.len(), 3);

        assert_eq!(functions[0].name, "hello");
        assert_eq!(functions[1].name, "add");
        assert_eq!(functions[2].name, "multiply");
    }

    #[test]
    fn test_function_span_line_count() {
        let span = FunctionSpan::new("test".to_string(), 10, 20);
        assert_eq!(span.line_count(), 11);
    }

    #[test]
    fn test_function_span_contains_line() {
        let span = FunctionSpan::new("test".to_string(), 10, 20);
        assert!(span.contains_line(10));
        assert!(span.contains_line(15));
        assert!(span.contains_line(20));
        assert!(!span.contains_line(9));
        assert!(!span.contains_line(21));
    }

    #[test]
    fn test_empty_code() {
        let code = "";
        let functions = detect_functions(code, Lang::Rust).unwrap();
        assert_eq!(functions.len(), 0);
    }

    #[test]
    fn test_nested_functions() {
        let code = r#"
fn outer() {
    fn inner() {
        println!("nested");
    }
    inner();
}
"#;
        let functions = detect_functions(code, Lang::Rust).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "outer");
        assert_eq!(functions[1].name, "inner");
    }

    #[test]
    fn test_javascript_function_detection() {
        let code = r#"
function greet(name) {
    console.log(`Hello, ${name}!`);
}

const add = (a, b) => {
    return a + b;
};

class Calculator {
    multiply(x, y) {
        return x * y;
    }
}
"#;

        let functions = detect_functions(code, Lang::JavaScript).unwrap();
        assert!(functions.len() >= 2, "Expected at least 2 functions, got {}", functions.len());

        assert_eq!(functions[0].name, "greet");
    }

    #[test]
    fn test_multiple_languages() {
        // Test Rust
        let rust_code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let rust_funcs = detect_functions(rust_code, Lang::Rust).unwrap();
        assert_eq!(rust_funcs.len(), 1);
        assert_eq!(rust_funcs[0].name, "add");

        // Test Python
        let python_code = "def add(a, b):\n    return a + b";
        let python_funcs = detect_functions(python_code, Lang::Python).unwrap();
        assert_eq!(python_funcs.len(), 1);
        assert_eq!(python_funcs[0].name, "add");

        // Test TypeScript
        let ts_code = "function add(a: number, b: number): number { return a + b; }";
        let ts_funcs = detect_functions(ts_code, Lang::TypeScript).unwrap();
        assert_eq!(ts_funcs.len(), 1);
        assert_eq!(ts_funcs[0].name, "add");
    }

    #[test]
    fn test_function_span_serialization() {
        let span = FunctionSpan::new("test_func".to_string(), 10, 20);
        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("test_func"));
        assert!(json.contains("10"));
        assert!(json.contains("20"));

        let deserialized: FunctionSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, span);
    }
}
