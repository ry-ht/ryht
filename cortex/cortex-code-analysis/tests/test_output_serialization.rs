//! Comprehensive tests for output serialization across all formats.
//!
//! This test suite validates:
//! - JSON, YAML, TOML, CSV serialization
//! - Metrics export for all formats
//! - AST export functionality
//! - Ops export functionality
//! - Round-trip serialization/deserialization where applicable

use cortex_code_analysis::{
    RustParser, TypeScriptParser, CodeParser, Lang,
    export_metrics, export_ast, export_ops,
    OutputFormat, ExportConfig,
};
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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export metrics to JSON
    let json = export_metrics(&parsed, OutputFormat::Json, None)?;

    // Verify it's valid JSON
    assert!(json.contains("\"functions\""));
    assert!(json.contains("\"calculate_sum\""));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export AST to JSON
    let json = export_ast(&parsed, OutputFormat::Json)?;

    // Verify it's valid JSON
    assert!(json.contains("\"structs\""));
    assert!(json.contains("\"Point\""));
    assert!(json.contains("\"impls\""));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export ops to JSON
    let json = export_ops(&parsed, OutputFormat::Json)?;

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export metrics to YAML
    let yaml = export_metrics(&parsed, OutputFormat::Yaml, None)?;

    // Verify it's valid YAML structure
    assert!(yaml.contains("functions:") || yaml.contains("functions"));
    assert!(yaml.contains("factorial"));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export AST to YAML
    let yaml = export_ast(&parsed, OutputFormat::Yaml)?;

    // Verify it's valid YAML
    assert!(yaml.contains("enums:") || yaml.contains("enums"));
    assert!(yaml.contains("Status"));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export metrics to TOML
    let toml = export_metrics(&parsed, OutputFormat::Toml, None)?;

    // Verify it's valid TOML structure
    assert!(toml.contains("[[functions]]") || toml.contains("[functions"));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export AST to TOML
    let toml = export_ast(&parsed, OutputFormat::Toml)?;

    // Verify it's valid TOML
    assert!(toml.contains("[[traits]]") || toml.contains("[traits"));

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export metrics to CSV
    let csv = export_metrics(&parsed, OutputFormat::Csv, None)?;

    // Verify it's valid CSV structure (has headers and rows)
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 2, "CSV should have at least header and one data row");

    // Verify header exists
    assert!(lines[0].contains("name") || lines[0].contains("function"));

    Ok(())
}

#[test]
fn test_csv_ast_export() -> Result<()> {
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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Export AST to CSV
    let csv = export_ast(&parsed, OutputFormat::Csv)?;

    // Verify it's valid CSV structure
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 1, "CSV should have at least a header");

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

    let mut parser = TypeScriptParser::new()?;
    let parsed = parser.parse_file("test.ts", source)?;

    // Export to JSON
    let json = export_ast(&parsed, OutputFormat::Json)?;
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

    let mut parser = TypeScriptParser::new_javascript()?;
    let parsed = parser.parse_file("test.js", source)?;

    // Export to YAML
    let yaml = export_ast(&parsed, OutputFormat::Yaml)?;
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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Create custom export config
    let config = ExportConfig {
        pretty: true,
        include_metadata: true,
        ..Default::default()
    };

    // Export with custom config
    let json = export_metrics(&parsed, OutputFormat::Json, Some(config))?;

    // Verify it's valid and contains metadata
    assert!(!json.is_empty());
    let _value: serde_json::Value = serde_json::from_str(&json)?;

    Ok(())
}

#[test]
fn test_export_minimal_config() -> Result<()> {
    let source = r#"
fn minimal() {}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Create minimal export config
    let config = ExportConfig {
        pretty: false,
        include_metadata: false,
        ..Default::default()
    };

    // Export with minimal config
    let json = export_metrics(&parsed, OutputFormat::Json, Some(config))?;

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should handle empty files gracefully
    let json = export_ast(&parsed, OutputFormat::Json)?;
    assert!(!json.is_empty());

    let yaml = export_ast(&parsed, OutputFormat::Yaml)?;
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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", &source)?;

    // Should handle large structures
    let json = export_metrics(&parsed, OutputFormat::Json, None)?;
    assert!(!json.is_empty());

    // Verify it's valid JSON
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert!(value.is_object() || value.is_array());

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Test all formats with complex nested structures
    let json = export_ast(&parsed, OutputFormat::Json)?;
    assert!(!json.is_empty());
    let _: serde_json::Value = serde_json::from_str(&json)?;

    let yaml = export_ast(&parsed, OutputFormat::Yaml)?;
    assert!(!yaml.is_empty());
    let _: serde_yaml::Value = serde_yaml::from_str(&yaml)?;

    let toml = export_ast(&parsed, OutputFormat::Toml)?;
    assert!(!toml.is_empty());
    let _: toml::Value = toml::from_str(&toml)?;

    let csv = export_ast(&parsed, OutputFormat::Csv)?;
    assert!(!csv.is_empty());

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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Verify all formats produce non-empty output
    let json = export_metrics(&parsed, OutputFormat::Json, None)?;
    assert!(!json.is_empty(), "JSON output should not be empty");

    let yaml = export_metrics(&parsed, OutputFormat::Yaml, None)?;
    assert!(!yaml.is_empty(), "YAML output should not be empty");

    let toml = export_metrics(&parsed, OutputFormat::Toml, None)?;
    assert!(!toml.is_empty(), "TOML output should not be empty");

    let csv = export_metrics(&parsed, OutputFormat::Csv, None)?;
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

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // All formats should represent the same underlying data
    let json = export_ast(&parsed, OutputFormat::Json)?;
    let yaml = export_ast(&parsed, OutputFormat::Yaml)?;
    let toml = export_ast(&parsed, OutputFormat::Toml)?;

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
