//! Cortex Code Analysis - Production-Ready Code Analysis Infrastructure
//!
//! A high-performance, modular code analysis framework built on tree-sitter with:
//! - Multi-language parsing (Rust, TypeScript, JavaScript, Python, C++, Java, Kotlin)
//! - Comprehensive metrics calculation (20+ metrics including complexity, maintainability)
//! - Advanced AST search and transformation
//! - Concurrent and async processing
//! - Intelligent caching for performance
//! - Dependency analysis and graph generation
//!
//! # Quick Start
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
//!
//! # Architecture
//!
//! ## Core Modules
//! - [`lang`] - Language identification and configuration
//! - [`node`] - AST node abstraction
//! - [`parser`] - Generic parser interface
//! - [`traits`] - Core trait definitions
//!
//! ## Parsing & Extraction
//! - [`ast_builder`] - AST construction with full span information
//! - [`ast_editor`] - AST editing and transformation
//! - [`extractor`] - High-level code element extraction
//! - [`function`] - Function detection and analysis
//! - [`dependency_extractor`] - Dependency graph generation
//!
//! ## Metrics & Analysis
//! - [`metrics`] - 20+ code quality metrics with strategy pattern
//! - [`analysis`] - Advanced search, counting, and transformation
//!
//! ## Concurrent Processing
//! - [`concurrent`] - Sync/async concurrent file processing
//!
//! ## Utilities
//! - [`ops`] - Code space operations
//! - [`preprocessor`] - C/C++ preprocessor analysis
//! - [`spaces`] - Function space metrics
//! - [`utils`] - File I/O and utility functions
//! - [`output`] - Serialization and export
//!
//! # Performance Features
//!
//! - LRU caching for parsed ASTs and computed metrics
//! - Stack-based iterative traversal (no recursion overhead)
//! - Parallel metrics calculation
//! - Async/await concurrent processing
//! - Pre-allocated data structures
//! - Memory-efficient operations

// ============================================================================
// Core Abstractions
// ============================================================================

pub mod lang;
pub mod node;
pub mod parser;
pub mod traits;
pub mod languages;

// ============================================================================
// Parsing & AST Operations
// ============================================================================

pub mod ast_builder;
pub mod ast_editor;
pub mod comment_removal;
pub mod extractor;
pub mod function;
pub mod rust_parser;
pub mod tree_sitter_wrapper;
pub mod types;
pub mod typescript_parser;

// ============================================================================
// Advanced Analysis
// ============================================================================

pub mod analysis;
pub mod dependency_extractor;
pub mod metrics;
pub mod ops;
pub mod preprocessor;
pub mod spaces;

// ============================================================================
// Concurrent Processing
// ============================================================================

pub mod concurrent;

// ============================================================================
// Output & Utilities
// ============================================================================

pub mod output;
pub mod utils;

// Re-export core abstractions
pub use lang::Lang;
pub use node::Node;
pub use parser::Parser;
pub use traits::{Callback, LanguageInfo, ParserTrait};
pub use languages::{
    RustLanguage, TypeScriptLanguage, JavaScriptLanguage, PythonLanguage,
    CppLanguage, JavaLanguage, KotlinLanguage, TsxLanguage as TsxLang,
};

// Re-export main types
pub use ast_builder::{build_ast, build_ast_with_config, AstConfig, AstNode, Span};
pub use ast_editor::{AstEditor, Edit, OptimizeImportsResult, Position, Range};
pub use comment_removal::{extract_comments, remove_comments, CommentSpan};
pub use rust_parser::RustParser;
pub use tree_sitter_wrapper::TreeSitterWrapper;
pub use types::*;
pub use typescript_parser::TypeScriptParser;
pub use dependency_extractor::{
    Dependency, DependencyExtractor, DependencyGraph, DependencyType, GraphStats, Import, Location,
};
pub use function::{detect_functions, FunctionSpan};
pub use ops::{extract_ops, Ops, SpaceKind};
pub use preprocessor::{
    PreprocFile, PreprocResults, extract_preprocessor, build_include_graph, get_all_macros,
};
pub use spaces::{compute_spaces, FuncSpace, SpaceMetrics};

// Re-export output module types
pub use output::{
    dump_node, dump_metrics, dump_ops, export_ast, export_metrics, export_ops,
    DumpConfig, ExportConfig, ExportMetadata, OutputFormat, Serializable,
};

// Re-export utility functions
pub use utils::{
    read_file_with_bom,
    guess_language_from_content,
    normalize_path,
    get_paths_dist,
};

// Re-export advanced analysis types
pub use analysis::{
    // Search and navigation
    AstFinder, FindConfig, FindConfigBuilder, FindResult, NodeFilter,
    // Counting and statistics
    AstCounter, ConcurrentCounter, CountConfig, CountFilter, CountStats,
    // AST transformation
    Alterator, TransformConfig, TransformConfigBuilder,
    // Caching
    Cache, CacheManager, CacheBuilder, AstCache, MetricsCache, SearchCache,
    CachedAst, CachedMetrics, CachedSearch, SourceKey, SearchKey,
    // Node analysis
    NodeChecker, DefaultNodeChecker, NodeGetter, DefaultNodeGetter,
    HalsteadType,
};

// Re-export metrics strategy types
pub use metrics::{
    CodeMetrics,
    MetricsStrategy, MetricsCalculatorType, MetricsBuilder, MetricsAggregator,
};

// Re-export concurrent types
pub use concurrent::{
    ConcurrentRunner, FilesData,
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

    #[test]
    fn test_detect_functions_integration() {
        // Test the function detection API
        let rust_code = r#"
fn main() {
    println!("Hello!");
}

fn helper() -> i32 {
    42
}
"#;
        let functions = detect_functions(rust_code, Lang::Rust).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "main");
        assert_eq!(functions[1].name, "helper");

        // Test with Python
        let python_code = "def greet():\n    print('hi')\n\ndef farewell():\n    print('bye')";
        let functions = detect_functions(python_code, Lang::Python).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "greet");
        assert_eq!(functions[1].name, "farewell");
    }

    #[test]
    fn test_function_span_utilities() {
        let code = "fn test() {\n    let x = 1;\n    let y = 2;\n}";
        let functions = detect_functions(code, Lang::Rust).unwrap();
        assert_eq!(functions.len(), 1);

        let func = &functions[0];
        assert_eq!(func.name, "test");
        assert_eq!(func.line_count(), 4);
        assert!(func.contains_line(2));
        assert!(!func.contains_line(10));
    }
}
