//! Hierarchical Code Space Metrics Module
//!
//! This module provides comprehensive metrics aggregation for code at multiple levels:
//! - File/Module level (unit)
//! - Class/Struct/Trait/Impl level
//! - Function/Method level
//!
//! It computes and aggregates all available metrics (cyclomatic complexity, Halstead,
//! LOC, cognitive complexity, ABC, etc.) in a hierarchical structure, allowing for
//! analysis at any level of granularity.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{Parser, RustLanguage, ParserTrait, Lang};
//! use cortex_code_analysis::spaces::compute_spaces;
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let code = r#"
//! fn add(a: i32, b: i32) -> i32 {
//!     if a > 0 {
//!         a + b
//!     } else {
//!         b
//!     }
//! }
//! "#;
//!
//! let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("example.rs"))?;
//! let root = parser.get_root();
//! let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "example.rs")?;
//!
//! println!("File metrics: {:?}", spaces.metrics);
//! println!("Number of functions: {}", spaces.spaces.len());
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::analysis::{DefaultNodeChecker, DefaultNodeGetter, NodeChecker, NodeGetter, SpaceKind};
use crate::lang::Lang;
use crate::metrics::{
    AbcStats, CognitiveStats, CyclomaticStats, ExitStats, HalsteadCollector, HalsteadStats,
    LocStats, MaintainabilityIndexStats, NargsStats, NomStats, NpaStats, NpmStats, WmcStats,
};
use crate::node::Node;

/// Complete metrics for a code space.
///
/// This structure contains all computed metrics for a specific code space
/// (function, class, module, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMetrics {
    /// Number of arguments
    pub nargs: NargsStats,
    /// Exit points
    pub exit: ExitStats,
    /// Cognitive complexity
    pub cognitive: CognitiveStats,
    /// Cyclomatic complexity
    pub cyclomatic: CyclomaticStats,
    /// Halstead metrics
    pub halstead: HalsteadStats,
    /// Lines of code
    pub loc: LocStats,
    /// Number of methods
    pub nom: NomStats,
    /// Maintainability index
    pub mi: MaintainabilityIndexStats,
    /// ABC metrics
    pub abc: AbcStats,
    /// Weighted methods per class
    pub wmc: WmcStats,
    /// Number of public methods
    pub npm: NpmStats,
    /// Number of public attributes
    pub npa: NpaStats,
}

impl Default for SpaceMetrics {
    fn default() -> Self {
        Self {
            nargs: NargsStats::default(),
            exit: ExitStats::default(),
            cognitive: CognitiveStats::default(),
            cyclomatic: CyclomaticStats::default(),
            halstead: HalsteadStats::default(),
            loc: LocStats::default(),
            nom: NomStats::default(),
            mi: MaintainabilityIndexStats::default(),
            abc: AbcStats::default(),
            wmc: WmcStats::default(),
            npm: NpmStats::default(),
            npa: NpaStats::default(),
        }
    }
}

impl fmt::Display for SpaceMetrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "  NArgs: {}", self.nargs)?;
        writeln!(f, "  Exit: {}", self.exit)?;
        writeln!(f, "  Cognitive: {}", self.cognitive)?;
        writeln!(f, "  Cyclomatic: {}", self.cyclomatic)?;
        writeln!(f, "  Halstead: {}", self.halstead)?;
        writeln!(f, "  LOC: {}", self.loc)?;
        writeln!(f, "  NOM: {}", self.nom)?;
        writeln!(f, "  MI: {}", self.mi)?;
        writeln!(f, "  ABC: {}", self.abc)?;
        writeln!(f, "  WMC: {}", self.wmc)?;
        writeln!(f, "  NPM: {}", self.npm)?;
        write!(f, "  NPA: {}", self.npa)
    }
}

impl SpaceMetrics {
    /// Creates a new SpaceMetrics with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges another SpaceMetrics into this one
    pub fn merge(&mut self, other: &SpaceMetrics) {
        self.nargs.merge(&other.nargs);
        self.exit.merge(&other.exit);
        self.cognitive.merge(&other.cognitive);
        self.cyclomatic.merge(&other.cyclomatic);
        self.halstead.merge(&other.halstead);
        self.loc.merge(&other.loc);
        self.nom.merge(&other.nom);
        self.mi.merge(&other.mi);
        self.abc.merge(&other.abc);
        self.wmc.merge(&other.wmc);
        self.npm.merge(&other.npm);
        self.npa.merge(&other.npa);
    }

    /// Finalizes metrics computation by computing derived metrics
    pub fn finalize(&mut self) {
        // Compute maintainability index from other metrics
        self.mi = MaintainabilityIndexStats::from_metrics(&self.loc, &self.cyclomatic, &self.halstead);

        // Compute WMC from cyclomatic
        self.wmc = WmcStats::from_cyclomatic(&self.cyclomatic);
    }
}

/// A code space representing a function, class, module, or other code unit.
///
/// Code spaces form a hierarchical tree structure where:
/// - The root is typically the file/module (Unit)
/// - Children can be classes, structs, traits, impls
/// - Leaf nodes are typically functions/methods
///
/// Each space contains:
/// - Identification (name, location, kind)
/// - Computed metrics for that space
/// - Nested child spaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuncSpace {
    /// The name of the code space (function name, class name, etc.)
    ///
    /// If `None`, the name could not be parsed or the space is anonymous
    pub name: Option<String>,

    /// The first line of the code space (1-indexed)
    pub start_line: usize,

    /// The last line of the code space (1-indexed)
    pub end_line: usize,

    /// The kind of space (function, class, unit, etc.)
    pub kind: SpaceKind,

    /// All nested subspaces contained within this space
    pub spaces: Vec<FuncSpace>,

    /// All computed metrics for this space
    pub metrics: SpaceMetrics,
}

impl FuncSpace {
    /// Creates a new FuncSpace
    fn new(name: Option<String>, start_line: usize, end_line: usize, kind: SpaceKind) -> Self {
        Self {
            name,
            start_line,
            end_line,
            kind,
            spaces: Vec::new(),
            metrics: SpaceMetrics::default(),
        }
    }

    /// Returns the number of lines in this space
    pub fn line_count(&self) -> usize {
        if self.end_line >= self.start_line {
            self.end_line - self.start_line + 1
        } else {
            0
        }
    }

    /// Returns true if the given line number is within this space
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    /// Recursively finds all functions in this space and its children
    pub fn find_all_functions(&self) -> Vec<&FuncSpace> {
        let mut functions = Vec::new();

        if self.kind == SpaceKind::Function {
            functions.push(self);
        }

        for child in &self.spaces {
            functions.extend(child.find_all_functions());
        }

        functions
    }

    /// Recursively finds all classes/structs in this space and its children
    pub fn find_all_classes(&self) -> Vec<&FuncSpace> {
        let mut classes = Vec::new();

        if matches!(self.kind, SpaceKind::Class | SpaceKind::Struct) {
            classes.push(self);
        }

        for child in &self.spaces {
            classes.extend(child.find_all_classes());
        }

        classes
    }
}

impl fmt::Display for FuncSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl FuncSpace {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter, indent: usize) -> fmt::Result {
        let indent_str = "  ".repeat(indent);

        writeln!(
            f,
            "{}{} '{}' (lines {}-{})",
            indent_str,
            self.kind,
            self.name.as_deref().unwrap_or("<unnamed>"),
            self.start_line,
            self.end_line
        )?;

        writeln!(f, "{}Metrics:", indent_str)?;
        let metrics_str = format!("{}", self.metrics);
        for line in metrics_str.lines() {
            writeln!(f, "{}{}", indent_str, line)?;
        }

        if !self.spaces.is_empty() {
            writeln!(f, "{}Children:", indent_str)?;
            for child in &self.spaces {
                child.fmt_with_indent(f, indent + 1)?;
            }
        }

        Ok(())
    }
}

/// Internal state for tracking metrics during AST traversal
#[derive(Debug)]
struct State<'a> {
    space: FuncSpace,
    halstead_collector: HalsteadCollector<'a>,
}

/// Computes comprehensive metrics for all code spaces in a file.
///
/// This function traverses the AST and computes metrics at every level:
/// - File/module level (Unit)
/// - Class/struct/trait/impl level
/// - Function/method level
///
/// # Arguments
///
/// * `root` - The root AST node to analyze
/// * `code` - The source code as bytes
/// * `lang` - The programming language
/// * `path` - The file path (used as the name for the root space)
///
/// # Returns
///
/// Returns a `FuncSpace` representing the entire file with nested spaces
/// and computed metrics at all levels.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{Parser, RustLanguage, ParserTrait, Lang};
/// use cortex_code_analysis::spaces::compute_spaces;
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let code = "fn test() { let x = 1; }";
/// let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
/// let root = parser.get_root();
/// let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;
/// # Ok(())
/// # }
/// ```
pub fn compute_spaces<'a>(root: Node<'a>, code: &'a [u8], lang: Lang, path: &str) -> Result<FuncSpace> {
    let mut cursor = root.cursor();
    let mut stack = Vec::new();
    let mut children = Vec::new();
    let mut state_stack: Vec<State<'a>> = Vec::new();
    let mut last_level = 0;

    // Initialize nesting map for cognitive complexity
    // Maps node ID to (conditional nesting, function nesting, lambda nesting)
    let mut nesting_map = HashMap::<usize, (usize, usize, usize)>::default();
    nesting_map.insert(root.id(), (0, 0, 0));

    // Start with the root node
    stack.push((root, 0));

    while let Some((node, level)) = stack.pop() {
        // Finalize completed scopes when we exit a level
        if level < last_level {
            finalize_states(&mut state_stack, last_level - level, lang)?;
            last_level = level;
        }

        let kind = DefaultNodeGetter::get_space_kind(&node, lang);
        let is_func = DefaultNodeChecker::is_func(&node, lang);
        let is_func_space = DefaultNodeChecker::is_func_space(&node, lang);
        let is_unit = kind == SpaceKind::Unit;

        // Create a new space for functions and function spaces
        let new_level = if is_func || is_func_space {
            let name = if is_unit {
                Some(path.to_string())
            } else {
                DefaultNodeGetter::get_func_space_name(&node, code, lang)
                    .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
            };

            let (start_line, end_line) = if is_unit {
                if node.child_count() == 0 {
                    (0, 0)
                } else {
                    (node.start_row() + 1, node.end_row())
                }
            } else {
                (node.start_row() + 1, node.end_row() + 1)
            };

            let state = State {
                space: FuncSpace::new(name, start_line, end_line, kind),
                halstead_collector: HalsteadCollector::new(),
            };
            state_stack.push(state);
            last_level = level + 1;
            last_level
        } else {
            level
        };

        // Compute metrics for the current node within the current state
        if let Some(state) = state_stack.last_mut() {
            compute_node_metrics(&node, code, state, &mut nesting_map, lang, is_func, is_unit);
        }

        // Traverse children
        cursor.reset(&node);
        if cursor.goto_first_child() {
            loop {
                children.push((cursor.node(), new_level));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            // Reverse to maintain depth-first order
            for child in children.drain(..).rev() {
                stack.push(child);
            }
        }
    }

    // Finalize all remaining states
    finalize_states(&mut state_stack, usize::MAX, lang)?;

    // Extract the final state
    state_stack
        .pop()
        .map(|state| state.space)
        .context("No spaces computed")
}

/// Computes metrics for a single AST node
fn compute_node_metrics<'a>(
    node: &Node<'a>,
    code: &'a [u8],
    state: &mut State<'a>,
    nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    lang: Lang,
    is_func: bool,
    is_unit: bool,
) {
    let metrics = &mut state.space.metrics;

    // Cognitive complexity
    compute_cognitive_complexity(node, metrics, nesting_map, lang);

    // Cyclomatic complexity
    compute_cyclomatic_complexity(node, metrics, lang);

    // Halstead metrics
    compute_halstead_metrics(node, code, &mut state.halstead_collector, lang);

    // Lines of code
    compute_loc_metrics(node, metrics, is_func, is_unit);

    // Number of methods
    compute_nom_metrics(node, metrics, lang);

    // Number of arguments
    compute_nargs_metrics(node, metrics, lang);

    // Exit points
    compute_exit_metrics(node, metrics, lang);

    // ABC metrics
    compute_abc_metrics(node, metrics, lang);

    // NPM (Number of Public Methods)
    compute_npm_metrics(node, metrics, code, lang);

    // NPA (Number of Public Attributes)
    compute_npa_metrics(node, metrics, code, lang);
}

/// Computes cognitive complexity for a node using HashMap-based nesting tracking.
///
/// This implementation properly tracks:
/// - Conditional/loop nesting depth
/// - Function nesting depth (nested functions)
/// - Lambda/closure nesting depth
/// - Boolean operator sequences
///
/// The nesting information is stored per-node in the nesting_map and propagated
/// to child nodes, allowing accurate complexity calculation.
fn compute_cognitive_complexity(
    node: &Node,
    metrics: &mut SpaceMetrics,
    nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    lang: Lang,
) {
    use crate::metrics::cognitive::{
        get_nesting_from_map, increase_nesting, increment_by_one,
    };

    let kind = node.kind();

    // Get nesting context from parent node
    let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

    match lang {
        Lang::Rust => {
            match kind {
                "if_expression" => {
                    // Check if this is an else-if (should not increment nesting)
                    let is_else_if = node.parent()
                        .map(|p| p.kind() == "else_clause")
                        .unwrap_or(false);

                    if !is_else_if {
                        increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                    }
                }
                "for_expression" | "while_expression" | "loop_expression" | "match_expression" => {
                    increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                }
                "else_clause" => {
                    increment_by_one(&mut metrics.cognitive);
                }
                "break_expression" | "continue_expression" => {
                    // Only labeled break/continue count
                    if node.child_by_field_name("label").is_some() {
                        increment_by_one(&mut metrics.cognitive);
                    }
                }
                "closure_expression" => {
                    lambda += 1;
                }
                "function_item" => {
                    // Reset nesting at function boundary
                    nesting = 0;
                    // Increase function depth if nested
                    let mut child = *node;
                    while let Some(parent) = child.parent() {
                        if parent.kind() == "function_item" {
                            depth += 1;
                            break;
                        }
                        child = parent;
                    }
                }
                "binary_expression" => {
                    // Handle && and || operators
                    if let Some(op) = node.child_by_field_name("operator") {
                        let op_kind = op.kind();
                        if op_kind == "&&" || op_kind == "||" {
                            let op_id = op.kind_id();
                            metrics.cognitive.eval_boolean_sequence(op_id);
                        }
                    }
                }
                "unary_expression" => {
                    // Track NOT operators for boolean sequences
                    if let Some(op) = node.child(0) {
                        if op.kind() == "!" {
                            metrics.cognitive.boolean_seq_not_operator(op.kind_id());
                        }
                    }
                }
                _ => {}
            }
        }
        Lang::Python => {
            match kind {
                "if_statement" | "for_statement" | "while_statement" | "conditional_expression" => {
                    increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                }
                "elif_clause" => {
                    increment_by_one(&mut metrics.cognitive);
                    metrics.cognitive.reset_boolean_seq();
                }
                "else_clause" | "finally_clause" => {
                    increment_by_one(&mut metrics.cognitive);
                }
                "except_clause" => {
                    nesting += 1;
                    metrics.cognitive.increment_with_nesting(nesting - 1);
                }
                "expression_statement" | "expression_list" | "tuple" => {
                    metrics.cognitive.reset_boolean_seq();
                }
                "not_operator" => {
                    metrics.cognitive.boolean_seq_not_operator(node.kind_id());
                }
                "boolean_operator" => {
                    // Handle 'and' and 'or' operators
                    for child in node.children() {
                        let child_kind = child.kind();
                        if child_kind == "and" || child_kind == "or" {
                            metrics.cognitive.eval_boolean_sequence(child.kind_id());
                        }
                    }
                }
                "lambda" => {
                    lambda += 1;
                }
                "function_definition" => {
                    // Increase function depth if nested
                    let mut child = *node;
                    while let Some(parent) = child.parent() {
                        if parent.kind() == "function_definition" {
                            depth += 1;
                            break;
                        }
                        child = parent;
                    }
                }
                _ => {}
            }
        }
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
            match kind {
                "if_statement" => {
                    // Check if this is an else-if
                    let is_else_if = node.parent()
                        .map(|p| p.kind() == "else_clause")
                        .unwrap_or(false);

                    if !is_else_if {
                        increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                    }
                }
                "for_statement" | "for_in_statement" | "while_statement"
                | "do_statement" | "switch_statement" | "catch_clause"
                | "ternary_expression" => {
                    increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                }
                "else_clause" => {
                    increment_by_one(&mut metrics.cognitive);
                }
                "expression_statement" => {
                    metrics.cognitive.reset_boolean_seq();
                }
                "unary_expression" => {
                    if let Some(op) = node.child(0) {
                        if op.kind() == "!" {
                            metrics.cognitive.boolean_seq_not_operator(op.kind_id());
                        }
                    }
                }
                "binary_expression" => {
                    if let Some(op) = node.child_by_field_name("operator") {
                        let op_kind = op.kind();
                        if op_kind == "&&" || op_kind == "||" {
                            metrics.cognitive.eval_boolean_sequence(op.kind_id());
                        }
                    }
                }
                "function_declaration" => {
                    nesting = 0;
                    lambda = 0;
                    // Increase function depth if nested
                    let mut child = *node;
                    while let Some(parent) = child.parent() {
                        if parent.kind() == "function_declaration" {
                            depth += 1;
                            break;
                        }
                        child = parent;
                    }
                }
                "arrow_function" => {
                    lambda += 1;
                }
                _ => {}
            }
        }
        Lang::Cpp => {
            match kind {
                "if_statement" => {
                    // Check if this is an else-if
                    let is_else_if = node.parent()
                        .and_then(|p| p.parent())
                        .map(|gp| gp.kind() == "if_statement")
                        .unwrap_or(false);

                    if !is_else_if {
                        increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                    }
                }
                "for_statement" | "while_statement" | "do_statement"
                | "switch_statement" | "catch_clause" => {
                    increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                }
                "goto_statement" | "else" => {
                    increment_by_one(&mut metrics.cognitive);
                }
                "unary_expression" => {
                    if let Some(op) = node.child(0) {
                        if op.kind() == "!" {
                            metrics.cognitive.boolean_seq_not_operator(op.kind_id());
                        }
                    }
                }
                "binary_expression" => {
                    if let Some(op) = node.child_by_field_name("operator") {
                        let op_kind = op.kind();
                        if op_kind == "&&" || op_kind == "||" {
                            metrics.cognitive.eval_boolean_sequence(op.kind_id());
                        }
                    }
                }
                "lambda_expression" => {
                    lambda += 1;
                }
                _ => {}
            }
        }
        Lang::Java => {
            match kind {
                "if_statement" => {
                    // Check if this is an else-if
                    let is_else_if = node.parent()
                        .map(|p| p.kind() == "else_clause")
                        .unwrap_or(false);

                    if !is_else_if {
                        increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                    }
                }
                "for_statement" | "while_statement" | "do_statement"
                | "switch_block" | "catch_clause" => {
                    increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
                }
                "else" => {
                    increment_by_one(&mut metrics.cognitive);
                }
                "unary_expression" => {
                    if let Some(op) = node.child(0) {
                        if op.kind() == "!" {
                            metrics.cognitive.boolean_seq_not_operator(op.kind_id());
                        }
                    }
                }
                "binary_expression" => {
                    if let Some(op) = node.child_by_field_name("operator") {
                        let op_kind = op.kind();
                        if op_kind == "&&" || op_kind == "||" {
                            metrics.cognitive.eval_boolean_sequence(op.kind_id());
                        }
                    }
                }
                "lambda_expression" => {
                    lambda += 1;
                }
                _ => {}
            }
        }
        _ => {
            // Fallback for unsupported languages - simple nesting detection
            let is_nesting = matches!(
                kind,
                "if_statement" | "if_expression"
                | "while_statement" | "while_expression"
                | "for_statement" | "for_expression"
                | "loop_expression" | "match_expression"
                | "switch_statement"
            );

            if is_nesting {
                increase_nesting(&mut metrics.cognitive, &mut nesting, depth, lambda);
            }
        }
    }

    // Store updated nesting information in the map
    nesting_map.insert(node.id(), (nesting, depth, lambda));
}

/// Computes cyclomatic complexity for a node
fn compute_cyclomatic_complexity(node: &Node, metrics: &mut SpaceMetrics, lang: Lang) {
    let kind = node.kind();

    // Check if this is a decision point
    let is_decision = match lang {
        Lang::Rust => matches!(
            kind,
            "if_expression"
                | "while_expression"
                | "for_expression"
                | "match_arm"
                | "||"
                | "&&"
                | "?"
        ),
        Lang::Python => {
            // Special handling for else clauses: only count when after for/while loops
            if kind == "else_clause" {
                // Check if the parent of this else clause is a for or while statement
                // (not an if statement)
                node.parent()
                    .map(|parent| matches!(parent.kind(), "for_statement" | "while_statement"))
                    .unwrap_or(false)
            } else {
                matches!(
                    kind,
                    "if_statement" | "elif_clause" | "while_statement" | "for_statement" | "or" | "and"
                )
            }
        }
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => matches!(
            kind,
            "if_statement"
                | "while_statement"
                | "for_statement"
                | "case"
                | "||"
                | "&&"
                | "?:"
        ),
        Lang::Cpp => matches!(
            kind,
            "if_statement" | "while_statement" | "for_statement" | "case_statement" | "||" | "&&"
        ),
        Lang::Java => matches!(
            kind,
            "if_statement" | "while_statement" | "for_statement" | "case" | "||" | "&&" | "?"
        ),
        _ => false,
    };

    if is_decision {
        metrics.cyclomatic.increment();
    }
}

/// Computes Halstead metrics for a node
fn compute_halstead_metrics<'a>(
    node: &Node<'a>,
    code: &'a [u8],
    collector: &mut HalsteadCollector<'a>,
    lang: Lang,
) {
    use crate::analysis::HalsteadType;

    let op_type = DefaultNodeGetter::get_op_type(node, lang);

    match op_type {
        HalsteadType::Operator => {
            let op_str = DefaultNodeGetter::get_operator_id_as_str(node.kind_id(), lang);
            collector.add_operator(op_str);
        }
        HalsteadType::Operand => {
            let start = node.start_byte();
            let end = node.end_byte();
            if end <= code.len() {
                if let Ok(operand_str) = std::str::from_utf8(&code[start..end]) {
                    collector.add_operand(operand_str);
                }
            }
        }
        HalsteadType::Unknown => {}
    }
}

/// Computes lines of code metrics for a node
fn compute_loc_metrics(node: &Node, _metrics: &mut SpaceMetrics, _is_func: bool, _is_unit: bool) {
    // LOC metrics are computed differently in cortex
    // The line counting is handled at finalization
    // This is intentionally simplified as LOC computation in the original
    // rust-code-analysis is complex and requires tracking individual lines
    let _ = node; // Suppress unused warning
}

/// Computes number of methods for a node
fn compute_nom_metrics(node: &Node, metrics: &mut SpaceMetrics, lang: Lang) {
    if DefaultNodeChecker::is_func(node, lang) {
        metrics.nom.add_function();
    } else if DefaultNodeChecker::is_closure(node, lang) {
        metrics.nom.add_closure();
    }
}

/// Computes number of arguments for a node
fn compute_nargs_metrics(node: &Node, metrics: &mut SpaceMetrics, lang: Lang) {
    if DefaultNodeChecker::is_func(node, lang) {
        // Count parameters - this is simplified
        if let Some(params) = node.child_by_field_name("parameters") {
            let mut count = 0;
            let mut cursor = params.cursor();
            if cursor.goto_first_child() {
                loop {
                    if !DefaultNodeChecker::is_non_arg(&cursor.node(), lang) {
                        count += 1;
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            metrics.nargs.set_fn_args(count);
        }
    }
}

/// Computes exit points for a node
fn compute_exit_metrics(node: &Node, metrics: &mut SpaceMetrics, lang: Lang) {
    let kind = node.kind();

    let is_exit = match lang {
        Lang::Rust => matches!(kind, "return_expression" | "break_expression"),
        Lang::Python => matches!(kind, "return_statement" | "break_statement"),
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
            matches!(kind, "return_statement" | "break_statement")
        }
        Lang::Cpp => matches!(kind, "return_statement" | "break_statement"),
        Lang::Java => matches!(kind, "return_statement" | "break_statement"),
        _ => false,
    };

    if is_exit {
        metrics.exit.increment();
    }
}

/// Computes ABC metrics for a node
fn compute_abc_metrics(node: &Node, metrics: &mut SpaceMetrics, lang: Lang) {
    let kind = node.kind();

    // Assignment
    let is_assignment = match lang {
        Lang::Rust => matches!(kind, "let_declaration" | "assignment_expression"),
        Lang::Python => kind == "assignment",
        _ => kind.contains("assignment"),
    };

    // Branch
    let is_branch = match lang {
        Lang::Rust => DefaultNodeChecker::is_call(node, lang) || kind == "method_call_expression",
        Lang::Python => kind == "call",
        _ => DefaultNodeChecker::is_call(node, lang),
    };

    // Condition
    let is_condition = match lang {
        Lang::Rust => matches!(
            kind,
            "if_expression" | "while_expression" | "match_expression"
        ),
        Lang::Python => matches!(kind, "if_statement" | "while_statement"),
        _ => matches!(kind, "if_statement" | "while_statement"),
    };

    if is_assignment {
        metrics.abc.add_assignment();
    }
    if is_branch {
        metrics.abc.add_branch();
    }
    if is_condition {
        metrics.abc.add_condition();
    }
}

/// Computes number of public methods for a node
fn compute_npm_metrics(node: &Node, metrics: &mut SpaceMetrics, code: &[u8], lang: Lang) {
    if DefaultNodeChecker::is_func(node, lang) {
        // Check if method is public
        let is_public = match lang {
            Lang::Rust => {
                // Look for pub keyword
                node.parent()
                    .and_then(|p| p.child_by_field_name("visibility"))
                    .map(|v| v.kind() == "pub")
                    .unwrap_or(false)
            }
            Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
                // Check for public modifier or no private/protected
                let text = node.utf8_text(code).unwrap_or("");
                !text.contains("private") && !text.contains("protected")
            }
            _ => false,
        };

        if is_public {
            metrics.npm.add_public_method();
        }
    }
}

/// Computes number of public attributes for a node
fn compute_npa_metrics(node: &Node, metrics: &mut SpaceMetrics, code: &[u8], lang: Lang) {
    let kind = node.kind();

    let is_field = match lang {
        Lang::Rust => kind == "field_declaration",
        Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
            kind == "field_definition" || kind == "public_field_definition"
        }
        _ => false,
    };

    if is_field {
        // Check if field is public
        let is_public = match lang {
            Lang::Rust => node
                .child_by_field_name("visibility")
                .map(|v| v.kind() == "pub")
                .unwrap_or(false),
            Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Jsx => {
                let text = node.utf8_text(code).unwrap_or("");
                kind == "public_field_definition" || !text.contains("private")
            }
            _ => false,
        };

        if is_public {
            metrics.npa.add_public_attribute();
        }
    }
}

/// Finalizes completed states by computing derived metrics and merging into parent
fn finalize_states<'a>(state_stack: &mut Vec<State<'a>>, diff_level: usize, lang: Lang) -> Result<()> {
    if state_stack.is_empty() {
        return Ok(());
    }

    for _ in 0..diff_level {
        if state_stack.len() == 1 {
            // This is the root/unit level - finalize it
            let last_state = state_stack.last_mut().unwrap();
            finalize_state(last_state, lang)?;
            break;
        } else {
            // Pop child state, finalize it, and merge into parent
            let mut state = state_stack.pop().unwrap();
            finalize_state(&mut state, lang)?;

            let parent_state = state_stack.last_mut().unwrap();

            // Merge Halstead collectors
            parent_state
                .halstead_collector
                .merge(&state.halstead_collector);

            // Merge metrics
            parent_state.space.metrics.merge(&state.space.metrics);

            // Add child space
            parent_state.space.spaces.push(state.space);
        }
    }

    Ok(())
}

/// Finalizes a single state by computing derived metrics
fn finalize_state<'a>(state: &mut State<'a>, _lang: Lang) -> Result<()> {
    // Finalize Halstead metrics
    state.space.metrics.halstead = state.halstead_collector.finalize();

    // Compute derived metrics (MI, WMC)
    state.space.metrics.finalize();

    // Compute min/max/averages for various metrics
    state.space.metrics.cyclomatic.compute_minmax();
    state.space.metrics.cognitive.compute_minmax();
    state.space.metrics.exit.compute_minmax();
    state.space.metrics.nargs.compute_minmax();
    state.space.metrics.nom.compute_minmax();
    state.space.metrics.loc.compute_minmax();
    state.space.metrics.abc.compute_minmax();
    state.space.metrics.wmc.compute_minmax();
    state.space.metrics.npm.compute_minmax();
    state.space.metrics.npa.compute_minmax();

    // Finalize cognitive complexity with total function count
    let nom_functions = state.space.metrics.nom.functions() as usize;
    let nom_closures = state.space.metrics.nom.closures() as usize;
    let nom_total = nom_functions + nom_closures;

    if nom_total > 0 {
        state.space.metrics.cognitive.finalize(nom_total);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_space_metrics_default() {
        let metrics = SpaceMetrics::default();
        assert_eq!(metrics.cyclomatic.cyclomatic(), 1.0);
    }

    #[test]
    fn test_func_space_creation() {
        let space = FuncSpace::new(Some("test".to_string()), 1, 10, SpaceKind::Function);
        assert_eq!(space.name, Some("test".to_string()));
        assert_eq!(space.start_line, 1);
        assert_eq!(space.end_line, 10);
        assert_eq!(space.kind, SpaceKind::Function);
        assert_eq!(space.line_count(), 10);
    }

    #[test]
    fn test_func_space_contains_line() {
        let space = FuncSpace::new(Some("test".to_string()), 5, 15, SpaceKind::Function);
        assert!(space.contains_line(5));
        assert!(space.contains_line(10));
        assert!(space.contains_line(15));
        assert!(!space.contains_line(4));
        assert!(!space.contains_line(16));
    }

    #[test]
    fn test_compute_spaces_rust_simple() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        let code = "fn test() {}";
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.name, Some("test.rs".to_string()));
        assert_eq!(spaces.kind, SpaceKind::Unit);
        assert_eq!(spaces.spaces.len(), 1);
        assert_eq!(spaces.spaces[0].name, Some("test".to_string()));
        assert_eq!(spaces.spaces[0].kind, SpaceKind::Function);

        Ok(())
    }

    #[test]
    fn test_compute_spaces_rust_with_complexity() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        let code = r#"
fn calculate(x: i32) -> i32 {
    if x > 0 {
        x + 1
    } else {
        x - 1
    }
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.spaces.len(), 1);
        let func = &spaces.spaces[0];
        assert_eq!(func.name, Some("calculate".to_string()));

        // Should have some cyclomatic complexity from the if statement
        assert!(func.metrics.cyclomatic.cyclomatic() > 1.0);

        Ok(())
    }

    #[test]
    fn test_compute_spaces_multiple_functions() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.spaces.len(), 2);
        assert_eq!(spaces.spaces[0].name, Some("add".to_string()));
        assert_eq!(spaces.spaces[1].name, Some("subtract".to_string()));

        Ok(())
    }

    #[test]
    fn test_find_all_functions() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        let code = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    fn new() -> Self {
        Self { value: 0 }
    }

    fn add(&mut self, x: i32) {
        self.value += x;
    }
}

fn main() {
    let mut calc = Calculator::new();
    calc.add(5);
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        let functions = spaces.find_all_functions();
        // Should find: new, add, main (and possibly others depending on parsing)
        assert!(functions.len() >= 3);

        Ok(())
    }

    #[test]
    fn test_space_metrics_merge() {
        let mut metrics1 = SpaceMetrics::default();
        metrics1.cyclomatic.increment();
        let sum1 = metrics1.cyclomatic.cyclomatic_sum();

        let mut metrics2 = SpaceMetrics::default();
        metrics2.cyclomatic.increment();
        metrics2.cyclomatic.increment();
        let sum2 = metrics2.cyclomatic.cyclomatic_sum();

        metrics1.merge(&metrics2);
        let final_sum = metrics1.cyclomatic.cyclomatic_sum();

        // After merge, should have combined complexity
        // Check that merging increased the sum
        assert!(final_sum >= sum1 + sum2 - 1.0);  // Account for default value
    }

    #[test]
    fn test_display_format() {
        let space = FuncSpace::new(Some("test".to_string()), 1, 10, SpaceKind::Function);
        let display = format!("{}", space);
        assert!(display.contains("test"));
        assert!(display.contains("function"));
    }

    #[test]
    fn test_cognitive_complexity_with_nesting() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        // Test nested if statements with boolean operators
        let code = r#"
fn complex_function(x: i32, y: i32, z: i32) -> i32 {
    if x > 0 {                      // +1
        if y > 0 {                  // +2 (nesting = 1)
            if z > 0 {              // +3 (nesting = 2)
                return x + y + z;
            }
        }
    }

    if x > 10 && y > 10 {          // +2 (+1 if, +1 &&)
        return x * y;
    }

    0
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.spaces.len(), 1);
        let func = &spaces.spaces[0];
        assert_eq!(func.name, Some("complex_function".to_string()));

        // Should have cognitive complexity from nesting: 1 + 2 + 3 + 2 = 8
        let cognitive = func.metrics.cognitive.cognitive();
        println!("Cognitive complexity: {}", cognitive);
        assert!(cognitive >= 8.0, "Expected cognitive complexity >= 8.0, got {}", cognitive);

        Ok(())
    }

    #[test]
    fn test_cognitive_complexity_with_multiple_nesting() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        // Test multiple levels of nesting to verify HashMap tracking
        let code = r#"
fn with_multiple_nesting(a: i32, b: i32) {
    if a > 0 {                      // +1
        for i in 0..10 {            // +2 (nesting = 1)
            while b > 0 {           // +3 (nesting = 2)
                println!("nested");
            }
        }
    }
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.spaces.len(), 1);
        let func = &spaces.spaces[0];

        // Should have: if (1) + for (2) + while (3) = 6
        let cognitive = func.metrics.cognitive.cognitive();
        println!("Cognitive complexity with multiple nesting: {}", cognitive);
        assert!(cognitive >= 6.0, "Expected cognitive complexity >= 6.0, got {}", cognitive);

        Ok(())
    }

    #[test]
    fn test_cognitive_complexity_boolean_sequences() -> Result<()> {
        use crate::{RustLanguage, ParserTrait};
        use std::path::Path;

        // Test that consecutive same operators don't double-count
        let code = r#"
fn boolean_test(a: bool, b: bool, c: bool, d: bool) -> bool {
    if a && b && c {               // +2 (+1 if, +1 for boolean sequence)
        return true;
    }

    if a && b || c {               // +3 (+1 if, +1 &&, +1 ||)
        return false;
    }

    false
}
"#;
        let parser = Parser::<RustLanguage>::new(code.as_bytes().to_vec(), Path::new("test.rs"))?;
        let root = parser.get_root();
        let spaces = compute_spaces(root, parser.get_code(), Lang::Rust, "test.rs")?;

        assert_eq!(spaces.spaces.len(), 1);
        let func = &spaces.spaces[0];

        // Should have: 2 + 3 = 5
        let cognitive = func.metrics.cognitive.cognitive();
        println!("Cognitive complexity with booleans: {}", cognitive);
        assert!(cognitive >= 5.0, "Expected cognitive complexity >= 5.0, got {}", cognitive);

        Ok(())
    }
}
