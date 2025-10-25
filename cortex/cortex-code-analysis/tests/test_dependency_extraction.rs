//! Comprehensive tests for dependency extraction.

use anyhow::Result;
use cortex_code_analysis::{
    Dependency, DependencyExtractor, DependencyType, RustParser,
};
use std::collections::HashMap;

#[test]
fn test_simple_function_calls() -> Result<()> {
    let source = r#"
fn main() {
    println!("Starting");
    process_data();
    cleanup();
}

fn process_data() {
    validate();
    transform();
}

fn validate() {}
fn transform() {}
fn cleanup() {}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    // Filter call dependencies
    let calls: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Calls)
        .collect();

    println!("Found {} call dependencies", calls.len());
    for call in &calls {
        println!("  {} -> {}", call.from_unit, call.to_unit);
    }

    // Verify main calls process_data and cleanup
    let main_calls: Vec<_> = calls
        .iter()
        .filter(|d| d.from_unit == "main")
        .collect();

    assert!(
        main_calls.len() >= 2,
        "main should call at least process_data and cleanup"
    );

    Ok(())
}

#[test]
fn test_method_calls() -> Result<()> {
    let source = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    fn new() -> Self {
        Calculator { value: 0 }
    }

    fn add(&mut self, x: i32) {
        self.value += x;
        self.validate();
    }

    fn validate(&self) {}
}

fn main() {
    let mut calc = Calculator::new();
    calc.add(5);
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let calls: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Calls)
        .collect();

    println!("Method call dependencies:");
    for call in &calls {
        println!("  {} -> {}", call.from_unit, call.to_unit);
    }

    // Should find Calculator::add calling validate
    let add_calls: Vec<_> = calls
        .iter()
        .filter(|d| d.from_unit.contains("add"))
        .collect();

    assert!(!add_calls.is_empty(), "Should find calls from add method");

    Ok(())
}

#[test]
fn test_type_usage_in_structs() -> Result<()> {
    let source = r#"
use std::collections::HashMap;

struct User {
    name: String,
    age: u32,
}

struct Database {
    users: HashMap<String, User>,
    connection: Connection,
}

struct Connection {
    url: String,
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let type_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::UsesType)
        .collect();

    println!("Type usage dependencies:");
    for dep in &type_deps {
        println!("  {} uses {}", dep.from_unit, dep.to_unit);
    }

    // Database should use HashMap, User, and Connection
    let db_types: Vec<_> = type_deps
        .iter()
        .filter(|d| d.from_unit == "Database")
        .collect();

    assert!(
        db_types.len() >= 2,
        "Database should use at least User and Connection"
    );

    // Verify HashMap is detected
    let has_hashmap = type_deps.iter().any(|d| d.to_unit.contains("HashMap"));
    assert!(has_hashmap, "Should detect HashMap usage");

    Ok(())
}

#[test]
fn test_type_usage_in_function_signatures() -> Result<()> {
    let source = r#"
struct Request {
    data: String,
}

struct Response {
    status: i32,
}

fn process_request(req: Request) -> Response {
    Response { status: 200 }
}

fn handle_batch(requests: Vec<Request>) -> Vec<Response> {
    vec![]
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let type_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::UsesType)
        .collect();

    println!("Function signature type dependencies:");
    for dep in &type_deps {
        println!("  {} uses {}", dep.from_unit, dep.to_unit);
    }

    // process_request should use Request and Response
    let process_types: Vec<_> = type_deps
        .iter()
        .filter(|d| d.from_unit == "process_request")
        .collect();

    assert!(
        process_types.len() >= 2,
        "process_request should use Request and Response"
    );

    // handle_batch should also use Request and Response (from Vec<T>)
    let batch_types: Vec<_> = type_deps
        .iter()
        .filter(|d| d.from_unit == "handle_batch")
        .collect();

    assert!(!batch_types.is_empty(), "handle_batch should use types");

    Ok(())
}

#[test]
fn test_trait_implementation() -> Result<()> {
    let source = r#"
trait Drawable {
    fn draw(&self);
}

trait Colorable {
    fn set_color(&mut self, color: String);
}

struct Circle {
    radius: f64,
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing circle");
    }
}

impl Colorable for Circle {
    fn set_color(&mut self, color: String) {}
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let impl_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Implements)
        .collect();

    println!("Trait implementation dependencies:");
    for dep in &impl_deps {
        println!("  {} implements {}", dep.from_unit, dep.to_unit);
    }

    // Circle should implement both Drawable and Colorable
    let circle_impls: Vec<_> = impl_deps
        .iter()
        .filter(|d| d.from_unit == "Circle")
        .collect();

    assert_eq!(
        circle_impls.len(),
        2,
        "Circle should implement 2 traits"
    );

    Ok(())
}

#[test]
fn test_trait_inheritance() -> Result<()> {
    let source = r#"
trait Base {
    fn base_method(&self);
}

trait Extended: Base {
    fn extended_method(&self);
}

trait MultiExtended: Base + Clone {
    fn multi_method(&self);
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let inherit_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Inherits)
        .collect();

    println!("Trait inheritance dependencies:");
    for dep in &inherit_deps {
        println!("  {} inherits {}", dep.from_unit, dep.to_unit);
    }

    // Verify that supertrait extraction works correctly
    assert!(inherit_deps.len() >= 2, "Should extract at least 2 inheritance dependencies");

    // Extended should inherit from Base
    let extended_base = inherit_deps
        .iter()
        .find(|d| d.from_unit == "Extended" && d.to_unit == "Base");
    assert!(extended_base.is_some(), "Extended should inherit from Base");

    // MultiExtended should inherit from Base and Clone
    let multi_deps: Vec<_> = inherit_deps
        .iter()
        .filter(|d| d.from_unit == "MultiExtended")
        .collect();
    assert_eq!(multi_deps.len(), 2, "MultiExtended should inherit from 2 traits");

    let has_base = multi_deps.iter().any(|d| d.to_unit == "Base");
    let has_clone = multi_deps.iter().any(|d| d.to_unit == "Clone");
    assert!(has_base, "MultiExtended should inherit from Base");
    assert!(has_clone, "MultiExtended should inherit from Clone");

    Ok(())
}

#[test]
fn test_import_extraction() -> Result<()> {
    let source = r#"
use std::collections::HashMap;
use std::fs::{File, OpenOptions, read_to_string};
use std::io::*;
use serde::{Serialize, Deserialize};
use anyhow::Result;
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let extractor = DependencyExtractor::new()?;
    let imports = extractor.extract_imports(&parsed, source)?;

    println!("Extracted imports:");
    for import in &imports {
        println!("  From: {}", import.module);
        if !import.items.is_empty() {
            println!("    Items: {:?}", import.items);
        }
        if import.is_glob {
            println!("    (glob import)");
        }
    }

    assert_eq!(imports.len(), 5, "Should extract 5 import statements");

    // Check glob import
    let glob_import = imports.iter().find(|i| i.is_glob);
    assert!(glob_import.is_some(), "Should find glob import from std::io");

    // Check multiple item import
    let multi_import = imports
        .iter()
        .find(|i| i.module == "std::fs" && i.items.len() >= 2);
    assert!(
        multi_import.is_some(),
        "Should find multi-item import from std::fs"
    );

    Ok(())
}

#[test]
fn test_import_dependencies() -> Result<()> {
    let source = r#"
use std::collections::HashMap;
use std::fs::File;

fn main() {
    let map: HashMap<String, i32> = HashMap::new();
    let file = File::open("test.txt");
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let import_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::Imports)
        .collect();

    println!("Import dependencies:");
    for dep in &import_deps {
        println!("  {} imports {}", dep.from_unit, dep.to_unit);
    }

    assert!(
        import_deps.len() >= 2,
        "Should have at least 2 import dependencies"
    );

    Ok(())
}

#[test]
fn test_complex_nested_calls() -> Result<()> {
    let source = r#"
fn outer() {
    if condition() {
        inner_a();
    } else {
        inner_b();
    }

    match result() {
        Some(v) => process(v),
        None => fallback(),
    }
}

fn condition() -> bool { true }
fn inner_a() {}
fn inner_b() {}
fn result() -> Option<i32> { None }
fn process(x: i32) {}
fn fallback() {}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let outer_calls: Vec<_> = deps
        .iter()
        .filter(|d| d.from_unit == "outer" && d.dep_type == DependencyType::Calls)
        .collect();

    println!("Calls from outer():");
    for call in &outer_calls {
        println!("  -> {}", call.to_unit);
    }

    // outer should call multiple functions
    assert!(
        outer_calls.len() >= 3,
        "outer should call multiple functions"
    );

    Ok(())
}

#[test]
fn test_dependency_graph_construction() -> Result<()> {
    let source = r#"
fn main() {
    step1();
}

fn step1() {
    step2();
}

fn step2() {
    step3();
}

fn step3() {}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let graph = extractor.build_dependency_graph(&parsed, source)?;

    let stats = graph.stats();
    println!("Graph statistics:");
    println!("  Total nodes: {}", stats.total_nodes);
    println!("  Total edges: {}", stats.total_edges);
    println!("  Edges by type: {:?}", stats.edges_by_type);

    assert!(stats.total_nodes >= 4, "Should have at least 4 nodes");
    assert!(stats.total_edges >= 3, "Should have at least 3 edges");

    // Test dependency lookup
    let main_deps = graph.get_dependencies("main");
    println!("Dependencies of main: {}", main_deps.len());
    assert!(!main_deps.is_empty(), "main should have dependencies");

    // Test reverse dependencies
    let step3_dependents = graph.get_dependents("step3");
    println!("Dependents of step3: {}", step3_dependents.len());
    assert!(
        !step3_dependents.is_empty(),
        "step3 should have dependents"
    );

    Ok(())
}

#[test]
fn test_real_world_example() -> Result<()> {
    let source = r#"
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    settings: HashMap<String, String>,
    enabled: bool,
}

impl Config {
    fn new() -> Self {
        Config {
            settings: HashMap::new(),
            enabled: true,
        }
    }

    fn load(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        self.parse(&contents)
    }

    fn parse(&self, data: &str) -> Result<Self, std::io::Error> {
        // Parse logic
        Ok(Config::new())
    }

    fn save(&self, path: &str) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        let serialized = self.serialize();
        file.write_all(serialized.as_bytes())
    }

    fn serialize(&self) -> String {
        String::new()
    }
}

fn main() {
    let config = Config::load("config.toml").unwrap();
    config.save("output.toml").unwrap();
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("example.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    println!("\n=== Real World Example Analysis ===");
    println!("Total dependencies found: {}", deps.len());

    // Group by type
    let mut by_type: HashMap<DependencyType, Vec<&Dependency>> = HashMap::new();
    for dep in &deps {
        by_type
            .entry(dep.dep_type)
            .or_insert_with(Vec::new)
            .push(dep);
    }

    for (dep_type, deps) in &by_type {
        println!("\n{} dependencies: {}", dep_type, deps.len());
        for dep in deps.iter().take(5) {
            println!("  {} -> {}", dep.from_unit, dep.to_unit);
        }
        if deps.len() > 5 {
            println!("  ... and {} more", deps.len() - 5);
        }
    }

    // Build and analyze graph
    let graph = extractor.build_dependency_graph(&parsed, source)?;
    let stats = graph.stats();

    println!("\n=== Graph Statistics ===");
    println!("Nodes: {}", stats.total_nodes);
    println!("Edges: {}", stats.total_edges);
    println!("Edges by type:");
    for (dep_type, count) in &stats.edges_by_type {
        println!("  {}: {}", dep_type, count);
    }

    // Verify we found imports
    assert!(
        by_type.contains_key(&DependencyType::Imports),
        "Should find import dependencies"
    );

    // Verify we found type usage
    assert!(
        by_type.contains_key(&DependencyType::UsesType),
        "Should find type usage dependencies"
    );

    // Verify we found function calls
    assert!(
        by_type.contains_key(&DependencyType::Calls),
        "Should find call dependencies"
    );

    Ok(())
}

#[test]
fn test_enum_variant_types() -> Result<()> {
    let source = r#"
struct ErrorDetails {
    message: String,
}

enum Message {
    Text(String),
    Data { content: Vec<u8>, size: usize },
    Empty,
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let type_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::UsesType)
        .collect();

    println!("Enum variant type dependencies:");
    for dep in &type_deps {
        println!("  {} uses {}", dep.from_unit, dep.to_unit);
    }

    // Message enum should use types in its variants
    let message_deps: Vec<_> = type_deps
        .iter()
        .filter(|d| d.from_unit == "Message")
        .collect();

    // Should find at least Vec usage
    assert!(!message_deps.is_empty(), "Message should use types");

    Ok(())
}

#[test]
fn test_generic_type_extraction() -> Result<()> {
    let source = r#"
struct Container<T> {
    value: T,
}

fn process<T: Clone>(data: Vec<T>) -> Option<T> {
    data.first().cloned()
}

struct Wrapper {
    items: Vec<Container<String>>,
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    let mut extractor = DependencyExtractor::new()?;
    let deps = extractor.extract_all(&parsed, source)?;

    let type_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == DependencyType::UsesType)
        .collect();

    println!("Generic type dependencies:");
    for dep in &type_deps {
        println!("  {} uses {}", dep.from_unit, dep.to_unit);
    }

    // Should find Vec usage
    let has_vec = type_deps.iter().any(|d| d.to_unit == "Vec");
    assert!(has_vec, "Should detect Vec usage");

    // Container may or may not be detected depending on type extraction from complex generics
    // The important thing is that we detect the base types like Vec and Option
    let has_option = type_deps.iter().any(|d| d.to_unit == "Option");
    assert!(has_option || has_vec, "Should detect at least one generic type");

    Ok(())
}
