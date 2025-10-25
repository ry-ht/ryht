//! Node checking trait and implementations.
//!
//! This module provides the `NodeChecker` trait for analyzing AST nodes
//! and determining their properties (e.g., is it a comment, function, closure, etc.).
//! Language-specific implementations provide precise classification for each supported language.

use crate::node::Node;
use crate::Lang;

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
            Lang::Kotlin => false, // Not implemented in original
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
    // This requires knowing the tree-sitter node type IDs
    // We'll use kind string comparison as a fallback
    false // Will be refined with proper language grammar integration
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
        if let Ok(text) = std::str::from_utf8(slice) {
            return text.contains("coding:") || text.contains("coding=");
        }
    }
    false
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
    check_if_ts_func(node)
}

fn is_typescript_closure(node: &Node) -> bool {
    check_if_ts_closure(node)
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
    // Will be refined with proper grammar integration
    false
}

// Helper for TypeScript/JavaScript function detection
fn check_if_ts_func(node: &Node) -> bool {
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
                || node.has_sibling_kind("property_identifier")
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
                || node.has_sibling_kind("property_identifier")
        }
        _ => false,
    }
}

fn check_if_ts_closure(node: &Node) -> bool {
    match node.kind() {
        "generator_function" | "generator_function_declaration" => true,
        "function_expression" => !check_if_ts_func(node),
        "arrow_function" => !check_if_ts_func(node),
        _ => false,
    }
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
    check_if_ts_func(node)
}

fn is_javascript_closure(node: &Node) -> bool {
    check_if_ts_closure(node)
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
        slice.windows(15).any(|w| w == b"<div rustbindgen")
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
    // Will be refined with proper grammar integration
    false
}

// Helper trait extension for Node
trait NodeExt {
    fn has_sibling_kind(&self, kind: &str) -> bool;
}

impl<'a> NodeExt for Node<'a> {
    fn has_sibling_kind(&self, kind: &str) -> bool {
        if let Some(parent) = self.parent() {
            parent.children().any(|child| child.kind() == kind)
        } else {
            false
        }
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
    fn test_rust_comment_detection() {
        // These would require actual tree-sitter nodes to test properly
        // Placeholder for integration tests
    }
}
