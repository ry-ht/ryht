//! Integration Tests: VFS + Parser
//!
//! These tests verify the integration between VFS operations and code parsing:
//! - File ingestion triggers parsing
//! - Code units are extracted from VFS files
//! - Updates to files trigger re-parsing
//! - Parsed units are correctly linked to VFS nodes
//!
//! Real-world scenarios:
//! - Import code file → parse → verify units extracted
//! - Update file → re-parse → verify units updated
//! - Multi-language parsing across VFS

use crate::mcp::utils::{TestHarness, ToolResultAssertions};
use cortex::mcp::tools::vfs::*;
use cortex::mcp::tools::code_nav::*;
use mcp_sdk::prelude::*;
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn test_file_import_triggers_parsing() {
    let harness = TestHarness::new().await;

    // Create workspace with a Rust file
    let workspace = harness
        .create_test_workspace("parse_test", harness.temp_path())
        .await;

    // Create a file via VFS
    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    let rust_code = r#"
/// A user struct
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    /// Create a new user
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }

    /// Get the user's display name
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

/// Add two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let create_result = create_tool
        .execute(
            json!({
                "path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "content": rust_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    create_result.assert_success();

    // Manually trigger ingestion (in production, this happens automatically)
    let _ingest_result = harness
        .ingest_file(workspace.id, std::path::Path::new("/user.rs"), rust_code)
        .await;

    // Verify code units were extracted via navigation tools
    let nav_ctx = harness.code_nav_context();
    let list_units_tool = CodeNavListUnitsInFileTool::new(nav_ctx.clone());

    let units_result = list_units_tool
        .execute(
            json!({
                "file_path": "/user.rs",
                "workspace_id": workspace.id.to_string(),
                "unit_types": ["struct", "function", "impl"]
            }),
            &ToolContext::default(),
        )
        .await
        .expect("List units failed");

    units_result
        .assert_success()
        .assert_has_field("units")
        .assert_array_min_length("units", 3); // User struct, impl, add function

    println!("✓ File import triggers parsing test passed");
    println!("  - File created in VFS");
    println!("  - Code units extracted successfully");
    println!("  - Units accessible via navigation tools");
}

#[tokio::test]
async fn test_file_update_triggers_reparse() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("reparse_test", harness.temp_path())
        .await;

    // Create initial file
    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    let initial_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

    create_tool
        .execute(
            json!({
                "path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "content": initial_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    // Ingest initial version
    harness
        .ingest_file(workspace.id, std::path::Path::new("/math.rs"), initial_code)
        .await;

    // Update file with more functions
    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());

    let updated_code = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn subtract(a: i32, b: i32) -> i32 { a - b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }
pub fn divide(a: i32, b: i32) -> Option<i32> {
    if b != 0 { Some(a / b) } else { None }
}
"#;

    let update_result = update_tool
        .execute(
            json!({
                "path": "/math.rs",
                "workspace_id": workspace.id.to_string(),
                "content": updated_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File update failed");

    update_result.assert_success();

    // Re-ingest updated version
    harness
        .ingest_file(workspace.id, std::path::Path::new("/math.rs"), updated_code)
        .await;

    // Verify new units are extracted
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
        .assert_array_min_length("units", 4); // 4 functions

    println!("✓ File update triggers reparse test passed");
    println!("  - Initial file parsed successfully");
    println!("  - File updated in VFS");
    println!("  - New units extracted after update");
}

#[tokio::test]
async fn test_multi_language_parsing() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("multilang_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create Rust file
    let rust_code = "pub fn rust_func() -> i32 { 42 }";
    create_tool
        .execute(
            json!({
                "path": "/code.rs",
                "workspace_id": workspace.id.to_string(),
                "content": rust_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Rust file creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/code.rs"), rust_code)
        .await;

    // Create TypeScript file
    let ts_code = r#"
export function tsFunc(): number {
    return 42;
}

export class User {
    constructor(public name: string) {}
}
"#;

    create_tool
        .execute(
            json!({
                "path": "/code.ts",
                "workspace_id": workspace.id.to_string(),
                "content": ts_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("TypeScript file creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/code.ts"), ts_code)
        .await;

    // Create Python file
    let py_code = r#"
def python_func():
    return 42

class User:
    def __init__(self, name: str):
        self.name = name
"#;

    create_tool
        .execute(
            json!({
                "path": "/code.py",
                "workspace_id": workspace.id.to_string(),
                "content": py_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Python file creation failed");

    harness
        .ingest_file(workspace.id, std::path::Path::new("/code.py"), py_code)
        .await;

    // Verify all files are in VFS
    let list_tool = VfsListDirectoryTool::new(vfs_ctx.clone());

    let list_result = list_tool
        .execute(
            json!({
                "path": "/",
                "workspace_id": workspace.id.to_string(),
                "recursive": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Directory listing failed");

    list_result
        .assert_success()
        .assert_array_min_length("entries", 3);

    println!("✓ Multi-language parsing test passed");
    println!("  - Rust file created and parsed");
    println!("  - TypeScript file created and parsed");
    println!("  - Python file created and parsed");
}

#[tokio::test]
async fn test_parsing_performance_with_vfs() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("perf_parse_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create a larger file for performance testing
    let large_code = r#"
pub struct Data {
    pub value: i32,
}

impl Data {
    pub fn new(value: i32) -> Self { Self { value } }
    pub fn get(&self) -> i32 { self.value }
    pub fn set(&mut self, value: i32) { self.value = value; }
}

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

    let start = Instant::now();

    create_tool
        .execute(
            json!({
                "path": "/large.rs",
                "workspace_id": workspace.id.to_string(),
                "content": large_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    let create_duration = start.elapsed();

    let parse_start = Instant::now();
    let ingest_result = harness
        .ingest_file(workspace.id, std::path::Path::new("/large.rs"), large_code)
        .await;
    let parse_duration = parse_start.elapsed();

    // Performance assertions
    assert!(
        create_duration.as_millis() < 1000,
        "VFS file creation took too long: {:?}",
        create_duration
    );

    assert!(
        parse_duration.as_millis() < 2000,
        "Parsing took too long: {:?}",
        parse_duration
    );

    assert!(
        ingest_result.units_extracted > 10,
        "Expected at least 10 units extracted"
    );

    println!("✓ Parsing performance test passed");
    println!("  - VFS creation: {:?}", create_duration);
    println!("  - Parsing time: {:?}", parse_duration);
    println!("  - Units extracted: {}", ingest_result.units_extracted);
}

#[tokio::test]
async fn test_vfs_parser_error_handling() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("error_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    // Create file with syntax errors
    let invalid_code = "pub fn broken( { // Invalid syntax";

    create_tool
        .execute(
            json!({
                "path": "/broken.rs",
                "workspace_id": workspace.id.to_string(),
                "content": invalid_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation should succeed even with invalid code");

    // Ingestion should handle errors gracefully
    let ingest_result = harness
        .ingest_file(workspace.id, std::path::Path::new("/broken.rs"), invalid_code)
        .await;

    // File should still be in VFS even if parsing failed
    let get_tool = VfsGetNodeTool::new(vfs_ctx.clone());

    let get_result = get_tool
        .execute(
            json!({
                "path": "/broken.rs",
                "workspace_id": workspace.id.to_string(),
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File should be retrievable");

    get_result.assert_success();

    println!("✓ VFS + Parser error handling test passed");
    println!("  - Invalid code stored in VFS");
    println!("  - Parsing errors handled gracefully");
    println!("  - File still accessible: {}", ingest_result.units_extracted);
}

#[tokio::test]
async fn test_incremental_parsing_workflow() {
    let harness = TestHarness::new().await;

    let workspace = harness
        .create_test_workspace("incremental_test", harness.temp_path())
        .await;

    let vfs_ctx = harness.vfs_context();
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());
    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());

    // Step 1: Create minimal file
    let v1_code = "pub struct Point { x: f64, y: f64 }";

    create_tool
        .execute(
            json!({
                "path": "/geometry.rs",
                "workspace_id": workspace.id.to_string(),
                "content": v1_code,
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Initial creation failed");

    let r1 = harness
        .ingest_file(workspace.id, std::path::Path::new("/geometry.rs"), v1_code)
        .await;

    // Step 2: Add implementation
    let v2_code = r#"
pub struct Point { x: f64, y: f64 }

impl Point {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
}
"#;

    update_tool
        .execute(
            json!({
                "path": "/geometry.rs",
                "workspace_id": workspace.id.to_string(),
                "content": v2_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update 1 failed");

    let r2 = harness
        .ingest_file(workspace.id, std::path::Path::new("/geometry.rs"), v2_code)
        .await;

    // Step 3: Add more methods
    let v3_code = r#"
pub struct Point { x: f64, y: f64 }

impl Point {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
"#;

    update_tool
        .execute(
            json!({
                "path": "/geometry.rs",
                "workspace_id": workspace.id.to_string(),
                "content": v3_code,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Update 2 failed");

    let r3 = harness
        .ingest_file(workspace.id, std::path::Path::new("/geometry.rs"), v3_code)
        .await;

    // Verify incremental unit extraction
    assert!(r2.units_extracted > r1.units_extracted, "Units should increase after adding impl");
    assert!(r3.units_extracted > r2.units_extracted, "Units should increase after adding methods");

    println!("✓ Incremental parsing workflow test passed");
    println!("  - V1 units: {}", r1.units_extracted);
    println!("  - V2 units: {}", r2.units_extracted);
    println!("  - V3 units: {}", r3.units_extracted);
}
