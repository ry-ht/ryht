//! Example demonstrating the AST builder functionality.
//!
//! This example shows how to build and serialize Abstract Syntax Trees (ASTs)
//! from source code using the cortex-code-analysis library.

use cortex_code_analysis::{build_ast, build_ast_with_config, AstConfig, Lang};
use anyhow::Result;

fn main() -> Result<()> {
    println!("=== AST Builder Demo ===\n");

    // Example 1: Simple Rust function
    let rust_code = r#"
/// Calculates the factorial of a number.
pub fn factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
"#;

    println!("1. Building AST for Rust code:");
    println!("{}", rust_code);
    let ast = build_ast(rust_code, Lang::Rust, true, false)?;
    println!("Root node type: {}", ast.r#type);
    println!("Total nodes: {}", ast.node_count());
    println!("Tree depth: {}", ast.depth());
    if let Some((start_row, start_col, end_row, end_col)) = ast.span {
        println!("Span: ({}, {}) to ({}, {})", start_row, start_col, end_row, end_col);
    }
    println!();

    // Example 2: TypeScript with comments
    let ts_code = r#"
// Calculate sum of an array
function sum(numbers: number[]): number {
    return numbers.reduce((a, b) => a + b, 0);
}
"#;

    println!("2. Building AST for TypeScript code:");
    println!("{}", ts_code);
    let ast_with_comments = build_ast(ts_code, Lang::TypeScript, false, false)?;
    let ast_without_comments = build_ast(ts_code, Lang::TypeScript, false, true)?;
    println!("Nodes with comments: {}", ast_with_comments.node_count());
    println!("Nodes without comments: {}", ast_without_comments.node_count());
    println!();

    // Example 3: Using AstConfig
    let config = AstConfig {
        include_span: true,
        filter_comments: true,
    };

    let python_code = r#"
def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"
"#;

    println!("3. Building AST with custom configuration:");
    println!("{}", python_code);
    let ast = build_ast_with_config(python_code, Lang::Python, config)?;
    println!("Root node type: {}", ast.r#type);
    println!("Total nodes: {}", ast.node_count());
    println!();

    // Example 4: Serialize to JSON
    let simple_code = "fn add(a: i32, b: i32) -> i32 { a + b }";
    println!("4. Serializing AST to JSON:");
    println!("{}", simple_code);
    let ast = build_ast(simple_code, Lang::Rust, true, false)?;
    let json = serde_json::to_string_pretty(&ast)?;
    println!("JSON representation (first 500 chars):");
    println!("{}", &json[..json.len().min(500)]);
    println!();

    // Example 5: Complex Rust code with multiple items
    let complex_rust = r#"
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

pub trait Shape {
    fn area(&self) -> f64;
}
"#;

    println!("5. Building AST for complex Rust code:");
    let ast = build_ast(complex_rust, Lang::Rust, false, false)?;
    println!("Total nodes: {}", ast.node_count());
    println!("Tree depth: {}", ast.depth());
    println!("Number of children: {}", ast.child_count());

    // Count different node types
    fn count_node_types(node: &cortex_code_analysis::AstNode) -> std::collections::HashMap<&'static str, usize> {
        let mut counts = std::collections::HashMap::new();
        *counts.entry(node.r#type).or_insert(0) += 1;
        for child in &node.children {
            for (k, v) in count_node_types(child) {
                *counts.entry(k).or_insert(0) += v;
            }
        }
        counts
    }

    let type_counts = count_node_types(&ast);
    println!("\nNode type distribution:");
    let mut types: Vec<_> = type_counts.iter().collect();
    types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (node_type, count) in types.iter().take(10) {
        println!("  {}: {}", node_type, count);
    }

    Ok(())
}
