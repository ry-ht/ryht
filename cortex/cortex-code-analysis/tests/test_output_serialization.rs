//! Comprehensive tests for output serialization across all formats.
//!
//! This test suite validates:
//! - JSON, YAML, TOML, CSV serialization
//! - Metrics export for all formats
//! - AST export functionality
//! - Ops export functionality
//! - Round-trip serialization/deserialization where applicable

use cortex_code_analysis::{
    Parser, RustLanguage, TypeScriptLanguage, ParserTrait, Lang,
    export_metrics, export_ast, export_ops,
    OutputFormat, ExportConfig,
};
use cortex_code_analysis::spaces::compute_spaces;
use cortex_code_analysis::ops::extract_ops;
use anyhow::Result;
use std::path::Path;

// ============================================================================
// SECTION 1: JSON Serialization Tests
// ============================================================================

#[test]
fn test_json_metrics_export() -> Result<()> {
    let source = r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    if a > 0 && b > 0 {
        a + b
    } else if a < 0 || b < 0 {
        a - b
    } else {
        0
    }
}
"#;

    // Use Parser instead of RustParser for metrics
    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;
    let spaces = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Rust,
        "test.rs"
    )?;

    // Export metrics to JSON
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_metrics(&spaces, &config)?;

    // Verify it's valid JSON
    assert!(json.contains("\"metrics\""));

    // Parse it back to verify it's valid JSON
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

#[test]
fn test_json_ast_export() -> Result<()> {
    let source = r#"
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Export AST to JSON
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ast(&parser, &config)?;

    // Verify it's valid JSON
    assert!(json.contains("\"ast\""));

    // Parse it back
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

#[test]
fn test_json_ops_export() -> Result<()> {
    let source = r#"
fn math_operations() {
    let a = 10 + 20;
    let b = a * 2;
    let c = b - 5;
    let d = c / 3;
}
"#;

    let ops = extract_ops(source, Lang::Rust)?;

    // Export ops to JSON
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ops(&ops, &config)?;

    // Verify it's valid JSON
    assert!(json.contains("{") && json.contains("}"));

    // Parse it back
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

// ============================================================================
// SECTION 2: YAML Serialization Tests
// ============================================================================

#[test]
fn test_yaml_metrics_export() -> Result<()> {
    let source = r#"
fn factorial(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Export metrics to YAML
    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_metrics(&spaces, &config)?;

    // Verify it's valid YAML structure
    assert!(yaml.contains("metrics:") || yaml.contains("metrics"));

    // Parse it back to verify it's valid YAML
    let _value: serde_yaml::Value = serde_yaml::from_str(&yaml)?;

    Ok(())
}

#[test]
fn test_yaml_ast_export() -> Result<()> {
    let source = r#"
enum Status {
    Active,
    Inactive,
    Pending,
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Export AST to YAML
    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_ast(&parser, &config)?;

    // Verify it's valid YAML
    assert!(yaml.contains("ast:") || yaml.contains("ast"));

    // Parse it back
    let _value: serde_yaml::Value = serde_yaml::from_str(&yaml)?;

    Ok(())
}

// ============================================================================
// SECTION 3: TOML Serialization Tests
// ============================================================================

#[test]
fn test_toml_metrics_export() -> Result<()> {
    let source = r#"
fn simple_function() {
    let x = 42;
    println!("{}", x);
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Export metrics to TOML
    let config = ExportConfig {
        format: OutputFormat::Toml,
        ..Default::default()
    };
    let toml = export_metrics(&spaces, &config)?;

    // Verify it's valid TOML structure
    assert!(toml.contains("[metrics") || toml.contains("metrics"));

    // Parse it back to verify it's valid TOML
    let _value: toml::Value = toml::from_str(&toml)?;

    Ok(())
}

#[test]
fn test_toml_ast_export() -> Result<()> {
    let source = r#"
trait Drawable {
    fn draw(&self);
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Export AST to TOML
    let config = ExportConfig {
        format: OutputFormat::Toml,
        ..Default::default()
    };
    let toml = export_ast(&parser, &config)?;

    // Verify it's valid TOML
    assert!(toml.contains("[ast") || toml.contains("ast"));

    // Parse it back
    let _value: toml::Value = toml::from_str(&toml)?;

    Ok(())
}

// ============================================================================
// SECTION 4: CSV Serialization Tests
// ============================================================================

#[test]
fn test_csv_metrics_export() -> Result<()> {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Export metrics to CSV
    let config = ExportConfig {
        format: OutputFormat::Csv,
        ..Default::default()
    };
    let csv = export_metrics(&spaces, &config)?;

    // Verify it's valid CSV structure (has headers and rows)
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 2, "CSV should have at least header and one data row");

    // Verify header exists
    assert!(lines[0].contains("name") || lines[0].contains("kind"));

    Ok(())
}

#[test]
fn test_csv_ast_export() -> Result<()> {
    // Note: CSV format is not supported for AST export
    // AST is a complex nested structure that doesn't map well to CSV
    // This test verifies that we get an appropriate error
    let source = r#"
struct User {
    id: u64,
    name: String,
}

struct Post {
    id: u64,
    title: String,
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Export AST to CSV - should fail with error
    let config = ExportConfig {
        format: OutputFormat::Csv,
        ..Default::default()
    };
    let result = export_ast(&parser, &config);

    // CSV export should fail for AST
    assert!(result.is_err(), "CSV format should not be supported for AST export");

    Ok(())
}

// ============================================================================
// SECTION 5: Multi-Language Serialization Tests
// ============================================================================

#[test]
fn test_typescript_json_export() -> Result<()> {
    let source = r#"
interface User {
    id: number;
    name: string;
}

function getUser(id: number): User {
    return { id, name: "User" + id };
}
"#;

    let parser = Parser::<TypeScriptLanguage>::new(source.as_bytes().to_vec(), Path::new("test.ts"))?;

    // Export to JSON
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ast(&parser, &config)?;
    assert!(json.contains("{") && json.contains("}"));

    // Parse it back
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

#[test]
fn test_javascript_yaml_export() -> Result<()> {
    let source = r#"
function greet(name) {
    console.log("Hello, " + name);
}

class Counter {
    constructor() {
        this.count = 0;
    }

    increment() {
        this.count++;
    }
}
"#;

    let parser = Parser::<TypeScriptLanguage>::new(source.as_bytes().to_vec(), Path::new("test.js"))?;

    // Export to YAML
    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_ast(&parser, &config)?;
    assert!(!yaml.is_empty());

    // Parse it back
    let _value: serde_yaml::Value = serde_yaml::from_str(&yaml)?;

    Ok(())
}

// ============================================================================
// SECTION 6: Export Configuration Tests
// ============================================================================

#[test]
fn test_export_with_custom_config() -> Result<()> {
    let source = r#"
fn test() {
    let x = 1;
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Create custom export config
    let config = ExportConfig {
        format: OutputFormat::Json,
        pretty: true,
        include_metadata: true,
        ..Default::default()
    };

    // Export with custom config
    let json = export_metrics(&spaces, &config)?;

    // Verify it's valid and contains metadata
    assert!(!json.is_empty());
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert!(value.get("metadata").is_some());

    Ok(())
}

#[test]
fn test_export_minimal_config() -> Result<()> {
    let source = r#"
fn minimal() {}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Create minimal export config
    let config = ExportConfig {
        format: OutputFormat::Json,
        pretty: false,
        include_metadata: false,
        ..Default::default()
    };

    // Export with minimal config
    let json = export_metrics(&spaces, &config)?;

    // Should still be valid JSON but more compact
    assert!(!json.is_empty());
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

// ============================================================================
// SECTION 7: Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_file_serialization() -> Result<()> {
    let source = "";

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Should handle empty files gracefully
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ast(&parser, &config)?;
    assert!(!json.is_empty());

    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_ast(&parser, &config)?;
    assert!(!yaml.is_empty());

    Ok(())
}

#[test]
fn test_large_structure_serialization() -> Result<()> {
    // Create a large structure with many elements
    let mut source = String::from("// Large structure test\n");

    for i in 0..50 {
        source.push_str(&format!("\nfn function_{}() {{\n", i));
        source.push_str(&format!("    let x = {};\n", i));
        source.push_str("}\n");
    }

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Should handle large structures
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_metrics(&spaces, &config)?;
    assert!(!json.is_empty());

    // Verify it's valid JSON
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert!(value.is_object());

    Ok(())
}

#[test]
fn test_complex_nested_structures() -> Result<()> {
    let source = r#"
mod outer {
    pub mod inner {
        pub struct Point {
            x: f64,
            y: f64,
        }

        impl Point {
            pub fn new(x: f64, y: f64) -> Self {
                Point { x, y }
            }

            pub fn distance(&self, other: &Point) -> f64 {
                let dx = self.x - other.x;
                let dy = self.y - other.y;
                (dx * dx + dy * dy).sqrt()
            }
        }

        pub trait Drawable {
            fn draw(&self);
        }

        impl Drawable for Point {
            fn draw(&self) {
                println!("Point({}, {})", self.x, self.y);
            }
        }
    }
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // Test all formats with complex nested structures
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ast(&parser, &config)?;
    assert!(!json.is_empty());
    let _: serde_json::Value = serde_json::from_str(&json)?;

    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_ast(&parser, &config)?;
    assert!(!yaml.is_empty());
    let _: serde_yaml::Value = serde_yaml::from_str(&yaml)?;

    let config = ExportConfig {
        format: OutputFormat::Toml,
        ..Default::default()
    };
    let toml = export_ast(&parser, &config)?;
    assert!(!toml.is_empty());
    let _: toml::Value = toml::from_str(&toml)?;

    // Note: CSV format is not supported for AST, so we skip it
    // CSV works for metrics but not for complex nested structures like AST

    Ok(())
}

// ============================================================================
// SECTION 8: Format Comparison Tests
// ============================================================================

#[test]
fn test_all_formats_produce_output() -> Result<()> {
    let source = r#"
fn test_function(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
    let spaces = compute_spaces(parser.get_root(), parser.get_code(), Lang::Rust, "test.rs")?;

    // Verify all formats produce non-empty output
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_metrics(&spaces, &config)?;
    assert!(!json.is_empty(), "JSON output should not be empty");

    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_metrics(&spaces, &config)?;
    assert!(!yaml.is_empty(), "YAML output should not be empty");

    let config = ExportConfig {
        format: OutputFormat::Toml,
        ..Default::default()
    };
    let toml = export_metrics(&spaces, &config)?;
    assert!(!toml.is_empty(), "TOML output should not be empty");

    let config = ExportConfig {
        format: OutputFormat::Csv,
        ..Default::default()
    };
    let csv = export_metrics(&spaces, &config)?;
    assert!(!csv.is_empty(), "CSV output should not be empty");

    Ok(())
}

#[test]
fn test_format_consistency() -> Result<()> {
    let source = r#"
struct Data {
    value: i32,
}

fn process() {}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;

    // All formats should represent the same underlying data
    let config = ExportConfig {
        format: OutputFormat::Json,
        ..Default::default()
    };
    let json = export_ast(&parser, &config)?;

    let config = ExportConfig {
        format: OutputFormat::Yaml,
        ..Default::default()
    };
    let yaml = export_ast(&parser, &config)?;

    let config = ExportConfig {
        format: OutputFormat::Toml,
        ..Default::default()
    };
    let toml = export_ast(&parser, &config)?;

    // Basic consistency check - all should mention "Data" struct
    assert!(json.contains("Data"));
    assert!(yaml.contains("Data"));
    assert!(toml.contains("Data"));

    // All should mention "process" function
    assert!(json.contains("process"));
    assert!(yaml.contains("process"));
    assert!(toml.contains("process"));

    Ok(())
}
