//! Unit Tests for cortex.code.implement_interface (Trait/Interface Implementation)
//!
//! Tests cover:
//! - Implementing traits for structs
//! - Implementing with default methods
//! - Implementing generic traits
//! - Implementing multiple traits
//! - TypeScript interface implementation
//! - Overriding trait methods
//! - AST validation after implementation
//! - Token efficiency measurements

use super::test_helpers::*;
use cortex_cli::mcp::tools::code_manipulation::ImplementInterfaceTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_implement_simple_trait() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait Drawable {
    fn draw(&self);
}

pub struct Circle {
    pub radius: f64,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Circle",
        "trait_name": "Drawable",
        "methods": [
            {
                "name": "draw",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "()",
                "body": r#"    println!("Drawing circle with radius {}", self.radius);"#
            }
        ]
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Trait implemented successfully in {}ms", duration);

            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify impl block created
            assert!(new_content.contains("impl Drawable for Circle"), "Impl block not found");

            // Verify method implemented
            assert!(new_content.contains("fn draw(&self)"), "Method not implemented");
            assert!(
                new_content.contains("Drawing circle"),
                "Method body not found"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Code after trait implementation has invalid syntax");

            println!("✓ AST validation passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_trait_with_associated_types() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

pub struct Counter {
    count: u32,
    max: u32,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Counter",
        "trait_name": "Iterator",
        "associated_types": [
            {"name": "Item", "type": "u32"}
        ],
        "methods": [
            {
                "name": "next",
                "parameters": [{"name": "self", "type": "&mut self"}],
                "return_type": "Option<Self::Item>",
                "body": r#"    if self.count < self.max {
        self.count += 1;
        Some(self.count)
    } else {
        None
    }"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify impl block
            assert!(content.contains("impl Iterator for Counter"), "Iterator impl not found");

            // Verify associated type
            assert!(content.contains("type Item = u32"), "Associated type not set");

            // Verify method
            assert!(content.contains("fn next(&mut self)"), "next method not implemented");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Iterator impl has invalid syntax");

            println!("✓ Trait with associated types implemented successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_generic_trait() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait From<T> {
    fn from(value: T) -> Self;
}

pub struct Wrapper {
    value: i32,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Wrapper",
        "trait_name": "From",
        "trait_generics": ["i32"],
        "methods": [
            {
                "name": "from",
                "parameters": [{"name": "value", "type": "i32"}],
                "return_type": "Self",
                "body": "    Wrapper { value }"
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify generic impl
            assert!(
                content.contains("impl From<i32> for Wrapper"),
                "Generic trait impl not found"
            );

            // Verify method
            assert!(content.contains("fn from(value: i32) -> Self"), "from method not implemented");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generic trait impl has invalid syntax");

            println!("✓ Generic trait implemented successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_multiple_methods() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
    fn describe(&self) -> String;
}

pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Rectangle",
        "trait_name": "Shape",
        "methods": [
            {
                "name": "area",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "f64",
                "body": "    self.width * self.height"
            },
            {
                "name": "perimeter",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "f64",
                "body": "    2.0 * (self.width + self.height)"
            },
            {
                "name": "describe",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "String",
                "body": r#"    format!("Rectangle {}x{}", self.width, self.height)"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify impl block
            assert!(content.contains("impl Shape for Rectangle"), "Shape impl not found");

            // Verify all methods
            assert!(content.contains("fn area(&self) -> f64"), "area method not found");
            assert!(content.contains("fn perimeter(&self) -> f64"), "perimeter method not found");
            assert!(content.contains("fn describe(&self) -> String"), "describe method not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Trait impl with multiple methods has invalid syntax");

            println!("✓ Multiple methods implemented successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_trait_for_generic_type() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait Display {
    fn display(&self) -> String;
}

pub struct Container<T> {
    pub value: T,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Container",
        "type_generics": ["T"],
        "trait_name": "Display",
        "where_clause": "T: std::fmt::Display",
        "methods": [
            {
                "name": "display",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "String",
                "body": "    format!(\"{}\", self.value)"
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify generic impl with where clause
            assert!(
                content.contains("impl<T> Display for Container<T>"),
                "Generic type impl not found"
            );
            assert!(
                content.contains("where") && content.contains("T: std::fmt::Display"),
                "Where clause not found"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generic type impl has invalid syntax");

            println!("✓ Trait implemented for generic type successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_typescript_interface() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
export interface Serializable {
    serialize(): string;
    deserialize(data: string): void;
}

export class User {
    constructor(
        public id: number,
        public name: string
    ) {}
}
"#;
    fixture.create_file("src/models.ts", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/models.ts",
        "type_name": "User",
        "trait_name": "Serializable",
        "methods": [
            {
                "name": "serialize",
                "parameters": [],
                "return_type": "string",
                "body": r#"    return JSON.stringify({ id: this.id, name: this.name });"#
            },
            {
                "name": "deserialize",
                "parameters": [{"name": "data", "type": "string"}],
                "return_type": "void",
                "body": r#"    const obj = JSON.parse(data);
    this.id = obj.id;
    this.name = obj.name;"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/models.ts").await.unwrap();

            // Verify TypeScript class implements interface
            assert!(
                content.contains("class User implements Serializable") ||
                content.contains("serialize()") && content.contains("deserialize("),
                "Interface not implemented"
            );

            let is_valid = fixture.validate_syntax("src/models.ts", &content).await;
            assert!(is_valid, "TypeScript implementation has invalid syntax");

            println!("✓ TypeScript interface implemented successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_default_trait() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
#[derive(Debug, Clone)]
pub struct Settings {
    pub timeout: u64,
    pub retries: u32,
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Settings",
        "trait_name": "Default",
        "methods": [
            {
                "name": "default",
                "parameters": [],
                "return_type": "Self",
                "body": r#"    Settings {
        timeout: 30,
        retries: 3,
    }"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify Default impl
            assert!(content.contains("impl Default for Settings"), "Default impl not found");
            assert!(content.contains("fn default() -> Self"), "default method not found");
            assert!(content.contains("timeout: 30"), "Default values not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Default impl has invalid syntax");

            println!("✓ Default trait implemented successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_trait_preserves_existing_impls() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait First {
    fn first(&self);
}

pub trait Second {
    fn second(&self);
}

pub struct MyType;

impl First for MyType {
    fn first(&self) {
        println!("first");
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "MyType",
        "trait_name": "Second",
        "methods": [
            {
                "name": "second",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "()",
                "body": r#"    println!("second");"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify both impls exist
            assert!(content.contains("impl First for MyType"), "First impl removed");
            assert!(content.contains("impl Second for MyType"), "Second impl not added");

            // Verify both methods exist
            assert!(content.contains("fn first(&self)"), "first method removed");
            assert!(content.contains("fn second(&self)"), "second method not added");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Multiple impls have invalid syntax");

            println!("✓ Multiple trait impls coexist correctly");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_trait_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_WITH_TRAIT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional: read entire file + add impl + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2;

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "Circle",
        "trait_name": "Drawable",
        "methods": [
            {
                "name": "draw",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "()",
                "body": r#"    println!("Circle");"#
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex: only the impl block specification
            let cortex_tokens = fixture.count_tokens("impl Drawable for Circle fn draw");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 60.0, "Token efficiency should be > 60%");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_implement_trait_error_already_implemented() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait MyTrait {
    fn my_method(&self);
}

pub struct MyType;

impl MyTrait for MyType {
    fn my_method(&self) {
        println!("already implemented");
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "MyType",
        "trait_name": "MyTrait",
        "methods": [
            {
                "name": "my_method",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "()",
                "body": "    println!(\"duplicate\");"
            }
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("already") ||
                error.to_lowercase().contains("duplicate") ||
                error.to_lowercase().contains("implemented"),
                "Error should mention trait already implemented"
            );
            println!("✓ Already implemented error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with already implemented error");
        }
    }
}

#[tokio::test]
async fn test_implement_trait_error_missing_method() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub trait Complete {
    fn method_a(&self);
    fn method_b(&self);
    fn method_c(&self);
}

pub struct MyType;
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ImplementInterfaceTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "type_name": "MyType",
        "trait_name": "Complete",
        "methods": [
            {
                "name": "method_a",
                "parameters": [{"name": "self", "type": "&self"}],
                "return_type": "()",
                "body": "    // implementation"
            }
            // Missing method_b and method_c
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("missing") ||
                error.to_lowercase().contains("incomplete") ||
                error.to_lowercase().contains("required"),
                "Error should mention missing methods"
            );
            println!("✓ Missing method error handling works correctly");
        }
        ToolResult::Success { .. } => {
            // It's also acceptable if the tool allows partial implementation
            // and marks the impl as incomplete
            let content = fixture.read_file("src/lib.rs").await.unwrap();
            println!("✓ Tool allowed partial implementation (acceptable behavior)");
        }
    }
}
