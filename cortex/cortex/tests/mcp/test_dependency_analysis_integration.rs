//! Integration tests for dependency analysis tools
//!
//! Tests the complete dependency analysis workflow:
//! 1. Parse source code to extract dependencies
//! 2. Store dependencies in database
//! 3. Build dependency graph
//! 4. Run various analysis tools

use cortex_core::config::GlobalConfig;
use cortex_code_analysis::{RustParser, DependencyExtractor};
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, PoolConnectionMode};
use std::sync::Arc;
use std::collections::HashSet;
use tempfile::tempdir;
use serde_json;

const TEST_RUST_CODE: &str = r#"
use std::collections::HashMap;
use std::fs::File;

pub struct DataProcessor {
    data: HashMap<String, String>,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn process(&self, input: &str) -> Result<String, String> {
        println!("Processing: {}", input);
        self.helper_method(input)
    }

    fn helper_method(&self, s: &str) -> Result<String, String> {
        Ok(s.to_uppercase())
    }
}

pub fn load_file(path: &str) -> Result<File, std::io::Error> {
    File::open(path)
}

#[test]
fn test_processor() {
    let processor = DataProcessor::new();
    let result = processor.process("test");
    assert!(result.is_ok());
}
"#;

#[tokio::test]
async fn test_dependency_extraction() {
    // Parse the code
    let mut parser = RustParser::new().expect("Failed to create parser");
    let parsed = parser.parse_file("test.rs", TEST_RUST_CODE)
        .expect("Failed to parse code");

    // Extract dependencies
    let mut extractor = DependencyExtractor::new().expect("Failed to create extractor");
    let dependencies = extractor.extract_all(&parsed, TEST_RUST_CODE)
        .expect("Failed to extract dependencies");

    println!("Extracted {} dependencies", dependencies.len());

    // Verify we found some dependencies
    assert!(!dependencies.is_empty(), "Should extract at least some dependencies");

    // Check for imports
    let import_deps: Vec<_> = dependencies.iter()
        .filter(|d| matches!(d.dep_type, cortex_code_analysis::DependencyType::Imports))
        .collect();
    println!("Found {} import dependencies", import_deps.len());

    // Check for function calls
    let call_deps: Vec<_> = dependencies.iter()
        .filter(|d| matches!(d.dep_type, cortex_code_analysis::DependencyType::Calls))
        .collect();
    println!("Found {} call dependencies", call_deps.len());

    // Check for type usage
    let type_deps: Vec<_> = dependencies.iter()
        .filter(|d| matches!(d.dep_type, cortex_code_analysis::DependencyType::UsesType))
        .collect();
    println!("Found {} type dependencies", type_deps.len());

    // Print all dependencies for debugging
    for dep in &dependencies {
        println!("  {} -> {} ({:?})", dep.from_unit, dep.to_unit, dep.dep_type);
    }
}

#[tokio::test]
async fn test_dependency_graph_building() {
    // Build dependency graph from extracted dependencies
    let mut parser = RustParser::new().expect("Failed to create parser");
    let parsed = parser.parse_file("test.rs", TEST_RUST_CODE)
        .expect("Failed to parse code");

    let mut extractor = DependencyExtractor::new().expect("Failed to create extractor");
    let graph = extractor.build_dependency_graph(&parsed, TEST_RUST_CODE)
        .expect("Failed to build dependency graph");

    println!("Graph has {} nodes and {} edges",
             graph.nodes.len(),
             graph.edges.len());

    // Verify graph structure
    assert!(!graph.nodes.is_empty(), "Graph should have nodes");

    // Print graph statistics
    let stats = graph.stats();
    println!("Total nodes: {}", stats.total_nodes);
    println!("Total edges: {}", stats.total_edges);
    println!("Edges by type: {:?}", stats.edges_by_type);
}

#[tokio::test]
#[ignore] // Requires running SurrealDB instance
async fn test_end_to_end_dependency_workflow() {
    // This test requires a running SurrealDB instance
    // Setup database connection
    let config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: "ws://127.0.0.1:8000".to_string(),
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        namespace: "test".to_string(),
        database: "cortex_test".to_string(),
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: std::time::Duration::from_secs(10),
            ..Default::default()
        },
    };

    let storage = Arc::new(ConnectionManager::new(config).await
        .expect("Failed to create connection manager"));

    // Parse and extract dependencies
    let mut parser = RustParser::new().expect("Failed to create parser");
    let parsed = parser.parse_file("test.rs", TEST_RUST_CODE)
        .expect("Failed to parse code");

    let mut extractor = DependencyExtractor::new().expect("Failed to create extractor");
    let dependencies = extractor.extract_all(&parsed, TEST_RUST_CODE)
        .expect("Failed to extract dependencies");

    println!("Extracted {} dependencies to store", dependencies.len());

    // Create DependencyAnalysisContext for database operations
    let ctx = cortex::mcp::tools::dependency_analysis::DependencyAnalysisContext::new(storage.clone());

    // Step 1: Store code units first (dependencies reference these)
    println!("Step 1: Creating code units in database...");
    let conn = storage.acquire().await.expect("Failed to acquire connection");

    // Collect all unique unit names from dependencies
    let mut unit_names = std::collections::HashSet::new();
    for dep in &dependencies {
        unit_names.insert(dep.from_unit.clone());
        unit_names.insert(dep.to_unit.clone());
    }

    // Create code_unit records for each unique unit
    for unit_name in &unit_names {
        let query = r#"
            CREATE code_unit CONTENT {
                id: $id,
                name: $name,
                qualified_name: $qualified_name,
                unit_type: 'function',
                file_path: 'test.rs',
                visibility: 'public',
                start_line: 1,
                end_line: 1,
                source_code: ''
            }
        "#;

        let _ = conn.connection()
            .query(query)
            .bind(("id", unit_name.clone()))
            .bind(("name", unit_name.split("::").last().unwrap_or(unit_name)))
            .bind(("qualified_name", unit_name.clone()))
            .await
            .expect("Failed to create code unit");
    }

    println!("Created {} code units", unit_names.len());

    // Step 2: Store dependencies in database
    println!("Step 2: Storing dependencies in database...");
    let stored_count = ctx.store_dependencies(dependencies.clone())
        .await
        .expect("Failed to store dependencies");

    println!("Stored {}/{} dependencies", stored_count, dependencies.len());
    assert!(stored_count > 0, "Should have stored at least some dependencies");

    // Step 3: Build graph from database
    println!("Step 3: Building dependency graph from database...");
    let graph = ctx.build_graph(Some("test.rs"))
        .await
        .expect("Failed to build graph from database");

    println!("Graph has {} nodes and {} edges",
             graph.nodes.len(),
             graph.adjacency.values().map(|v| v.len()).sum::<usize>());

    // Verify graph is not empty
    assert!(!graph.nodes.is_empty(), "Graph should have nodes");
    assert!(graph.adjacency.values().any(|v| !v.is_empty()), "Graph should have edges");

    // Step 4: Run analysis tools to verify everything works
    println!("Step 4: Running dependency analysis tools...");

    // Tool 1: Find cycles
    use cortex::mcp::tools::dependency_analysis::*;
    let find_cycles_tool = DepsFindCyclesTool::new(ctx.clone());
    let cycles_input = serde_json::json!({
        "scope_path": "test.rs",
        "max_cycle_length": 10
    });
    let cycles_result = find_cycles_tool.execute_impl(
        serde_json::from_value(cycles_input).unwrap()
    ).await.expect("Failed to find cycles");

    println!("Found {} cycles", cycles_result.total_cycles);

    // Tool 2: Get dependency metrics
    let metrics_tool = DepsDependencyMetricsTool::new(ctx.clone());
    let metrics_input = serde_json::json!({
        "scope_path": "test.rs"
    });
    let metrics_result = metrics_tool.execute_impl(
        serde_json::from_value(metrics_input).unwrap()
    ).await.expect("Failed to get metrics");

    println!("Dependency metrics:");
    println!("  Total units: {}", metrics_result.total_units);
    println!("  Total dependencies: {}", metrics_result.total_dependencies);
    println!("  Avg dependencies per unit: {:.2}", metrics_result.avg_dependencies_per_unit);
    println!("  Max dependencies: {}", metrics_result.max_dependencies);
    println!("  Circular dependency count: {}", metrics_result.circular_dependency_count);

    // Verify metrics make sense
    assert!(metrics_result.total_units > 0, "Should have analyzed some units");
    assert!(metrics_result.total_dependencies > 0, "Should have some dependencies");

    println!("✓ End-to-end dependency workflow completed successfully!");
}
