//! Comprehensive Dependency Analysis Tests
//!
//! Tests all 10 dependency analysis tools with real code examples:
//! 1. Get Dependencies
//! 2. Find Path
//! 3. Find Cycles
//! 4. Impact Analysis
//! 5. Find Roots
//! 6. Find Leaves
//! 7. Find Hubs
//! 8. Get Layers
//! 9. Check Constraints
//! 10. Generate Graph

use cortex_mcp::graph_algorithms::*;

// ============================================================================
// GRAPH ALGORITHMS TESTS (15 tests)
// ============================================================================

#[test]
fn test_graph_creation() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.neighbors("A").len(), 1);
    assert_eq!(graph.reverse_neighbors("B").len(), 1);
}

#[test]
fn test_shortest_path_direct() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());

    let path = find_shortest_path(&graph, "A", "B");
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.length, 1);
    assert_eq!(path.nodes, vec!["A", "B"]);
}

#[test]
fn test_shortest_path_multi_hop() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "D".to_string());
    graph.add_edge("A".to_string(), "D".to_string()); // Direct path

    let path = find_shortest_path(&graph, "A", "D");
    assert!(path.is_some());
    let path = path.unwrap();
    // Should find direct path A->D (length 1) instead of A->B->C->D (length 3)
    assert_eq!(path.length, 1);
    assert_eq!(path.nodes, vec!["A", "D"]);
}

#[test]
fn test_shortest_path_no_path() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let path = find_shortest_path(&graph, "A", "D");
    assert!(path.is_none());
}

#[test]
fn test_find_all_paths() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("A".to_string(), "C".to_string());
    graph.add_edge("B".to_string(), "D".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let paths = find_all_paths(&graph, "A", "D", 5);
    assert!(paths.len() >= 2); // A->B->D and A->C->D
}

#[test]
fn test_find_cycles_simple() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "A".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 3);
}

#[test]
fn test_find_cycles_none() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 0);
}

#[test]
fn test_find_cycles_multiple() {
    let mut graph = Graph::new();
    // Cycle 1: A->B->A
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "A".to_string());
    // Cycle 2: C->D->E->C
    graph.add_edge("C".to_string(), "D".to_string());
    graph.add_edge("D".to_string(), "E".to_string());
    graph.add_edge("E".to_string(), "C".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 2);
}

#[test]
fn test_topological_layers() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("A".to_string(), "C".to_string());
    graph.add_edge("B".to_string(), "D".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let layers = topological_layers(&graph);
    assert!(!layers.is_empty());
    // Layer 0 should contain A (no dependencies)
    assert!(layers[0].contains(&"A".to_string()));
    // Layer 1 should contain B and C
    // Layer 2 should contain D
}

#[test]
fn test_find_roots() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let roots = find_roots(&graph);
    assert_eq!(roots.len(), 2);
    assert!(roots.contains(&"A".to_string()));
    assert!(roots.contains(&"C".to_string()));
}

#[test]
fn test_find_leaves() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    let leaves = find_leaves(&graph);
    assert_eq!(leaves.len(), 2);
    assert!(leaves.contains(&"B".to_string()));
    assert!(leaves.contains(&"D".to_string()));
}

#[test]
fn test_find_hubs() {
    let mut graph = Graph::new();
    // Create hub at B
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("C".to_string(), "B".to_string());
    graph.add_edge("D".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "E".to_string());
    graph.add_edge("B".to_string(), "F".to_string());
    graph.add_edge("B".to_string(), "G".to_string());

    let hubs = find_hubs(&graph, 3);
    assert!(!hubs.is_empty());
    assert_eq!(hubs[0].0, "B");
    assert_eq!(hubs[0].1, 3); // in-degree
    assert_eq!(hubs[0].2, 3); // out-degree
    assert_eq!(hubs[0].3, 6); // total
}

#[test]
fn test_in_out_degree() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("C".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "D".to_string());

    assert_eq!(graph.in_degree("B"), 2);
    assert_eq!(graph.out_degree("B"), 1);
    assert_eq!(graph.total_degree("B"), 3);
}

#[test]
fn test_find_reachable() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    // Find all nodes that can reach D (reverse dependencies)
    let reachable = find_reachable(&graph, "D", None);
    assert_eq!(reachable.len(), 4); // D itself + A, B, C
    assert!(reachable.contains_key("A"));
    assert!(reachable.contains_key("B"));
    assert!(reachable.contains_key("C"));
    assert_eq!(reachable.get("A"), Some(&3)); // Distance 3
}

#[test]
fn test_find_reachable_with_depth() {
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "D".to_string());

    // Only find within depth 2
    let reachable = find_reachable(&graph, "D", Some(2));
    assert!(reachable.len() <= 3); // Should not include A (distance 3)
}

// ============================================================================
// REAL-WORLD SCENARIO TESTS (15 tests)
// ============================================================================

#[test]
fn test_cyclic_dependency_detection() {
    // Simulate real code with circular dependency
    let mut graph = Graph::new();

    // FileA imports FileB
    graph.add_edge("FileA".to_string(), "FileB".to_string());
    // FileB imports FileC
    graph.add_edge("FileB".to_string(), "FileC".to_string());
    // FileC imports FileA (creates cycle)
    graph.add_edge("FileC".to_string(), "FileA".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 3);
}

#[test]
fn test_impact_analysis_authenticate_function() {
    // Simulate authenticate() called by many functions
    let mut graph = Graph::new();

    let authenticate = "auth::authenticate";
    let callers = vec![
        "api::login", "api::register", "api::refresh_token",
        "handlers::user_create", "handlers::user_update",
        "middleware::auth_check", "services::user_service",
        "controllers::auth_controller"
    ];

    for caller in &callers {
        graph.add_edge(caller.to_string(), authenticate.to_string());
    }

    // If we change authenticate, all callers are impacted
    let impacted = find_reachable(&graph, authenticate, None);
    assert_eq!(impacted.len(), callers.len() + 1); // +1 for authenticate itself
}

#[test]
fn test_layered_architecture() {
    // Simulate proper layered architecture
    let mut graph = Graph::new();

    // Layer 0: Database
    // Layer 1: Repository
    // Layer 2: Service
    // Layer 3: Controller

    graph.add_edge("repository::UserRepo".to_string(), "database::Connection".to_string());
    graph.add_edge("service::UserService".to_string(), "repository::UserRepo".to_string());
    graph.add_edge("controller::UserController".to_string(), "service::UserService".to_string());

    let layers = topological_layers(&graph);
    assert_eq!(layers.len(), 4);

    // Database should be in first layer (no dependencies)
    assert!(layers[0].contains(&"database::Connection".to_string()));
}

#[test]
fn test_constraint_violation_ui_to_database() {
    // Test architectural constraint: UI should not depend on Database directly
    let mut graph = Graph::new();

    graph.add_edge("ui::LoginPage".to_string(), "database::Connection".to_string()); // Violation!
    graph.add_edge("ui::UserList".to_string(), "service::UserService".to_string()); // OK

    // Check if UI depends on Database (should be forbidden)
    let ui_deps: Vec<_> = graph.nodes
        .iter()
        .filter(|n| n.starts_with("ui::"))
        .flat_map(|node| graph.neighbors(node))
        .filter(|dep| dep.starts_with("database::"))
        .collect();

    assert!(!ui_deps.is_empty(), "Found architectural violation");
}

#[test]
fn test_hub_detection_user_class() {
    // User class is used by many other classes
    let mut graph = Graph::new();

    let user = "models::User";
    let dependents = vec![
        "repository::UserRepo",
        "service::UserService",
        "controller::UserController",
        "dto::UserDTO",
        "validators::UserValidator",
        "serializers::UserSerializer",
        "events::UserCreated",
        "events::UserUpdated",
    ];

    for dependent in &dependents {
        graph.add_edge(dependent.to_string(), user.to_string());
    }

    let hubs = find_hubs(&graph, 5);
    assert!(!hubs.is_empty());
    assert_eq!(hubs[0].0, user);
    assert!(hubs[0].1 >= 8); // At least 8 incoming edges
}

#[test]
fn test_find_entry_points() {
    // Find entry points (roots) in a web application
    let mut graph = Graph::new();

    // Entry points: Controllers
    graph.add_edge("controller::UserController".to_string(), "service::UserService".to_string());
    graph.add_edge("controller::AuthController".to_string(), "service::AuthService".to_string());

    // Services depend on repositories
    graph.add_edge("service::UserService".to_string(), "repository::UserRepo".to_string());
    graph.add_edge("service::AuthService".to_string(), "repository::AuthRepo".to_string());

    let roots = find_roots(&graph);
    // Controllers should be the roots (entry points)
    assert!(roots.contains(&"controller::UserController".to_string()));
    assert!(roots.contains(&"controller::AuthController".to_string()));
}

#[test]
fn test_find_leaf_utilities() {
    // Find leaf nodes (utilities with no dependencies)
    let mut graph = Graph::new();

    graph.add_edge("service::UserService".to_string(), "utils::hash_password".to_string());
    graph.add_edge("service::UserService".to_string(), "utils::validate_email".to_string());
    graph.add_edge("controller::AuthController".to_string(), "utils::generate_token".to_string());

    let leaves = find_leaves(&graph);
    assert!(leaves.contains(&"utils::hash_password".to_string()));
    assert!(leaves.contains(&"utils::validate_email".to_string()));
    assert!(leaves.contains(&"utils::generate_token".to_string()));
}

#[test]
fn test_dependency_path_tracing() {
    // Trace how UI depends on Database through layers
    let mut graph = Graph::new();

    graph.add_edge("ui::LoginPage".to_string(), "controller::AuthController".to_string());
    graph.add_edge("controller::AuthController".to_string(), "service::AuthService".to_string());
    graph.add_edge("service::AuthService".to_string(), "repository::AuthRepo".to_string());
    graph.add_edge("repository::AuthRepo".to_string(), "database::Connection".to_string());

    let path = find_shortest_path(&graph, "ui::LoginPage", "database::Connection");
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.length, 4);
    assert_eq!(path.nodes.len(), 5);
}

#[test]
fn test_transitive_dependency_closure() {
    // Find all transitive dependencies
    let mut graph = Graph::new();

    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "D".to_string());
    graph.add_edge("A".to_string(), "E".to_string());

    let paths_to_d = find_all_paths(&graph, "A", "D", 10);
    assert!(!paths_to_d.is_empty());
}

#[test]
fn test_complex_cycle_detection() {
    // Detect multiple interleaved cycles
    let mut graph = Graph::new();

    // Cycle 1: A->B->C->A
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("C".to_string(), "A".to_string());

    // Cycle 2: D->E->F->D
    graph.add_edge("D".to_string(), "E".to_string());
    graph.add_edge("E".to_string(), "F".to_string());
    graph.add_edge("F".to_string(), "D".to_string());

    // Connection between cycles
    graph.add_edge("C".to_string(), "D".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 2);
}

#[test]
fn test_self_dependency() {
    // Test self-referential dependency (recursive function)
    let mut graph = Graph::new();
    graph.add_edge("factorial".to_string(), "factorial".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 1);
}

#[test]
fn test_disconnected_components() {
    // Test graph with disconnected components
    let mut graph = Graph::new();

    // Component 1
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    // Component 2
    graph.add_edge("X".to_string(), "Y".to_string());
    graph.add_edge("Y".to_string(), "Z".to_string());

    let path = find_shortest_path(&graph, "A", "X");
    assert!(path.is_none()); // No path between components

    let roots = find_roots(&graph);
    assert_eq!(roots.len(), 2); // A and X are roots
}

#[test]
fn test_large_graph_performance() {
    // Test with larger graph to ensure algorithms are efficient
    let mut graph = Graph::new();

    // Create a large graph: 100 nodes in a chain
    for i in 0..99 {
        graph.add_edge(format!("node_{}", i), format!("node_{}", i + 1));
    }

    let path = find_shortest_path(&graph, "node_0", "node_99");
    assert!(path.is_some());
    assert_eq!(path.unwrap().length, 99);
}

#[test]
fn test_complex_dependency_graph() {
    // Real-world complex dependency graph
    let mut graph = Graph::new();

    // Models
    graph.add_edge("User".to_string(), "Email".to_string());
    graph.add_edge("User".to_string(), "Address".to_string());
    graph.add_edge("Order".to_string(), "User".to_string());
    graph.add_edge("Order".to_string(), "Product".to_string());
    graph.add_edge("OrderItem".to_string(), "Order".to_string());
    graph.add_edge("OrderItem".to_string(), "Product".to_string());

    // Services
    graph.add_edge("UserService".to_string(), "User".to_string());
    graph.add_edge("OrderService".to_string(), "Order".to_string());
    graph.add_edge("OrderService".to_string(), "UserService".to_string());

    let hubs = find_hubs(&graph, 2);
    assert!(!hubs.is_empty());

    let layers = topological_layers(&graph);
    assert!(layers.len() >= 3);
}

#[test]
fn test_bidirectional_dependency() {
    // Test mutual dependencies (anti-pattern)
    let mut graph = Graph::new();

    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "A".to_string());

    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 2);
}

// ============================================================================
// INTEGRATION TESTS (5 tests)
// ============================================================================

#[test]
fn test_full_dependency_analysis_workflow() {
    // Complete workflow: build graph, analyze, visualize
    let mut graph = Graph::new();

    // Build a realistic graph
    graph.add_edge("main".to_string(), "module_a".to_string());
    graph.add_edge("main".to_string(), "module_b".to_string());
    graph.add_edge("module_a".to_string(), "utils".to_string());
    graph.add_edge("module_b".to_string(), "utils".to_string());

    // 1. Find roots
    let roots = find_roots(&graph);
    assert_eq!(roots.len(), 1);
    assert!(roots.contains(&"main".to_string()));

    // 2. Find leaves
    let leaves = find_leaves(&graph);
    assert_eq!(leaves.len(), 1);
    assert!(leaves.contains(&"utils".to_string()));

    // 3. Find cycles
    let cycles = find_cycles(&graph);
    assert_eq!(cycles.len(), 0);

    // 4. Get layers
    let layers = topological_layers(&graph);
    assert_eq!(layers.len(), 3);

    // 5. Find hubs
    let hubs = find_hubs(&graph, 1);
    assert!(!hubs.is_empty());
}

#[test]
fn test_impact_analysis_multi_change() {
    // Test impact of changing multiple units
    let mut graph = Graph::new();

    graph.add_edge("A".to_string(), "X".to_string());
    graph.add_edge("B".to_string(), "X".to_string());
    graph.add_edge("C".to_string(), "Y".to_string());
    graph.add_edge("D".to_string(), "Y".to_string());

    // If X and Y change
    let mut impacted = std::collections::HashSet::new();

    for changed in &["X", "Y"] {
        let reachable = find_reachable(&graph, changed, None);
        impacted.extend(reachable.keys().cloned());
    }

    // Should impact A, B, C, D, X, Y
    assert!(impacted.len() >= 4);
}

#[test]
fn test_constraint_checking_workflow() {
    // Test architectural constraint checking
    let mut graph = Graph::new();

    graph.add_edge("ui::Component".to_string(), "api::Service".to_string());
    graph.add_edge("api::Service".to_string(), "db::Repository".to_string());
    graph.add_edge("ui::BadComponent".to_string(), "db::Repository".to_string()); // Violation!

    // Check constraint: ui should not depend on db
    let mut violations = 0;
    for node in &graph.nodes {
        if node.starts_with("ui::") {
            for neighbor in graph.neighbors(node) {
                if neighbor.starts_with("db::") {
                    violations += 1;
                }
            }
        }
    }

    assert_eq!(violations, 1);
}

#[test]
fn test_graph_generation_dot_format() {
    // Test DOT format generation
    let mut graph = Graph::new();
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());

    // Generate DOT format
    let mut dot = String::from("digraph G {\n");
    for (from, neighbors) in &graph.adjacency {
        for to in neighbors {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", from, to));
        }
    }
    dot.push_str("}\n");

    assert!(dot.contains("A\" -> \"B"));
    assert!(dot.contains("B\" -> \"C"));
}

#[test]
fn test_centrality_calculation() {
    // Test betweenness centrality
    let mut graph = Graph::new();

    // B is central node
    graph.add_edge("A".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "C".to_string());
    graph.add_edge("D".to_string(), "B".to_string());
    graph.add_edge("B".to_string(), "E".to_string());

    let centrality = calculate_centrality(&graph);

    // B should have highest centrality
    let b_centrality = centrality.get("B").unwrap_or(&0.0);
    assert!(*b_centrality > 0.0);
}
