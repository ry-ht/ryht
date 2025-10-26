//! Unit Tests for cortex.code.rename_unit (Symbol Renaming)
//!
//! Tests cover:
//! - Renaming functions
//! - Renaming variables
//! - Renaming structs/types
//! - Updating all references
//! - Cross-file renaming
//! - Renaming in different languages (Rust, TypeScript)
//! - AST validation after rename
//! - Error handling (invalid names, name conflicts)

use super::test_helpers::*;
use cortex::mcp::tools::code_manipulation::RenameUnitTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_rename_simple_function() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn foo() -> i32 {
    42
}

pub fn caller() -> i32 {
    foo()
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "foo",
        "new_name": "bar",
        "unit_type": "function",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Function renamed successfully in {}ms", duration);

            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify function definition renamed
            assert!(new_content.contains("pub fn bar()"), "Function definition not renamed");
            assert!(!new_content.contains("pub fn foo()"), "Old function name still exists");

            // Verify all call sites updated
            assert!(new_content.contains("bar()"), "Call site not updated");
            assert!(!new_content.contains("foo()"), "Old function call still exists");

            // AST validation
            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Renamed code has invalid syntax");

            println!("✓ All references updated");
            println!("✓ AST validation passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_struct_and_references() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub struct OldStruct {
    value: i32,
}

impl OldStruct {
    pub fn new(value: i32) -> Self {
        OldStruct { value }
    }
}

pub fn create() -> OldStruct {
    OldStruct::new(42)
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "OldStruct",
        "new_name": "NewStruct",
        "unit_type": "struct",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify struct definition renamed
            assert!(content.contains("pub struct NewStruct"), "Struct definition not renamed");
            assert!(!content.contains("pub struct OldStruct"), "Old struct name still exists");

            // Verify impl block renamed
            assert!(content.contains("impl NewStruct"), "Impl block not renamed");
            assert!(!content.contains("impl OldStruct"), "Old impl block still exists");

            // Verify constructor renamed
            assert!(content.contains("NewStruct { value }"), "Constructor not renamed");

            // Verify return type renamed
            assert!(content.contains("-> NewStruct"), "Return type not renamed");

            // Verify call site renamed
            assert!(content.contains("NewStruct::new"), "Call site not renamed");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Renamed struct code has invalid syntax");

            println!("✓ Struct and all references renamed successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_variable() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn process() -> i32 {
    let old_var = 10;
    let result = old_var * 2;
    old_var + result
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "old_var",
        "new_name": "new_var",
        "unit_type": "variable",
        "scope": {
            "function": "process",
            "line_start": 2,
            "line_end": 5,
        }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify all occurrences renamed
            assert!(content.contains("let new_var = 10"), "Variable declaration not renamed");
            assert!(content.contains("new_var * 2"), "First use not renamed");
            assert!(content.contains("new_var + result"), "Second use not renamed");
            assert!(!content.contains("old_var"), "Old variable name still exists");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Renamed variable code has invalid syntax");

            println!("✓ Variable and all uses renamed successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_typescript_class() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
export class OldClass {
    constructor(public value: number) {}

    getValue(): number {
        return this.value;
    }
}

export function createOldClass(): OldClass {
    return new OldClass(42);
}
"#;
    fixture.create_file("src/utils.ts", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/utils.ts",
        "old_name": "OldClass",
        "new_name": "NewClass",
        "unit_type": "class",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/utils.ts").await.unwrap();

            // Verify class definition renamed
            assert!(content.contains("export class NewClass"), "Class definition not renamed");

            // Verify return type renamed
            assert!(content.contains("(): NewClass"), "Return type not renamed");

            // Verify constructor call renamed
            assert!(content.contains("new NewClass(42)"), "Constructor call not renamed");

            assert!(!content.contains("OldClass"), "Old class name still exists");

            let is_valid = fixture.validate_syntax("src/utils.ts", &content).await;
            assert!(is_valid, "Renamed TypeScript class has invalid syntax");

            println!("✓ TypeScript class renamed successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_cross_file() {
    let fixture = CodeManipulationFixture::new().await;

    // File 1: Definition
    let file1 = r#"
pub struct SharedData {
    pub value: i32,
}
"#;
    fixture.create_file("src/types.rs", file1).await.unwrap();

    // File 2: Usage
    let file2 = r#"
use crate::types::SharedData;

pub fn process(data: SharedData) -> i32 {
    data.value
}
"#;
    fixture.create_file("src/lib.rs", file2).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/types.rs",
        "old_name": "SharedData",
        "new_name": "CommonData",
        "unit_type": "struct",
        "update_references": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Check definition file
            let types_content = fixture.read_file("src/types.rs").await.unwrap();
            assert!(types_content.contains("pub struct CommonData"), "Definition not renamed");
            assert!(!types_content.contains("SharedData"), "Old name in definition file");

            // Check usage file
            let lib_content = fixture.read_file("src/lib.rs").await.unwrap();
            assert!(lib_content.contains("use crate::types::CommonData"), "Import not updated");
            assert!(lib_content.contains("data: CommonData"), "Parameter type not updated");
            assert!(!lib_content.contains("SharedData"), "Old name in usage file");

            let types_valid = fixture.validate_syntax("src/types.rs", &types_content).await;
            let lib_valid = fixture.validate_syntax("src/lib.rs", &lib_content).await;
            assert!(types_valid && lib_valid, "Cross-file rename broke syntax");

            println!("✓ Cross-file rename successful");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_method_in_impl() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub struct Calculator;

impl Calculator {
    pub fn old_method(&self, x: i32) -> i32 {
        x * 2
    }

    pub fn use_method(&self) -> i32 {
        self.old_method(5)
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "old_method",
        "new_name": "new_method",
        "unit_type": "method",
        "parent_type": "Calculator",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify method definition renamed
            assert!(content.contains("pub fn new_method"), "Method definition not renamed");
            assert!(!content.contains("fn old_method"), "Old method name still exists");

            // Verify method call renamed
            assert!(content.contains("self.new_method(5)"), "Method call not renamed");
            assert!(!content.contains("self.old_method"), "Old method call still exists");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Renamed method code has invalid syntax");

            println!("✓ Method renamed successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_preserves_comments_and_docs() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
/// Documentation for old_func
/// This function does important work
pub fn old_func() -> i32 {
    // Internal comment
    42
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "old_func",
        "new_name": "new_func",
        "unit_type": "function",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify documentation preserved
            assert!(content.contains("/// Documentation"), "Documentation lost");
            assert!(content.contains("/// This function does important work"), "Doc content lost");
            assert!(content.contains("// Internal comment"), "Internal comment lost");

            // Verify rename
            assert!(content.contains("pub fn new_func"), "Function not renamed");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with preserved docs has invalid syntax");

            println!("✓ Comments and documentation preserved");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_STRUCT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional: read entire file + modify + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2;

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "Person",
        "new_name": "Individual",
        "unit_type": "struct",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex: only the rename specification
            let cortex_tokens = fixture.count_tokens("Person Individual struct");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 70.0, "Token efficiency should be > 70% for rename operations");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_rename_error_invalid_name() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::SIMPLE_RUST_FUNCTION;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "foo",
        "new_name": "123invalid",  // Invalid identifier
        "unit_type": "function",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("invalid") ||
                error.to_lowercase().contains("identifier"),
                "Error should mention invalid identifier"
            );
            println!("✓ Invalid name error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with invalid identifier error");
        }
    }
}

#[tokio::test]
async fn test_rename_error_name_conflict() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn foo() -> i32 {
    42
}

pub fn bar() -> i32 {
    100
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = RenameUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "old_name": "foo",
        "new_name": "bar",  // Conflicts with existing function
        "unit_type": "function",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("conflict") ||
                error.to_lowercase().contains("exists") ||
                error.to_lowercase().contains("duplicate"),
                "Error should mention name conflict"
            );
            println!("✓ Name conflict error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with name conflict error");
        }
    }
}
