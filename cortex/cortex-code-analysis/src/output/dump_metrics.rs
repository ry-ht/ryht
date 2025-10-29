//! Metrics dumping and serialization.
//!
//! This module provides comprehensive metrics export capabilities:
//! - Console dumping with colored output
//! - JSON/YAML/TOML/CSV serialization
//! - Hierarchical metrics aggregation
//! - Filtering and custom formatting
//! - Metadata inclusion

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

use crate::spaces::{FuncSpace, SpaceMetrics};
use super::{ExportConfig, ExportMetadata, OutputFormat, serialize_to_format};

/// Serializable representation of a function space with metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSpace {
    /// Space kind (function, class, module, etc.)
    pub kind: String,
    /// Space name
    pub name: Option<String>,
    /// Start line
    pub start_line: usize,
    /// End line
    pub end_line: usize,
    /// Metrics for this space
    pub metrics: SpaceMetrics,
    /// Child spaces
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SerializableSpace>,
}

impl SerializableSpace {
    /// Create from a FuncSpace.
    pub fn from_func_space(space: &FuncSpace, max_depth: i32, current_depth: i32) -> Self {
        let children = if max_depth == -1 || current_depth < max_depth {
            space
                .spaces
                .iter()
                .map(|s| Self::from_func_space(s, max_depth, current_depth + 1))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            kind: format!("{:?}", space.kind),
            name: space.name.clone(),
            start_line: space.start_line,
            end_line: space.end_line,
            metrics: space.metrics.clone(),
            children,
        }
    }
}

/// Flattened metrics for CSV export.
#[derive(Debug, Clone, Serialize)]
pub struct FlatMetrics {
    pub kind: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,

    // LOC metrics
    pub sloc: f64,
    pub ploc: f64,
    pub lloc: f64,
    pub cloc: f64,
    pub blank: f64,

    // Complexity metrics
    pub cyclomatic: f64,
    pub cyclomatic_avg: f64,
    pub cognitive: f64,
    pub cognitive_avg: f64,

    // Halstead metrics
    /// Number of unique operators (n1 in Halstead metrics)
    pub halstead_unique_operators: f64,
    /// Total number of operators (N1 in Halstead metrics)
    pub halstead_total_operators: f64,
    /// Number of unique operands (n2 in Halstead metrics)
    pub halstead_unique_operands: f64,
    /// Total number of operands (N2 in Halstead metrics)
    pub halstead_total_operands: f64,
    pub halstead_vocabulary: f64,
    pub halstead_length: f64,
    pub halstead_volume: f64,
    pub halstead_difficulty: f64,
    pub halstead_effort: f64,
    pub halstead_time: f64,
    pub halstead_bugs: f64,

    // Other metrics
    pub nargs_total: f64,
    pub nargs_avg: f64,
    pub exit: f64,
    pub nom_total: f64,
    pub mi_original: f64,
    pub mi_sei: f64,
    pub mi_visual_studio: f64,

    // ABC metrics
    pub abc_assignments: f64,
    pub abc_branches: f64,
    pub abc_conditions: f64,
    pub abc_magnitude: f64,
}

impl FlatMetrics {
    /// Create from a SerializableSpace.
    pub fn from_serializable_space(space: &SerializableSpace) -> Self {
        let m = &space.metrics;

        Self {
            kind: space.kind.clone(),
            name: space.name.clone().unwrap_or_else(|| "<anonymous>".to_string()),
            start_line: space.start_line,
            end_line: space.end_line,

            sloc: m.loc.sloc(),
            ploc: m.loc.ploc(),
            lloc: m.loc.lloc(),
            cloc: m.loc.cloc(),
            blank: m.loc.blank(),

            cyclomatic: m.cyclomatic.cyclomatic(),
            cyclomatic_avg: m.cyclomatic.cyclomatic_average(),
            cognitive: m.cognitive.cognitive(),
            cognitive_avg: m.cognitive.cognitive_average(),

            halstead_unique_operators: m.halstead.u_operators(),
            halstead_total_operators: m.halstead.operators(),
            halstead_unique_operands: m.halstead.u_operands(),
            halstead_total_operands: m.halstead.operands(),
            halstead_vocabulary: m.halstead.vocabulary(),
            halstead_length: m.halstead.length(),
            halstead_volume: m.halstead.volume(),
            halstead_difficulty: m.halstead.difficulty(),
            halstead_effort: m.halstead.effort(),
            halstead_time: m.halstead.time(),
            halstead_bugs: m.halstead.bugs(),

            nargs_total: m.nargs.nargs_total(),
            nargs_avg: m.nargs.nargs_average(),
            exit: m.exit.exit(),
            nom_total: m.nom.total(),
            mi_original: m.mi.mi_original(),
            mi_sei: m.mi.mi_sei(),
            mi_visual_studio: m.mi.mi_visual_studio(),

            abc_assignments: m.abc.assignments_sum(),
            abc_branches: m.abc.branches_sum(),
            abc_conditions: m.abc.conditions_sum(),
            abc_magnitude: m.abc.magnitude_sum(),
        }
    }
}

/// Dumps metrics to stdout with pretty-printing.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Parser, RustLanguage, ParserTrait, Lang};
/// use cortex_code_analysis::spaces::compute_spaces;
/// use cortex_code_analysis::output::dump_metrics;
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
/// let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
/// let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;
///
/// dump_metrics(&spaces, true)?;
/// # Ok(())
/// # }
/// ```
pub fn dump_metrics(space: &FuncSpace, use_colors: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    dump_space(space, "", true, &mut stdout, use_colors)?;
    Ok(())
}

/// Helper function for recursive metrics dumping.
fn dump_space<W: Write>(
    space: &FuncSpace,
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    // Write space header
    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        write!(writer, "\x1b[1;33m{:?}: \x1b[0m", space.kind)?;
        write!(writer, "\x1b[1;36m{}\x1b[0m", space.name.as_deref().unwrap_or(""))?;
        write!(writer, "\x1b[1;31m (@{})\x1b[0m\n", space.start_line)?;
    } else {
        writeln!(
            writer,
            "{}{}{:?}: {} (@{})",
            prefix,
            pref,
            space.kind,
            space.name.as_deref().unwrap_or(""),
            space.start_line
        )?;
    }

    let prefix = format!("{}{}", prefix, pref_child);
    dump_metrics_inner(&space.metrics, &prefix, space.spaces.is_empty(), writer, use_colors)?;

    // Recursively dump child spaces
    if let Some((last, spaces)) = space.spaces.split_last() {
        for s in spaces {
            dump_space(s, &prefix, false, writer, use_colors)?;
        }
        dump_space(last, &prefix, true, writer, use_colors)?;
    }

    Ok(())
}

/// Dump metrics values.
fn dump_metrics_inner<W: Write>(
    metrics: &SpaceMetrics,
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    let (pref_child, pref) = if last { ("   ", "`- ") } else { ("|  ", "|- ") };

    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        writeln!(writer, "\x1b[1;33mmetrics\x1b[0m")?;
    } else {
        writeln!(writer, "{}{}metrics", prefix, pref)?;
    }

    let prefix = format!("{}{}", prefix, pref_child);

    // LOC metrics
    dump_section("loc", writer, &prefix, use_colors)?;
    let loc_prefix = format!("{}   ", prefix);
    dump_value("sloc", metrics.loc.sloc(), &loc_prefix, false, writer, use_colors)?;
    dump_value("ploc", metrics.loc.ploc(), &loc_prefix, false, writer, use_colors)?;
    dump_value("lloc", metrics.loc.lloc(), &loc_prefix, false, writer, use_colors)?;
    dump_value("cloc", metrics.loc.cloc(), &loc_prefix, false, writer, use_colors)?;
    dump_value("blank", metrics.loc.blank(), &loc_prefix, true, writer, use_colors)?;

    // Complexity metrics
    dump_section("cyclomatic", writer, &prefix, use_colors)?;
    let cyc_prefix = format!("{}   ", prefix);
    dump_value("sum", metrics.cyclomatic.cyclomatic(), &cyc_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.cyclomatic.cyclomatic_average(), &cyc_prefix, true, writer, use_colors)?;

    dump_section("cognitive", writer, &prefix, use_colors)?;
    let cog_prefix = format!("{}   ", prefix);
    dump_value("sum", metrics.cognitive.cognitive(), &cog_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.cognitive.cognitive_average(), &cog_prefix, true, writer, use_colors)?;

    // Halstead metrics
    dump_section("halstead", writer, &prefix, use_colors)?;
    let hal_prefix = format!("{}   ", prefix);
    dump_value("n1", metrics.halstead.u_operators(), &hal_prefix, false, writer, use_colors)?;
    dump_value("N1", metrics.halstead.operators(), &hal_prefix, false, writer, use_colors)?;
    dump_value("n2", metrics.halstead.u_operands(), &hal_prefix, false, writer, use_colors)?;
    dump_value("N2", metrics.halstead.operands(), &hal_prefix, false, writer, use_colors)?;
    dump_value("vocabulary", metrics.halstead.vocabulary(), &hal_prefix, false, writer, use_colors)?;
    dump_value("length", metrics.halstead.length(), &hal_prefix, false, writer, use_colors)?;
    dump_value("volume", metrics.halstead.volume(), &hal_prefix, false, writer, use_colors)?;
    dump_value("difficulty", metrics.halstead.difficulty(), &hal_prefix, false, writer, use_colors)?;
    dump_value("effort", metrics.halstead.effort(), &hal_prefix, false, writer, use_colors)?;
    dump_value("time", metrics.halstead.time(), &hal_prefix, false, writer, use_colors)?;
    dump_value("bugs", metrics.halstead.bugs(), &hal_prefix, true, writer, use_colors)?;

    // Other metrics
    dump_section("nargs", writer, &prefix, use_colors)?;
    let nargs_prefix = format!("{}   ", prefix);
    dump_value("total", metrics.nargs.nargs_total(), &nargs_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.nargs.nargs_average(), &nargs_prefix, true, writer, use_colors)?;

    dump_value_inline("exit", metrics.exit.exit(), &prefix, false, writer, use_colors)?;

    dump_section("nom", writer, &prefix, use_colors)?;
    let nom_prefix = format!("{}   ", prefix);
    dump_value("functions", metrics.nom.functions(), &nom_prefix, false, writer, use_colors)?;
    dump_value("closures", metrics.nom.closures(), &nom_prefix, false, writer, use_colors)?;
    dump_value("total", metrics.nom.total(), &nom_prefix, true, writer, use_colors)?;

    dump_section("mi", writer, &prefix, use_colors)?;
    let mi_prefix = format!("{}   ", prefix);
    dump_value("mi_original", metrics.mi.mi_original(), &mi_prefix, false, writer, use_colors)?;
    dump_value("mi_sei", metrics.mi.mi_sei(), &mi_prefix, false, writer, use_colors)?;
    dump_value("mi_visual_studio", metrics.mi.mi_visual_studio(), &mi_prefix, true, writer, use_colors)?;

    dump_section("abc", writer, &prefix, use_colors)?;
    let abc_prefix = format!("{}   ", prefix);
    dump_value("assignments", metrics.abc.assignments_sum(), &abc_prefix, false, writer, use_colors)?;
    dump_value("branches", metrics.abc.branches_sum(), &abc_prefix, false, writer, use_colors)?;
    dump_value("conditions", metrics.abc.conditions_sum(), &abc_prefix, false, writer, use_colors)?;
    dump_value("magnitude", metrics.abc.magnitude_sum(), &abc_prefix, true, writer, use_colors)?;

    // WMC
    dump_section("wmc", writer, &prefix, use_colors)?;
    let wmc_prefix = format!("{}   ", prefix);
    dump_value("sum", metrics.wmc.wmc_sum(), &wmc_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.wmc.wmc_average(), &wmc_prefix, false, writer, use_colors)?;
    dump_value("min", metrics.wmc.wmc_min(), &wmc_prefix, false, writer, use_colors)?;
    dump_value("max", metrics.wmc.wmc_max(), &wmc_prefix, true, writer, use_colors)?;

    // NPM
    dump_section("npm", writer, &prefix, use_colors)?;
    let npm_prefix = format!("{}   ", prefix);
    dump_value("sum", metrics.npm.npm_sum(), &npm_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.npm.npm_average(), &npm_prefix, false, writer, use_colors)?;
    dump_value("min", metrics.npm.npm_min(), &npm_prefix, false, writer, use_colors)?;
    dump_value("max", metrics.npm.npm_max(), &npm_prefix, true, writer, use_colors)?;

    // NPA
    dump_section("npa", writer, &prefix, use_colors)?;
    let npa_prefix = format!("{}   ", prefix);
    dump_value("sum", metrics.npa.npa_sum(), &npa_prefix, false, writer, use_colors)?;
    dump_value("average", metrics.npa.npa_average(), &npa_prefix, false, writer, use_colors)?;
    dump_value("min", metrics.npa.npa_min(), &npa_prefix, false, writer, use_colors)?;
    dump_value("max", metrics.npa.npa_max(), &npa_prefix, true, writer, use_colors)?;

    Ok(())
}

fn dump_section<W: Write>(name: &str, writer: &mut W, prefix: &str, use_colors: bool) -> Result<()> {
    if use_colors {
        write!(writer, "\x1b[34m{}|- \x1b[0m", prefix)?;
        writeln!(writer, "\x1b[32m{}\x1b[0m", name)?;
    } else {
        writeln!(writer, "{}|- {}", prefix, name)?;
    }
    Ok(())
}

fn dump_value<W: Write>(
    name: &str,
    val: f64,
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    let pref = if last { "`- " } else { "|- " };

    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        write!(writer, "\x1b[1;35m{}: \x1b[0m", name)?;
        writeln!(writer, "\x1b[37m{}\x1b[0m", val)?;
    } else {
        writeln!(writer, "{}{}{}: {}", prefix, pref, name, val)?;
    }
    Ok(())
}

fn dump_value_inline<W: Write>(
    name: &str,
    val: f64,
    prefix: &str,
    last: bool,
    writer: &mut W,
    use_colors: bool,
) -> Result<()> {
    let pref = if last { "`- " } else { "|- " };

    if use_colors {
        write!(writer, "\x1b[34m{}{}\x1b[0m", prefix, pref)?;
        write!(writer, "\x1b[32m{}: \x1b[0m", name)?;
        writeln!(writer, "\x1b[37m{}\x1b[0m", val)?;
    } else {
        writeln!(writer, "{}{}{}: {}", prefix, pref, name, val)?;
    }
    Ok(())
}

/// Export metrics in the specified format.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Parser, RustLanguage, ParserTrait, Lang};
/// use cortex_code_analysis::spaces::compute_spaces;
/// use cortex_code_analysis::output::{export_metrics, ExportConfig, OutputFormat};
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
/// let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
/// let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;
///
/// let config = ExportConfig {
///     format: OutputFormat::Json,
///     pretty: true,
///     ..Default::default()
/// };
///
/// let json = export_metrics(&spaces, &config)?;
/// println!("{}", json);
/// # Ok(())
/// # }
/// ```
pub fn export_metrics(space: &FuncSpace, config: &ExportConfig) -> Result<String> {
    match config.format {
        OutputFormat::Csv => export_metrics_csv(space),
        _ => export_metrics_structured(space, config),
    }
}

/// Export metrics in a structured format (JSON, YAML, TOML).
fn export_metrics_structured(space: &FuncSpace, config: &ExportConfig) -> Result<String> {
    let serializable = SerializableSpace::from_func_space(space, config.max_depth, 0);

    #[derive(Serialize)]
    struct MetricsExport {
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<ExportMetadata>,
        metrics: SerializableSpace,
    }

    let export = MetricsExport {
        metadata: if config.include_metadata {
            Some(ExportMetadata::new())
        } else {
            None
        },
        metrics: serializable,
    };

    serialize_to_format(&export, config)
}

/// Export metrics as CSV.
fn export_metrics_csv(space: &FuncSpace) -> Result<String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    fn flatten_spaces(space: &FuncSpace, flat_list: &mut Vec<FlatMetrics>) {
        let serializable = SerializableSpace::from_func_space(space, -1, 0);
        flat_list.push(FlatMetrics::from_serializable_space(&serializable));

        for child in &space.spaces {
            flatten_spaces(child, flat_list);
        }
    }

    let mut flat_list = Vec::new();
    flatten_spaces(space, &mut flat_list);

    for metrics in &flat_list {
        wtr.serialize(metrics)?;
    }

    let data = wtr.into_inner()?;
    Ok(String::from_utf8(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage, ParserTrait, Lang};
    use crate::spaces::compute_spaces;
    use std::path::Path;

    #[test]
    fn test_dump_metrics() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

        dump_metrics(&spaces, false)?;
        Ok(())
    }

    #[test]
    fn test_export_metrics_json() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

        let config = ExportConfig {
            format: OutputFormat::Json,
            pretty: true,
            include_metadata: false,
            ..Default::default()
        };

        let json = export_metrics(&spaces, &config)?;
        assert!(json.contains("\"metrics\""));
        Ok(())
    }

    #[test]
    fn test_export_metrics_yaml() -> Result<()> {
        let code = "fn test() {}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

        let config = ExportConfig {
            format: OutputFormat::Yaml,
            include_metadata: false,
            ..Default::default()
        };

        let yaml = export_metrics(&spaces, &config)?;
        assert!(yaml.contains("metrics:"));
        Ok(())
    }

    #[test]
    fn test_export_metrics_csv() -> Result<()> {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

        let csv = export_metrics_csv(&spaces)?;
        assert!(csv.contains("kind"));
        assert!(csv.contains("sloc"));
        Ok(())
    }
}
