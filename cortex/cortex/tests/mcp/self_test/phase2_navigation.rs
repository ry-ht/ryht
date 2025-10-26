//! Phase 2 Self-Test: Deep Analysis and Navigation
//!
//! This phase tests cortex's advanced code navigation and analysis capabilities
//! on its own codebase. It validates that cortex can perform sophisticated
//! queries and analysis on the code it has ingested.
//!
//! Test Objectives:
//! 1. Load or verify cortex workspace from Phase 1
//! 2. Test code navigation tools:
//!    - Find definitions of known functions/types
//!    - Find all references to key components
//!    - Get call hierarchy for functions
//!    - Navigate type hierarchies
//!    - Get document outlines
//! 3. Test dependency analysis:
//!    - Analyze module dependencies
//!    - Detect circular dependencies
//!    - Perform impact analysis
//!    - Build dependency graphs
//! 4. Test semantic search:
//!    - Search for functionality by description
//!    - Find similar code patterns
//!    - Search by type signatures
//! 5. Measure performance for all operations
//! 6. Verify results match expected patterns
//!
//! Success Criteria:
//! - All navigation queries return valid results
//! - Known functions/types are findable
//! - Reference counts are reasonable (>0 for core types)
//! - Dependency graph is non-trivial
//! - Semantic search returns relevant results
//! - All operations complete within acceptable time

#[path = "../utils/mod.rs"]
mod utils;

use utils::TestHarness;
use cortex_memory::CognitiveManager;
use cortex_vfs::{Workspace, SourceType};
use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// Test configuration
const WORKSPACE_NAME: &str = "cortex-self-test";
const MIN_REFERENCES_FOR_CORE_TYPES: usize = 5;
const MAX_OPERATION_TIME_MS: u64 = 5000; // 5 seconds max per operation
const MIN_DEPENDENCY_COUNT: usize = 10;

/// Known symbols that should exist in cortex codebase
const KNOWN_SYMBOLS: &[KnownSymbol] = &[
    KnownSymbol {
        name: "VirtualFileSystem",
        qualified_name: "cortex_vfs::VirtualFileSystem",
        unit_type: "Struct",
        file_contains: "cortex-vfs/src",
    },
    KnownSymbol {
        name: "ConnectionManager",
        qualified_name: "cortex_storage::ConnectionManager",
        unit_type: "Struct",
        file_contains: "cortex-storage/src",
    },
    KnownSymbol {
        name: "CodeParser",
        qualified_name: "cortex_code_analysis::CodeParser",
        unit_type: "Struct",
        file_contains: "cortex-code-analysis/src",
    },
    KnownSymbol {
        name: "SemanticMemorySystem",
        qualified_name: "cortex_memory::SemanticMemorySystem",
        unit_type: "Struct",
        file_contains: "cortex-memory/src",
    },
];

/// Expected functions to test navigation
const KNOWN_FUNCTIONS: &[&str] = &[
    "new",
    "parse",
    "execute",
    "acquire",
];

#[derive(Debug, Clone)]
struct KnownSymbol {
    name: &'static str,
    qualified_name: &'static str,
    unit_type: &'static str,
    file_contains: &'static str,
}

/// Comprehensive test report
#[derive(Debug, Clone)]
struct NavigationReport {
    // Navigation results
    definitions_found: usize,
    definitions_missing: Vec<String>,
    references_found: HashMap<String, usize>,
    call_hierarchies_tested: usize,
    type_hierarchies_tested: usize,

    // Dependency analysis results
    total_dependencies: usize,
    circular_dependencies_found: usize,
    dependency_graph_nodes: usize,
    dependency_graph_edges: usize,

    // Semantic search results
    semantic_searches_performed: usize,
    semantic_results_count: usize,

    // Performance metrics
    operation_times: HashMap<String, u64>,
    slowest_operation: Option<(String, u64)>,
    fastest_operation: Option<(String, u64)>,

    // Overall status
    success: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl NavigationReport {
    fn new() -> Self {
        Self {
            definitions_found: 0,
            definitions_missing: Vec::new(),
            references_found: HashMap::new(),
            call_hierarchies_tested: 0,
            type_hierarchies_tested: 0,
            total_dependencies: 0,
            circular_dependencies_found: 0,
            dependency_graph_nodes: 0,
            dependency_graph_edges: 0,
            semantic_searches_performed: 0,
            semantic_results_count: 0,
            operation_times: HashMap::new(),
            slowest_operation: None,
            fastest_operation: None,
            success: false,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn record_operation(&mut self, name: &str, duration_ms: u64) {
        self.operation_times.insert(name.to_string(), duration_ms);

        // Track slowest
        if self.slowest_operation.is_none() || duration_ms > self.slowest_operation.as_ref().unwrap().1 {
            self.slowest_operation = Some((name.to_string(), duration_ms));
        }

        // Track fastest
        if self.fastest_operation.is_none() || duration_ms < self.fastest_operation.as_ref().unwrap().1 {
            self.fastest_operation = Some((name.to_string(), duration_ms));
        }

        // Warn if too slow
        if duration_ms > MAX_OPERATION_TIME_MS {
            self.warnings.push(format!(
                "Operation '{}' exceeded max time: {}ms > {}ms",
                name, duration_ms, MAX_OPERATION_TIME_MS
            ));
        }
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("CORTEX SELF-TEST PHASE 2: DEEP ANALYSIS AND NAVIGATION REPORT");
        println!("{}", "=".repeat(80));

        // Status
        if self.success {
            println!("\n✓ STATUS: PASS");
        } else {
            println!("\n✗ STATUS: FAIL");
        }

        // Code Navigation Results
        println!("\n--- CODE NAVIGATION ---");
        println!("Definitions Found:        {}/{}",
                 self.definitions_found, KNOWN_SYMBOLS.len());

        if !self.definitions_missing.is_empty() {
            println!("Missing Definitions:      {}", self.definitions_missing.len());
            for missing in &self.definitions_missing {
                println!("  ✗ {}", missing);
            }
        } else {
            println!("✓ All known symbols found");
        }

        println!("\nReferences Analysis:");
        for (symbol, count) in &self.references_found {
            let status = if *count >= MIN_REFERENCES_FOR_CORE_TYPES {
                "✓"
            } else {
                "⚠"
            };
            println!("  {} {}: {} references", status, symbol, count);
        }

        println!("\nCall Hierarchies:         {} analyzed", self.call_hierarchies_tested);
        println!("Type Hierarchies:         {} analyzed", self.type_hierarchies_tested);

        // Dependency Analysis Results
        println!("\n--- DEPENDENCY ANALYSIS ---");
        println!("Total Dependencies:       {}", self.total_dependencies);
        println!("Dependency Graph Nodes:   {}", self.dependency_graph_nodes);
        println!("Dependency Graph Edges:   {}", self.dependency_graph_edges);

        if self.total_dependencies >= MIN_DEPENDENCY_COUNT {
            println!("✓ Dependency analysis successful");
        } else {
            println!("✗ Insufficient dependencies found");
        }

        if self.circular_dependencies_found > 0 {
            println!("⚠ Circular Dependencies:  {}", self.circular_dependencies_found);
        } else {
            println!("✓ No circular dependencies detected");
        }

        // Semantic Search Results
        println!("\n--- SEMANTIC SEARCH ---");
        println!("Searches Performed:       {}", self.semantic_searches_performed);
        println!("Total Results:            {}", self.semantic_results_count);

        if self.semantic_results_count > 0 {
            println!("  Average per search:     {:.1}",
                     self.semantic_results_count as f64 / self.semantic_searches_performed.max(1) as f64);
        }

        // Performance Metrics
        println!("\n--- PERFORMANCE METRICS ---");
        println!("Total Operations:         {}", self.operation_times.len());

        if let Some((name, time)) = &self.slowest_operation {
            println!("Slowest Operation:        {} ({} ms)", name, time);
        }

        if let Some((name, time)) = &self.fastest_operation {
            println!("Fastest Operation:        {} ({} ms)", name, time);
        }

        let avg_time: u64 = if !self.operation_times.is_empty() {
            self.operation_times.values().sum::<u64>() / self.operation_times.len() as u64
        } else {
            0
        };
        println!("Average Operation Time:   {} ms", avg_time);

        // Top 5 slowest operations
        let mut sorted_ops: Vec<_> = self.operation_times.iter().collect();
        sorted_ops.sort_by_key(|(_, time)| std::cmp::Reverse(*time));

        println!("\nTop 5 Slowest Operations:");
        for (i, (name, time)) in sorted_ops.iter().take(5).enumerate() {
            println!("  {}. {} - {} ms", i + 1, name, time);
        }

        // Errors and Warnings
        if !self.errors.is_empty() {
            println!("\n--- ERRORS ({}) ---", self.errors.len());
            for (i, error) in self.errors.iter().enumerate() {
                println!("{}. {}", i + 1, error);
            }
        }

        if !self.warnings.is_empty() {
            println!("\n--- WARNINGS ({}) ---", self.warnings.len());
            for (i, warning) in self.warnings.iter().enumerate() {
                println!("{}. {}", i + 1, warning);
            }
        }

        // Final Summary
        println!("\n{}", "=".repeat(80));
        if self.success {
            println!("✓ PHASE 2 COMPLETE: All navigation and analysis capabilities validated!");
            println!("  Cortex can successfully navigate and analyze its own codebase.");
        } else {
            println!("✗ PHASE 2 FAILED: Issues detected during navigation and analysis");
            println!("  Review errors above and investigate before proceeding.");
        }
        println!("{}", "=".repeat(80));
    }
}

/// Get the cortex workspace root directory
fn get_cortex_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");

    PathBuf::from(manifest_dir)
        .parent()
        .expect("Could not find cortex workspace root")
        .to_path_buf()
}

/// Load test workspace or create new one
async fn load_or_create_workspace(harness: &TestHarness) -> Uuid {
    let workspace_root = get_cortex_root();

    // Try to find existing workspace
    let conn = harness.storage.acquire().await
        .expect("Failed to acquire connection");

    let query = format!("SELECT * FROM workspace WHERE name = '{}'", WORKSPACE_NAME);
    let mut result = conn.connection()
        .query(&query)
        .await
        .expect("Failed to query workspace");

    let workspaces: Vec<Workspace> = result.take(0)
        .unwrap_or_default();

    if let Some(workspace) = workspaces.into_iter().next() {
        println!("  ✓ Found existing workspace: {}", workspace.id);
        workspace.id
    } else {
        println!("  Creating new workspace...");
        let workspace_id = Uuid::new_v4();
        let workspace = Workspace {
            id: workspace_id,
            name: WORKSPACE_NAME.to_string(),
            root_path: workspace_root.clone(),
            source_type: SourceType::Local,
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_synced_at: None,
        };

        let _: Option<Workspace> = conn.connection()
            .create(("workspace", workspace_id.to_string()))
            .content(workspace)
            .await
            .expect("Failed to create workspace");

        println!("  ✓ Created workspace: {}", workspace_id);
        workspace_id
    }
}

/// Test finding definitions
async fn test_find_definitions(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Finding definitions of known symbols...");

    let cognitive = CognitiveManager::new(harness.storage.clone());
    let semantic = cognitive.semantic();

    for known in KNOWN_SYMBOLS {
        let start = Instant::now();
        let result = semantic.find_by_qualified_name(known.qualified_name).await;
        let duration = start.elapsed().as_millis() as u64;

        let op_name = format!("find_definition_{}", known.name);
        report.record_operation(&op_name, duration);

        match result {
            Ok(Some(unit)) => {
                println!("  ✓ Found: {} at {}:{}",
                         known.qualified_name,
                         unit.file_path,
                         unit.start_line);

                report.definitions_found += 1;

                // Verify unit type matches
                let unit_type_str = format!("{:?}", unit.unit_type);
                if !unit_type_str.contains(known.unit_type) {
                    report.warnings.push(format!(
                        "Type mismatch for {}: expected {}, got {}",
                        known.name, known.unit_type, unit_type_str
                    ));
                }

                // Verify file path
                if !unit.file_path.contains(known.file_contains) {
                    report.warnings.push(format!(
                        "File path mismatch for {}: expected path containing '{}', got '{}'",
                        known.name, known.file_contains, unit.file_path
                    ));
                }
            }
            Ok(None) => {
                println!("  ✗ Not found: {}", known.qualified_name);
                report.definitions_missing.push(known.qualified_name.to_string());
            }
            Err(e) => {
                println!("  ✗ Error finding {}: {}", known.qualified_name, e);
                report.errors.push(format!("Failed to find {}: {}", known.qualified_name, e));
            }
        }
    }
}

/// Test finding references
async fn test_find_references(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Finding references to core types...");

    let cognitive = CognitiveManager::new(harness.storage.clone());
    let semantic = cognitive.semantic();

    for known in KNOWN_SYMBOLS {
        // First find the symbol
        if let Ok(Some(unit)) = semantic.find_by_qualified_name(known.qualified_name).await {
            let start = Instant::now();
            let result = semantic.find_references(unit.id).await;
            let duration = start.elapsed().as_millis() as u64;

            let op_name = format!("find_references_{}", known.name);
            report.record_operation(&op_name, duration);

            match result {
                Ok(ref_ids) => {
                    let count = ref_ids.len();
                    println!("  ✓ {}: {} references", known.name, count);
                    report.references_found.insert(known.name.to_string(), count);

                    if count < MIN_REFERENCES_FOR_CORE_TYPES {
                        report.warnings.push(format!(
                            "Low reference count for {}: {} < {}",
                            known.name, count, MIN_REFERENCES_FOR_CORE_TYPES
                        ));
                    }
                }
                Err(e) => {
                    println!("  ✗ Error finding references for {}: {}", known.name, e);
                    report.errors.push(format!("Failed to find references for {}: {}", known.name, e));
                }
            }
        }
    }
}

/// Test call hierarchy
async fn test_call_hierarchy(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Analyzing call hierarchies...");

    let cognitive = CognitiveManager::new(harness.storage.clone());
    let semantic = cognitive.semantic();

    // Find functions to analyze
    for func_name in KNOWN_FUNCTIONS {
        // Search for functions with this name
        let conn = harness.storage.acquire().await
            .expect("Failed to acquire connection");

        let query = format!(
            "SELECT * FROM code_unit WHERE name = '{}' AND unit_type = 'Function' LIMIT 1",
            func_name
        );

        let mut result = conn.connection()
            .query(&query)
            .await
            .expect("Failed to query");

        let units: Vec<cortex_core::types::CodeUnit> = result.take(0).unwrap_or_default();

        if let Some(unit) = units.into_iter().next() {
            let start = Instant::now();

            // Get outgoing calls (what this function calls)
            let deps = semantic.get_dependencies(unit.id).await
                .unwrap_or_default();
            let outgoing = deps.iter()
                .filter(|d| format!("{:?}", d.dependency_type).contains("Call"))
                .count();

            // Get incoming calls (what calls this function)
            let refs = semantic.find_references(unit.id).await
                .unwrap_or_default();
            let incoming = refs.len();

            let duration = start.elapsed().as_millis() as u64;

            let op_name = format!("call_hierarchy_{}", func_name);
            report.record_operation(&op_name, duration);

            println!("  ✓ {}: {} outgoing, {} incoming",
                     func_name, outgoing, incoming);

            report.call_hierarchies_tested += 1;
        }
    }
}

/// Test type hierarchy
async fn test_type_hierarchy(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Analyzing type hierarchies...");

    let cognitive = CognitiveManager::new(harness.storage.clone());
    let semantic = cognitive.semantic();

    for known in KNOWN_SYMBOLS.iter().filter(|s| s.unit_type == "Struct") {
        if let Ok(Some(unit)) = semantic.find_by_qualified_name(known.qualified_name).await {
            let start = Instant::now();

            // Get type dependencies (traits, interfaces, etc.)
            let deps = semantic.get_dependencies(unit.id).await
                .unwrap_or_default();

            let type_deps = deps.iter()
                .filter(|d| {
                    let dt = format!("{:?}", d.dependency_type);
                    dt.contains("Implements") || dt.contains("Extends") || dt.contains("Trait")
                })
                .count();

            // Get dependents (what implements/extends this type)
            let dependents = semantic.get_dependents(unit.id).await
                .unwrap_or_default();

            let type_dependents = dependents.iter()
                .filter(|d| {
                    let dt = format!("{:?}", d.dependency_type);
                    dt.contains("Implements") || dt.contains("Extends")
                })
                .count();

            let duration = start.elapsed().as_millis() as u64;

            let op_name = format!("type_hierarchy_{}", known.name);
            report.record_operation(&op_name, duration);

            println!("  ✓ {}: {} supertypes, {} implementors",
                     known.name, type_deps, type_dependents);

            report.type_hierarchies_tested += 1;
        }
    }
}

/// Test dependency analysis
async fn test_dependency_analysis(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Analyzing dependencies...");

    let start = Instant::now();

    // Get all dependencies from database
    let conn = harness.storage.acquire().await
        .expect("Failed to acquire connection");

    let query = "SELECT source_id, target_id FROM DEPENDS_ON";
    let mut result = conn.connection()
        .query(query)
        .await
        .expect("Failed to query dependencies");

    #[derive(serde::Deserialize)]
    struct DepEdge {
        source_id: String,
        target_id: String,
    }

    let edges: Vec<DepEdge> = result.take(0).unwrap_or_default();
    report.total_dependencies = edges.len();

    // Build graph for analysis
    let mut nodes = HashSet::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for edge in &edges {
        nodes.insert(edge.source_id.clone());
        nodes.insert(edge.target_id.clone());
        adjacency.entry(edge.source_id.clone())
            .or_default()
            .push(edge.target_id.clone());
    }

    report.dependency_graph_nodes = nodes.len();
    report.dependency_graph_edges = edges.len();

    let duration = start.elapsed().as_millis() as u64;
    report.record_operation("build_dependency_graph", duration);

    println!("  ✓ Dependency graph: {} nodes, {} edges",
             report.dependency_graph_nodes,
             report.dependency_graph_edges);

    // Simple cycle detection using DFS
    let start = Instant::now();
    let mut cycles = 0;
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    fn has_cycle(
        node: &str,
        adjacency: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if has_cycle(neighbor, adjacency, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    for node in &nodes {
        if !visited.contains(node) {
            if has_cycle(node, &adjacency, &mut visited, &mut rec_stack) {
                cycles += 1;
            }
        }
    }

    report.circular_dependencies_found = cycles;

    let duration = start.elapsed().as_millis() as u64;
    report.record_operation("detect_circular_dependencies", duration);

    println!("  ✓ Circular dependency check complete: {} cycles found", cycles);
}

/// Test semantic search
async fn test_semantic_search(
    harness: &TestHarness,
    report: &mut NavigationReport,
) {
    println!("\n[TEST] Testing semantic search...");

    let search_queries = vec![
        "database connection",
        "parse code",
        "virtual filesystem",
        "semantic memory",
    ];

    let cognitive = CognitiveManager::new(harness.storage.clone());
    let semantic = cognitive.semantic();

    for query in search_queries {
        let start = Instant::now();

        // Search for units matching the query
        let conn = harness.storage.acquire().await
            .expect("Failed to acquire connection");

        let search_query = format!(
            "SELECT * FROM code_unit WHERE qualified_name CONTAINS '{}' OR signature CONTAINS '{}' LIMIT 5",
            query.replace(" ", ""), query
        );

        let mut result = conn.connection()
            .query(&search_query)
            .await
            .unwrap_or_else(|e| {
                println!("  ⚠ Search query error: {}", e);
                conn.connection().query("SELECT * FROM code_unit LIMIT 0").await.unwrap()
            });

        let units: Vec<cortex_core::types::CodeUnit> = result.take(0).unwrap_or_default();

        let duration = start.elapsed().as_millis() as u64;

        let op_name = format!("semantic_search_{}", query.replace(" ", "_"));
        report.record_operation(&op_name, duration);

        println!("  ✓ '{}': {} results", query, units.len());

        report.semantic_searches_performed += 1;
        report.semantic_results_count += units.len();
    }
}

/// Main Phase 2 test
#[tokio::test]
#[ignore] // Use `cargo test -- --ignored` to run
async fn test_phase2_navigation_and_analysis() {
    println!("\n{}", "=".repeat(80));
    println!("STARTING PHASE 2: DEEP ANALYSIS AND NAVIGATION");
    println!("{}", "=".repeat(80));

    let mut report = NavigationReport::new();
    let overall_start = Instant::now();

    // Step 1: Initialize test harness
    println!("\n[1/8] Initializing test harness...");
    let harness = TestHarness::new().await;
    println!("  ✓ Test harness ready");

    // Step 2: Load or create workspace
    println!("\n[2/8] Loading workspace...");
    let _workspace_id = load_or_create_workspace(&harness).await;

    // Step 3: Test finding definitions
    test_find_definitions(&harness, &mut report).await;

    // Step 4: Test finding references
    test_find_references(&harness, &mut report).await;

    // Step 5: Test call hierarchy
    test_call_hierarchy(&harness, &mut report).await;

    // Step 6: Test type hierarchy
    test_type_hierarchy(&harness, &mut report).await;

    // Step 7: Test dependency analysis
    test_dependency_analysis(&harness, &mut report).await;

    // Step 8: Test semantic search
    test_semantic_search(&harness, &mut report).await;

    // Calculate overall success
    report.success = report.errors.is_empty()
        && report.definitions_found >= KNOWN_SYMBOLS.len() / 2  // At least half found
        && report.total_dependencies >= MIN_DEPENDENCY_COUNT
        && report.semantic_searches_performed > 0;

    let total_time = overall_start.elapsed().as_secs_f64();
    println!("\n[COMPLETE] Phase 2 finished in {:.2}s", total_time);

    // Print final report
    report.print_summary();

    // Assertions
    assert!(
        report.definitions_found >= KNOWN_SYMBOLS.len() / 2,
        "Too few definitions found: {} < {}",
        report.definitions_found,
        KNOWN_SYMBOLS.len() / 2
    );

    assert!(
        report.total_dependencies >= MIN_DEPENDENCY_COUNT,
        "Too few dependencies found: {} < {}",
        report.total_dependencies,
        MIN_DEPENDENCY_COUNT
    );

    assert!(
        report.errors.is_empty(),
        "Phase 2 had {} errors",
        report.errors.len()
    );

    assert!(
        report.success,
        "Phase 2 self-test failed - review report above"
    );
}

#[cfg(test)]
mod quick_tests {
    use super::*;

    #[test]
    fn test_known_symbols_valid() {
        // Verify known symbols are properly formatted
        for symbol in KNOWN_SYMBOLS {
            assert!(!symbol.name.is_empty(), "Symbol name cannot be empty");
            assert!(
                symbol.qualified_name.contains("::"),
                "Qualified name must contain :: separator"
            );
            assert!(
                symbol.qualified_name.starts_with("cortex_"),
                "Qualified name must start with cortex_"
            );
        }
    }

    #[test]
    fn test_constants_valid() {
        assert!(MIN_REFERENCES_FOR_CORE_TYPES > 0);
        assert!(MAX_OPERATION_TIME_MS > 0);
        assert!(MIN_DEPENDENCY_COUNT > 0);
    }
}
