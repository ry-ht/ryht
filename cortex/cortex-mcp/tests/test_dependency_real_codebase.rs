//! Test dependency analysis on real codebase (cortex-parser)

use cortex_mcp::graph_algorithms::*;

/// Build a graph from cortex-parser actual dependencies
fn build_cortex_parser_graph() -> Graph {
    let mut graph = Graph::new();

    // Real dependencies from cortex-parser/src/lib.rs
    graph.add_edge("cortex_parser".to_string(), "dependency_extractor".to_string());
    graph.add_edge("cortex_parser".to_string(), "types".to_string());

    // dependency_extractor.rs dependencies
    graph.add_edge("dependency_extractor".to_string(), "types".to_string());
    graph.add_edge("dependency_extractor".to_string(), "tree_sitter".to_string());
    graph.add_edge("dependency_extractor".to_string(), "serde".to_string());
    graph.add_edge("dependency_extractor".to_string(), "anyhow".to_string());

    // DependencyExtractor methods
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_import_dependencies".to_string());
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_from_function".to_string());
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_from_struct".to_string());
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_from_enum".to_string());
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_from_trait".to_string());
    graph.add_edge("DependencyExtractor::extract_all".to_string(), "DependencyExtractor::extract_from_impl".to_string());

    // Extract from function dependencies
    graph.add_edge("DependencyExtractor::extract_from_function".to_string(), "DependencyExtractor::extract_function_calls".to_string());
    graph.add_edge("DependencyExtractor::extract_from_function".to_string(), "DependencyExtractor::extract_type_usage".to_string());
    graph.add_edge("DependencyExtractor::extract_from_function".to_string(), "DependencyExtractor::extract_type_name".to_string());
    graph.add_edge("DependencyExtractor::extract_from_function".to_string(), "DependencyExtractor::is_primitive_type".to_string());

    // DependencyGraph dependencies
    graph.add_edge("DependencyGraph::from_dependencies".to_string(), "DependencyGraph::new".to_string());
    graph.add_edge("DependencyGraph::get_dependencies".to_string(), "Dependency".to_string());
    graph.add_edge("DependencyGraph::get_dependents".to_string(), "Dependency".to_string());

    graph
}

#[test]
fn test_cortex_parser_has_no_cycles() {
    let graph = build_cortex_parser_graph();

    let cycles = find_cycles(&graph);

    // Good code should have no cycles
    assert_eq!(cycles.len(), 0, "cortex-parser should have no circular dependencies");
}

#[test]
fn test_cortex_parser_layers() {
    let graph = build_cortex_parser_graph();

    let layers = topological_layers(&graph);

    // Should have multiple layers
    assert!(layers.len() >= 3, "Should have at least 3 architectural layers");

    // External dependencies should be in early layers
    let first_layer = &layers[0];
    assert!(
        first_layer.contains(&"tree_sitter".to_string()) ||
        first_layer.contains(&"serde".to_string()) ||
        first_layer.contains(&"anyhow".to_string()),
        "External dependencies should be in base layer"
    );
}

#[test]
fn test_cortex_parser_entry_points() {
    let graph = build_cortex_parser_graph();

    let roots = find_roots(&graph);

    // Main module should be a root
    assert!(roots.contains(&"cortex_parser".to_string()), "Main module should be an entry point");
}

#[test]
fn test_cortex_parser_utilities() {
    let graph = build_cortex_parser_graph();

    let leaves = find_leaves(&graph);

    // Utility functions should be leaves
    assert!(
        leaves.contains(&"DependencyExtractor::is_primitive_type".to_string()),
        "is_primitive_type should be a leaf (utility)"
    );
}

#[test]
fn test_cortex_parser_hub_detection() {
    let graph = build_cortex_parser_graph();

    let hubs = find_hubs(&graph, 3);

    // types module should be a hub (used by many modules)
    let types_is_hub = hubs.iter().any(|(id, _, _, _)| id == "types");
    assert!(types_is_hub, "types module should be a hub");
}

#[test]
fn test_dependency_path_cortex_parser() {
    let graph = build_cortex_parser_graph();

    // Find path from main to a deep dependency
    let path = find_shortest_path(
        &graph,
        "DependencyExtractor::extract_all",
        "DependencyExtractor::is_primitive_type"
    );

    assert!(path.is_some(), "Should find path from extract_all to is_primitive_type");
    let path = path.unwrap();
    assert!(path.length <= 3, "Path should be reasonably short");
}

#[test]
fn test_impact_of_changing_types() {
    let graph = build_cortex_parser_graph();

    // If we change types module, what's impacted?
    let impacted = find_reachable(&graph, "types", None);

    // Should impact dependency_extractor and cortex_parser
    assert!(impacted.len() >= 2, "Changing types should impact multiple modules");
    assert!(impacted.contains_key("dependency_extractor"), "dependency_extractor depends on types");
}

#[test]
fn test_extract_all_method_dependencies() {
    let graph = build_cortex_parser_graph();

    // extract_all is a hub method that calls many others
    let out_degree = graph.out_degree("DependencyExtractor::extract_all");

    assert!(out_degree >= 5, "extract_all should call at least 5 other methods");
}

#[test]
fn test_no_architectural_violations() {
    let graph = build_cortex_parser_graph();

    // Rule: Public API should not depend on internal tree-sitter details
    let mut violations = 0;

    for node in &graph.nodes {
        if node.starts_with("DependencyGraph::") {
            for neighbor in graph.neighbors(node) {
                if neighbor.contains("tree_sitter") {
                    violations += 1;
                }
            }
        }
    }

    assert_eq!(violations, 0, "DependencyGraph should not depend directly on tree_sitter");
}

#[test]
fn test_dependency_graph_structure() {
    let graph = build_cortex_parser_graph();

    // Verify graph structure
    assert!(graph.nodes.len() > 10, "Should have at least 10 nodes");
    assert!(graph.adjacency.len() > 5, "Should have at least 5 nodes with outgoing edges");
}
