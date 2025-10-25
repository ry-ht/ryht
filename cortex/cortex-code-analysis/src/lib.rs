//! Cortex Parser - Tree-sitter based code parsing infrastructure.
//!
//! This crate provides high-level parsing capabilities for multiple programming languages
//! using tree-sitter. It extracts structured information about code elements including
//! functions, structs, enums, traits, and more.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{RustParser, ParsedFile};
//!
//! # fn main() -> anyhow::Result<()> {
//! let source = r#"
//! /// Adds two numbers together.
//! pub fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//! "#;
//!
//! let mut parser = RustParser::new()?;
//! let parsed = parser.parse_file("example.rs", source)?;
//!
//! assert_eq!(parsed.functions.len(), 1);
//! let func = &parsed.functions[0];
//! assert_eq!(func.name, "add");
//! assert_eq!(func.parameters.len(), 2);
//! # Ok(())
//! # }
//! ```

// Core abstraction modules
pub mod lang;
pub mod node;
pub mod parser;
pub mod traits;
pub mod languages;

// High-level parsing modules
pub mod ast_editor;
pub mod extractor;
pub mod rust_parser;
pub mod tree_sitter_wrapper;
pub mod types;
pub mod typescript_parser;
pub mod dependency_extractor;
pub mod metrics;

// Re-export core abstractions
pub use lang::Lang;
pub use node::Node;
pub use parser::Parser;
pub use traits::{Callback, LanguageInfo, ParserTrait};
pub use languages::{RustLanguage, TypeScriptLanguage, JavaScriptLanguage, PythonLanguage};

// Re-export main types
pub use ast_editor::{AstEditor, Edit, OptimizeImportsResult, Position, Range};
pub use rust_parser::RustParser;
pub use tree_sitter_wrapper::TreeSitterWrapper;
pub use types::*;
pub use typescript_parser::TypeScriptParser;
pub use dependency_extractor::{
    Dependency, DependencyExtractor, DependencyGraph, DependencyType, GraphStats, Import, Location,
};

use anyhow::{Context, Result};
use std::path::Path;

/// Generic code parser that supports multiple languages.
pub struct CodeParser {
    rust_parser: Option<RustParser>,
    typescript_parser: Option<TypeScriptParser>,
    javascript_parser: Option<TypeScriptParser>,
}

impl CodeParser {
    /// Create a new code parser with support for all languages.
    pub fn new() -> Result<Self> {
        Ok(Self {
            rust_parser: Some(RustParser::new()?),
            typescript_parser: Some(TypeScriptParser::new()?),
            javascript_parser: Some(TypeScriptParser::new_javascript()?),
        })
    }

    /// Create a parser for a specific language only.
    pub fn for_language(language: Lang) -> Result<Self> {
        let mut parser = Self {
            rust_parser: None,
            typescript_parser: None,
            javascript_parser: None,
        };

        match language {
            Lang::Rust => {
                parser.rust_parser = Some(RustParser::new()?);
            }
            Lang::TypeScript | Lang::Tsx => {
                parser.typescript_parser = Some(TypeScriptParser::new()?);
            }
            Lang::JavaScript | Lang::Jsx => {
                parser.javascript_parser = Some(TypeScriptParser::new_javascript()?);
            }
            _ => {
                anyhow::bail!("Language {:?} not yet fully supported in CodeParser", language);
            }
        }

        Ok(parser)
    }

    /// Parse a file based on its language.
    pub fn parse_file(&mut self, path: &str, source: &str, language: Lang) -> Result<ParsedFile> {
        match language {
            Lang::Rust => {
                let parser = self
                    .rust_parser
                    .as_mut()
                    .context("Rust parser not initialized")?;
                parser.parse_file(path, source)
            }
            Lang::TypeScript | Lang::Tsx => {
                let parser = self
                    .typescript_parser
                    .as_mut()
                    .context("TypeScript parser not initialized")?;
                parser.parse_file(path, source)
            }
            Lang::JavaScript | Lang::Jsx => {
                let parser = self
                    .javascript_parser
                    .as_mut()
                    .context("JavaScript parser not initialized")?;
                parser.parse_file(path, source)
            }
            _ => {
                anyhow::bail!("Language {:?} not yet fully supported in CodeParser", language);
            }
        }
    }

    /// Parse a file, auto-detecting the language from the path.
    pub fn parse_file_auto(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        let path_buf = Path::new(path);
        let language = Lang::from_path(path_buf)
            .context("Could not determine language from file extension")?;

        self.parse_file(path, source, language)
    }

    /// Parse a Rust file specifically.
    pub fn parse_rust(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Lang::Rust)
    }

    /// Parse a TypeScript file specifically.
    pub fn parse_typescript(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Lang::TypeScript)
    }

    /// Parse a JavaScript file specifically.
    pub fn parse_javascript(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Lang::JavaScript)
    }
}

impl Default for CodeParser {
    fn default() -> Self {
        Self::new().expect("Failed to create CodeParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_path() {
        assert_eq!(
            Lang::from_path(Path::new("test.rs")),
            Some(Lang::Rust)
        );
        assert_eq!(
            Lang::from_path(Path::new("test.ts")),
            Some(Lang::TypeScript)
        );
        assert_eq!(
            Lang::from_path(Path::new("test.js")),
            Some(Lang::JavaScript)
        );
        assert_eq!(Lang::from_path(Path::new("test.py")), Some(Lang::Python));
    }

    #[test]
    fn test_code_parser_creation() {
        let parser = CodeParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parse_rust_auto() {
        let mut parser = CodeParser::new().unwrap();
        let source = "fn test() {}";
        let result = parser.parse_file_auto("test.rs", source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_typescript_auto() {
        let mut parser = CodeParser::new().unwrap();
        let source = "function test() {}";
        let result = parser.parse_file_auto("test.ts", source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_language_specific_parser() {
        let mut parser = CodeParser::for_language(Lang::Rust).unwrap();
        let source = "fn test() {}";
        let result = parser.parse_rust("test.rs", source);
        assert!(result.is_ok());
    }
}
