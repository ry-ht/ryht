//! Output serialization and dumping module.
//!
//! This module provides comprehensive output capabilities for code analysis results:
//! - AST dumping with pretty-printing
//! - Metrics serialization to JSON, YAML, TOML, and CSV
//! - Operations (operands/operators) export
//! - Streaming output for large files
//! - Compression support (gzip)
//! - Flexible formatting and filtering options
//!
//! # Examples
//!
//! ## Dump AST to console
//!
//! ```
//! use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};
//! use cortex_code_analysis::output::{dump_node, DumpConfig};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let code = "fn test() { let x = 42; }";
//! let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
//! let root = parser.get_root();
//!
//! let config = DumpConfig::default();
//! dump_node(parser.get_code(), &root, &config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Export metrics to JSON
//!
//! ```
//! use cortex_code_analysis::spaces::compute_spaces;
//! use cortex_code_analysis::output::{export_metrics, OutputFormat, ExportConfig};
//! use cortex_code_analysis::{Parser, RustLanguage, ParserTrait, Lang};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
//! let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
//! let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;
//!
//! let config = ExportConfig {
//!     format: OutputFormat::Json,
//!     pretty: true,
//!     ..Default::default()
//! };
//!
//! let json = export_metrics(&spaces, &config)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

pub(crate) mod dump;
pub(crate) mod dump_metrics;
pub(crate) mod dump_ops;

pub use dump::*;
pub use dump_metrics::*;
pub use dump_ops::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Output format for serialized data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// TOML format
    Toml,
    /// CSV format (for tabular data)
    Csv,
    /// Plain text format (human-readable)
    Text,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Json
    }
}

impl OutputFormat {
    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Csv => "csv",
            Self::Text => "txt",
        }
    }

    /// Get the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Yaml => "application/yaml",
            Self::Toml => "application/toml",
            Self::Csv => "text/csv",
            Self::Text => "text/plain",
        }
    }
}

/// Configuration for exporting data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Output format
    pub format: OutputFormat,
    /// Enable pretty-printing (for JSON, YAML, TOML)
    pub pretty: bool,
    /// Enable compression (gzip)
    pub compress: bool,
    /// Maximum depth for nested structures (-1 for unlimited)
    pub max_depth: i32,
    /// Include metadata (timestamps, versions, etc.)
    pub include_metadata: bool,
    /// Filter function names (regex patterns)
    pub filter_functions: Option<Vec<String>>,
    /// Exclude empty metrics
    pub exclude_empty: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Json,
            pretty: true,
            compress: false,
            max_depth: -1,
            include_metadata: true,
            filter_functions: None,
            exclude_empty: false,
        }
    }
}

/// Configuration for dumping AST nodes.
#[derive(Debug, Clone)]
pub struct DumpConfig {
    /// The first line of code to dump (None = from beginning)
    pub line_start: Option<usize>,
    /// The last line of code to dump (None = to end)
    pub line_end: Option<usize>,
    /// Maximum depth to traverse (-1 = unlimited)
    pub max_depth: i32,
    /// Use colored output
    pub use_colors: bool,
    /// Show node IDs
    pub show_node_ids: bool,
    /// Show byte positions
    pub show_byte_positions: bool,
}

impl Default for DumpConfig {
    fn default() -> Self {
        Self {
            line_start: None,
            line_end: None,
            max_depth: -1,
            use_colors: true,
            show_node_ids: true,
            show_byte_positions: false,
        }
    }
}

/// Trait for serializable output.
pub trait Serializable: Serialize {
    /// Serialize to the specified format.
    fn serialize_to(&self, config: &ExportConfig) -> Result<String>;

    /// Serialize to a writer with the specified format.
    fn serialize_to_writer<W: Write>(&self, writer: W, config: &ExportConfig) -> Result<()>;
}

/// Helper to serialize any Serialize type to various formats.
pub fn serialize_to_format<T: Serialize>(value: &T, config: &ExportConfig) -> Result<String> {
    let result = match config.format {
        OutputFormat::Json => {
            if config.pretty {
                serde_json::to_string_pretty(value)?
            } else {
                serde_json::to_string(value)?
            }
        }
        OutputFormat::Yaml => serde_yaml::to_string(value)?,
        OutputFormat::Toml => toml::to_string_pretty(value)?,
        OutputFormat::Csv => {
            anyhow::bail!("CSV format requires specific handling for each type")
        }
        OutputFormat::Text => {
            anyhow::bail!("Text format requires specific handling for each type")
        }
    };

    Ok(result)
}

/// Helper to serialize any Serialize type to a writer.
pub fn serialize_to_writer<T: Serialize, W: Write>(
    value: &T,
    mut writer: W,
    config: &ExportConfig,
) -> Result<()> {
    match config.format {
        OutputFormat::Json => {
            if config.pretty {
                serde_json::to_writer_pretty(&mut writer, value)?;
            } else {
                serde_json::to_writer(&mut writer, value)?;
            }
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(value)?;
            writer.write_all(yaml.as_bytes())?;
        }
        OutputFormat::Toml => {
            let toml = toml::to_string_pretty(value)?;
            writer.write_all(toml.as_bytes())?;
        }
        OutputFormat::Csv => {
            anyhow::bail!("CSV format requires specific handling for each type")
        }
        OutputFormat::Text => {
            anyhow::bail!("Text format requires specific handling for each type")
        }
    }

    Ok(())
}

/// Metadata included with exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Timestamp of export
    pub timestamp: String,
    /// Version of cortex-code-analysis
    pub version: String,
    /// Source file path
    pub source_file: Option<String>,
    /// Language
    pub language: Option<String>,
}

impl ExportMetadata {
    /// Create new metadata with current timestamp.
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            source_file: None,
            language: None,
        }
    }

    /// Set the source file.
    pub fn with_source_file(mut self, path: impl Into<String>) -> Self {
        self.source_file = Some(path.into());
        self
    }

    /// Set the language.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}

impl Default for ExportMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_extension() {
        assert_eq!(OutputFormat::Json.extension(), "json");
        assert_eq!(OutputFormat::Yaml.extension(), "yaml");
        assert_eq!(OutputFormat::Toml.extension(), "toml");
        assert_eq!(OutputFormat::Csv.extension(), "csv");
        assert_eq!(OutputFormat::Text.extension(), "txt");
    }

    #[test]
    fn test_output_format_mime_type() {
        assert_eq!(OutputFormat::Json.mime_type(), "application/json");
        assert_eq!(OutputFormat::Yaml.mime_type(), "application/yaml");
        assert_eq!(OutputFormat::Toml.mime_type(), "application/toml");
        assert_eq!(OutputFormat::Csv.mime_type(), "text/csv");
        assert_eq!(OutputFormat::Text.mime_type(), "text/plain");
    }

    #[test]
    fn test_export_config_defaults() {
        let config = ExportConfig::default();
        assert_eq!(config.format, OutputFormat::Json);
        assert!(config.pretty);
        assert!(!config.compress);
        assert_eq!(config.max_depth, -1);
        assert!(config.include_metadata);
    }

    #[test]
    fn test_dump_config_defaults() {
        let config = DumpConfig::default();
        assert!(config.line_start.is_none());
        assert!(config.line_end.is_none());
        assert_eq!(config.max_depth, -1);
        assert!(config.use_colors);
        assert!(config.show_node_ids);
        assert!(!config.show_byte_positions);
    }

    #[test]
    fn test_export_metadata() {
        let meta = ExportMetadata::new()
            .with_source_file("test.rs")
            .with_language("Rust");

        assert_eq!(meta.source_file, Some("test.rs".to_string()));
        assert_eq!(meta.language, Some("Rust".to_string()));
        assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
    }
}
