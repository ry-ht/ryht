//! Comment removal module for code analysis.
//!
//! This module provides functionality to remove comments from source code while
//! preserving line numbers and useful comments like documentation and pragmas.

use anyhow::{Context, Result};

use crate::lang::Lang;
use crate::node::Node;

/// Represents a span of text in the source code where a comment exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSpan {
    /// Start byte offset in the source
    pub start: usize,
    /// End byte offset in the source
    pub end: usize,
    /// Number of lines the comment spans
    pub lines: usize,
}

impl CommentSpan {
    /// Create a new CommentSpan from a node.
    pub fn from_node(node: &Node) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            lines: node.end_row() - node.start_row(),
        }
    }

    /// Get the length in bytes of this span.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if this span is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Check if a node is a comment based on its kind string.
fn is_comment_node(node: &Node) -> bool {
    let kind = node.kind();
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "doc_comment"
    )
}

/// Check if a comment should be preserved (doc comments, pragmas, etc).
fn is_useful_comment(node: &Node, code: &[u8], lang: Lang) -> bool {
    let comment_bytes = &code[node.start_byte()..node.end_byte()];

    // Convert to string for easier checking
    let comment_text = match std::str::from_utf8(comment_bytes) {
        Ok(text) => text,
        Err(_) => return false,
    };

    match lang {
        Lang::Rust => {
            // Check if comment is inside a token tree (macro)
            if let Some(parent) = node.parent() {
                if parent.kind() == "token_tree" {
                    return true;
                }
            }

            // For tree-sitter-rust, doc comment markers (///, //!, /**, /*!) might not be
            // included in the comment node itself. We need to check the text before the comment.
            let start = node.start_byte();

            // Check a few characters before the comment to see if it's a doc comment
            if start >= 3 {
                let prefix_start = start.saturating_sub(4);
                let prefix = &code[prefix_start..start];
                let prefix_str = std::str::from_utf8(prefix).unwrap_or("");

                if prefix_str.contains("///") || prefix_str.contains("//!") {
                    return true;
                }
            }

            // Also check if the comment text itself starts with doc markers
            let trimmed = comment_text.trim();
            if trimmed.starts_with("///")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("/**")
                || trimmed.starts_with("/*!")
            {
                return true;
            }

            // Check for special pragmas
            comment_text.contains("cbindgen:") || comment_text.contains("rustbindgen")
        }

        Lang::Python => {
            // Preserve encoding comments at the top of the file
            if node.start_row() <= 1 {
                comment_text.contains("coding:") || comment_text.contains("coding=")
            } else {
                false
            }
        }

        Lang::Cpp => {
            // Preserve rustbindgen comments in C/C++ headers
            comment_text.contains("rustbindgen")
        }

        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
            // Preserve JSDoc comments and special directives
            comment_text.starts_with("/**")
                || comment_text.starts_with("/*!")
                || comment_text.contains("@ts-")
                || comment_text.contains("@type")
                || comment_text.contains("eslint-")
                || comment_text.contains("prettier-")
        }

        Lang::Java => {
            // Preserve JavaDoc comments
            comment_text.starts_with("/**")
        }

        Lang::Kotlin => {
            // Preserve KDoc comments
            comment_text.starts_with("/**")
        }
    }
}

/// Extract all comment spans from an AST.
fn extract_comment_spans(root: Node, code: &[u8], lang: Lang) -> Vec<CommentSpan> {
    let mut spans = Vec::new();
    let mut stack = Vec::new();
    let mut cursor = root.cursor();

    stack.push(root);

    while let Some(node) = stack.pop() {
        if is_comment_node(&node) {
            let is_useful = is_useful_comment(&node, code, lang);
            if !is_useful {
                spans.push(CommentSpan::from_node(&node));
            }
        } else {
            // Traverse children
            cursor.reset(&node);
            if cursor.goto_first_child() {
                let mut children = Vec::new();
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                // Push in reverse order for depth-first traversal
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
        }
    }

    spans
}

/// Remove comments from code and replace them with newlines to preserve line numbers.
fn remove_comments_from_code(code: &[u8], mut spans: Vec<CommentSpan>) -> Vec<u8> {
    if spans.is_empty() {
        return code.to_vec();
    }

    // Sort spans in reverse order by start position
    spans.sort_by(|a, b| b.start.cmp(&a.start));

    let mut new_code = Vec::with_capacity(code.len());
    let mut code_start = 0;

    // Pre-allocate a buffer of newlines for efficiency
    const NEWLINES: [u8; 8192] = [b'\n'; 8192];

    for span in spans.iter().rev() {
        // Copy code before this comment
        new_code.extend_from_slice(&code[code_start..span.start]);

        // Replace comment with newlines to preserve line numbers
        if span.lines > 0 {
            if span.lines <= NEWLINES.len() {
                new_code.extend_from_slice(&NEWLINES[..span.lines]);
            } else {
                // For very large comments, extend with additional newlines
                new_code.resize(new_code.len() + span.lines, b'\n');
            }
        }

        code_start = span.end;
    }

    // Copy remaining code after the last comment
    if code_start < code.len() {
        new_code.extend_from_slice(&code[code_start..]);
    }

    new_code
}

/// Remove comments from source code while preserving line numbers.
///
/// This function parses the source code using tree-sitter, identifies all comments,
/// and removes them while replacing them with newlines to maintain line number consistency.
///
/// Useful comments such as documentation comments and pragmas are preserved.
///
/// # Arguments
///
/// * `code` - The source code as a string
/// * `lang` - The programming language of the source code
///
/// # Returns
///
/// A `Result` containing the code with comments removed, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{remove_comments, Lang};
///
/// let rust_code = r#"
/// // This is a comment
/// fn main() {
///     println!("Hello"); // inline comment
/// }
/// "#;
///
/// let cleaned = remove_comments(rust_code, Lang::Rust).unwrap();
/// assert!(!cleaned.contains("This is a comment"));
/// assert!(cleaned.contains("fn main()"));
/// ```
pub fn remove_comments(code: &str, lang: Lang) -> Result<String> {
    let code_bytes = code.as_bytes();

    // Parse the code with tree-sitter
    let ts_lang = lang.get_ts_language();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&ts_lang)
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(code_bytes, None)
        .context("Failed to parse source code")?;

    let root = Node::new(tree.root_node());

    // Extract comment spans
    let spans = extract_comment_spans(root, code_bytes, lang);

    // If no comments found, return original code
    if spans.is_empty() {
        return Ok(code.to_string());
    }

    // Remove comments
    let new_code = remove_comments_from_code(code_bytes, spans);

    // Convert back to string
    String::from_utf8(new_code).context("Failed to convert result to UTF-8 string")
}

/// Extract all comment spans from source code without removing them.
///
/// This is useful for analyzing comments without modifying the code.
///
/// # Arguments
///
/// * `code` - The source code as a string
/// * `lang` - The programming language of the source code
///
/// # Returns
///
/// A `Result` containing a vector of `CommentSpan` structs.
pub fn extract_comments(code: &str, lang: Lang) -> Result<Vec<CommentSpan>> {
    let code_bytes = code.as_bytes();

    // Parse the code with tree-sitter
    let ts_lang = lang.get_ts_language();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&ts_lang)
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(code_bytes, None)
        .context("Failed to parse source code")?;

    let root = Node::new(tree.root_node());

    Ok(extract_comment_spans(root, code_bytes, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_remove_line_comments() {
        let source = r#"// This is a comment
fn main() {
    let x = 42; // inline comment
    println!("{}", x);
}
// Another comment
"#;

        let result = remove_comments(source, Lang::Rust).unwrap();

        // Comments should be removed
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("inline comment"));
        assert!(!result.contains("Another comment"));

        // Code should be preserved
        assert!(result.contains("fn main()"));
        assert!(result.contains("let x = 42;"));
        assert!(result.contains("println!"));

        // Line numbers should be preserved
        let original_lines: Vec<_> = source.lines().collect();
        let result_lines: Vec<_> = result.lines().collect();
        assert_eq!(original_lines.len(), result_lines.len());
    }

    #[test]
    fn test_rust_remove_block_comments() {
        let source = r#"/* Block comment
 * with multiple lines
 */
fn test() {
    /* inline block */ let x = 1;
}
"#;

        let result = remove_comments(source, Lang::Rust).unwrap();

        assert!(!result.contains("Block comment"));
        assert!(!result.contains("multiple lines"));
        assert!(!result.contains("inline block"));
        assert!(result.contains("fn test()"));
        assert!(result.contains("let x = 1;"));
    }

    #[test]
    fn test_rust_preserve_doc_comments() {
        let source = r#"/// This is a doc comment
/// It should be preserved
pub fn documented() {}

//! Module-level doc comment
"#;

        let result = remove_comments(source, Lang::Rust).unwrap();

        // Doc comments should be preserved
        assert!(result.contains("This is a doc comment"));
        assert!(result.contains("It should be preserved"));
        assert!(result.contains("Module-level doc comment"));
    }

    #[test]
    fn test_python_remove_comments() {
        let source = r#"# This is a comment
def hello():
    # Another comment
    print("Hello")  # inline comment
"#;

        let result = remove_comments(source, Lang::Python).unwrap();

        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("Another comment"));
        assert!(!result.contains("inline comment"));
        assert!(result.contains("def hello()"));
        assert!(result.contains("print(\"Hello\")"));
    }

    #[test]
    fn test_python_preserve_encoding() {
        let source = r#"# -*- coding: utf-8 -*-
# This should be removed
def test():
    pass
"#;

        let result = remove_comments(source, Lang::Python).unwrap();

        // Encoding comment should be preserved
        assert!(result.contains("coding: utf-8"));

        // Other comments should be removed
        assert!(!result.contains("This should be removed"));
    }

    #[test]
    fn test_typescript_remove_comments() {
        let source = r#"// Single line comment
function test() {
    /* Block comment */
    const x = 42; // inline
}
"#;

        let result = remove_comments(source, Lang::TypeScript).unwrap();

        assert!(!result.contains("Single line comment"));
        assert!(!result.contains("Block comment"));
        assert!(!result.contains("inline"));
        assert!(result.contains("function test()"));
        assert!(result.contains("const x = 42;"));
    }

    #[test]
    fn test_typescript_preserve_jsdoc() {
        let source = r#"/**
 * This is JSDoc
 * @param x The parameter
 */
function documented(x: number) {
    // This should be removed
    return x;
}
"#;

        let result = remove_comments(source, Lang::TypeScript).unwrap();

        // JSDoc should be preserved
        assert!(result.contains("This is JSDoc"));
        assert!(result.contains("@param"));

        // Regular comments should be removed
        assert!(!result.contains("This should be removed"));
    }

    #[test]
    fn test_javascript_remove_comments() {
        let source = r#"// Comment
const add = (a, b) => {
    /* multi
       line */
    return a + b; // result
};
"#;

        let result = remove_comments(source, Lang::JavaScript).unwrap();

        assert!(!result.contains("Comment"));
        assert!(!result.contains("multi"));
        assert!(!result.contains("line"));
        assert!(!result.contains("result"));
        assert!(result.contains("const add"));
        assert!(result.contains("return a + b;"));
    }

    #[test]
    fn test_cpp_remove_comments() {
        let source = r#"// C++ comment
int main() {
    /* Block
       comment */
    int x = 42; // inline
    return 0;
}
"#;

        let result = remove_comments(source, Lang::Cpp).unwrap();

        assert!(!result.contains("C++ comment"));
        assert!(!result.contains("Block"));
        assert!(!result.contains("inline"));
        assert!(result.contains("int main()"));
        assert!(result.contains("int x = 42;"));
    }

    #[test]
    fn test_java_remove_comments() {
        let source = r#"// Class comment
public class Test {
    /* Method comment */
    public void test() {
        int x = 1; // inline
    }
}
"#;

        let result = remove_comments(source, Lang::Java).unwrap();

        assert!(!result.contains("Class comment"));
        assert!(!result.contains("Method comment"));
        assert!(!result.contains("inline"));
        assert!(result.contains("public class Test"));
        assert!(result.contains("public void test()"));
    }

    #[test]
    fn test_java_preserve_javadoc() {
        let source = r#"/**
 * JavaDoc comment
 * @return something
 */
public int getValue() {
    // regular comment
    return 42;
}
"#;

        let result = remove_comments(source, Lang::Java).unwrap();

        // JavaDoc should be preserved
        assert!(result.contains("JavaDoc comment"));
        assert!(result.contains("@return"));

        // Regular comments should be removed
        assert!(!result.contains("regular comment"));
    }

    #[test]
    fn test_extract_comments() {
        let source = r#"// Comment 1
fn main() {
    // Comment 2
    let x = 42;
}
"#;

        let spans = extract_comments(source, Lang::Rust).unwrap();

        assert_eq!(spans.len(), 2);
        assert!(spans[0].len() > 0);
        assert!(spans[1].len() > 0);
    }

    #[test]
    fn test_no_comments() {
        let source = r#"fn main() {
    let x = 42;
    println!("{}", x);
}
"#;

        let result = remove_comments(source, Lang::Rust).unwrap();

        // Should be identical to input
        assert_eq!(result, source);
    }

    #[test]
    fn test_empty_source() {
        let result = remove_comments("", Lang::Rust).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_comment_span() {
        let source = "// test\ncode\n";
        let ts_lang = Lang::Rust.get_ts_language();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&ts_lang).unwrap();
        let tree = parser.parse(source.as_bytes(), None).unwrap();
        let root = Node::new(tree.root_node());

        // Find comment node
        let mut found_comment = false;
        root.act_on_node(&mut |node| {
            if is_comment_node(node) {
                let span = CommentSpan::from_node(node);
                assert!(span.len() > 0);
                assert!(!span.is_empty());
                found_comment = true;
            }
        });

        assert!(found_comment, "Should have found a comment");
    }
}
