//! Demonstration of the CodeUnit schema
//!
//! This example shows how to create and use CodeUnit instances
//! with all the comprehensive features of the new schema.

use cortex_core::types::*;
use std::collections::HashMap;

fn main() {
    println!("=== CodeUnit Schema Demo ===\n");

    // Example 1: Simple Rust function
    demo_rust_function();

    // Example 2: TypeScript class with decorators
    demo_typescript_class();

    // Example 3: Python async function
    demo_python_async();

    // Example 4: Complex unit with full metadata
    demo_complex_unit();

    println!("\n=== All examples completed successfully! ===");
}

fn demo_rust_function() {
    println!("1. Rust Function Example:");

    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "calculate_total".to_string(),
        "finance::calculate_total".to_string(),
        "src/finance.rs".to_string(),
        Language::Rust,
    );

    // Set location
    unit.start_line = 42;
    unit.end_line = 58;
    unit.start_column = 0;
    unit.end_column = 1;

    // Set signature and metadata
    unit.signature = "pub fn calculate_total(values: &[f64]) -> Result<f64>".to_string();
    unit.visibility = Visibility::Public;
    unit.return_type = Some("Result<f64>".to_string());

    // Set parameters
    unit.parameters = vec![Parameter {
        name: "values".to_string(),
        param_type: Some("&[f64]".to_string()),
        default_value: None,
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    }];

    // Set complexity
    unit.complexity = Complexity {
        cyclomatic: 5,
        cognitive: 7,
        nesting: 2,
        lines: 16,
        parameters: 1,
        returns: 1,
    };

    // Set quality metrics
    unit.has_tests = true;
    unit.has_documentation = true;
    unit.test_coverage = Some(0.95);

    println!("   Name: {}", unit.name);
    println!("   Qualified Name: {}", unit.qualified_name);
    println!("   Language: {:?}", unit.language);
    println!("   Visibility: {:?}", unit.visibility);
    println!("   Complexity Score: {:.2}", unit.complexity_score());
    println!("   Has Tests: {}", unit.has_tests);
    println!("   Needs Documentation: {}", unit.needs_documentation());
    println!();
}

fn demo_typescript_class() {
    println!("2. TypeScript Class Example:");

    let mut unit = CodeUnit::new(
        CodeUnitType::Class,
        "UserService".to_string(),
        "services::UserService".to_string(),
        "src/services/user.service.ts".to_string(),
        Language::TypeScript,
    );

    // Add decorators
    unit.attributes = vec![
        Attribute {
            name: "Injectable".to_string(),
            arguments: vec![],
            metadata: HashMap::new(),
        },
        Attribute {
            name: "Service".to_string(),
            arguments: vec![],
            metadata: HashMap::new(),
        },
    ];

    // Set type parameters
    unit.type_parameters = vec![TypeParameter {
        name: "T".to_string(),
        bounds: vec!["User".to_string()],
        default_type: Some("DefaultUser".to_string()),
        variance: None,
    }];

    unit.visibility = Visibility::Public;
    unit.is_exported = true;

    println!("   Name: {}", unit.name);
    println!("   Type: {:?}", unit.unit_type);
    println!("   Attributes: {} decorators", unit.attributes.len());
    println!("   Type Parameters: {:?}", unit.type_parameters.len());
    println!("   Is Exported: {}", unit.is_exported);
    println!("   Is Type Definition: {}", unit.is_type_definition());
    println!();
}

fn demo_python_async() {
    println!("3. Python Async Function Example:");

    let mut unit = CodeUnit::new(
        CodeUnitType::AsyncFunction,
        "fetch_user_data".to_string(),
        "api.users.fetch_user_data".to_string(),
        "api/users.py".to_string(),
        Language::Python,
    );

    unit.signature = "async def fetch_user_data(user_id: int) -> dict:".to_string();
    unit.is_async = true;
    unit.visibility = Visibility::Public;

    unit.parameters = vec![Parameter {
        name: "user_id".to_string(),
        param_type: Some("int".to_string()),
        default_value: None,
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    }];

    unit.return_type = Some("dict".to_string());
    unit.docstring = Some("Fetches user data from the API".to_string());

    println!("   Name: {}", unit.name);
    println!("   Is Async: {}", unit.is_async);
    println!("   Is Callable: {}", unit.is_callable());
    println!("   Return Type: {:?}", unit.return_type);
    println!("   Has Docstring: {}", unit.docstring.is_some());
    println!();
}

fn demo_complex_unit() {
    println!("4. Complex Unit with Full Metadata:");

    let mut unit = CodeUnit::new(
        CodeUnitType::Method,
        "process_transaction".to_string(),
        "Payment::process_transaction".to_string(),
        "src/payment.rs".to_string(),
        Language::Rust,
    );

    // Full location info
    unit.start_line = 100;
    unit.end_line = 150;
    unit.start_column = 4;
    unit.end_column = 5;
    unit.start_byte = 2500;
    unit.end_byte = 4000;

    // Rich signature
    unit.signature = "pub async unsafe fn process_transaction<T: Transaction>(&mut self, tx: T) -> Result<Receipt>".to_string();

    // All the boolean flags
    unit.is_async = true;
    unit.is_unsafe = true;
    unit.visibility = Visibility::Public;

    // Type information
    unit.type_parameters = vec![TypeParameter {
        name: "T".to_string(),
        bounds: vec!["Transaction".to_string()],
        default_type: None,
        variance: None,
    }];

    unit.parameters = vec![
        Parameter {
            name: "self".to_string(),
            param_type: Some("&mut Self".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
        Parameter {
            name: "tx".to_string(),
            param_type: Some("T".to_string()),
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        },
    ];

    unit.return_type = Some("Result<Receipt>".to_string());
    unit.throws = vec!["TransactionError".to_string()];

    // Complex metrics
    unit.complexity = Complexity {
        cyclomatic: 25,
        cognitive: 45,
        nesting: 5,
        lines: 50,
        parameters: 2,
        returns: 3,
    };

    // Language-specific metadata
    unit.language_specific
        .insert("lifetimes".to_string(), serde_json::json!(["'a"]));
    unit.language_specific
        .insert("unsafe_reason".to_string(), serde_json::json!("raw pointer manipulation"));

    // Tags
    unit.tags = vec![
        "critical".to_string(),
        "payment".to_string(),
        "async".to_string(),
    ];

    // Quality metrics
    unit.has_tests = true;
    unit.has_documentation = true;
    unit.test_coverage = Some(0.88);

    println!("   Name: {}", unit.name);
    println!("   Qualified Name: {}", unit.qualified_name);
    println!("   Is Async: {}", unit.is_async);
    println!("   Is Unsafe: {}", unit.is_unsafe);
    println!("   Is Callable: {}", unit.is_callable());
    println!("   Parameters: {}", unit.parameters.len());
    println!("   Type Parameters: {}", unit.type_parameters.len());
    println!("   Complexity - Cyclomatic: {}", unit.complexity.cyclomatic);
    println!("   Complexity - Cognitive: {}", unit.complexity.cognitive);
    println!("   Complexity Score: {:.2}", unit.complexity_score());
    println!("   Test Coverage: {:.0}%", unit.test_coverage.unwrap_or(0.0) * 100.0);
    println!("   Tags: {:?}", unit.tags);
    println!("   Needs Tests: {}", unit.needs_tests());
    println!("   Needs Documentation: {}", unit.needs_documentation());

    // Serialize to JSON
    match serde_json::to_string_pretty(&unit) {
        Ok(json) => {
            println!("\n   Serialized to JSON ({} bytes)", json.len());
            println!("   First 200 chars: {}...", &json[..200.min(json.len())]);
        }
        Err(e) => println!("   Failed to serialize: {}", e),
    }

    println!();
}
