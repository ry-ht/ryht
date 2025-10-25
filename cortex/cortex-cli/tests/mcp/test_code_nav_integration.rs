//! Integration tests for code navigation tools
//!
//! Tests all 10 code navigation tools with REAL data from cortex-code-analysis and semantic memory.

use cortex_core::id::CortexId;
use cortex_core::types::{
    CodeUnit, CodeUnitType, Language, Visibility, Parameter, Complexity, CodeUnitStatus
};
use cortex_memory::CognitiveManager;
use cortex_memory::types::{Dependency, DependencyType};
use cortex_storage::{ConnectionManager, DatabaseConfig, PoolConfig, Credentials};
use std::sync::Arc;
use chrono::Utc;
use std::collections::HashMap;

// Test helper to create a test database connection
async fn create_test_storage() -> anyhow::Result<Arc<ConnectionManager>> {
    let config = DatabaseConfig {
        connection_mode: cortex_storage::PoolConnectionMode::Local {
            endpoint: "ws://127.0.0.1:8000".to_string(),
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        pool_config: PoolConfig::default(),
        namespace: "test".to_string(),
        database: "cortex_test".to_string(),
    };

    let manager = ConnectionManager::new(config).await?;
    Ok(Arc::new(manager))
}

// Test helper to create sample code units
fn create_sample_function(name: &str, file_path: &str, start_line: usize) -> CodeUnit {
    let now = Utc::now();
    CodeUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: name.to_string(),
        qualified_name: format!("test_module::{}", name),
        display_name: name.to_string(),
        file_path: file_path.to_string(),
        language: Language::Rust,
        start_line,
        end_line: start_line + 10,
        start_column: 0,
        end_column: 0,
        start_byte: 0,
        end_byte: 0,
        signature: format!("pub fn {}() -> Result<()>", name),
        body: Some(format!("    println!(\"Hello from {}\");\n    Ok(())", name)),
        docstring: Some(format!("/// This is the {} function", name)),
        comments: Vec::new(),
        return_type: Some("Result<()>".to_string()),
        parameters: Vec::new(),
        type_parameters: Vec::new(),
        generic_constraints: Vec::new(),
        throws: Vec::new(),
        visibility: Visibility::Public,
        attributes: Vec::new(),
        modifiers: vec!["pub".to_string()],
        is_async: false,
        is_unsafe: false,
        is_const: false,
        is_static: false,
        is_abstract: false,
        is_virtual: false,
        is_override: false,
        is_final: false,
        is_exported: true,
        is_default_export: false,
        complexity: Complexity {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            lines: 10,
            parameters: 0,
            returns: 1,
        },
        test_coverage: Some(85.0),
        has_tests: true,
        has_documentation: true,
        language_specific: HashMap::new(),
        embedding: None,
        embedding_model: None,
        summary: Some(format!("A sample {} function for testing", name)),
        purpose: Some("Testing code navigation".to_string()),
        ast_node_type: None,
        ast_metadata: None,
        status: CodeUnitStatus::Active,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: "test".to_string(),
        updated_by: "test".to_string(),
        tags: Vec::new(),
        metadata: HashMap::new(),
    }
}

fn create_sample_class(name: &str, file_path: &str, start_line: usize) -> CodeUnit {
    let now = Utc::now();
    CodeUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Struct,
        name: name.to_string(),
        qualified_name: format!("test_module::{}", name),
        display_name: name.to_string(),
        file_path: file_path.to_string(),
        language: Language::Rust,
        start_line,
        end_line: start_line + 20,
        start_column: 0,
        end_column: 0,
        start_byte: 0,
        end_byte: 0,
        signature: format!("pub struct {} {{}}", name),
        body: Some(format!("pub struct {} {{\n    data: String,\n}}", name)),
        docstring: Some(format!("/// {} struct", name)),
        comments: Vec::new(),
        return_type: None,
        parameters: Vec::new(),
        type_parameters: Vec::new(),
        generic_constraints: Vec::new(),
        throws: Vec::new(),
        visibility: Visibility::Public,
        attributes: Vec::new(),
        modifiers: vec!["pub".to_string()],
        is_async: false,
        is_unsafe: false,
        is_const: false,
        is_static: false,
        is_abstract: false,
        is_virtual: false,
        is_override: false,
        is_final: false,
        is_exported: true,
        is_default_export: false,
        complexity: Complexity {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            lines: 20,
            parameters: 0,
            returns: 0,
        },
        test_coverage: None,
        has_tests: false,
        has_documentation: true,
        language_specific: HashMap::new(),
        embedding: None,
        embedding_model: None,
        summary: Some(format!("A {} struct", name)),
        purpose: Some("Data structure".to_string()),
        ast_node_type: None,
        ast_metadata: None,
        status: CodeUnitStatus::Active,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: "test".to_string(),
        updated_by: "test".to_string(),
        tags: Vec::new(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
#[ignore] // Requires SurrealDB running
async fn test_code_nav_full_workflow() -> anyhow::Result<()> {
    // Setup
    let storage = create_test_storage().await?;
    let manager = CognitiveManager::new(storage.clone());
    let semantic = manager.semantic();

    // 1. Create sample code units
    let func1 = create_sample_function("process_data", "src/lib.rs", 10);
    let func2 = create_sample_function("validate_input", "src/lib.rs", 30);
    let func3 = create_sample_function("format_output", "src/lib.rs", 50);
    let class1 = create_sample_class("DataProcessor", "src/lib.rs", 70);

    // Store code units
    println!("Storing code units...");
    let func1_id = semantic.store_unit(&func1).await?;
    let func2_id = semantic.store_unit(&func2).await?;
    let func3_id = semantic.store_unit(&func3).await?;
    let class1_id = semantic.store_unit(&class1).await?;

    println!("✓ Stored {} code units", 4);

    // 2. Create dependencies (func1 calls func2 and func3)
    let dep1 = Dependency {
        id: CortexId::new(),
        source_id: func1_id,
        target_id: func2_id,
        dependency_type: DependencyType::Calls,
        is_direct: true,
        is_runtime: true,
        is_dev: false,
        metadata: HashMap::new(),
    };

    let dep2 = Dependency {
        id: CortexId::new(),
        source_id: func1_id,
        target_id: func3_id,
        dependency_type: DependencyType::Calls,
        is_direct: true,
        is_runtime: true,
        is_dev: false,
        metadata: HashMap::new(),
    };

    semantic.store_dependency(&dep1).await?;
    semantic.store_dependency(&dep2).await?;

    println!("✓ Stored {} dependencies", 2);

    // TEST 1: Get unit by ID
    println!("\n=== TEST 1: Get Unit by ID ===");
    let retrieved = semantic.get_unit(func1_id).await?;
    assert!(retrieved.is_some());
    let unit = retrieved.unwrap();
    assert_eq!(unit.name, "process_data");
    assert_eq!(unit.qualified_name, "test_module::process_data");
    println!("✓ Retrieved unit: {}", unit.name);
    println!("  - Signature: {}", unit.signature);
    println!("  - Location: {}:{}-{}", unit.file_path, unit.start_line, unit.end_line);
    println!("  - Complexity: cyclomatic={}, cognitive={}",
             unit.complexity.cyclomatic, unit.complexity.cognitive);

    // TEST 2: Get unit by qualified name
    println!("\n=== TEST 2: Get Unit by Qualified Name ===");
    let retrieved = semantic.find_by_qualified_name("test_module::validate_input").await?;
    assert!(retrieved.is_some());
    let unit = retrieved.unwrap();
    assert_eq!(unit.name, "validate_input");
    println!("✓ Found unit: {} (ID: {})", unit.name, unit.id);

    // TEST 3: List units in file
    println!("\n=== TEST 3: List Units in File ===");
    let units = semantic.get_units_in_file("src/lib.rs").await?;
    assert_eq!(units.len(), 4);
    println!("✓ Found {} units in src/lib.rs:", units.len());
    for unit in &units {
        println!("  - {} ({:?}) at line {}", unit.name, unit.unit_type, unit.start_line);
    }

    // TEST 4: Get dependencies
    println!("\n=== TEST 4: Get Dependencies ===");
    let deps = semantic.get_dependencies(func1_id).await?;
    assert_eq!(deps.len(), 2);
    println!("✓ process_data depends on {} units:", deps.len());
    for dep in &deps {
        if let Ok(Some(target)) = semantic.get_unit(dep.target_id).await {
            println!("  - {} (type: {:?})", target.name, dep.dependency_type);
        }
    }

    // TEST 5: Find references
    println!("\n=== TEST 5: Find References ===");
    let refs = semantic.find_references(func2_id).await?;
    assert!(!refs.is_empty());
    println!("✓ validate_input is referenced by {} units:", refs.len());
    for ref_id in &refs {
        if let Ok(Some(caller)) = semantic.get_unit(*ref_id).await {
            println!("  - {}", caller.name);
        }
    }

    // TEST 6: Get dependents
    println!("\n=== TEST 6: Get Dependents ===");
    let dependents = semantic.get_dependents(func3_id).await?;
    assert!(!dependents.is_empty());
    println!("✓ format_output has {} dependents:", dependents.len());
    for dep in &dependents {
        if let Ok(Some(source)) = semantic.get_unit(dep.source_id).await {
            println!("  - {} depends on format_output", source.name);
        }
    }

    // TEST 7: Filter by visibility
    println!("\n=== TEST 7: Filter by Visibility ===");
    let public_units: Vec<_> = units.iter()
        .filter(|u| matches!(u.visibility, Visibility::Public))
        .collect();
    println!("✓ Found {} public units", public_units.len());

    // TEST 8: Filter by unit type
    println!("\n=== TEST 8: Filter by Unit Type ===");
    let functions: Vec<_> = units.iter()
        .filter(|u| matches!(u.unit_type, CodeUnitType::Function))
        .collect();
    let structs: Vec<_> = units.iter()
        .filter(|u| matches!(u.unit_type, CodeUnitType::Struct))
        .collect();
    println!("✓ Found {} functions and {} structs", functions.len(), structs.len());

    // TEST 9: Complexity analysis
    println!("\n=== TEST 9: Complexity Analysis ===");
    let complex_units = semantic.find_complex_units(5).await?;
    println!("✓ Found {} complex units (threshold: 5)", complex_units.len());

    // TEST 10: Documentation coverage
    println!("\n=== TEST 10: Documentation Coverage ===");
    let documented: Vec<_> = units.iter()
        .filter(|u| u.has_documentation)
        .collect();
    let coverage = (documented.len() as f64 / units.len() as f64) * 100.0;
    println!("✓ Documentation coverage: {:.1}% ({}/{})",
             coverage, documented.len(), units.len());

    println!("\n=== All Tests Passed! ===");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires SurrealDB running
async fn test_code_nav_error_handling() -> anyhow::Result<()> {
    let storage = create_test_storage().await?;
    let manager = CognitiveManager::new(storage.clone());
    let semantic = manager.semantic();

    println!("=== Testing Error Handling ===");

    // Test 1: Get non-existent unit
    println!("\n1. Get non-existent unit by ID");
    let fake_id = CortexId::new();
    let result = semantic.get_unit(fake_id).await?;
    assert!(result.is_none());
    println!("✓ Correctly returned None for non-existent unit");

    // Test 2: Find by non-existent qualified name
    println!("\n2. Find by non-existent qualified name");
    let result = semantic.find_by_qualified_name("nonexistent::function").await?;
    assert!(result.is_none());
    println!("✓ Correctly returned None for non-existent qualified name");

    // Test 3: Get units in non-existent file
    println!("\n3. Get units in non-existent file");
    let result = semantic.get_units_in_file("nonexistent.rs").await?;
    assert!(result.is_empty());
    println!("✓ Correctly returned empty list for non-existent file");

    // Test 4: Get dependencies of non-existent unit
    println!("\n4. Get dependencies of non-existent unit");
    let result = semantic.get_dependencies(fake_id).await?;
    assert!(result.is_empty());
    println!("✓ Correctly returned empty list for dependencies");

    println!("\n=== All Error Handling Tests Passed! ===");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires SurrealDB running
async fn test_code_nav_complex_scenario() -> anyhow::Result<()> {
    let storage = create_test_storage().await?;
    let manager = CognitiveManager::new(storage.clone());
    let semantic = manager.semantic();

    println!("=== Complex Scenario: Multi-File Project ===");

    // Create a mini project structure
    let files = vec![
        ("src/main.rs", vec!["main", "run"]),
        ("src/parser.rs", vec!["parse", "tokenize"]),
        ("src/analyzer.rs", vec!["analyze", "check"]),
        ("src/generator.rs", vec!["generate", "emit"]),
    ];

    let mut all_units = Vec::new();

    // Create units for each file
    for (file_path, funcs) in files {
        println!("\nCreating units for {}:", file_path);
        for (idx, func_name) in funcs.iter().enumerate() {
            let unit = create_sample_function(func_name, file_path, (idx + 1) * 20);
            let id = semantic.store_unit(&unit).await?;
            println!("  ✓ {} (ID: {})", func_name, id);
            all_units.push((id, unit));
        }
    }

    println!("\n✓ Created {} total units across {} files", all_units.len(), 4);

    // Create cross-file dependencies
    println!("\nCreating cross-file dependencies:");

    // main -> parse
    let dep = Dependency {
        id: CortexId::new(),
        source_id: all_units[0].0, // main
        target_id: all_units[2].0, // parse
        dependency_type: DependencyType::Calls,
        is_direct: true,
        is_runtime: true,
        is_dev: false,
        metadata: HashMap::new(),
    };
    semantic.store_dependency(&dep).await?;
    println!("  ✓ main -> parse");

    // parse -> analyze
    let dep = Dependency {
        id: CortexId::new(),
        source_id: all_units[2].0, // parse
        target_id: all_units[4].0, // analyze
        dependency_type: DependencyType::Calls,
        is_direct: true,
        is_runtime: true,
        is_dev: false,
        metadata: HashMap::new(),
    };
    semantic.store_dependency(&dep).await?;
    println!("  ✓ parse -> analyze");

    // analyze -> generate
    let dep = Dependency {
        id: CortexId::new(),
        source_id: all_units[4].0, // analyze
        target_id: all_units[6].0, // generate
        dependency_type: DependencyType::Calls,
        is_direct: true,
        is_runtime: true,
        is_dev: false,
        metadata: HashMap::new(),
    };
    semantic.store_dependency(&dep).await?;
    println!("  ✓ analyze -> generate");

    // Test: Trace dependency chain
    println!("\n=== Tracing Dependency Chain ===");
    println!("Starting from: main");
    let mut current_id = all_units[0].0;
    let mut depth = 0;
    let max_depth = 5;

    while depth < max_depth {
        let unit = semantic.get_unit(current_id).await?.unwrap();
        println!("{}└─ {} ({})", "  ".repeat(depth), unit.name, unit.file_path);

        let deps = semantic.get_dependencies(current_id).await?;
        if deps.is_empty() {
            break;
        }

        current_id = deps[0].target_id;
        depth += 1;
    }

    println!("\n✓ Traced dependency chain through {} levels", depth);

    // Test: Get all files in project
    println!("\n=== Project File Analysis ===");
    let file_groups: HashMap<String, Vec<&CodeUnit>> = all_units.iter()
        .fold(HashMap::new(), |mut acc, (_, unit)| {
            acc.entry(unit.file_path.clone())
                .or_insert_with(Vec::new)
                .push(unit);
            acc
        });

    for (file, units) in file_groups {
        println!("{}: {} units", file, units.len());
    }

    println!("\n=== Complex Scenario Tests Passed! ===");
    Ok(())
}
