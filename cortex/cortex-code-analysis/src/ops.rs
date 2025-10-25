//! Operator and Operand Extraction Module
//!
//! This module provides functionality for extracting operators and operands from source code,
//! which is essential for computing Halstead complexity metrics. It supports nested scopes
//! and multiple programming languages.
//!
//! # Overview
//!
//! The module works by parsing source code using tree-sitter and traversing the AST to identify
//! operators (keywords, operators, function calls) and operands (variables, literals, identifiers).
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{Lang, extract_ops};
//!
//! let code = r#"
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//! "#;
//!
//! let ops = extract_ops(code, Lang::Rust).unwrap();
//! assert!(ops.operators.contains(&"fn".to_string()));
//! assert!(ops.operands.contains(&"add".to_string()));
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::{Node as TSNode, Parser as TSParser};

use crate::lang::Lang;

/// All operators and operands extracted from a code space.
///
/// This structure represents the Halstead elements in a piece of code,
/// including nested function spaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ops {
    /// The name of the space (function, file, etc.)
    pub name: Option<String>,

    /// The first line of the space
    pub start_line: usize,

    /// The last line of the space
    pub end_line: usize,

    /// The kind of space (function, class, unit, etc.)
    pub kind: SpaceKind,

    /// All nested subspaces contained in this space
    pub spaces: Vec<Ops>,

    /// All operands (variables, literals, identifiers) in this space
    pub operands: Vec<String>,

    /// All operators (keywords, operators, function calls) in this space
    pub operators: Vec<String>,
}

/// The type of code space being analyzed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceKind {
    /// An unknown space type
    Unknown,
    /// A function space
    Function,
    /// A class space
    Class,
    /// A struct space
    Struct,
    /// A trait space (Rust)
    Trait,
    /// An implementation space (Rust)
    Impl,
    /// The entire file/module (unit space)
    Unit,
    /// A namespace (C++, etc.)
    Namespace,
    /// An interface (TypeScript, Java, etc.)
    Interface,
}

impl Default for SpaceKind {
    fn default() -> Self {
        SpaceKind::Unknown
    }
}

impl Ops {
    /// Creates a new Ops instance for a given space
    fn new(name: Option<String>, start_line: usize, end_line: usize, kind: SpaceKind) -> Self {
        Self {
            name,
            start_line,
            end_line,
            kind,
            spaces: Vec::new(),
            operands: Vec::new(),
            operators: Vec::new(),
        }
    }

    /// Merges operators and operands from another Ops instance
    fn merge_ops(&mut self, other: &Ops) {
        self.operands.extend_from_slice(&other.operands);
        self.operators.extend_from_slice(&other.operators);
    }
}

/// Internal state for tracking operators and operands during traversal
#[derive(Debug)]
struct OpsCollector {
    ops: Ops,
    operators: HashMap<String, u64>,
    operands: HashMap<Vec<u8>, u64>,
}

impl OpsCollector {
    fn new(name: Option<String>, start_line: usize, end_line: usize, kind: SpaceKind) -> Self {
        Self {
            ops: Ops::new(name, start_line, end_line, kind),
            operators: HashMap::new(),
            operands: HashMap::new(),
        }
    }

    /// Finalizes the collection by converting maps to vectors
    fn finalize(&mut self, lang: Lang) {
        // Convert operators map to vector
        self.ops.operators = self
            .operators
            .keys()
            .filter(|k| !is_primitive_operator(k, lang))
            .cloned()
            .collect();

        // Convert operands map to vector
        self.ops.operands = self
            .operands
            .keys()
            .filter_map(|k| String::from_utf8(k.clone()).ok())
            .collect();
    }
}

/// Extracts all operators and operands from source code.
///
/// This function parses the code using tree-sitter and traverses the AST
/// to identify operators (keywords, operators, symbols) and operands
/// (identifiers, literals, values).
///
/// # Arguments
///
/// * `code` - The source code to analyze
/// * `lang` - The programming language of the source code
///
/// # Returns
///
/// Returns `Ok(Ops)` containing all extracted operators and operands,
/// organized by scope. Returns an error if parsing fails.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Lang, extract_ops};
///
/// let code = "let x = 5 + 3;";
/// let ops = extract_ops(code, Lang::Rust).unwrap();
/// ```
pub fn extract_ops(code: &str, lang: Lang) -> Result<Ops> {
    let mut parser = TSParser::new();
    parser
        .set_language(&lang.get_ts_language())
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(code, None)
        .context("Failed to parse source code")?;

    let root = tree.root_node();
    let code_bytes = code.as_bytes();

    // Initialize state stack for nested scopes
    let mut state_stack: Vec<OpsCollector> = Vec::new();
    let mut stack: Vec<(TSNode, usize)> = Vec::new();
    let mut last_level = 0;

    // Start with the root node at level 0
    stack.push((root, 0));

    // Create the unit-level collector
    let unit_collector = OpsCollector::new(
        None,
        if root.child_count() == 0 {
            0
        } else {
            root.start_position().row + 1
        },
        root.end_position().row,
        SpaceKind::Unit,
    );
    state_stack.push(unit_collector);

    while let Some((node, level)) = stack.pop() {
        // Finalize completed scopes
        if level < last_level {
            finalize_ops(&mut state_stack, last_level - level, lang);
            last_level = level;
        }

        let kind = get_space_kind(&node, lang);
        // Only create a new scope if it's a function, not if it's the top-level unit
        let is_func_space = (is_function(&node, lang) || is_function_space(&node, lang)) && kind != SpaceKind::Unit;

        let new_level = if is_func_space {
            let name = get_function_name(&node, code_bytes, lang);
            let start_line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;

            let collector = OpsCollector::new(name, start_line, end_line, kind);
            state_stack.push(collector);
            last_level = level + 1;
            last_level
        } else {
            level
        };

        // Process current node for operators and operands
        if let Some(collector) = state_stack.last_mut() {
            process_node(&node, code_bytes, collector, lang);
        }

        // Add children to stack in reverse order (for depth-first traversal)
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            let mut children = Vec::new();
            loop {
                children.push((cursor.node(), new_level));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
    }

    // Finalize all remaining scopes
    finalize_ops(&mut state_stack, usize::MAX, lang);

    state_stack
        .pop()
        .map(|mut collector| {
            collector.finalize(lang);
            collector.ops
        })
        .context("No ops collected")
}

/// Processes a single AST node to extract operators and operands
fn process_node(node: &TSNode, code: &[u8], collector: &mut OpsCollector, lang: Lang) {
    let kind = node.kind();
    let node_id = node.kind_id();

    // Check if this is an operator or operand
    match get_op_type(node, lang) {
        OpType::Operator => {
            let op_str = get_operator_string(node_id, kind, lang);
            *collector.operators.entry(op_str).or_insert(0) += 1;
        }
        OpType::Operand => {
            let operand = &code[node.start_byte()..node.end_byte()];
            *collector.operands.entry(operand.to_vec()).or_insert(0) += 1;
        }
        OpType::Unknown => {}
    }

    // Handle primitive types separately
    if is_primitive_type(node_id, lang) {
        let operand = &code[node.start_byte()..node.end_byte()];
        if let Ok(type_str) = std::str::from_utf8(operand) {
            *collector.operators.entry(type_str.to_string()).or_insert(0) += 1;
        }
    }
}

/// Finalizes completed scopes by merging them into parent scopes
fn finalize_ops(state_stack: &mut Vec<OpsCollector>, diff_level: usize, lang: Lang) {
    if state_stack.is_empty() {
        return;
    }

    for _ in 0..diff_level {
        if state_stack.len() == 1 {
            let last_state = state_stack.last_mut().unwrap();
            last_state.finalize(lang);
            break;
        } else {
            let mut state = state_stack.pop().unwrap();
            state.finalize(lang);

            let last_state = state_stack.last_mut().unwrap();

            // Merge hash maps before finalizing parent
            for (k, v) in state.operators.iter() {
                *last_state.operators.entry(k.clone()).or_insert(0) += v;
            }
            for (k, v) in state.operands.iter() {
                *last_state.operands.entry(k.clone()).or_insert(0) += v;
            }

            // Add child's ops as a subspace
            last_state.ops.spaces.push(state.ops);
        }
    }
}

/// Type of AST node for Halstead metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpType {
    Operator,
    Operand,
    Unknown,
}

/// Determines if a node is an operator, operand, or unknown
fn get_op_type(node: &TSNode, lang: Lang) -> OpType {
    let kind = node.kind();

    match lang {
        Lang::Rust => rust_op_type(kind, node),
        Lang::Python => python_op_type(kind, node),
        Lang::TypeScript | Lang::Tsx => typescript_op_type(kind, node),
        Lang::JavaScript | Lang::Jsx => javascript_op_type(kind, node),
        Lang::Cpp => cpp_op_type(kind, node),
        Lang::Java => java_op_type(kind, node),
        _ => OpType::Unknown,
    }
}

/// Gets the string representation of an operator
fn get_operator_string(_node_id: u16, kind: &str, _lang: Lang) -> String {
    kind.to_string()
}

/// Checks if a node kind represents a function
fn is_function(node: &TSNode, lang: Lang) -> bool {
    let kind = node.kind();
    match lang {
        Lang::Rust => kind == "function_item",
        Lang::Python => kind == "function_definition",
        Lang::TypeScript | Lang::Tsx => {
            matches!(kind, "function_declaration" | "method_definition")
        }
        Lang::JavaScript | Lang::Jsx => {
            matches!(kind, "function_declaration" | "method_definition")
        }
        Lang::Cpp => matches!(kind, "function_definition"),
        Lang::Java => matches!(kind, "method_declaration" | "constructor_declaration"),
        _ => false,
    }
}

/// Checks if a node represents a function space (including closures, etc.)
fn is_function_space(node: &TSNode, lang: Lang) -> bool {
    let kind = node.kind();
    match lang {
        Lang::Rust => matches!(
            kind,
            "source_file" | "function_item" | "impl_item" | "trait_item" | "closure_expression"
        ),
        Lang::Python => matches!(kind, "module" | "function_definition" | "class_definition"),
        Lang::TypeScript | Lang::Tsx => matches!(
            kind,
            "program"
                | "function_expression"
                | "class"
                | "function_declaration"
                | "method_definition"
                | "class_declaration"
                | "interface_declaration"
                | "arrow_function"
        ),
        Lang::JavaScript | Lang::Jsx => matches!(
            kind,
            "program"
                | "function_expression"
                | "class"
                | "function_declaration"
                | "method_definition"
                | "class_declaration"
                | "arrow_function"
        ),
        Lang::Cpp => matches!(
            kind,
            "translation_unit"
                | "function_definition"
                | "struct_specifier"
                | "class_specifier"
                | "namespace_definition"
        ),
        Lang::Java => matches!(kind, "program" | "class_declaration" | "interface_declaration"),
        _ => false,
    }
}

/// Gets the space kind for a node
fn get_space_kind(node: &TSNode, lang: Lang) -> SpaceKind {
    let kind = node.kind();
    match lang {
        Lang::Rust => match kind {
            "function_item" => SpaceKind::Function,
            "impl_item" => SpaceKind::Impl,
            "trait_item" => SpaceKind::Trait,
            "source_file" => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        },
        Lang::Python => match kind {
            "function_definition" => SpaceKind::Function,
            "class_definition" => SpaceKind::Class,
            "module" => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        },
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => match kind {
            "function_declaration" | "function_expression" | "arrow_function" => {
                SpaceKind::Function
            }
            "class_declaration" | "class" => SpaceKind::Class,
            "interface_declaration" => SpaceKind::Interface,
            "program" => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        },
        Lang::Cpp => match kind {
            "function_definition" => SpaceKind::Function,
            "struct_specifier" => SpaceKind::Struct,
            "class_specifier" => SpaceKind::Class,
            "namespace_definition" => SpaceKind::Namespace,
            "translation_unit" => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        },
        Lang::Java => match kind {
            "method_declaration" | "constructor_declaration" => SpaceKind::Function,
            "class_declaration" => SpaceKind::Class,
            "interface_declaration" => SpaceKind::Interface,
            "program" => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        },
        _ => SpaceKind::Unknown,
    }
}

/// Extracts the function name from a node
fn get_function_name(node: &TSNode, code: &[u8], _lang: Lang) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        let name_bytes = &code[name_node.start_byte()..name_node.end_byte()];
        std::str::from_utf8(name_bytes).ok().map(|s| s.to_string())
    } else {
        Some("<anonymous>".to_string())
    }
}

/// Checks if a node represents a primitive type
fn is_primitive_type(_node_id: u16, _lang: Lang) -> bool {
    // This is language-specific and would need tree-sitter node IDs
    // For now, we'll return false and handle this in a more robust way
    false
}

/// Checks if an operator string is a primitive operator
fn is_primitive_operator(op: &str, _lang: Lang) -> bool {
    // Primitive operators that should be filtered out
    matches!(op, "(" | ")" | "[" | "]" | "{" | "}" | "," | ";" | ":")
}

// Language-specific operator/operand type detection

fn rust_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators
        "fn" | "let" | "mut" | "if" | "else" | "match" | "for" | "while" | "loop"
        | "return" | "break" | "continue" | "impl" | "trait" | "struct" | "enum" | "use"
        | "pub" | "mod" | "const" | "static" => OpType::Operator,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||"
        | "!" | "&" | "|" | "^" | "<<" | ">>" | "=" | "+=" | "-=" | "*=" | "/=" | "%="
        | "&=" | "|=" | "^=" | "<<=" | ">>=" | "." | "::" | "->" | "=>" | "?" | "as" => {
            OpType::Operator
        }
        "(" | "[" | "{" | "}" | "]" | ")" | "," | ";" | ":" => OpType::Operator,

        // Operands
        "identifier" | "integer_literal" | "float_literal" | "string_literal"
        | "raw_string_literal" | "boolean_literal" | "char_literal" => OpType::Operand,

        _ => OpType::Unknown,
    }
}

fn python_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators
        "import" | "from" | "as" | "def" | "class" | "if" | "elif" | "else" | "for" | "while"
        | "return" | "break" | "continue" | "pass" | "raise" | "try" | "except" | "finally"
        | "with" | "assert" | "del" | "global" | "nonlocal" | "lambda" | "yield" | "await"
        | "async" | "print" => OpType::Operator,
        "+" | "-" | "*" | "/" | "//" | "%" | "**" | "==" | "!=" | "<" | ">" | "<=" | ">="
        | "and" | "or" | "not" | "is" | "in" | "&" | "|" | "^" | "~" | "<<" | ">>" | "="
        | "+=" | "-=" | "*=" | "/=" | "//=" | "%=" | "**=" | "&=" | "|=" | "^=" | "<<="
        | ">>=" | "." | "," | ":" | ";" => OpType::Operator,

        // Operands
        "identifier" | "integer" | "float" | "string" | "true" | "false" | "none" => {
            OpType::Operand
        }

        _ => OpType::Unknown,
    }
}

fn typescript_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators
        "function" | "const" | "let" | "var" | "if" | "else" | "for" | "while" | "do"
        | "switch" | "case" | "default" | "return" | "break" | "continue" | "throw" | "try"
        | "catch" | "finally" | "class" | "interface" | "type" | "enum" | "import" | "export"
        | "async" | "await" | "yield" | "new" | "delete" | "typeof" | "instanceof" | "in"
        | "void" | "as" => OpType::Operator,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "===" | "!==" | "<" | ">" | "<=" | ">="
        | "&&" | "||" | "!" | "&" | "|" | "^" | "<<" | ">>" | ">>>" | "=" | "+=" | "-="
        | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>=" | "." | "?."
        | "=>" | "?" | ":" | "," | ";" => OpType::Operator,
        "(" | "[" | "{" | "}" | "]" | ")" => OpType::Operator,

        // Operands - simplified to avoid unreachable patterns
        "identifier" | "number" | "string" | "template_string" | "true" | "false" | "null"
        | "undefined" | "this" | "super" => OpType::Operand,

        _ => OpType::Unknown,
    }
}

fn javascript_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators (similar to TypeScript but without type-specific keywords)
        "function" | "const" | "let" | "var" | "if" | "else" | "for" | "while" | "do"
        | "switch" | "case" | "default" | "return" | "break" | "continue" | "throw" | "try"
        | "catch" | "finally" | "class" | "import" | "export" | "async" | "await" | "yield"
        | "new" | "delete" | "typeof" | "instanceof" | "in" | "void" => OpType::Operator,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "===" | "!==" | "<" | ">" | "<=" | ">="
        | "&&" | "||" | "!" | "&" | "|" | "^" | "<<" | ">>" | ">>>" | "=" | "+=" | "-="
        | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>=" | "." | "?."
        | "=>" | "?" | ":" | "," | ";" => OpType::Operator,
        "(" | "[" | "{" | "}" | "]" | ")" => OpType::Operator,

        // Operands
        "identifier" | "number" | "string" | "template_string" | "true" | "false" | "null"
        | "undefined" | "this" | "super" => OpType::Operand,

        _ => OpType::Unknown,
    }
}

fn cpp_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators
        "int" | "float" | "double" | "char" | "void" | "bool" | "if" | "else" | "for"
        | "while" | "do" | "switch" | "case" | "default" | "return" | "break" | "continue"
        | "goto" | "class" | "struct" | "enum" | "union" | "namespace" | "using" | "typedef"
        | "template" | "typename" | "public" | "private" | "protected" | "virtual" | "static"
        | "const" | "volatile" | "throw" | "try" | "catch" | "new" | "delete" => {
            OpType::Operator
        }
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||"
        | "!" | "&" | "|" | "^" | "<<" | ">>" | "=" | "+=" | "-=" | "*=" | "/=" | "%="
        | "&=" | "|=" | "^=" | "<<=" | ">>=" | "." | "->" | "::" | "," | ";" | ":" => {
            OpType::Operator
        }
        "(" | "[" | "{" | "}" | "]" | ")" => OpType::Operator,

        // Operands
        "identifier" | "number_literal" | "string_literal" | "raw_string_literal" | "true"
        | "false" | "nullptr" | "this" => OpType::Operand,

        _ => OpType::Unknown,
    }
}

fn java_op_type(kind: &str, _node: &TSNode) -> OpType {
    match kind {
        // Operators
        "void" | "boolean" | "byte" | "short" | "int" | "long" | "float" | "double" | "char"
        | "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" | "return"
        | "break" | "continue" | "class" | "interface" | "enum" | "extends" | "implements"
        | "public" | "private" | "protected" | "static" | "final" | "abstract" | "synchronized"
        | "volatile" | "transient" | "native" | "strictfp" | "throw" | "throws" | "try"
        | "catch" | "finally" | "new" | "instanceof" => OpType::Operator,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||"
        | "!" | "&" | "|" | "^" | "<<" | ">>" | ">>>" | "=" | "+=" | "-=" | "*=" | "/="
        | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>=" | "." | "," | ";" | ":" => {
            OpType::Operator
        }
        "(" | "[" | "{" | "}" | "]" | ")" => OpType::Operator,

        // Operands
        "identifier" | "decimal_integer_literal" | "hex_integer_literal"
        | "octal_integer_literal" | "binary_integer_literal" | "decimal_floating_point_literal"
        | "hex_floating_point_literal" | "string_literal" | "character_literal" | "true"
        | "false" | "null" | "this" | "super" => OpType::Operand,

        _ => OpType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_ops_basic() {
        let code = "let x = 5 + 3;";
        let ops = extract_ops(code, Lang::Rust).unwrap();

        assert!(ops.operators.contains(&"let".to_string()));
        assert!(ops.operators.contains(&"=".to_string()));
        assert!(ops.operators.contains(&"+".to_string()));
        assert!(ops.operands.contains(&"x".to_string()));
        assert!(ops.operands.contains(&"5".to_string()));
        assert!(ops.operands.contains(&"3".to_string()));
    }

    #[test]
    fn test_rust_function_ops() {
        let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let ops = extract_ops(code, Lang::Rust).unwrap();

        // Operators and operands should be accumulated at the unit level
        assert!(ops.operators.contains(&"fn".to_string()));
        assert!(ops.operators.contains(&"+".to_string()));
        assert!(ops.operands.contains(&"add".to_string()));
        assert!(ops.operands.contains(&"a".to_string()));
        assert!(ops.operands.contains(&"b".to_string()));

        // Should have one function subspace
        assert_eq!(ops.spaces.len(), 1);
        assert_eq!(ops.spaces[0].name, Some("add".to_string()));
        assert_eq!(ops.spaces[0].kind, SpaceKind::Function);
    }

    #[test]
    fn test_python_ops() {
        let code = "if True:\n    a = 1 + 2";
        let ops = extract_ops(code, Lang::Python).unwrap();

        assert!(ops.operators.contains(&"if".to_string()));
        assert!(ops.operators.contains(&"=".to_string()));
        assert!(ops.operators.contains(&"+".to_string()));
        // Python's True is a keyword operand
        assert!(ops.operands.contains(&"a".to_string()));
        assert!(ops.operands.contains(&"1".to_string()));
        assert!(ops.operands.contains(&"2".to_string()));
    }

    #[test]
    fn test_typescript_ops() {
        let code = "const x: number = 42;";
        let ops = extract_ops(code, Lang::TypeScript).unwrap();

        assert!(ops.operators.contains(&"const".to_string()));
        assert!(ops.operators.contains(&"=".to_string()));
        assert!(ops.operands.contains(&"x".to_string()));
        assert!(ops.operands.contains(&"42".to_string()));
    }

    #[test]
    fn test_javascript_ops() {
        let code = "let sum = a + b + c;";
        let ops = extract_ops(code, Lang::JavaScript).unwrap();

        assert!(ops.operators.contains(&"let".to_string()));
        assert!(ops.operators.contains(&"=".to_string()));
        assert!(ops.operators.contains(&"+".to_string()));
        assert!(ops.operands.contains(&"sum".to_string()));
        assert!(ops.operands.contains(&"a".to_string()));
        assert!(ops.operands.contains(&"b".to_string()));
        assert!(ops.operands.contains(&"c".to_string()));
    }

    #[test]
    fn test_nested_functions() {
        let code = r#"
fn outer() {
    fn inner() {
        let x = 1;
    }
    let y = 2;
}
"#;
        let ops = extract_ops(code, Lang::Rust).unwrap();

        // All operators and operands should be accumulated at unit level
        assert!(ops.operands.contains(&"outer".to_string()));
        assert!(ops.operands.contains(&"inner".to_string()));
        assert!(ops.operands.contains(&"x".to_string()));
        assert!(ops.operands.contains(&"y".to_string()));

        // Should have nested structure
        assert_eq!(ops.spaces.len(), 1); // outer function
        assert_eq!(ops.spaces[0].name, Some("outer".to_string()));
        assert_eq!(ops.spaces[0].spaces.len(), 1); // inner function
        assert_eq!(ops.spaces[0].spaces[0].name, Some("inner".to_string()));
    }

    #[test]
    fn test_empty_code() {
        let code = "";
        let result = extract_ops(code, Lang::Rust);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ops_merge() {
        let mut ops1 = Ops::new(Some("test".to_string()), 1, 10, SpaceKind::Function);
        ops1.operators.push("fn".to_string());
        ops1.operands.push("x".to_string());

        let mut ops2 = Ops::new(Some("test2".to_string()), 5, 8, SpaceKind::Function);
        ops2.operators.push("let".to_string());
        ops2.operands.push("y".to_string());

        ops1.merge_ops(&ops2);

        assert_eq!(ops1.operators.len(), 2);
        assert_eq!(ops1.operands.len(), 2);
        assert!(ops1.operators.contains(&"fn".to_string()));
        assert!(ops1.operators.contains(&"let".to_string()));
    }
}
