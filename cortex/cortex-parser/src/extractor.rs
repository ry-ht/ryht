//! Core extraction logic for pulling structured data from syntax trees.

use tree_sitter::Node;

/// Helper trait for extracting text from nodes.
pub trait NodeExtractor {
    /// Get the text content of a node.
    fn text<'a>(&self, source: &'a str) -> &'a str;

    /// Get the text of a named child.
    fn child_text<'a>(&self, source: &'a str, field_name: &str) -> Option<&'a str>;

    /// Get all children of a specific kind.
    fn children_by_kind(&self, kind: &str) -> Vec<Node<'_>>;

    /// Check if node has a specific kind.
    fn is_kind(&self, kind: &str) -> bool;

    /// Get start line (1-indexed).
    fn start_line(&self) -> usize;

    /// Get end line (1-indexed).
    fn end_line(&self) -> usize;
}

impl<'a> NodeExtractor for Node<'a> {
    fn text<'b>(&self, source: &'b str) -> &'b str {
        let start = self.start_byte();
        let end = self.end_byte();
        &source[start..end]
    }

    fn child_text<'b>(&self, source: &'b str, field_name: &str) -> Option<&'b str> {
        self.child_by_field_name(field_name)
            .map(|node| node.text(source))
    }

    fn children_by_kind(&self, kind: &str) -> Vec<Node<'_>> {
        let mut cursor = self.walk();
        self.children(&mut cursor)
            .filter(|child| child.kind() == kind)
            .collect()
    }

    fn is_kind(&self, kind: &str) -> bool {
        self.kind() == kind
    }

    fn start_line(&self) -> usize {
        self.start_position().row + 1
    }

    fn end_line(&self) -> usize {
        self.end_position().row + 1
    }
}

/// Extract documentation comments from preceding siblings.
pub fn extract_docstring(node: Node, source: &str) -> Option<String> {
    let mut docs = Vec::new();
    let mut current = node.prev_sibling();

    while let Some(sibling) = current {
        match sibling.kind() {
            "line_comment" => {
                let text = sibling.text(source);
                if text.starts_with("///") || text.starts_with("//!") {
                    docs.push(text.trim_start_matches('/').trim().to_string());
                    current = sibling.prev_sibling();
                } else {
                    break;
                }
            }
            "block_comment" => {
                let text = sibling.text(source);
                if text.starts_with("/**") || text.starts_with("/*!") {
                    let cleaned = text
                        .trim_start_matches("/**")
                        .trim_start_matches("/*!")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|line| line.trim().trim_start_matches('*').trim())
                        .collect::<Vec<_>>()
                        .join("\n");
                    docs.push(cleaned);
                    current = sibling.prev_sibling();
                } else {
                    break;
                }
            }
            _ => {
                current = sibling.prev_sibling();
            }
        }
    }

    docs.reverse();
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

/// Extract attributes from a node.
pub fn extract_attributes(node: Node, source: &str) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut current = node.prev_sibling();

    while let Some(sibling) = current {
        if sibling.kind() == "attribute_item" || sibling.kind() == "inner_attribute_item" {
            attrs.push(sibling.text(source).to_string());
            current = sibling.prev_sibling();
        } else if sibling.kind() == "line_comment" || sibling.kind() == "block_comment" {
            current = sibling.prev_sibling();
        } else {
            break;
        }
    }

    attrs.reverse();
    attrs
}

/// Calculate cyclomatic complexity of a function body.
pub fn calculate_complexity(node: Node) -> u32 {
    let mut complexity = 1; // Base complexity

    let mut cursor = node.walk();
    let mut stack = vec![node];

    while let Some(current) = stack.pop() {
        match current.kind() {
            // Decision points
            "if_expression" | "match_expression" | "while_expression" | "for_expression"
            | "loop_expression" => {
                complexity += 1;
            }
            // Logical operators
            "||" | "&&" => {
                complexity += 1;
            }
            // Match arms (each arm is a decision point)
            "match_arm" => {
                complexity += 1;
            }
            _ => {}
        }

        // Add children to stack
        for child in current.children(&mut cursor) {
            stack.push(child);
        }
    }

    complexity
}

/// Extract generic parameters from a node.
pub fn extract_generics(node: Node, source: &str) -> Vec<String> {
    if let Some(type_params) = node.child_by_field_name("type_parameters") {
        let mut cursor = type_params.walk();
        type_params
            .children(&mut cursor)
            .filter(|child| {
                matches!(
                    child.kind(),
                    "type_parameter" | "lifetime" | "const_parameter"
                )
            })
            .map(|child| child.text(source).to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// Extract where clause from a node.
pub fn extract_where_clause(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("where_clause")
        .map(|wc| wc.text(source).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_complexity_simple() {
        // Test with a simple tree-sitter parse
        let source = "fn test() { if x { y } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the function body
        let func = root
            .child(0)
            .unwrap()
            .child_by_field_name("body")
            .unwrap();
        let complexity = calculate_complexity(func);

        // Should be 2: base (1) + if statement (1)
        assert!(complexity >= 2);
    }

    #[test]
    fn test_node_extractor() {
        let source = "fn main() {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        assert_eq!(root.text(source), source);
        assert!(root.start_line() >= 1);
    }
}
