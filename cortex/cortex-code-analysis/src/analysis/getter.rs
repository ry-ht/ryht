//! Node getter trait and implementations.
//!
//! This module provides the `NodeGetter` trait for extracting information
//! from AST nodes, including function names, space kinds, operator types,
//! and Halstead complexity classifications.

use crate::node::Node;
use crate::Lang;

use super::types::{HalsteadType, SpaceKind};

/// Node getter trait for extracting information from AST nodes.
///
/// Provides methods to extract metadata and classify nodes for
/// various code analysis purposes, including metrics computation.
pub trait NodeGetter {
    /// Get the function name from a function node.
    fn get_func_name<'a>(node: &Node<'a>, code: &'a [u8], lang: Lang) -> Option<&'a str> {
        Self::get_func_space_name(node, code, lang)
    }

    /// Get the name of a function space (function, class, impl, etc.).
    fn get_func_space_name<'a>(node: &Node<'a>, code: &'a [u8], lang: Lang) -> Option<&'a str>;

    /// Get the space kind (function, class, trait, etc.).
    fn get_space_kind(node: &Node, lang: Lang) -> SpaceKind;

    /// Get the Halstead operator/operand type.
    fn get_op_type(node: &Node, lang: Lang) -> HalsteadType;

    /// Get the operator string representation for a node kind ID.
    fn get_operator_id_as_str(id: u16, lang: Lang) -> &'static str;
}

/// Default implementation of NodeGetter.
pub struct DefaultNodeGetter;

impl NodeGetter for DefaultNodeGetter {
    fn get_func_space_name<'a>(node: &Node<'a>, code: &'a [u8], lang: Lang) -> Option<&'a str> {
        match lang {
            Lang::Rust => get_rust_func_space_name(node, code),
            Lang::Python => get_python_func_space_name(node, code),
            Lang::TypeScript | Lang::Tsx => get_typescript_func_space_name(node, code),
            Lang::JavaScript | Lang::Jsx => get_javascript_func_space_name(node, code),
            Lang::Java => get_java_func_space_name(node, code),
            Lang::Cpp => get_cpp_func_space_name(node, code),
            Lang::Kotlin => None,
        }
    }

    fn get_space_kind(node: &Node, lang: Lang) -> SpaceKind {
        match lang {
            Lang::Rust => get_rust_space_kind(node),
            Lang::Python => get_python_space_kind(node),
            Lang::TypeScript | Lang::Tsx => get_typescript_space_kind(node),
            Lang::JavaScript | Lang::Jsx => get_javascript_space_kind(node),
            Lang::Java => get_java_space_kind(node),
            Lang::Cpp => get_cpp_space_kind(node),
            Lang::Kotlin => SpaceKind::Unknown,
        }
    }

    fn get_op_type(node: &Node, lang: Lang) -> HalsteadType {
        match lang {
            Lang::Rust => get_rust_op_type(node),
            Lang::Python => get_python_op_type(node),
            Lang::TypeScript | Lang::Tsx => get_typescript_op_type(node),
            Lang::JavaScript | Lang::Jsx => get_javascript_op_type(node),
            Lang::Java => get_java_op_type(node),
            Lang::Cpp => get_cpp_op_type(node),
            Lang::Kotlin => HalsteadType::Unknown,
        }
    }

    fn get_operator_id_as_str(id: u16, lang: Lang) -> &'static str {
        match lang {
            Lang::Rust => get_rust_operator_str(id),
            Lang::Python => get_python_operator_str(id),
            Lang::TypeScript | Lang::Tsx => get_typescript_operator_str(id),
            Lang::JavaScript | Lang::Jsx => get_javascript_operator_str(id),
            Lang::Java => get_java_operator_str(id),
            Lang::Cpp => get_cpp_operator_str(id),
            Lang::Kotlin => "",
        }
    }
}

// ===== Rust implementations =====

fn get_rust_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    // For impl blocks, get the type name
    if let Some(name) = node
        .child_by_field_name("name")
        .or_else(|| node.child_by_field_name("type"))
    {
        let start = name.start_byte();
        let end = name.end_byte();
        if end <= code.len() {
            return std::str::from_utf8(&code[start..end]).ok();
        }
    }
    Some("<anonymous>")
}

fn get_rust_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "function_item" | "closure_expression" => SpaceKind::Function,
        "trait_item" => SpaceKind::Trait,
        "impl_item" => SpaceKind::Impl,
        "source_file" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_rust_op_type(node: &Node) -> HalsteadType {
    let kind = node.kind();

    // Special handling for || and / to avoid misclassification
    if kind == "||" || kind == "/" {
        if let Some(parent) = node.parent() {
            if parent.kind() == "binary_expression" {
                return HalsteadType::Operator;
            }
        }
        return HalsteadType::Unknown;
    }

    // Special handling for ! to avoid InnerDocCommentMarker
    if kind == "!" {
        if let Some(parent) = node.parent() {
            if parent.kind() != "inner_doc_comment" {
                return HalsteadType::Operator;
            }
        }
        return HalsteadType::Unknown;
    }

    match kind {
        "(" | "{" | "[" | "=>" | "+" | "*" | "async" | "await" | "continue" | "for" | "if"
        | "let" | "loop" | "match" | "return" | "unsafe" | "while" | "=" | "," | "->" | "?"
        | "<" | ">" | "&" | "mut" | ".." | "..=" | "-" | "&&" | "|" | "^" | "==" | "!="
        | "<=" | ">=" | "<<" | ">>" | "%" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|="
        | "^=" | "<<=" | ">>=" | "move" | "." | "primitive_type" | "fn" | ";" => {
            HalsteadType::Operator
        }
        "identifier" | "string_literal" | "raw_string_literal" | "integer_literal"
        | "float_literal" | "boolean_literal" | "self" | "char_literal" | "_" => {
            HalsteadType::Operand
        }
        _ => HalsteadType::Unknown,
    }
}

fn get_rust_operator_str(id: u16) -> &'static str {
    // Placeholder - would need tree-sitter grammar integration for proper mapping
    ""
}

// ===== Python implementations =====

fn get_python_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    if let Some(name) = node.child_by_field_name("name") {
        let start = name.start_byte();
        let end = name.end_byte();
        if end <= code.len() {
            return std::str::from_utf8(&code[start..end]).ok();
        }
    }
    Some("<anonymous>")
}

fn get_python_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "function_definition" => SpaceKind::Function,
        "class_definition" => SpaceKind::Class,
        "module" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_python_op_type(node: &Node) -> HalsteadType {
    let kind = node.kind();

    // Special handling for strings - check if it's a docstring
    if kind == "string" {
        if let Some(parent) = node.parent() {
            if parent.kind() == "expression_statement" && parent.child_count() == 1 {
                return HalsteadType::Unknown;
            }
        }
        return HalsteadType::Operand;
    }

    match kind {
        "import" | "." | "from" | "," | "as" | "*" | ">>" | "assert" | ":=" | "return" | "def"
        | "del" | "raise" | "pass" | "break" | "continue" | "if" | "elif" | "else" | "async"
        | "for" | "in" | "while" | "try" | "except" | "finally" | "with" | "->" | "=" | "global"
        | "exec" | "@" | "not" | "and" | "or" | "+" | "-" | "/" | "%" | "//" | "**" | "|"
        | "&" | "^" | "<<" | "~" | "<" | "<=" | "==" | "!=" | ">=" | ">" | "<>" | "is" | "+="
        | "-=" | "*=" | "/=" | "@=" | "//=" | "%=" | "**=" | ">>=" | "<<=" | "&=" | "^=" | "|="
        | "yield" | "await" | "print" => HalsteadType::Operator,
        "identifier" | "integer" | "float" | "true" | "false" | "none" => HalsteadType::Operand,
        _ => HalsteadType::Unknown,
    }
}

fn get_python_operator_str(_id: u16) -> &'static str {
    ""
}

// ===== TypeScript implementations =====

fn get_typescript_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    if let Some(name) = node.child_by_field_name("name") {
        let start = name.start_byte();
        let end = name.end_byte();
        if end <= code.len() {
            return std::str::from_utf8(&code[start..end]).ok();
        }
    } else {
        // Check for pair: foo: function() {} or variable declaration
        if let Some(parent) = node.parent() {
            match parent.kind() {
                "pair" => {
                    if let Some(name) = parent.child_by_field_name("key") {
                        let start = name.start_byte();
                        let end = name.end_byte();
                        if end <= code.len() {
                            return std::str::from_utf8(&code[start..end]).ok();
                        }
                    }
                }
                "variable_declarator" => {
                    if let Some(name) = parent.child_by_field_name("name") {
                        let start = name.start_byte();
                        let end = name.end_byte();
                        if end <= code.len() {
                            return std::str::from_utf8(&code[start..end]).ok();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Some("<anonymous>")
}

fn get_typescript_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "function_expression" | "method_definition" | "generator_function"
        | "function_declaration" | "generator_function_declaration" | "arrow_function" => {
            SpaceKind::Function
        }
        "class" | "class_declaration" => SpaceKind::Class,
        "interface_declaration" => SpaceKind::Interface,
        "program" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_typescript_op_type(node: &Node) -> HalsteadType {
    match node.kind() {
        "export" | "import" | "extends" | "." | "from" | "(" | "," | "as" | "*" | ">>" | ">>>"
        | ":" | "return" | "delete" | "throw" | "break" | "continue" | "if" | "else" | "switch"
        | "case" | "default" | "async" | "for" | "in" | "of" | "while" | "try" | "catch"
        | "finally" | "with" | "=" | "@" | "&&" | "||" | "+" | "-" | "--" | "++" | "/" | "%"
        | "**" | "|" | "&" | "<<" | "~" | "<" | "<=" | "==" | "!=" | ">=" | ">" | "+=" | "!"
        | "!==" | "===" | "-=" | "*=" | "/=" | "%=" | "**=" | ">>=" | ">>>=" | "<<=" | "&="
        | "^" | "^=" | "|=" | "yield" | "[" | "{" | "await" | "??" | "?" | "new" | "let"
        | "var" | "const" | "function" | ";" => HalsteadType::Operator,
        "identifier" | "member_expression" | "property_identifier" | "string" | "number"
        | "true" | "false" | "null" | "void" | "this" | "super" | "undefined" | "set" | "get"
        | "typeof" | "instanceof" => HalsteadType::Operand,
        _ => HalsteadType::Unknown,
    }
}

fn get_typescript_operator_str(_id: u16) -> &'static str {
    ""
}

// ===== JavaScript implementations =====

fn get_javascript_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    get_typescript_func_space_name(node, code)
}

fn get_javascript_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "function_expression" | "method_definition" | "generator_function"
        | "function_declaration" | "generator_function_declaration" | "arrow_function" => {
            SpaceKind::Function
        }
        "class" | "class_declaration" => SpaceKind::Class,
        "program" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_javascript_op_type(node: &Node) -> HalsteadType {
    get_typescript_op_type(node)
}

fn get_javascript_operator_str(_id: u16) -> &'static str {
    ""
}

// ===== Java implementations =====

fn get_java_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    if let Some(name) = node.child_by_field_name("name") {
        let start = name.start_byte();
        let end = name.end_byte();
        if end <= code.len() {
            return std::str::from_utf8(&code[start..end]).ok();
        }
    }
    Some("<anonymous>")
}

fn get_java_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "class_declaration" => SpaceKind::Class,
        "method_declaration" | "constructor_declaration" | "lambda_expression" => {
            SpaceKind::Function
        }
        "interface_declaration" => SpaceKind::Interface,
        "program" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_java_op_type(node: &Node) -> HalsteadType {
    match node.kind() {
        "if" | "else" | "switch" | "case" | "try" | "catch" | "throw" | "throws" | "for"
        | "while" | "continue" | "break" | "do" | "finally" | "new" | "return" | "default"
        | "abstract" | "assert" | "instanceof" | "extends" | "final" | "implements"
        | "transient" | "synchronized" | "super" | "this" | "void" | ";" | "," | "::" | "{"
        | "[" | "(" | "=" | "<" | ">" | "!" | "~" | "?" | ":" | "==" | "<=" | ">=" | "!="
        | "&&" | "||" | "++" | "--" | "+" | "-" | "*" | "/" | "&" | "|" | "^" | "%" | "<<"
        | ">>" | ">>>" | "+=" | "-=" | "*=" | "/=" | "&=" | "|=" | "^=" | "%=" | "<<=" | ">>="
        | ">>>=" | "int" | "float" => HalsteadType::Operator,
        "identifier" | "null_literal" | "string_literal" | "character_literal"
        | "decimal_integer_literal" | "hex_integer_literal" | "octal_integer_literal"
        | "binary_integer_literal" | "decimal_floating_point_literal"
        | "hex_floating_point_literal" => HalsteadType::Operand,
        _ => HalsteadType::Unknown,
    }
}

fn get_java_operator_str(_id: u16) -> &'static str {
    ""
}

// ===== C++ implementations =====

fn get_cpp_func_space_name<'a>(node: &Node<'a>, code: &'a [u8]) -> Option<&'a str> {
    match node.kind() {
        "function_definition" => {
            // Check for operator cast
            if let Some(op_cast) = node.children().find(|n| n.kind() == "operator_cast") {
                let start = op_cast.start_byte();
                let end = op_cast.end_byte();
                if end <= code.len() {
                    return std::str::from_utf8(&code[start..end]).ok();
                }
            }

            // Get the declarator
            if let Some(declarator) = node.child_by_field_name("declarator") {
                // Find function_declarator
                if let Some(fd) = find_function_declarator(&declarator) {
                    if let Some(first) = fd.child(0) {
                        match first.kind() {
                            "type_identifier" | "identifier" | "field_identifier"
                            | "destructor_name" | "operator_name" | "qualified_identifier"
                            | "template_function" | "template_method" => {
                                let start = first.start_byte();
                                let end = first.end_byte();
                                if end <= code.len() {
                                    return std::str::from_utf8(&code[start..end]).ok();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {
            if let Some(name) = node.child_by_field_name("name") {
                let start = name.start_byte();
                let end = name.end_byte();
                if end <= code.len() {
                    return std::str::from_utf8(&code[start..end]).ok();
                }
            }
        }
    }
    None
}

fn find_function_declarator<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "function_declarator" {
        return Some(*node);
    }
    for child in node.children() {
        if let Some(found) = find_function_declarator(&child) {
            return Some(found);
        }
    }
    None
}

fn get_cpp_space_kind(node: &Node) -> SpaceKind {
    match node.kind() {
        "function_definition" => SpaceKind::Function,
        "struct_specifier" => SpaceKind::Struct,
        "class_specifier" => SpaceKind::Class,
        "namespace_definition" => SpaceKind::Namespace,
        "translation_unit" => SpaceKind::Unit,
        _ => SpaceKind::Unknown,
    }
}

fn get_cpp_op_type(node: &Node) -> HalsteadType {
    match node.kind() {
        "." | "(" | "," | "*" | ">>" | ":" | ";" | "return" | "break" | "continue" | "if"
        | "else" | "switch" | "case" | "default" | "for" | "while" | "goto" | "do" | "delete"
        | "new" | "try" | "catch" | "throw" | "=" | "&&" | "||" | "-" | "--" | "->" | "+"
        | "++" | "/" | "%" | "|" | "&" | "<<" | "~" | "<" | "<=" | "==" | "!=" | ">=" | ">"
        | "+=" | "!" | "*=" | "/=" | "%=" | ">>=" | "<<=" | "&=" | "^" | "^=" | "|=" | "["
        | "{" | "?" | "::" | "primitive_type" | "type_specifier" | "sizeof" => {
            HalsteadType::Operator
        }
        "identifier" | "type_identifier" | "field_identifier" | "raw_string_literal"
        | "string_literal" | "number_literal" | "true" | "false" | "null" | "..." => {
            HalsteadType::Operand
        }
        "namespace_identifier" => {
            // Only count as operand in namespace definitions
            if let Some(parent) = node.parent() {
                if parent.kind() == "namespace_definition" {
                    return HalsteadType::Operand;
                }
            }
            HalsteadType::Unknown
        }
        _ => HalsteadType::Unknown,
    }
}

fn get_cpp_operator_str(_id: u16) -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_getter_trait_exists() {
        fn _assert_impl<T: NodeGetter>() {}
        _assert_impl::<DefaultNodeGetter>();
    }

    #[test]
    fn test_space_kind_mapping() {
        // Would require actual tree-sitter nodes for proper testing
        // These are placeholders for integration tests
    }

    #[test]
    fn test_halstead_type_classification() {
        // Would require actual tree-sitter nodes for proper testing
        // These are placeholders for integration tests
    }
}
