//! Operands and operators dumping and serialization.
//!
//! This module provides export capabilities for operands and operators:
//! - Console dumping with colored output
//! - JSON/YAML/TOML serialization
//! - Hierarchical structure preservation
//! - CSV export for analysis

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use crate::ops::Ops;
use super::{ExportConfig, ExportMetadata, OutputFormat, serialize_to_format};

/// Serializable representation of operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableOps {
    /// Operation space kind
    pub kind: String,
    /// Operation space name
    pub name: Option<String>,
    /// Start line
    pub start_line: usize,
    /// Operators list
    pub operators: Vec<String>,
    /// Operands list
    pub operands: Vec<String>,
    /// Unique operators count
    pub unique_operators: usize,
    /// Unique operands count
    pub unique_operands: usize,
    /// Total operators
    pub total_operators: usize,
    /// Total operands
    pub total_operands: usize,
    /// Child operation spaces
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SerializableOps>,
}

impl SerializableOps {
    /// Create from an Ops structure.
    pub fn from_ops(ops: &Ops, max_depth: i32, current_depth: i32) -> Self {
        use std::collections::HashSet;

        let unique_operators: HashSet<_> = ops.operators.iter().collect();
        let unique_operands: HashSet<_> = ops.operands.iter().collect();

        let children = if max_depth == -1 || current_depth < max_depth {
            ops.spaces
                .iter()
                .map(|s| Self::from_ops(s, max_depth, current_depth + 1))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            kind: format!("{:?}", ops.kind),
            name: ops.name.clone(),
            start_line: ops.start_line,
            operators: ops.operators.clone(),
            operands: ops.operands.clone(),
            unique_operators: unique_operators.len(),
            unique_operands: unique_operands.len(),
            total_operators: ops.operators.len(),
            total_operands: ops.operands.len(),
            children,
        }
    }
}

/// Flattened operations for CSV export.
#[derive(Debug, Clone, Serialize)]
pub struct FlatOps {
    pub kind: String,
    pub name: String,
    pub start_line: usize,
    pub unique_operators: usize,
    pub unique_operands: usize,
    pub total_operators: usize,
    pub total_operands: usize,
    pub operators: String,
    pub operands: String,
}

impl FlatOps {
    /// Create from a SerializableOps.
    pub fn from_serializable_ops(ops: &SerializableOps) -> Self {
        Self {
            kind: ops.kind.clone(),
            name: ops.name.clone().unwrap_or_else(|| "<anonymous>".to_string()),
            start_line: ops.start_line,
            unique_operators: ops.unique_operators,
            unique_operands: ops.unique_operands,
            total_operators: ops.total_operators,
            total_operands: ops.total_operands,
            operators: ops.operators.join(", "),
            operands: ops.operands.join(", "),
        }
    }
}

/// Dumps operations to stdout with pretty-printing.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::Lang;
/// use cortex_code_analysis::ops::extract_ops;
/// use cortex_code_analysis::output::dump_ops;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
/// let ops = extract_ops(code, Lang::Rust)?;
///
/// dump_ops(&ops, true)?;
/// # Ok(())
/// # }
/// ```
pub fn dump_ops(ops: &Ops, use_colors: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    dump_space(ops, "", true, &mut stdout, use_colors)?;
    Ok(())
}

/// Helper function for recursive operations dumping.
fn dump_space<W: Write>(
    ops: &Ops,
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    // Write space header
    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        write!(writer, "\x1b[1;33m{:?}: \x1b[0m", ops.kind)?;
        write!(writer, "\x1b[1;36m{}\x1b[0m", ops.name.as_deref().unwrap_or(""))?;
        write!(writer, "\x1b[1;31m (@{})\x1b[0m\n", ops.start_line)?;
    } else {
        writeln!(
            writer,
            "{}{}{:?}: {} (@{})",
            prefix,
            pref,
            ops.kind,
            ops.name.as_deref().unwrap_or(""),
            ops.start_line
        )?;
    }

    let prefix = format!("{}{}", prefix, pref_child);
    dump_ops_values("operators", &ops.operators, &prefix, false, writer, use_colors)?;
    dump_ops_values("operands", &ops.operands, &prefix, ops.spaces.is_empty(), writer, use_colors)?;

    // Recursively dump child spaces
    if let Some((last, spaces)) = ops.spaces.split_last() {
        for s in spaces {
            dump_space(s, &prefix, false, writer, use_colors)?;
        }
        dump_space(last, &prefix, true, writer, use_colors)?;
    }

    Ok(())
}

/// Dump a list of operations values.
fn dump_ops_values<W: Write>(
    name: &str,
    ops: &[String],
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    if ops.is_empty() {
        return Ok(());
    }

    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        writeln!(writer, "\x1b[32m{}\x1b[0m", name)?;
    } else {
        writeln!(writer, "{}{}{}", prefix, pref, name)?;
    }

    let prefix = format!("{}{}", prefix, pref_child);
    for (i, op) in ops.iter().enumerate() {
        let is_last = i == ops.len() - 1;
        let item_pref = if is_last { "`- " } else { "|- " };

        if use_colors {
            write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, item_pref)?;
            writeln!(writer, "\x1b[37m{}\x1b[0m", op)?;
        } else {
            writeln!(writer, "{}{}{}", prefix, item_pref, op)?;
        }
    }

    Ok(())
}

/// Export operations in the specified format.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::Lang;
/// use cortex_code_analysis::ops::extract_ops;
/// use cortex_code_analysis::output::{export_ops, ExportConfig, OutputFormat};
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
/// let ops = extract_ops(code, Lang::Rust)?;
///
/// let config = ExportConfig {
///     format: OutputFormat::Json,
///     pretty: true,
///     ..Default::default()
/// };
///
/// let json = export_ops(&ops, &config)?;
/// println!("{}", json);
/// # Ok(())
/// # }
/// ```
pub fn export_ops(ops: &Ops, config: &ExportConfig) -> Result<String> {
    match config.format {
        OutputFormat::Csv => export_ops_csv(ops),
        _ => export_ops_structured(ops, config),
    }
}

/// Export operations in a structured format (JSON, YAML, TOML).
fn export_ops_structured(ops: &Ops, config: &ExportConfig) -> Result<String> {
    let serializable = SerializableOps::from_ops(ops, config.max_depth, 0);

    #[derive(Serialize)]
    struct OpsExport {
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<ExportMetadata>,
        operations: SerializableOps,
    }

    let export = OpsExport {
        metadata: if config.include_metadata {
            Some(ExportMetadata::new())
        } else {
            None
        },
        operations: serializable,
    };

    serialize_to_format(&export, config)
}

/// Export operations as CSV.
fn export_ops_csv(ops: &Ops) -> Result<String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    fn flatten_ops(ops: &Ops, flat_list: &mut Vec<FlatOps>) {
        let serializable = SerializableOps::from_ops(ops, -1, 0);
        flat_list.push(FlatOps::from_serializable_ops(&serializable));

        for child in &ops.spaces {
            flatten_ops(child, flat_list);
        }
    }

    let mut flat_list = Vec::new();
    flatten_ops(ops, &mut flat_list);

    for op in &flat_list {
        wtr.serialize(op)?;
    }

    let data = wtr.into_inner()?;
    Ok(String::from_utf8(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage, ParserTrait, Lang};
    use crate::ops::extract_ops;
    use std::path::Path;

    #[test]
    fn test_dump_ops() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let ops = extract_ops(code, Lang::Rust)?;

        dump_ops(&ops, false)?;
        Ok(())
    }

    #[test]
    fn test_export_ops_json() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let ops = extract_ops(code, Lang::Rust)?;

        let config = ExportConfig {
            format: OutputFormat::Json,
            pretty: true,
            include_metadata: false,
            ..Default::default()
        };

        let json = export_ops(&ops, &config)?;
        assert!(json.contains("\"operations\""));
        assert!(json.contains("\"operators\""));
        assert!(json.contains("\"operands\""));
        Ok(())
    }

    #[test]
    fn test_export_ops_yaml() -> Result<()> {
        let code = "fn test() { let x = 1; }";
        let ops = extract_ops(code, Lang::Rust)?;

        let config = ExportConfig {
            format: OutputFormat::Yaml,
            include_metadata: false,
            ..Default::default()
        };

        let yaml = export_ops(&ops, &config)?;
        assert!(yaml.contains("operations:"));
        Ok(())
    }

    #[test]
    fn test_export_ops_csv() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let ops = extract_ops(code, Lang::Rust)?;

        let csv = export_ops_csv(&ops)?;
        assert!(csv.contains("kind"));
        assert!(csv.contains("operators"));
        assert!(csv.contains("operands"));
        Ok(())
    }

    #[test]
    fn test_serializable_ops() -> Result<()> {
        let code = "fn test() { let x = 1 + 2; }";
        let ops = extract_ops(code, Lang::Rust)?;

        let serializable = SerializableOps::from_ops(&ops, -1, 0);
        assert!(serializable.total_operators > 0 || serializable.total_operands > 0);

        Ok(())
    }
}
