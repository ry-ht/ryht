//! AST dumping with pretty-printing and serialization support.
//!
//! This module provides comprehensive AST visualization and export capabilities:
//! - Console dumping with colored output
//! - JSON/YAML/TOML serialization of AST structure
//! - Configurable depth limiting
//! - Line range filtering
//! - Node metadata display

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use crate::node::Node;
use crate::traits::ParserTrait;
use super::{DumpConfig, ExportConfig, ExportMetadata, serialize_to_format};

/// Serializable representation of an AST node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableNode {
    /// Node type/kind
    pub kind: String,
    /// Node kind ID
    pub kind_id: u16,
    /// Start position (row, column)
    pub start_position: (usize, usize),
    /// End position (row, column)
    pub end_position: (usize, usize),
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Text content (if single line)
    pub text: Option<String>,
    /// Child nodes
    pub children: Vec<SerializableNode>,
}

impl SerializableNode {
    /// Create from a tree-sitter node.
    pub fn from_node(node: &Node, code: &[u8], max_depth: i32, current_depth: i32) -> Self {
        let (start_row, start_col) = node.start_position();
        let (end_row, end_col) = node.end_position();

        let text = if node.start_row() == node.end_row() {
            let bytes = &code[node.start_byte()..node.end_byte()];
            String::from_utf8(bytes.to_vec()).ok()
        } else {
            None
        };

        let children = if max_depth == -1 || current_depth < max_depth {
            let mut cursor = node.cursor();
            let mut children = Vec::new();

            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    children.push(Self::from_node(&child, code, max_depth, current_depth + 1));
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            children
        } else {
            Vec::new()
        };

        Self {
            kind: node.kind().to_string(),
            kind_id: node.kind_id(),
            start_position: (start_row + 1, start_col + 1),
            end_position: (end_row + 1, end_col + 1),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            text,
            children,
        }
    }
}

/// Dumps the AST of code to stdout with pretty-printing.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};
/// use cortex_code_analysis::output::{dump_node, DumpConfig};
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn test() { let x = 42; }";
/// let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
/// let root = parser.get_root();
///
/// let config = DumpConfig::default();
/// dump_node(parser.get_code(), &root, &config)?;
/// # Ok(())
/// # }
/// ```
pub fn dump_node(code: &[u8], node: &Node, config: &DumpConfig) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    dump_tree_helper(code, node, "", true, &mut stdout, config, 0)?;
    Ok(())
}

/// Helper function for recursive tree dumping.
fn dump_tree_helper<W: Write>(
    code: &[u8],
    node: &Node,
    prefix: &str,
    last: bool,
    writer: &mut W,
    config: &DumpConfig,
    current_depth: i32,
) -> Result<()> {
    // Check depth limit
    if config.max_depth != -1 && current_depth >= config.max_depth {
        return Ok(());
    }

    // Check line range filter
    let node_row = node.start_row() + 1;
    let mut display = true;
    if let Some(line_start) = config.line_start {
        display = node_row >= line_start;
    }
    if let Some(line_end) = config.line_end {
        display = display && node_row <= line_end;
    }

    if !display {
        return Ok(());
    }

    let (pref_child, pref) = if node.parent().is_none() {
        ("", "")
    } else if last {
        ("   ", "╰─ ")
    } else {
        ("│  ", "├─ ")
    };

    // Write node information
    if config.use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
    } else {
        write!(writer, "{}{}", prefix, pref)?;
    }

    // Kind and kind ID
    if config.use_colors {
        write!(writer, "\x1b[1;33m")?;
    }
    write!(writer, "{{{}}}", node.kind())?;
    if config.show_node_ids {
        write!(writer, ":{}", node.kind_id())?;
    }
    if config.use_colors {
        write!(writer, "\x1b[0m")?;
    }
    write!(writer, " ")?;

    // Position information
    if config.use_colors {
        write!(writer, "\x1b[37mfrom \x1b[32m")?;
    } else {
        write!(writer, "from ")?;
    }
    let (start_row, start_col) = node.start_position();
    write!(writer, "({}, {}) ", start_row + 1, start_col + 1)?;

    if config.use_colors {
        write!(writer, "\x1b[37mto \x1b[32m")?;
    } else {
        write!(writer, "to ")?;
    }
    let (end_row, end_col) = node.end_position();
    write!(writer, "({}, {}) ", end_row + 1, end_col + 1)?;

    // Byte positions (optional)
    if config.show_byte_positions {
        if config.use_colors {
            write!(writer, "\x1b[37m")?;
        }
        write!(
            writer,
            "[{}-{}] ",
            node.start_byte(),
            node.end_byte()
        )?;
    }

    // Text content (if single line)
    if node.start_row() == node.end_row() {
        if config.use_colors {
            write!(writer, "\x1b[37m: \x1b[1;31m")?;
        } else {
            write!(writer, ": ")?;
        }

        let bytes = &code[node.start_byte()..node.end_byte()];
        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
            write!(writer, "{}", text)?;
        } else {
            write!(writer, "<binary>")?;
        }
    }

    if config.use_colors {
        write!(writer, "\x1b[0m")?;
    }
    writeln!(writer)?;

    // Recursively dump children
    let child_count = node.child_count();
    if child_count != 0 {
        let prefix = format!("{}{}", prefix, pref_child);
        let mut i = child_count;
        let mut cursor = node.cursor();
        cursor.goto_first_child();

        loop {
            i -= 1;
            dump_tree_helper(
                code,
                &cursor.node(),
                &prefix,
                i == 0,
                writer,
                config,
                current_depth + 1,
            )?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    Ok(())
}

/// Export AST as a serializable structure.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};
/// use cortex_code_analysis::output::{export_ast, ExportConfig, OutputFormat};
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn test() {}";
/// let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
///
/// let config = ExportConfig {
///     format: OutputFormat::Json,
///     pretty: true,
///     max_depth: 3,
///     ..Default::default()
/// };
///
/// let json = export_ast(&parser, &config)?;
/// println!("{}", json);
/// # Ok(())
/// # }
/// ```
pub fn export_ast<T: ParserTrait>(parser: &T, config: &ExportConfig) -> Result<String> {
    let root = parser.get_root();
    let code = parser.get_code();

    let serializable = SerializableNode::from_node(&root, code, config.max_depth, 0);

    #[derive(Serialize)]
    struct AstExport {
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<ExportMetadata>,
        ast: SerializableNode,
    }

    let export = AstExport {
        metadata: if config.include_metadata {
            Some(ExportMetadata::new())
        } else {
            None
        },
        ast: serializable,
    };

    serialize_to_format(&export, config)
}

/// Callback for dumping AST using the Callback trait pattern.
pub struct Dump {
    _guard: (),
}

impl crate::traits::Callback for Dump {
    type Res = Result<()>;
    type Cfg = DumpConfig;

    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res {
        dump_node(parser.get_code(), &parser.get_root(), &cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage, ParserTrait};
    use std::path::Path;

    #[test]
    fn test_dump_node() -> Result<()> {
        let code = "fn test() { let x = 42; }";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();

        let config = DumpConfig {
            use_colors: false,
            ..Default::default()
        };

        dump_node(parser.get_code(), &root, &config)?;
        Ok(())
    }

    #[test]
    fn test_serializable_node() -> Result<()> {
        let code = "fn test() {}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();

        let serializable = SerializableNode::from_node(&root, parser.get_code(), -1, 0);
        assert_eq!(serializable.kind, "source_file");
        assert!(!serializable.children.is_empty());

        Ok(())
    }

    #[test]
    fn test_export_ast_json() -> Result<()> {
        let code = "fn test() {}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;

        let config = ExportConfig {
            format: OutputFormat::Json,
            pretty: true,
            max_depth: 2,
            include_metadata: false,
            ..Default::default()
        };

        let json = export_ast(&parser, &config)?;
        assert!(json.contains("\"kind\""));
        assert!(json.contains("\"ast\""));

        Ok(())
    }

    #[test]
    fn test_export_ast_yaml() -> Result<()> {
        let code = "fn test() {}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;

        let config = ExportConfig {
            format: OutputFormat::Yaml,
            max_depth: 2,
            include_metadata: false,
            ..Default::default()
        };

        let yaml = export_ast(&parser, &config)?;
        assert!(yaml.contains("kind:"));
        assert!(yaml.contains("ast:"));

        Ok(())
    }

    #[test]
    fn test_dump_with_depth_limit() -> Result<()> {
        let code = "fn test() { if true { let x = 1; } }";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();

        let config = DumpConfig {
            max_depth: 2,
            use_colors: false,
            ..Default::default()
        };

        dump_node(parser.get_code(), &root, &config)?;
        Ok(())
    }

    #[test]
    fn test_dump_with_line_range() -> Result<()> {
        let code = "fn test() {\n    let x = 1;\n    let y = 2;\n}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();

        let config = DumpConfig {
            line_start: Some(2),
            line_end: Some(3),
            use_colors: false,
            ..Default::default()
        };

        dump_node(parser.get_code(), &root, &config)?;
        Ok(())
    }
}
