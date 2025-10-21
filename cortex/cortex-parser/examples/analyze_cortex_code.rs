//! Analyze real Cortex code to demonstrate dependency extraction.
//!
//! This example analyzes actual code from the cortex-parser crate itself.
//!
//! Run with: cargo run --example analyze_cortex_code

use cortex_parser::{DependencyExtractor, DependencyType, RustParser};
use std::collections::HashMap;
use std::fs;

fn main() -> anyhow::Result<()> {
    println!("🔬 Analyzing Real Cortex Code\n");

    // Analyze the types.rs file from cortex-parser
    let source = fs::read_to_string("src/types.rs")?;
    println!("📄 File: src/types.rs");
    println!("   Size: {} bytes", source.len());
    println!("   Lines: {}\n", source.lines().count());

    // Parse the file
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("src/types.rs", &source)?;

    println!("📊 Code Structure:");
    println!("   Structs: {}", parsed.structs.len());
    println!("   Enums: {}", parsed.enums.len());
    println!("   Functions: {}", parsed.functions.len());
    println!("   Traits: {}", parsed.traits.len());
    println!("   Impls: {}", parsed.impls.len());
    println!();

    // Extract dependencies
    let mut extractor = DependencyExtractor::new()?;
    let dependencies = extractor.extract_all(&parsed, &source)?;

    println!("🔗 Dependency Analysis:");
    println!("   Total dependencies: {}\n", dependencies.len());

    // Group by type
    let mut by_type: HashMap<DependencyType, Vec<_>> = HashMap::new();
    for dep in &dependencies {
        by_type
            .entry(dep.dep_type)
            .or_insert_with(Vec::new)
            .push(dep);
    }

    for (dep_type, deps) in &by_type {
        println!("   {}: {}", dep_type, deps.len());
    }
    println!();

    // Build graph
    let graph = extractor.build_dependency_graph(&parsed, &source)?;
    let stats = graph.stats();

    println!("📈 Dependency Graph:");
    println!("   Nodes: {}", stats.total_nodes);
    println!("   Edges: {}", stats.total_edges);
    println!("   Density: {:.2}%",
        (stats.total_edges as f64 / (stats.total_nodes as f64 * stats.total_nodes as f64) * 100.0));
    println!();

    // Find most important types
    let mut type_usage: HashMap<String, usize> = HashMap::new();
    for dep in &dependencies {
        if dep.dep_type == DependencyType::UsesType {
            *type_usage.entry(dep.to_unit.clone()).or_insert(0) += 1;
        }
    }

    let mut sorted_types: Vec<_> = type_usage.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));

    println!("🏆 Most Used Types:");
    for (type_name, count) in sorted_types.iter().take(10) {
        println!("   {} (used {} times)", type_name, count);
    }
    println!();

    // Analyze specific structs
    if let Some(struct_info) = parsed.structs.iter().find(|s| s.name == "FunctionInfo") {
        println!("🔎 Deep Dive: FunctionInfo struct");
        println!("   Fields: {}", struct_info.fields.len());
        println!("   Public: {}", struct_info.visibility);

        let deps = graph.get_dependencies("FunctionInfo");
        println!("   Dependencies: {}", deps.len());

        let type_deps: Vec<_> = deps
            .iter()
            .filter(|d| d.dep_type == DependencyType::UsesType)
            .collect();

        println!("   Type dependencies:");
        for dep in type_deps {
            println!("      → {}", dep.to_unit);
        }
        println!();
    }

    // Find implementation relationships
    if let Some(impl_deps) = by_type.get(&DependencyType::Implements) {
        println!("🎯 Trait Implementations:");
        for dep in impl_deps.iter().take(10) {
            println!("   {} implements {}", dep.from_unit, dep.to_unit);
        }
        if impl_deps.len() > 10 {
            println!("   ... and {} more", impl_deps.len() - 10);
        }
        println!();
    }

    // Complexity metrics
    println!("📐 Complexity Metrics:");
    let avg_deps = stats.total_edges as f64 / stats.total_nodes as f64;
    println!("   Average dependencies per node: {:.2}", avg_deps);

    // Count nodes by role
    let struct_nodes = parsed.structs.len();
    let enum_nodes = parsed.enums.len();
    let total_type_nodes = struct_nodes + enum_nodes;

    println!("   Type nodes (structs + enums): {}", total_type_nodes);
    println!("   Function nodes: {}", parsed.functions.len());
    println!("   Type/Function ratio: {:.2}",
        total_type_nodes as f64 / (parsed.functions.len() as f64 + 0.1));
    println!();

    println!("✅ Analysis complete!");

    Ok(())
}
