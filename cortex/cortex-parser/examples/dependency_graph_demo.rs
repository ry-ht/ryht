//! Demonstration of dependency extraction and graph construction.
//!
//! Run with: cargo run --example dependency_graph_demo

use cortex_parser::{DependencyExtractor, DependencyType, RustParser};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // Example source code to analyze
    let source = r#"
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

/// User management system
struct UserManager {
    users: HashMap<String, User>,
    config: Config,
}

struct User {
    id: String,
    name: String,
    email: String,
}

struct Config {
    max_users: usize,
    enabled: bool,
}

impl UserManager {
    fn new(config: Config) -> Self {
        UserManager {
            users: HashMap::new(),
            config,
        }
    }

    fn add_user(&mut self, user: User) -> Result<(), String> {
        if self.users.len() >= self.config.max_users {
            return Err("Max users reached".to_string());
        }
        self.users.insert(user.id.clone(), user);
        Ok(())
    }

    fn get_user(&self, id: &str) -> Option<&User> {
        self.users.get(id)
    }

    fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let data = self.serialize();
        file.write_all(data.as_bytes())
    }

    fn serialize(&self) -> String {
        // Serialization logic
        String::new()
    }
}

impl User {
    fn new(id: String, name: String, email: String) -> Self {
        User { id, name, email }
    }

    fn validate(&self) -> bool {
        !self.email.is_empty() && self.email.contains('@')
    }
}

fn main() {
    let config = Config {
        max_users: 100,
        enabled: true,
    };

    let mut manager = UserManager::new(config);

    let user = User::new(
        "1".to_string(),
        "Alice".to_string(),
        "alice@example.com".to_string(),
    );

    manager.add_user(user).unwrap();
    manager.save_to_file("users.dat").unwrap();
}
"#;

    println!("🔍 Analyzing code dependencies...\n");

    // Parse the source code
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("user_manager.rs", source)?;

    println!("📊 Parsed Code Elements:");
    println!("  - Functions: {}", parsed.functions.len());
    println!("  - Structs: {}", parsed.structs.len());
    println!("  - Impl blocks: {}", parsed.impls.len());
    println!("  - Imports: {}", parsed.imports.len());
    println!();

    // Extract dependencies
    let mut extractor = DependencyExtractor::new()?;
    let dependencies = extractor.extract_all(&parsed, source)?;

    println!("🔗 Total Dependencies Found: {}\n", dependencies.len());

    // Group dependencies by type
    let mut by_type: HashMap<DependencyType, Vec<_>> = HashMap::new();
    for dep in &dependencies {
        by_type
            .entry(dep.dep_type)
            .or_insert_with(Vec::new)
            .push(dep);
    }

    // Display dependencies by type
    for (dep_type, deps) in &by_type {
        println!("📌 {} dependencies: {}", dep_type, deps.len());
        for dep in deps.iter().take(10) {
            println!("   {} → {}", dep.from_unit, dep.to_unit);
        }
        if deps.len() > 10 {
            println!("   ... and {} more", deps.len() - 10);
        }
        println!();
    }

    // Build and analyze the dependency graph
    let graph = extractor.build_dependency_graph(&parsed, source)?;
    let stats = graph.stats();

    println!("📈 Dependency Graph Statistics:");
    println!("  Total nodes: {}", stats.total_nodes);
    println!("  Total edges: {}", stats.total_edges);
    println!("\n  Edges by type:");
    for (dep_type, count) in &stats.edges_by_type {
        println!("    {}: {}", dep_type, count);
    }
    println!();

    // Analyze specific components
    println!("🔎 Analyzing UserManager dependencies:");
    let manager_deps = graph.get_dependencies("UserManager");
    println!("  Direct dependencies: {}", manager_deps.len());
    for dep in manager_deps {
        println!("    → {} ({})", dep.to_unit, dep.dep_type);
    }
    println!();

    // Analyze main function
    println!("🔎 Analyzing main function calls:");
    let all_main_deps = graph.get_dependencies("main");
    let main_deps: Vec<_> = all_main_deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Calls)
        .collect();
    println!("  Function calls: {}", main_deps.len());
    for dep in main_deps {
        println!("    → {}", dep.to_unit);
    }
    println!();

    // Find who depends on User
    println!("🔎 Finding dependents of User:");
    let user_dependents = graph.get_dependents("User");
    println!("  Used by {} components:", user_dependents.len());
    for dep in user_dependents {
        println!("    ← {} ({})", dep.from_unit, dep.dep_type);
    }
    println!();

    // Complexity analysis
    println!("🎯 Key Insights:");
    let total_calls = stats
        .edges_by_type
        .get(&DependencyType::Calls)
        .unwrap_or(&0);
    let total_types = stats
        .edges_by_type
        .get(&DependencyType::UsesType)
        .unwrap_or(&0);

    println!("  - {} function calls create execution flow", total_calls);
    println!("  - {} type dependencies define data structures", total_types);
    println!(
        "  - Average dependencies per node: {:.2}",
        stats.total_edges as f64 / stats.total_nodes as f64
    );

    // Find most connected nodes
    let mut node_degrees: HashMap<String, usize> = HashMap::new();
    for dep in &dependencies {
        *node_degrees.entry(dep.from_unit.clone()).or_insert(0) += 1;
    }

    let mut sorted_nodes: Vec<_> = node_degrees.iter().collect();
    sorted_nodes.sort_by(|a, b| b.1.cmp(a.1));

    println!("\n  Most connected components:");
    for (node, degree) in sorted_nodes.iter().take(5) {
        println!("    {} ({} outgoing dependencies)", node, degree);
    }

    println!("\n✅ Analysis complete!");

    Ok(())
}
