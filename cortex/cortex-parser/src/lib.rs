//! Cortex Parser - Tree-sitter based code parsing infrastructure.
//!
//! This crate provides high-level parsing capabilities for multiple programming languages
//! using tree-sitter. It extracts structured information about code elements including
//! functions, structs, enums, traits, and more.
//!
//! # Examples
//!
//! ```
//! use cortex_parser::{RustParser, ParsedFile};
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

pub mod ast_editor;
pub mod extractor;
pub mod rust_parser;
pub mod tree_sitter_wrapper;
pub mod types;
pub mod typescript_parser;
pub mod dependency_extractor;

// Re-export main types
pub use ast_editor::{AstEditor, Edit, ExtractFunctionResult, OptimizeImportsResult, Position, Range};
pub use rust_parser::RustParser;
pub use tree_sitter_wrapper::TreeSitterWrapper;
pub use types::*;
pub use typescript_parser::TypeScriptParser;
pub use dependency_extractor::{
    Dependency, DependencyExtractor, DependencyGraph, DependencyType, GraphStats, Import, Location,
};

use anyhow::{Context, Result};
use std::path::Path;

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()?.to_str().and_then(|ext| match ext {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            _ => None,
        })
    }

    /// Get file extensions for this language.
    pub fn extensions(&self) -> &[&str] {
        match self {
            Language::Rust => &["rs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx"],
        }
    }
}

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
    pub fn for_language(language: Language) -> Result<Self> {
        let mut parser = Self {
            rust_parser: None,
            typescript_parser: None,
            javascript_parser: None,
        };

        match language {
            Language::Rust => {
                parser.rust_parser = Some(RustParser::new()?);
            }
            Language::TypeScript => {
                parser.typescript_parser = Some(TypeScriptParser::new()?);
            }
            Language::JavaScript => {
                parser.javascript_parser = Some(TypeScriptParser::new_javascript()?);
            }
        }

        Ok(parser)
    }

    /// Parse a file based on its language.
    pub fn parse_file(&mut self, path: &str, source: &str, language: Language) -> Result<ParsedFile> {
        match language {
            Language::Rust => {
                let parser = self
                    .rust_parser
                    .as_mut()
                    .context("Rust parser not initialized")?;
                parser.parse_file(path, source)
            }
            Language::TypeScript => {
                let parser = self
                    .typescript_parser
                    .as_mut()
                    .context("TypeScript parser not initialized")?;
                parser.parse_file(path, source)
            }
            Language::JavaScript => {
                let parser = self
                    .javascript_parser
                    .as_mut()
                    .context("JavaScript parser not initialized")?;
                parser.parse_file(path, source)
            }
        }
    }

    /// Parse a file, auto-detecting the language from the path.
    pub fn parse_file_auto(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        let path_buf = Path::new(path);
        let language = Language::from_path(path_buf)
            .context("Could not determine language from file extension")?;

        self.parse_file(path, source, language)
    }

    /// Parse a Rust file specifically.
    pub fn parse_rust(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Language::Rust)
    }

    /// Parse a TypeScript file specifically.
    pub fn parse_typescript(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Language::TypeScript)
    }

    /// Parse a JavaScript file specifically.
    pub fn parse_javascript(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        self.parse_file(path, source, Language::JavaScript)
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
            Language::from_path(Path::new("test.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("test.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("test.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(Language::from_path(Path::new("test.py")), None);
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
        let mut parser = CodeParser::for_language(Language::Rust).unwrap();
        let source = "fn test() {}";
        let result = parser.parse_rust("test.rs", source);
        assert!(result.is_ok());
    }
}
