//! Function detection and boundary analysis.
//!
//! This module provides comprehensive functionality to detect functions in source code and
//! determine their boundaries (start and end lines). It supports all languages in the cortex
//! ecosystem with advanced detection logic for language-specific patterns.
//!
//! # Features
//!
//! - Detects functions, methods, constructors, and lambdas
//! - Handles nested functions and closures
//! - Language-specific detection patterns
//! - Error reporting for ambiguous cases
//! - Support for anonymous functions
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
    CppLanguage, JavaLanguage, KotlinLanguage, TsxLanguage,
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

    /// If `true`, an error occurred in determining the span or name of the function.
    ///
    /// This can happen in ambiguous cases where the function name cannot be
    /// reliably extracted from the AST, such as:
    /// - Malformed code with parse errors
    /// - Complex macro-generated functions
    /// - Unusual language constructs
    #[serde(default)]
    pub error: bool,
}

impl FunctionSpan {
    /// Create a new function span.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function
    /// * `start_line` - The first line of the function (1-indexed)
    /// * `end_line` - The last line of the function (1-indexed)
    pub fn new(name: String, start_line: usize, end_line: usize) -> Self {
        Self {
            name,
            start_line,
            end_line,
            error: false,
        }
    }

    /// Create a new function span with an error flag.
    ///
    /// This is used when the function name cannot be reliably determined.
    pub fn new_with_error(name: String, start_line: usize, end_line: usize) -> Self {
        Self {
            name,
            start_line,
            end_line,
            error: true,
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

    /// Check if this function span has an error.
    pub fn has_error(&self) -> bool {
        self.error
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
        Lang::TypeScript => detect_functions_impl::<TypeScriptLanguage>(code),
        Lang::Tsx => detect_functions_impl::<TsxLanguage>(code),
        Lang::JavaScript => detect_functions_impl::<JavaScriptLanguage>(code),
        Lang::Jsx => detect_functions_impl::<JavaScriptLanguage>(code),
        Lang::Python => detect_functions_impl::<PythonLanguage>(code),
        Lang::Java => detect_functions_impl::<JavaLanguage>(code),
        Lang::Cpp => detect_functions_impl::<CppLanguage>(code),
        Lang::Kotlin => detect_functions_impl::<KotlinLanguage>(code),
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
                    error: false,
                });
            } else {
                // Function without a clear name - mark as error
                spans.push(FunctionSpan {
                    name: "".to_string(),
                    start_line,
                    end_line,
                    error: true,
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
            // TypeScript/TSX function detection
            match kind {
                "function_declaration" | "method_definition" => true,
                "function_expression" | "arrow_function" => {
                    // Check if this is a named function (not a closure/callback)
                    is_named_function_typescript(node)
                }
                "generator_function" | "generator_function_declaration" => true,
                _ => false,
            }
        }
        Lang::JavaScript | Lang::Jsx => {
            // JavaScript function detection
            match kind {
                "function_declaration" | "method_definition" => true,
                "function_expression" | "arrow_function" => {
                    // Check if this is a named function (not a closure/callback)
                    is_named_function_javascript(node)
                }
                "generator_function" | "generator_function_declaration" => true,
                _ => false,
            }
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
        Lang::Kotlin => {
            kind == "function_declaration"
        }
    }
}

/// Check if a TypeScript node is a named function (not a closure/callback).
///
/// This function implements sophisticated heuristics to distinguish between
/// actual function definitions and inline callbacks/closures.
fn is_named_function_typescript(node: &Node) -> bool {
    let kind = node.kind();

    match kind {
        "function_declaration" | "method_definition" => true,
        "function_expression" => {
            // Check if this is assigned to a variable or property
            // by looking for specific ancestor patterns
            count_specific_ancestors(
                node,
                |n| matches!(
                    n.kind(),
                    "variable_declarator" | "assignment_expression" | "labeled_statement" | "pair"
                ),
                |n| matches!(
                    n.kind(),
                    "statement_block" | "return_statement" | "new_expression" | "arguments"
                ),
            ) > 0 || node.has_child_of_kind("identifier")
        }
        "arrow_function" => {
            // Arrow functions are functions if they're assigned to a variable
            count_specific_ancestors(
                node,
                |n| matches!(
                    n.kind(),
                    "variable_declarator" | "assignment_expression" | "labeled_statement"
                ),
                |n| matches!(
                    n.kind(),
                    "statement_block" | "return_statement" | "new_expression" | "call_expression"
                ),
            ) > 0 || has_sibling_of_kind(node, "property_identifier")
        }
        _ => false,
    }
}

/// Check if a JavaScript node is a named function (not a closure/callback).
fn is_named_function_javascript(node: &Node) -> bool {
    // Use the same logic as TypeScript for now
    is_named_function_typescript(node)
}

/// Helper to count specific ancestors matching a predicate.
///
/// Walks up the AST from the given node, counting ancestors that match
/// the `check` predicate. Stops when a `stop` predicate matches.
///
/// # Arguments
///
/// * `node` - The starting node
/// * `check` - Predicate to count matching ancestors
/// * `stop` - Predicate to stop traversal
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

/// Node helper methods for function detection.
impl Node<'_> {
    /// Helper method to check if node has children of a specific kind.
    fn has_child_of_kind(&self, kind: &str) -> bool {
        for child in self.children() {
            if child.kind() == kind {
                return true;
            }
        }
        false
    }
}

/// Extract the function name from a node.
///
/// This function handles language-specific naming patterns including:
/// - Field-based naming (Rust, Python, Java)
/// - Parent-based naming (JS/TS function expressions in assignments)
/// - Declarator-based naming (C++)
/// - Special cases for constructors, operators, etc.
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
        Lang::Kotlin => {
            // Kotlin functions have a "simple_identifier" child
            if let Some(name_node) = node.child_by_field_name("name") {
                let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
                return std::str::from_utf8(name_bytes).ok();
            }
            // Try to find simple_identifier as a child
            for child in node.children() {
                if child.kind() == "simple_identifier" {
                    let name_bytes = &code[child.start_byte()..child.end_byte()];
                    return std::str::from_utf8(name_bytes).ok();
                }
            }
            None
        }
    }
}

/// Extract function name from C++ function definition.
///
/// C++ function names can appear in various forms:
/// - Simple identifiers
/// - Qualified identifiers (namespace::function)
/// - Operator overloads
/// - Destructors
/// - Template functions
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
        assert!(!functions[0].error);

        assert_eq!(functions[1].name, "add");
        assert!(!functions[1].error);

        assert_eq!(functions[2].name, "fetch_data");
        assert!(!functions[2].error);
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
        assert!(!functions[0].error);

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
        assert!(!functions[0].error);

        assert_eq!(functions[1].name, "add");
        assert!(!functions[1].error);

        assert_eq!(functions[2].name, "multiply");
        assert!(!functions[2].error);
    }

    #[test]
    fn test_java_function_detection() {
        let code = r#"
public class Example {
    public void greet() {
        System.out.println("Hello!");
    }

    public int add(int a, int b) {
        return a + b;
    }

    public Example() {
        // Constructor
    }
}
"#;

        let functions = detect_functions(code, Lang::Java).unwrap();
        assert_eq!(functions.len(), 3);

        assert_eq!(functions[0].name, "greet");
        assert!(!functions[0].error);

        assert_eq!(functions[1].name, "add");
        assert!(!functions[1].error);

        // Constructor
        assert_eq!(functions[2].name, "Example");
        assert!(!functions[2].error);
    }

    #[test]
    fn test_cpp_function_detection() {
        let code = r#"
void hello() {
    std::cout << "Hello" << std::endl;
}

int add(int a, int b) {
    return a + b;
}

namespace Math {
    double multiply(double x, double y) {
        return x * y;
    }
}
"#;

        let functions = detect_functions(code, Lang::Cpp).unwrap();
        assert_eq!(functions.len(), 3);

        assert_eq!(functions[0].name, "hello");
        assert!(!functions[0].error);

        assert_eq!(functions[1].name, "add");
        assert!(!functions[1].error);

        assert_eq!(functions[2].name, "multiply");
        assert!(!functions[2].error);
    }

    #[test]
    fn test_kotlin_function_detection() {
        let code = r#"
fun greet() {
    println("Hello!")
}

fun add(a: Int, b: Int): Int {
    return a + b
}

class Calculator {
    fun multiply(x: Int, y: Int): Int {
        return x * y
    }
}
"#;

        let functions = detect_functions(code, Lang::Kotlin).unwrap();
        assert!(functions.len() >= 2, "Expected at least 2 functions, got {}", functions.len());

        assert_eq!(functions[0].name, "greet");
        assert!(!functions[0].error);

        assert_eq!(functions[1].name, "add");
        assert!(!functions[1].error);
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
    fn test_function_span_with_error() {
        let span = FunctionSpan::new_with_error("".to_string(), 10, 20);
        assert!(span.error);
        assert!(span.has_error());
        assert_eq!(span.name, "");
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
        assert!(!functions[0].error);
        assert!(!functions[1].error);
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
        assert!(!functions[0].error);
    }

    #[test]
    fn test_multiple_languages() {
        // Test Rust
        let rust_code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let rust_funcs = detect_functions(rust_code, Lang::Rust).unwrap();
        assert_eq!(rust_funcs.len(), 1);
        assert_eq!(rust_funcs[0].name, "add");
        assert!(!rust_funcs[0].error);

        // Test Python
        let python_code = "def add(a, b):\n    return a + b";
        let python_funcs = detect_functions(python_code, Lang::Python).unwrap();
        assert_eq!(python_funcs.len(), 1);
        assert_eq!(python_funcs[0].name, "add");
        assert!(!python_funcs[0].error);

        // Test TypeScript
        let ts_code = "function add(a: number, b: number): number { return a + b; }";
        let ts_funcs = detect_functions(ts_code, Lang::TypeScript).unwrap();
        assert_eq!(ts_funcs.len(), 1);
        assert_eq!(ts_funcs[0].name, "add");
        assert!(!ts_funcs[0].error);
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

    #[test]
    fn test_function_with_complex_names() {
        // Test C++ namespace qualified names
        let cpp_code = r#"
namespace ns {
    void qualified_func() {}
}
"#;
        let cpp_funcs = detect_functions(cpp_code, Lang::Cpp).unwrap();
        assert_eq!(cpp_funcs.len(), 1);
        assert!(!cpp_funcs[0].error);
    }

    #[test]
    fn test_closures_not_detected_as_functions() {
        // JavaScript inline callbacks should not be detected
        let code = r#"
function outer() {
    // This arrow function is a callback, not a function
    arr.map(x => x * 2);
}
"#;
        let functions = detect_functions(code, Lang::JavaScript).unwrap();
        // Should only detect the outer function, not the arrow function callback
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "outer");
    }

    #[test]
    fn test_async_functions() {
        let code = r#"
async function fetchData() {
    return await fetch('/api/data');
}

const processData = async () => {
    return await fetchData();
};
"#;
        let functions = detect_functions(code, Lang::JavaScript).unwrap();
        assert!(functions.len() >= 1);
        assert_eq!(functions[0].name, "fetchData");
    }

    #[test]
    fn test_generator_functions() {
        let code = r#"
function* fibonacci() {
    yield 1;
    yield 2;
}

const gen = function* generator() {
    yield 'a';
};
"#;
        let functions = detect_functions(code, Lang::JavaScript).unwrap();
        assert!(functions.len() >= 1);
        assert_eq!(functions[0].name, "fibonacci");
    }

    #[test]
    fn test_method_definitions() {
        let code = r#"
const obj = {
    method1() {
        return 1;
    },
    method2: function() {
        return 2;
    }
};
"#;
        let functions = detect_functions(code, Lang::JavaScript).unwrap();
        // Both method definitions should be detected
        assert!(functions.len() >= 2);
    }

    #[test]
    fn test_python_nested_functions() {
        let code = r#"
def outer():
    def inner():
        pass
    return inner
"#;
        let functions = detect_functions(code, Lang::Python).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "outer");
        assert_eq!(functions[1].name, "inner");
    }

    #[test]
    fn test_tsx_components_as_functions() {
        let code = r#"
function Component() {
    return <div>Hello</div>;
}

const ArrowComponent = () => {
    return <div>World</div>;
};
"#;
        let functions = detect_functions(code, Lang::Tsx).unwrap();
        assert!(functions.len() >= 1);
        assert_eq!(functions[0].name, "Component");
    }
}
