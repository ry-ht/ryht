//! Generic parser implementation.
//!
//! This module provides a generic Parser struct that wraps tree-sitter
//! and implements the ParserTrait for any language.

use std::marker::PhantomData;
use std::path::Path;
use tree_sitter::Parser as TSParser;
use anyhow::{Context, Result};

use crate::lang::Lang;
use crate::node::Node;
use crate::traits::{LanguageInfo, ParserTrait};

/// Generic parser that works with any language implementing LanguageInfo.
#[derive(Debug)]
pub struct Parser<T: LanguageInfo> {
    code: Vec<u8>,
    tree: tree_sitter::Tree,
    _phantom: PhantomData<T>,
}

impl<T: LanguageInfo> Parser<T> {
    /// Create a new parser and parse the given code.
    pub fn parse(code: Vec<u8>, path: &Path) -> Result<Self> {
        let mut ts_parser = TSParser::new();
        let language = T::get_lang().get_ts_language();

        ts_parser
            .set_language(&language)
            .context("Failed to set tree-sitter language")?;

        let tree = ts_parser
            .parse(&code, None)
            .context("Failed to parse code")?;

        Ok(Self {
            code,
            tree,
            _phantom: PhantomData,
        })
    }

    /// Get the parsed tree.
    pub fn tree(&self) -> &tree_sitter::Tree {
        &self.tree
    }
}

impl<T: LanguageInfo> ParserTrait for Parser<T> {
    fn new(code: Vec<u8>, path: &Path) -> Result<Self> {
        Self::parse(code, path)
    }

    fn get_language(&self) -> Lang {
        T::get_lang()
    }

    fn get_root(&self) -> Node {
        Node::new(self.tree.root_node())
    }

    fn get_code(&self) -> &[u8] {
        &self.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::RustLanguage;

    #[test]
    fn test_rust_parser_creation() {
        let code = b"fn main() {}".to_vec();
        let path = Path::new("test.rs");
        let parser = Parser::<RustLanguage>::new(code, path);
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parser_get_root() {
        let code = b"fn main() {}".to_vec();
        let path = Path::new("test.rs");
        let parser = Parser::<RustLanguage>::new(code, path).unwrap();
        let root = parser.get_root();
        assert_eq!(root.kind(), "source_file");
    }

    #[test]
    fn test_parser_get_language() {
        let code = b"fn main() {}".to_vec();
        let path = Path::new("test.rs");
        let parser = Parser::<RustLanguage>::new(code, path).unwrap();
        assert_eq!(parser.get_language(), Lang::Rust);
    }
}
