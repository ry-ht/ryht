//! Generic tree-sitter wrapper for parsing source code.

use anyhow::{Context, Result};
use tree_sitter::{Language, Parser, Tree};

/// Generic wrapper around tree-sitter parser.
pub struct TreeSitterWrapper {
    parser: Parser,
    language: Language,
}

impl TreeSitterWrapper {
    /// Create a new parser for the given language.
    pub fn new(language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .context("Failed to set parser language")?;

        Ok(Self { parser, language })
    }

    /// Parse source code and return the syntax tree.
    pub fn parse(&mut self, source: &str) -> Result<Tree> {
        self.parser
            .parse(source, None)
            .context("Failed to parse source code")
    }

    /// Parse source code with an old tree for incremental parsing.
    pub fn parse_with_old_tree(&mut self, source: &str, old_tree: &Tree) -> Result<Tree> {
        self.parser
            .parse(source, Some(old_tree))
            .context("Failed to parse source code incrementally")
    }

    /// Get the language used by this parser.
    pub fn language(&self) -> Language {
        self.language.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_parser_creation() {
        let result = TreeSitterWrapper::new(tree_sitter_rust::LANGUAGE.into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_typescript_parser_creation() {
        let result = TreeSitterWrapper::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_simple_rust() {
        let mut wrapper = TreeSitterWrapper::new(tree_sitter_rust::LANGUAGE.into()).unwrap();
        let source = "fn main() {}";
        let tree = wrapper.parse(source);
        assert!(tree.is_ok());
        let tree = tree.unwrap();
        assert!(!tree.root_node().has_error());
    }
}
