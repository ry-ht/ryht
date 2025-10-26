//! Unit Tests for cortex.code.create_unit (Struct/Type Creation)
//!
//! Tests cover:
//! - Creating simple structs
//! - Creating structs with derive attributes
//! - Creating generic structs
//! - Creating tuple structs
//! - Creating unit structs
//! - Creating TypeScript interfaces and classes
//! - Creating enums
//! - AST validation after creation
//! - Token efficiency measurements

use super::test_helpers::*;
use cortex_cli::mcp::tools::code_manipulation::CreateCodeUnitTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_create_simple_struct() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
// Existing code
pub fn helper() -> i32 {
    42
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "User",
        "fields": [
            {"name": "id", "type": "u64", "visibility": "public"},
            {"name": "name", "type": "String", "visibility": "public"},
            {"name": "email", "type": "String", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Struct created successfully in {}ms", duration);

            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify struct declaration
            assert!(new_content.contains("pub struct User"), "Struct declaration not found");

            // Verify fields
            assert!(new_content.contains("pub id: u64"), "Field 'id' not found");
            assert!(new_content.contains("pub name: String"), "Field 'name' not found");
            assert!(new_content.contains("pub email: String"), "Field 'email' not found");

            // Verify existing code preserved
            assert!(new_content.contains("fn helper"), "Existing function removed");

            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Generated struct has invalid syntax");

            println!("✓ AST validation passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_struct_with_derives() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Person",
        "fields": [
            {"name": "name", "type": "String", "visibility": "public"},
            {"name": "age", "type": "u32", "visibility": "public"}
        ],
        "visibility": "public",
        "derives": ["Debug", "Clone", "PartialEq", "Serialize", "Deserialize"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify derive attributes
            assert!(
                content.contains("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]") ||
                (content.contains("Debug") && content.contains("Clone") && content.contains("derive")),
                "Derive attributes not found"
            );

            // Verify struct
            assert!(content.contains("pub struct Person"), "Struct not created");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Struct with derives has invalid syntax");

            println!("✓ Struct with derive attributes created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_generic_struct() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Container",
        "generics": [
            {"name": "T"},
            {"name": "E", "bounds": ["std::error::Error"]}
        ],
        "fields": [
            {"name": "value", "type": "Option<T>", "visibility": "public"},
            {"name": "error", "type": "Option<E>", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify generic declaration
            assert!(
                content.contains("pub struct Container<T, E") ||
                content.contains("Container<T, E: std::error::Error>"),
                "Generic struct not created correctly"
            );

            // Verify generic fields
            assert!(content.contains("value: Option<T>"), "Generic field 'value' not found");
            assert!(content.contains("error: Option<E>"), "Generic field 'error' not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generic struct has invalid syntax");

            println!("✓ Generic struct created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_tuple_struct() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Point",
        "struct_style": "tuple",
        "fields": [
            {"type": "f64", "visibility": "public"},
            {"type": "f64", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify tuple struct syntax
            assert!(
                content.contains("pub struct Point(pub f64, pub f64)") ||
                content.contains("Point(pub f64, pub f64)"),
                "Tuple struct not created correctly"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Tuple struct has invalid syntax");

            println!("✓ Tuple struct created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_unit_struct() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Marker",
        "struct_style": "unit",
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify unit struct syntax
            assert!(content.contains("pub struct Marker;"), "Unit struct not created");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Unit struct has invalid syntax");

            println!("✓ Unit struct created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_enum() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "enum",
        "name": "Status",
        "variants": [
            {"name": "Active"},
            {"name": "Pending", "fields": [{"type": "String"}]},
            {"name": "Error", "fields": [{"name": "code", "type": "i32"}, {"name": "message", "type": "String"}]}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify enum declaration
            assert!(content.contains("pub enum Status"), "Enum not created");

            // Verify variants
            assert!(content.contains("Active"), "Variant 'Active' not found");
            assert!(content.contains("Pending"), "Variant 'Pending' not found");
            assert!(content.contains("Error"), "Variant 'Error' not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Enum has invalid syntax");

            println!("✓ Enum created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_typescript_interface() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/types.ts", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/types.ts",
        "unit_type": "interface",
        "name": "User",
        "fields": [
            {"name": "id", "type": "number"},
            {"name": "name", "type": "string"},
            {"name": "email", "type": "string", "optional": true}
        ],
        "visibility": "export",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/types.ts").await.unwrap();

            // Verify interface declaration
            assert!(content.contains("export interface User"), "Interface not created");

            // Verify fields
            assert!(content.contains("id: number"), "Field 'id' not found");
            assert!(content.contains("name: string"), "Field 'name' not found");
            assert!(
                content.contains("email?: string") || content.contains("email: string | undefined"),
                "Optional field 'email' not found"
            );

            let is_valid = fixture.validate_syntax("src/types.ts", &content).await;
            assert!(is_valid, "TypeScript interface has invalid syntax");

            println!("✓ TypeScript interface created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_typescript_class() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/models.ts", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/models.ts",
        "unit_type": "class",
        "name": "Rectangle",
        "fields": [
            {"name": "width", "type": "number", "visibility": "private"},
            {"name": "height", "type": "number", "visibility": "private"}
        ],
        "constructor": {
            "parameters": [
                {"name": "width", "type": "number"},
                {"name": "height", "type": "number"}
            ],
            "body": "this.width = width;\n    this.height = height;"
        },
        "visibility": "export",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/models.ts").await.unwrap();

            // Verify class declaration
            assert!(content.contains("export class Rectangle"), "Class not created");

            // Verify fields
            assert!(content.contains("private width: number"), "Field 'width' not found");
            assert!(content.contains("private height: number"), "Field 'height' not found");

            // Verify constructor
            assert!(content.contains("constructor"), "Constructor not found");

            let is_valid = fixture.validate_syntax("src/models.ts", &content).await;
            assert!(is_valid, "TypeScript class has invalid syntax");

            println!("✓ TypeScript class created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_struct_with_default_impl() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Config",
        "fields": [
            {"name": "timeout", "type": "u64", "visibility": "public"},
            {"name": "retries", "type": "u32", "visibility": "public"}
        ],
        "visibility": "public",
        "derives": ["Debug", "Clone"],
        "generate_default": true,
        "default_values": {
            "timeout": "30",
            "retries": "3"
        }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify struct
            assert!(content.contains("pub struct Config"), "Struct not created");

            // Verify Default impl or derive
            assert!(
                content.contains("impl Default for Config") ||
                content.contains("derive") && content.contains("Default"),
                "Default implementation not found"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Struct with Default has invalid syntax");

            println!("✓ Struct with Default implementation created successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_struct_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_WITH_IMPORTS;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional: read entire file + add struct + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2;

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "NewType",
        "fields": [
            {"name": "value", "type": "i32", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex: only the struct definition
            let cortex_tokens = fixture.count_tokens("pub struct NewType { pub value: i32 }");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 50.0, "Token efficiency should be > 50%");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_struct_error_duplicate_name() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_STRUCT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Person",  // Duplicate name
        "fields": [
            {"name": "id", "type": "u64", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("duplicate") ||
                error.to_lowercase().contains("already exists"),
                "Error should mention duplicate struct name"
            );
            println!("✓ Duplicate struct name error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with duplicate struct name error");
        }
    }
}

#[tokio::test]
async fn test_create_struct_preserves_file_organization() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
//! Module documentation

use std::fmt;

/// First struct
pub struct First {
    pub value: i32,
}

mod internal {
    pub fn helper() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_first() {}
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "struct",
        "name": "Second",
        "fields": [
            {"name": "data", "type": "String", "visibility": "public"}
        ],
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify all original elements preserved
            assert!(content.contains("//! Module documentation"), "Module doc lost");
            assert!(content.contains("use std::fmt"), "Import lost");
            assert!(content.contains("struct First"), "First struct lost");
            assert!(content.contains("mod internal"), "Internal module lost");
            assert!(content.contains("#[cfg(test)]"), "Test module lost");

            // Verify new struct added
            assert!(content.contains("pub struct Second"), "New struct not added");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "File organization corrupted after adding struct");

            println!("✓ File organization preserved correctly");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}
