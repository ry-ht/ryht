//! Comprehensive tests for the CodeUnit schema
//!
//! Tests cover:
//! - Creation and initialization
//! - Serialization/deserialization
//! - Language detection
//! - Type conversions
//! - Utility methods
//! - Edge cases

use cortex_core::types::*;
use std::collections::HashMap;

#[test]
fn test_code_unit_creation() {
    let unit = CodeUnit::new(
        CodeUnitType::Function,
        "test_function".to_string(),
        "module::test_function".to_string(),
        "/path/to/file.rs".to_string(),
        Language::Rust,
    );

    assert_eq!(unit.name, "test_function");
    assert_eq!(unit.qualified_name, "module::test_function");
    assert_eq!(unit.display_name, "test_function");
    assert_eq!(unit.file_path, "/path/to/file.rs");
    assert_eq!(unit.language, Language::Rust);
    assert_eq!(unit.unit_type, CodeUnitType::Function);
    assert_eq!(unit.visibility, Visibility::Private);
    assert_eq!(unit.status, CodeUnitStatus::Active);
    assert_eq!(unit.version, 1);
}

#[test]
fn test_language_from_extension() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    assert_eq!(Language::from_extension("js"), Language::JavaScript);
    assert_eq!(Language::from_extension("jsx"), Language::JavaScript);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("go"), Language::Go);
    assert_eq!(Language::from_extension("java"), Language::Java);
    assert_eq!(Language::from_extension("cpp"), Language::Cpp);
    assert_eq!(Language::from_extension("c"), Language::C);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}

#[test]
fn test_code_unit_type_variants() {
    // Test all code unit types can be created
    let types = vec![
        CodeUnitType::Function,
        CodeUnitType::Method,
        CodeUnitType::AsyncFunction,
        CodeUnitType::Generator,
        CodeUnitType::Lambda,
        CodeUnitType::Class,
        CodeUnitType::Struct,
        CodeUnitType::Enum,
        CodeUnitType::Union,
        CodeUnitType::Interface,
        CodeUnitType::Trait,
        CodeUnitType::TypeAlias,
        CodeUnitType::Typedef,
        CodeUnitType::Const,
        CodeUnitType::Static,
        CodeUnitType::Variable,
        CodeUnitType::Module,
        CodeUnitType::Namespace,
        CodeUnitType::Package,
        CodeUnitType::ImplBlock,
        CodeUnitType::Decorator,
        CodeUnitType::Macro,
        CodeUnitType::Template,
        CodeUnitType::Test,
        CodeUnitType::Benchmark,
        CodeUnitType::Example,
    ];

    for unit_type in types {
        let unit = CodeUnit::new(
            unit_type,
            "test".to_string(),
            "test".to_string(),
            "test.rs".to_string(),
            Language::Rust,
        );
        assert_eq!(unit.unit_type, unit_type);
    }
}

#[test]
fn test_visibility_variants() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );

    let visibilities = vec![
        Visibility::Public,
        Visibility::Private,
        Visibility::Protected,
        Visibility::Internal,
        Visibility::Package,
    ];

    for visibility in visibilities {
        unit.visibility = visibility;
        assert_eq!(unit.visibility, visibility);
    }
}

#[test]
fn test_parameter_creation() {
    let param = Parameter {
        name: "value".to_string(),
        param_type: Some("i32".to_string()),
        default_value: Some("0".to_string()),
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    };

    assert_eq!(param.name, "value");
    assert_eq!(param.param_type, Some("i32".to_string()));
    assert_eq!(param.default_value, Some("0".to_string()));
    assert!(!param.is_optional);
    assert!(!param.is_variadic);
}

#[test]
fn test_type_parameter_creation() {
    let type_param = TypeParameter {
        name: "T".to_string(),
        bounds: vec!["Clone".to_string(), "Debug".to_string()],
        default_type: None,
        variance: Some(Variance::Covariant),
    };

    assert_eq!(type_param.name, "T");
    assert_eq!(type_param.bounds.len(), 2);
    assert_eq!(type_param.variance, Some(Variance::Covariant));
}

#[test]
fn test_attribute_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), serde_json::json!("value"));

    let attr = Attribute {
        name: "derive".to_string(),
        arguments: vec!["Debug".to_string(), "Clone".to_string()],
        metadata,
    };

    assert_eq!(attr.name, "derive");
    assert_eq!(attr.arguments.len(), 2);
    assert_eq!(attr.metadata.len(), 1);
}

#[test]
fn test_complexity_default() {
    let complexity = Complexity::default();

    assert_eq!(complexity.cyclomatic, 1);
    assert_eq!(complexity.cognitive, 1);
    assert_eq!(complexity.nesting, 0);
    assert_eq!(complexity.lines, 0);
    assert_eq!(complexity.parameters, 0);
    assert_eq!(complexity.returns, 0);
}

#[test]
fn test_complexity_custom() {
    let complexity = Complexity {
        cyclomatic: 15,
        cognitive: 25,
        nesting: 4,
        lines: 150,
        parameters: 5,
        returns: 2,
    };

    assert_eq!(complexity.cyclomatic, 15);
    assert_eq!(complexity.cognitive, 25);
    assert_eq!(complexity.nesting, 4);
    assert_eq!(complexity.lines, 150);
}

#[test]
fn test_code_unit_full_featured() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Method,
        "calculate".to_string(),
        "MyClass::calculate".to_string(),
        "src/lib.rs".to_string(),
        Language::Rust,
    );

    // Set location
    unit.start_line = 10;
    unit.end_line = 25;
    unit.start_column = 4;
    unit.end_column = 5;
    unit.start_byte = 150;
    unit.end_byte = 500;

    // Set signature and body
    unit.signature = "pub fn calculate(&self, value: i32) -> Result<i32>".to_string();
    unit.body = Some("// implementation".to_string());
    unit.docstring = Some("Calculates a value".to_string());
    unit.comments = vec!["// Important note".to_string()];

    // Set type information
    unit.return_type = Some("Result<i32>".to_string());
    unit.parameters = vec![Parameter {
        name: "value".to_string(),
        param_type: Some("i32".to_string()),
        default_value: None,
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    }];

    // Set metadata
    unit.visibility = Visibility::Public;
    unit.is_async = false;
    unit.is_unsafe = false;
    unit.is_const = false;

    // Set metrics
    unit.complexity = Complexity {
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        lines: 15,
        parameters: 1,
        returns: 1,
    };
    unit.has_tests = true;
    unit.has_documentation = true;

    assert_eq!(unit.name, "calculate");
    assert_eq!(unit.start_line, 10);
    assert_eq!(unit.end_line, 25);
    assert_eq!(unit.visibility, Visibility::Public);
    assert_eq!(unit.parameters.len(), 1);
    assert_eq!(unit.complexity.cyclomatic, 5);
    assert!(unit.has_tests);
    assert!(unit.has_documentation);
}

#[test]
fn test_code_unit_deprecation() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "old_function".to_string(),
        "module::old_function".to_string(),
        "src/lib.rs".to_string(),
        Language::Rust,
    );

    assert_eq!(unit.status, CodeUnitStatus::Active);

    unit.deprecate();

    assert_eq!(unit.status, CodeUnitStatus::Deprecated);
}

#[test]
fn test_code_unit_is_callable() {
    let function = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(function.is_callable());

    let method = CodeUnit::new(
        CodeUnitType::Method,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(method.is_callable());

    let async_fn = CodeUnit::new(
        CodeUnitType::AsyncFunction,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(async_fn.is_callable());

    let struct_unit = CodeUnit::new(
        CodeUnitType::Struct,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(!struct_unit.is_callable());
}

#[test]
fn test_code_unit_is_type_definition() {
    let class = CodeUnit::new(
        CodeUnitType::Class,
        "test".to_string(),
        "test".to_string(),
        "test.ts".to_string(),
        Language::TypeScript,
    );
    assert!(class.is_type_definition());

    let struct_unit = CodeUnit::new(
        CodeUnitType::Struct,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(struct_unit.is_type_definition());

    let trait_unit = CodeUnit::new(
        CodeUnitType::Trait,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(trait_unit.is_type_definition());

    let function = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(!function.is_type_definition());
}

#[test]
fn test_code_unit_is_test() {
    let test_unit = CodeUnit::new(
        CodeUnitType::Test,
        "test_something".to_string(),
        "tests::test_something".to_string(),
        "tests/mod.rs".to_string(),
        Language::Rust,
    );
    assert!(test_unit.is_test());

    let benchmark = CodeUnit::new(
        CodeUnitType::Benchmark,
        "bench_something".to_string(),
        "benches::bench_something".to_string(),
        "benches/mod.rs".to_string(),
        Language::Rust,
    );
    assert!(benchmark.is_test());

    let function = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );
    assert!(!function.is_test());
}

#[test]
fn test_complexity_score() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );

    // Low complexity
    unit.complexity = Complexity {
        cyclomatic: 1,
        cognitive: 1,
        nesting: 0,
        lines: 10,
        parameters: 0,
        returns: 0,
    };
    let score = unit.complexity_score();
    assert!(score < 0.1, "Low complexity should have low score");

    // High complexity
    unit.complexity = Complexity {
        cyclomatic: 50,
        cognitive: 100,
        nesting: 10,
        lines: 500,
        parameters: 0,
        returns: 0,
    };
    let score = unit.complexity_score();
    assert!(score > 0.9, "High complexity should have high score");
}

#[test]
fn test_needs_documentation() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );

    // Private function without docs - doesn't need docs
    unit.visibility = Visibility::Private;
    unit.has_documentation = false;
    assert!(!unit.needs_documentation());

    // Public function without docs - needs docs
    unit.visibility = Visibility::Public;
    unit.has_documentation = false;
    assert!(unit.needs_documentation());

    // Public function with docs - doesn't need docs
    unit.visibility = Visibility::Public;
    unit.has_documentation = true;
    assert!(!unit.needs_documentation());
}

#[test]
fn test_needs_tests() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );

    // Private function without tests - doesn't need tests
    unit.visibility = Visibility::Private;
    unit.has_tests = false;
    assert!(!unit.needs_tests());

    // Public function without tests - needs tests
    unit.visibility = Visibility::Public;
    unit.has_tests = false;
    assert!(unit.needs_tests());

    // Public function with tests - doesn't need tests
    unit.visibility = Visibility::Public;
    unit.has_tests = true;
    assert!(!unit.needs_tests());

    // Public struct without tests - doesn't need tests (not callable)
    unit.unit_type = CodeUnitType::Struct;
    unit.visibility = Visibility::Public;
    unit.has_tests = false;
    assert!(!unit.needs_tests());
}

#[test]
fn test_code_unit_serialization() {
    let unit = CodeUnit::new(
        CodeUnitType::Function,
        "serialize_test".to_string(),
        "module::serialize_test".to_string(),
        "src/lib.rs".to_string(),
        Language::Rust,
    );

    // Test serialization
    let json = serde_json::to_string(&unit).expect("Should serialize");
    assert!(json.contains("serialize_test"));
    assert!(json.contains("function"));

    // Test deserialization
    let deserialized: CodeUnit = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.name, unit.name);
    assert_eq!(deserialized.qualified_name, unit.qualified_name);
    assert_eq!(deserialized.unit_type, unit.unit_type);
    assert_eq!(deserialized.language, unit.language);
}

#[test]
fn test_rust_function_with_generics() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "generic_fn".to_string(),
        "module::generic_fn".to_string(),
        "src/lib.rs".to_string(),
        Language::Rust,
    );

    unit.signature = "pub fn generic_fn<T: Clone + Debug>(value: T) -> T".to_string();
    unit.type_parameters = vec![TypeParameter {
        name: "T".to_string(),
        bounds: vec!["Clone".to_string(), "Debug".to_string()],
        default_type: None,
        variance: None,
    }];
    unit.parameters = vec![Parameter {
        name: "value".to_string(),
        param_type: Some("T".to_string()),
        default_value: None,
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    }];
    unit.return_type = Some("T".to_string());

    assert_eq!(unit.type_parameters.len(), 1);
    assert_eq!(unit.type_parameters[0].name, "T");
    assert_eq!(unit.type_parameters[0].bounds.len(), 2);
}

#[test]
fn test_typescript_class_with_decorators() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Class,
        "UserController".to_string(),
        "controllers::UserController".to_string(),
        "src/controllers/user.ts".to_string(),
        Language::TypeScript,
    );

    unit.attributes = vec![
        Attribute {
            name: "Controller".to_string(),
            arguments: vec!["/api/users".to_string()],
            metadata: HashMap::new(),
        },
        Attribute {
            name: "Injectable".to_string(),
            arguments: vec![],
            metadata: HashMap::new(),
        },
    ];

    assert_eq!(unit.attributes.len(), 2);
    assert_eq!(unit.attributes[0].name, "Controller");
    assert_eq!(unit.attributes[0].arguments[0], "/api/users");
}

#[test]
fn test_python_async_function() {
    let mut unit = CodeUnit::new(
        CodeUnitType::AsyncFunction,
        "fetch_data".to_string(),
        "api.fetch_data".to_string(),
        "api.py".to_string(),
        Language::Python,
    );

    unit.signature = "async def fetch_data(url: str) -> dict:".to_string();
    unit.is_async = true;
    unit.parameters = vec![Parameter {
        name: "url".to_string(),
        param_type: Some("str".to_string()),
        default_value: None,
        is_optional: false,
        is_variadic: false,
        attributes: vec![],
    }];
    unit.return_type = Some("dict".to_string());

    assert!(unit.is_async);
    assert_eq!(unit.unit_type, CodeUnitType::AsyncFunction);
    assert!(unit.is_callable());
}

#[test]
fn test_code_unit_with_embedding() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "test".to_string(),
        "test".to_string(),
        "test.rs".to_string(),
        Language::Rust,
    );

    // Add embedding
    let embedding: Vec<f32> = (0..1536).map(|i| i as f32 * 0.001).collect();
    unit.embedding = Some(embedding.clone());
    unit.embedding_model = Some("text-embedding-3-small".to_string());

    assert!(unit.embedding.is_some());
    assert_eq!(unit.embedding.as_ref().unwrap().len(), 1536);
    assert_eq!(
        unit.embedding_model,
        Some("text-embedding-3-small".to_string())
    );
}

#[test]
fn test_code_unit_with_language_specific_metadata() {
    let mut unit = CodeUnit::new(
        CodeUnitType::Function,
        "unsafe_fn".to_string(),
        "module::unsafe_fn".to_string(),
        "src/lib.rs".to_string(),
        Language::Rust,
    );

    // Add Rust-specific metadata
    unit.is_unsafe = true;
    unit.language_specific
        .insert("lifetimes".to_string(), serde_json::json!(["'a", "'b"]));
    unit.language_specific.insert(
        "trait_bounds".to_string(),
        serde_json::json!(["Clone", "Send", "Sync"]),
    );

    assert!(unit.is_unsafe);
    assert!(unit.language_specific.contains_key("lifetimes"));
    assert!(unit.language_specific.contains_key("trait_bounds"));
}
