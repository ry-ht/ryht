//! Integration tests for dependency analysis tools
//!
//! Tests the complete dependency analysis workflow:
//! 1. Parse source code to extract dependencies
//! 2. Store dependencies in database
//! 3. Build dependency graph
//! 4. Run various analysis tools

use cortex_core::config::GlobalConfig;
use cortex_parser::{RustParser, DependencyExtractor};
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, PoolConnectionMode};
use std::sync::Arc;
use tempfile::tempdir;

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
        .filter(|d| matches!(d.dep_type, cortex_parser::DependencyType::Imports))
        .collect();
    println!("Found {} import dependencies", import_deps.len());

    // Check for function calls
    let call_deps: Vec<_> = dependencies.iter()
        .filter(|d| matches!(d.dep_type, cortex_parser::DependencyType::Calls))
        .collect();
    println!("Found {} call dependencies", call_deps.len());

    // Check for type usage
    let type_deps: Vec<_> = dependencies.iter()
        .filter(|d| matches!(d.dep_type, cortex_parser::DependencyType::UsesType))
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

    // TODO: Store dependencies in database using DependencyAnalysisContext
    // TODO: Build graph from database
    // TODO: Run analysis tools
}
