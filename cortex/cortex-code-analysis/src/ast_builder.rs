//! AST serialization and builder module.
//!
//! This module provides functionality to build and serialize Abstract Syntax Trees (ASTs)
//! from parsed code using tree-sitter. It supports building complete AST representations
//! with optional span information and comment filtering.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{build_ast, Lang};
//!
//! # fn main() -> anyhow::Result<()> {
//! let source = r#"
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//! "#;
//!
//! let ast = build_ast(source, Lang::Rust, true, false)?;
//! assert_eq!(ast.r#type, "source_file");
//! assert!(!ast.children.is_empty());
//! # Ok(())
//! # }
//! ```

use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use std::path::Path;
use anyhow::{Context, Result};

use crate::lang::Lang;
use crate::node::Node;
use crate::parser::Parser;
use crate::traits::{LanguageInfo, ParserTrait};
use crate::languages::*;

/// Start and end positions of a node in code in terms of rows and columns.
///
/// The tuple contains: (start_row, start_column, end_row, end_column).
/// All positions are 1-indexed to match common editor conventions.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::Span;
///
/// // A span from line 1, column 1 to line 5, column 10
/// let span: Span = Some((1, 1, 5, 10));
/// ```
pub type Span = Option<(usize, usize, usize, usize)>;

/// An AST node containing type, value, span, and children information.
///
/// This structure represents a single node in the Abstract Syntax Tree,
/// including its type (kind), text value, optional position information,
/// and child nodes.
#[derive(Debug, Clone)]
pub struct AstNode {
    /// The type/kind of the node (e.g., "function_item", "identifier")
    pub r#type: &'static str,
    /// The text content associated with this node
    pub value: String,
    /// Optional position information (start_row, start_col, end_row, end_col)
    pub span: Span,
    /// Child nodes of this node
    pub children: Vec<AstNode>,
}

impl AstNode {
    /// Create a new AST node.
    ///
    /// # Arguments
    ///
    /// * `r#type` - The node type/kind
    /// * `value` - The text content of the node
    /// * `span` - Optional position information
    /// * `children` - Child nodes
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::AstNode;
    ///
    /// let node = AstNode::new(
    ///     "identifier",
    ///     "foo".to_string(),
    ///     Some((1, 1, 1, 4)),
    ///     vec![]
    /// );
    /// assert_eq!(node.r#type, "identifier");
    /// assert_eq!(node.value, "foo");
    /// ```
    pub fn new(r#type: &'static str, value: String, span: Span, children: Vec<AstNode>) -> Self {
        Self {
            r#type,
            value,
            span,
            children,
        }
    }

    /// Get the number of children in this node.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Check if this node is a leaf node (no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the depth of this AST subtree.
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
        }
    }

    /// Count total nodes in this AST subtree.
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }
}

impl Serialize for AstNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("AstNode", 4)?;
        st.serialize_field("type", &self.r#type)?;
        st.serialize_field("value", &self.value)?;
        st.serialize_field("span", &self.span)?;
        st.serialize_field("children", &self.children)?;
        st.end()
    }
}

/// Configuration for AST building.
#[derive(Debug, Clone)]
pub struct AstConfig {
    /// Include span (position) information in the AST
    pub include_span: bool,
    /// Filter out comment nodes from the AST
    pub filter_comments: bool,
}

impl Default for AstConfig {
    fn default() -> Self {
        Self {
            include_span: true,
            filter_comments: false,
        }
    }
}

/// Build an AST node from a tree-sitter node.
///
/// This is a generic function that works with any language implementing `LanguageInfo`.
/// It constructs the AST using a bottom-up approach to avoid reference cycles.
fn build_ast_generic<T: LanguageInfo>(
    parser: &Parser<T>,
    include_span: bool,
    filter_comments: bool,
) -> Option<AstNode> {
    let code = parser.get_code();
    let root = parser.get_root();
    let mut cursor = root.cursor();
    let mut node_stack = Vec::new();
    let mut child_stack = Vec::new();

    node_stack.push(root);
    child_stack.push(Vec::new());

    // Build AST from bottom-to-top and left-to-right to avoid Rc/RefCell
    loop {
        let ts_node = node_stack.last().unwrap();
        cursor.reset(ts_node);

        if cursor.goto_first_child() {
            let node = cursor.node();
            child_stack.push(Vec::with_capacity(node.child_count()));
            node_stack.push(node);
        } else {
            loop {
                let ts_node = node_stack.pop().unwrap();

                if let Some(ast_node) = create_ast_node(&ts_node, code, include_span, filter_comments, child_stack.pop().unwrap()) {
                    if !child_stack.is_empty() {
                        child_stack.last_mut().unwrap().push(ast_node);
                    } else {
                        return Some(ast_node);
                    }
                }

                if let Some(next_node) = ts_node.next_sibling() {
                    child_stack.push(Vec::with_capacity(next_node.child_count()));
                    node_stack.push(next_node);
                    break;
                }
            }
        }
    }
}

/// Create an AST node from a tree-sitter node.
///
/// This function handles the logic for creating an AstNode, including:
/// - Filtering comments if requested
/// - Extracting text content for leaf nodes
/// - Computing span information if requested
fn create_ast_node(
    node: &Node,
    code: &[u8],
    include_span: bool,
    filter_comments: bool,
    children: Vec<AstNode>,
) -> Option<AstNode> {
    // Filter comments if requested
    if filter_comments && is_comment_node(node) {
        return None;
    }

    let node_kind = node.kind();

    // Extract text for leaf nodes or for specific node types
    let value = if node.child_count() == 0 || should_extract_text(node_kind) {
        extract_text(node, code)
    } else {
        String::new()
    };

    // Compute span if requested
    let span = if include_span {
        let (start_row, start_col) = node.start_position();
        let (end_row, end_col) = node.end_position();
        // Convert to 1-indexed positions
        Some((start_row + 1, start_col + 1, end_row + 1, end_col + 1))
    } else {
        None
    };

    Some(AstNode::new(node_kind, value, span, children))
}

/// Check if a node represents a comment.
fn is_comment_node(node: &Node) -> bool {
    let kind = node.kind();
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "doc_comment"
    )
}

/// Check if we should extract text for a given node kind.
///
/// For certain node types like string literals, we want to extract the text
/// even if they have children.
fn should_extract_text(kind: &str) -> bool {
    matches!(
        kind,
        "string_literal" | "char_literal" | "string" | "string_fragment" |
        "raw_string_literal" | "byte_string_literal" | "identifier" |
        "field_identifier" | "property_identifier" | "type_identifier"
    )
}

/// Extract text content from a node.
fn extract_text(node: &Node, code: &[u8]) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&code[start..end]).into_owned()
}

/// Build an AST from source code.
///
/// This is the main entry point for building ASTs. It automatically handles
/// the language-specific parser setup and AST construction.
///
/// # Arguments
///
/// * `source` - The source code to parse
/// * `language` - The programming language of the source
/// * `include_span` - Whether to include position information
/// * `filter_comments` - Whether to filter out comment nodes
///
/// # Returns
///
/// Returns an `AstNode` representing the root of the AST, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{build_ast, Lang};
///
/// # fn main() -> anyhow::Result<()> {
/// let source = "fn main() {}";
/// let ast = build_ast(source, Lang::Rust, true, false)?;
/// assert_eq!(ast.r#type, "source_file");
/// # Ok(())
/// # }
/// ```
pub fn build_ast(
    source: &str,
    language: Lang,
    include_span: bool,
    filter_comments: bool,
) -> Result<AstNode> {
    let code = source.as_bytes().to_vec();
    let path = Path::new("anonymous");

    match language {
        Lang::Rust => {
            let parser = Parser::<RustLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for Rust code")
        }
        Lang::TypeScript => {
            let parser = Parser::<TypeScriptLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for TypeScript code")
        }
        Lang::Tsx => {
            let parser = Parser::<TypeScriptLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for TSX code")
        }
        Lang::JavaScript => {
            let parser = Parser::<JavaScriptLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for JavaScript code")
        }
        Lang::Jsx => {
            let parser = Parser::<JavaScriptLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for JSX code")
        }
        Lang::Python => {
            let parser = Parser::<PythonLanguage>::new(code, path)?;
            build_ast_generic(&parser, include_span, filter_comments)
                .context("Failed to build AST for Python code")
        }
        _ => {
            anyhow::bail!("AST building not yet supported for language: {:?}", language)
        }
    }
}

/// Build an AST with custom configuration.
///
/// This function provides more control over AST building through a configuration object.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{build_ast_with_config, AstConfig, Lang};
///
/// # fn main() -> anyhow::Result<()> {
/// let config = AstConfig {
///     include_span: true,
///     filter_comments: true,
/// };
/// let source = "fn main() { /* comment */ }";
/// let ast = build_ast_with_config(source, Lang::Rust, config)?;
/// # Ok(())
/// # }
/// ```
pub fn build_ast_with_config(
    source: &str,
    language: Lang,
    config: AstConfig,
) -> Result<AstNode> {
    build_ast(source, language, config.include_span, config.filter_comments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_node_creation() {
        let node = AstNode::new("identifier", "foo".to_string(), Some((1, 1, 1, 4)), vec![]);
        assert_eq!(node.r#type, "identifier");
        assert_eq!(node.value, "foo");
        assert_eq!(node.span, Some((1, 1, 1, 4)));
        assert!(node.is_leaf());
    }

    #[test]
    fn test_ast_node_depth() {
        let leaf = AstNode::new("identifier", "x".to_string(), None, vec![]);
        assert_eq!(leaf.depth(), 1);

        let parent = AstNode::new("expression", String::new(), None, vec![leaf]);
        assert_eq!(parent.depth(), 2);
    }

    #[test]
    fn test_ast_node_count() {
        let child1 = AstNode::new("identifier", "a".to_string(), None, vec![]);
        let child2 = AstNode::new("identifier", "b".to_string(), None, vec![]);
        let parent = AstNode::new("expression", String::new(), None, vec![child1, child2]);
        assert_eq!(parent.node_count(), 3);
    }

    #[test]
    fn test_build_ast_rust() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let ast = build_ast(source, Lang::Rust, true, false).unwrap();
        assert_eq!(ast.r#type, "source_file");
        assert!(!ast.children.is_empty());
    }

    #[test]
    fn test_build_ast_rust_no_span() {
        let source = "fn main() {}";
        let ast = build_ast(source, Lang::Rust, false, false).unwrap();
        assert_eq!(ast.r#type, "source_file");
        assert_eq!(ast.span, None);
    }

    #[test]
    fn test_build_ast_with_span() {
        let source = "fn test() {}";
        let ast = build_ast(source, Lang::Rust, true, false).unwrap();
        assert!(ast.span.is_some());
        let (start_row, start_col, _end_row, _end_col) = ast.span.unwrap();
        assert_eq!(start_row, 1);
        assert_eq!(start_col, 1);
    }

    #[test]
    fn test_build_ast_filter_comments() {
        let source = r#"
// This is a comment
fn main() {
    // Another comment
    let x = 1;
}
"#;
        let ast_with_comments = build_ast(source, Lang::Rust, false, false).unwrap();
        let ast_without_comments = build_ast(source, Lang::Rust, false, true).unwrap();

        // The version without comments should have fewer nodes
        assert!(ast_without_comments.node_count() < ast_with_comments.node_count());
    }

    #[test]
    fn test_build_ast_typescript() {
        let source = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#;
        let ast = build_ast(source, Lang::TypeScript, true, false).unwrap();
        assert_eq!(ast.r#type, "program");
        assert!(!ast.children.is_empty());
    }

    #[test]
    fn test_build_ast_javascript() {
        let source = r#"
function add(a, b) {
    return a + b;
}
"#;
        let ast = build_ast(source, Lang::JavaScript, true, false).unwrap();
        assert_eq!(ast.r#type, "program");
        assert!(!ast.children.is_empty());
    }

    #[test]
    fn test_build_ast_python() {
        let source = r#"
def greet(name):
    return f"Hello, {name}!"
"#;
        let ast = build_ast(source, Lang::Python, true, false).unwrap();
        assert_eq!(ast.r#type, "module");
        assert!(!ast.children.is_empty());
    }

    #[test]
    fn test_build_ast_with_config() {
        let config = AstConfig {
            include_span: true,
            filter_comments: true,
        };
        let source = "fn main() { /* test */ }";
        let ast = build_ast_with_config(source, Lang::Rust, config).unwrap();
        assert_eq!(ast.r#type, "source_file");
        assert!(ast.span.is_some());
    }

    #[test]
    fn test_ast_serialization() {
        let source = "fn main() {}";
        let ast = build_ast(source, Lang::Rust, true, false).unwrap();

        // Test that serialization works
        let json = serde_json::to_string(&ast).unwrap();
        assert!(json.contains("source_file"));
        assert!(json.contains("type"));
        assert!(json.contains("children"));
    }

    #[test]
    fn test_is_comment_node() {
        let source = r#"
// line comment
fn main() {}
"#;
        let code = source.as_bytes().to_vec();
        let path = Path::new("test.rs");
        let parser = Parser::<RustLanguage>::new(code, path).unwrap();
        let root = parser.get_root();

        let mut found_comment = false;
        for child in root.children() {
            if is_comment_node(&child) {
                found_comment = true;
                assert_eq!(child.kind(), "line_comment");
                break;
            }
        }
        assert!(found_comment);
    }

    #[test]
    fn test_extract_text() {
        let source = "let x = 42;";
        let code = source.as_bytes().to_vec();
        let path = Path::new("test.rs");
        let parser = Parser::<RustLanguage>::new(code, path).unwrap();
        let root = parser.get_root();

        // The full source text should be extractable
        let text = extract_text(&root, parser.get_code());
        assert_eq!(text, source);
    }

    #[test]
    fn test_ast_config_default() {
        let config = AstConfig::default();
        assert!(config.include_span);
        assert!(!config.filter_comments);
    }

    #[test]
    fn test_complex_rust_ast() {
        let source = r#"
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}
"#;
        let ast = build_ast(source, Lang::Rust, true, false).unwrap();
        assert_eq!(ast.r#type, "source_file");
        assert!(ast.node_count() > 10);
        assert!(ast.depth() > 3);
    }

    #[test]
    fn test_string_literal_text_extraction() {
        let source = r#"let s = "hello world";"#;
        let ast = build_ast(source, Lang::Rust, false, false).unwrap();

        // Find the string literal node
        fn find_string_literal(node: &AstNode) -> Option<String> {
            if node.r#type == "string_literal" {
                return Some(node.value.clone());
            }
            for child in &node.children {
                if let Some(value) = find_string_literal(child) {
                    return Some(value);
                }
            }
            None
        }

        let string_value = find_string_literal(&ast);
        assert!(string_value.is_some());
        assert!(string_value.unwrap().contains("hello world"));
    }

    #[test]
    fn test_empty_source() {
        let source = "";
        let ast = build_ast(source, Lang::Rust, false, false).unwrap();
        assert_eq!(ast.r#type, "source_file");
        assert_eq!(ast.children.len(), 0);
    }

    #[test]
    fn test_ast_with_errors() {
        // Invalid Rust code
        let source = "fn main( {";
        let result = build_ast(source, Lang::Rust, false, false);
        // Should still build an AST, even with syntax errors
        assert!(result.is_ok());
    }
}
