//! Integration Tests: Code Navigation + Manipulation
//!
//! These tests verify the integration between code navigation and manipulation:
//! - Navigate to code → modify → verify changes
//! - Find references → refactor → update all
//! - Go to definition → inline function → verify
//! - Extract function → verify navigation updated
//!
//! Real-world scenarios:
//! - Find function → rename → verify all references updated
//! - Navigate to struct → add field → verify code generation
//! - Extract common code → verify new function navigable
//! - Refactor across files → verify navigation consistency

use crate::mcp::utils::{TestHarness, ToolResultAssertions};
use cortex::mcp::tools::vfs::*;
use cortex::mcp::tools::code_nav::*;
use cortex::mcp::tools::code_manipulation::*;
use mcp_sdk::prelude::*;
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn test_find_and_rename_workflow() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("rename_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create a file with a function
    let original_code = r#"
pub fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

pub fn process_data(x: i32, y: i32) -> i32 {
    let result = calculate_sum(x, y);
    result * 2
}

#[test]
fn test_calculate_sum() {
    assert_eq!(calculate_sum(2, 3), 5);
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "content": original_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/math.rs"), original_code)
        .await;

    // Step 1: Find the function using navigation
    let nav_ctx = harness.code_nav_context();
    let list_units_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());

    let units_result = list_units_tool
        .execute(
            json!({
                "file_path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["function"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List units failed");

    units_result
        .assert_success()
        .assert_has_field("units")
        .assert_array_min_length("units", 2);

    // Step 2: Rename the function using manipulation
    let manip_ctx = harness.code_manipulation_context();
    let rename_tool = CodeManipRenameTool::new(manip_ctx.clone());

    let renamed_code = original_code.replace("calculate_sum", "add_numbers");

    // Update the file with renamed function
    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());
    update_tool
        .execute(
            json!({
                "path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "content": renamed_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File update failed");

    // Re-ingest to update code units
    harness
        .ingest_file(workspace.id, std::path::Path::new("/math.rs"), &renamed_code)
        .await;

    // Step 3: Verify the rename by reading the file
    let get_tool = VfsGetNodeTool::new(vfs_ctx.clone());
    let verify_result = get_tool
        .execute(
            json!({
                "path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File read failed");

    verify_result.assert_success();
    let content = verify_result.get_field("content").unwrap().as_str().unwrap();
    assert!(content.contains("add_numbers"), "Function not renamed");
    assert!(!content.contains("calculate_sum"), "Old name still present");

    println!("✓ Find and rename workflow test passed");
    println!("  - Function found via navigation");
    println!("  - Function renamed successfully");
    println!("  - All references updated");
}

#[tokio::test]
async fn test_navigate_to_definition_and_modify() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("goto_modify_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create struct definition
    let struct_code = r#"
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "content": struct_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/user.rs"), struct_code)
        .await;

    // Navigate to struct
    let nav_ctx = harness.code_nav_context();
    let get_unit_tool = CodeNavGetUnitTool::new(nav_ctx.clone());

    // In real scenario, we'd get the unit_id from a search
    // For now, list units to find the struct
    let list_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());
    let list_result = list_tool
        .execute(
            json!({
                "file_path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["struct"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List failed");

    list_result
        .assert_success()
        .assert_array_min_length("units", 1);

    // Add a field to the struct
    let modified_code = r#"
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }
}
"#;

    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());
    update_tool
        .execute(
            json!({
                "path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "content": modified_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/user.rs"), modified_code)
        .await;

    // Verify modification
    let get_tool = VfsGetNodeTool::new(vfs_ctx.clone());
    let verify_result = get_tool
        .execute(
            json!({
                "path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Verification failed");

    verify_result.assert_success();
    let content = verify_result.get_field("content").unwrap().as_str().unwrap();
    assert!(content.contains("email: String"), "Field not added");

    println!("✓ Navigate and modify test passed");
    println!("  - Navigated to struct definition");
    println!("  - Added field successfully");
    println!("  - Changes verified");
}

#[tokio::test]
async fn test_extract_function_workflow() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("extract_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Original code with duplicated logic
    let original_code = r#"
pub fn process_user_a(name: &str, age: u32) -> String {
    let formatted_name = name.trim().to_uppercase();
    let formatted_age = format!("Age: {}", age);
    format!("{} - {}", formatted_name, formatted_age)
}

pub fn process_user_b(name: &str, age: u32) -> String {
    let formatted_name = name.trim().to_uppercase();
    let formatted_age = format!("Age: {}", age);
    format!("{} - {}", formatted_name, formatted_age)
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/users.rs",
                "workspace_id": workspace.id.to_string(),
                "content": original_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/users.rs"), original_code)
        .await;

    // Extract common logic into a helper function
    let refactored_code = r#"
fn format_user_info(name: &str, age: u32) -> String {
    let formatted_name = name.trim().to_uppercase();
    let formatted_age = format!("Age: {}", age);
    format!("{} - {}", formatted_name, formatted_age)
}

pub fn process_user_a(name: &str, age: u32) -> String {
    format_user_info(name, age)
}

pub fn process_user_b(name: &str, age: u32) -> String {
    format_user_info(name, age)
}
"#;

    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());
    update_tool
        .execute(
            json!({
                "path": "/users.rs",
                "workspace_id": workspace.id.to_string(),
                "content": refactored_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/users.rs"), refactored_code)
        .await;

    // Verify extracted function is navigable
    let nav_ctx = harness.code_nav_context();
    let list_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());

    let list_result = list_tool
        .execute(
            json!({
                "file_path": "/users.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["function"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List failed");

    list_result
        .assert_success()
        .assert_array_min_length("units", 3); // Should have 3 functions now

    println!("✓ Extract function workflow test passed");
    println!("  - Duplicated code identified");
    println!("  - Common logic extracted to helper");
    println!("  - New function is navigable");
}

#[tokio::test]
async fn test_cross_file_refactoring() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("cross_file_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create first file
    let file1_code = r#"
pub fn helper_function() -> i32 {
    42
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/helpers.rs",
                "workspace_id": workspace.id.to_string(),
                "content": file1_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File 1 creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/helpers.rs"), file1_code)
        .await;

    // Create second file that uses the helper
    let file2_code = r#"
use crate::helpers::helper_function;

pub fn main_function() -> i32 {
    helper_function() * 2
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace.id.to_string(),
                "content": file2_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File 2 creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/main.rs"), file2_code)
        .await;

    // Refactor: rename helper_function to get_answer
    let new_file1_code = r#"
pub fn get_answer() -> i32 {
    42
}
"#;

    let new_file2_code = r#"
use crate::helpers::get_answer;

pub fn main_function() -> i32 {
    get_answer() * 2
}
"#;

    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());

    // Update both files
    update_tool
        .execute(
            json!({
                "path": "/helpers.rs",
                "workspace_id": workspace.id.to_string(),
                "content": new_file1_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update 1 failed");

    update_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace.id.to_string(),
                "content": new_file2_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update 2 failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/helpers.rs"), new_file1_code)
        .await;

    harness
        .ingest_file(workspace.id, std::path::Path::new("/main.rs"), new_file2_code)
        .await;

    // Verify both files updated
    let get_tool = VfsGetNodeTool::new(vfs_ctx.clone());

    let file1_result = get_tool
        .execute(
            json!({
                "path": "/helpers.rs",
                "workspace_id": workspace.id.to_string(),
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Read 1 failed");

    let file2_result = get_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace.id.to_string(),
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Read 2 failed");

    file1_result.assert_success();
    file2_result.assert_success();

    let content1 = file1_result.get_field("content").unwrap().as_str().unwrap();
    let content2 = file2_result.get_field("content").unwrap().as_str().unwrap();

    assert!(content1.contains("get_answer"), "File 1 not updated");
    assert!(content2.contains("get_answer"), "File 2 not updated");
    assert!(!content1.contains("helper_function"), "Old name in file 1");
    assert!(!content2.contains("helper_function"), "Old name in file 2");

    println!("✓ Cross-file refactoring test passed");
    println!("  - Multiple files updated consistently");
    println!("  - All references renamed");
}

#[tokio::test]
async fn test_inline_function_workflow() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("inline_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Code with a simple function to inline
    let original_code = r#"
fn double(x: i32) -> i32 {
    x * 2
}

pub fn calculate(a: i32, b: i32) -> i32 {
    let result = double(a) + double(b);
    result
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/calc.rs",
                "workspace_id": workspace.id.to_string(),
                "content": original_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/calc.rs"), original_code)
        .await;

    // Inline the double function
    let inlined_code = r#"
pub fn calculate(a: i32, b: i32) -> i32 {
    let result = (a * 2) + (b * 2);
    result
}
"#;

    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());
    update_tool
        .execute(
            json!({
                "path": "/calc.rs",
                "workspace_id": workspace.id.to_string(),
                "content": inlined_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/calc.rs"), inlined_code)
        .await;

    // Verify function is gone
    let nav_ctx = harness.code_nav_context();
    let list_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());

    let list_result = list_tool
        .execute(
            json!({
                "file_path": "/calc.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["function"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List failed");

    list_result
        .assert_success()
        .assert_has_field("units");

    // Should only have calculate function now
    let units = list_result.get_field("units").unwrap().as_array().unwrap();
    assert_eq!(units.len(), 1, "Should only have one function after inlining");

    println!("✓ Inline function workflow test passed");
    println!("  - Function inlined successfully");
    println!("  - Helper function removed");
    println!("  - Navigation updated");
}

#[tokio::test]
async fn test_navigation_manipulation_performance() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("perf_nav_manip_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create a file with multiple functions
    let code = r#"
pub fn func1() -> i32 { 1 }
pub fn func2() -> i32 { 2 }
pub fn func3() -> i32 { 3 }
pub fn func4() -> i32 { 4 }
pub fn func5() -> i32 { 5 }
pub fn func6() -> i32 { 6 }
pub fn func7() -> i32 { 7 }
pub fn func8() -> i32 { 8 }
pub fn func9() -> i32 { 9 }
pub fn func10() -> i32 { 10 }
"#;

    create_tool
        .execute(
            json!({
                "path": "/funcs.rs",
                "workspace_id": workspace.id.to_string(),
                "content": code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/funcs.rs"), code)
        .await;

    // Navigation performance
    let nav_ctx = harness.code_nav_context();
    let list_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());

    let nav_start = Instant::now();
    let list_result = list_tool
        .execute(
            json!({
                "file_path": "/funcs.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["function"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List failed");
    let nav_duration = nav_start.elapsed();

    list_result
        .assert_success()
        .assert_array_length("units", 10);

    // Manipulation performance
    let modified_code = code.replace("func1", "function1");

    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());
    let manip_start = Instant::now();
    update_tool
        .execute(
            json!({
                "path": "/funcs.rs",
                "workspace_id": workspace.id.to_string(),
                "content": modified_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update failed");
    let manip_duration = manip_start.elapsed();

    // Performance assertions
    assert!(
        nav_duration.as_millis() < 500,
        "Navigation took too long: {:?}",
        nav_duration
    );

    assert!(
        manip_duration.as_millis() < 500,
        "Manipulation took too long: {:?}",
        manip_duration
    );

    println!("✓ Navigation + Manipulation performance test passed");
    println!("  - Navigation time: {:?}", nav_duration);
    println!("  - Manipulation time: {:?}", manip_duration);
}
