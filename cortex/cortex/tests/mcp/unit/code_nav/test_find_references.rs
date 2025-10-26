//! Unit Tests for cortex.code.find_references
//!
//! Tests cover:
//! - Finding all references to a function
//! - Finding references to structs/classes
//! - Finding references across multiple files
//! - Reference count verification
//! - Finding references by unit_id
//! - Finding references by qualified_name
//! - No references case
//! - Error handling
//! - Performance measurement

use super::test_helpers::*;
use cortex_cli::mcp::tools::code_nav::CodeFindReferencesTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_find_references_basic() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a target function and callers
    let target = fixtures::create_rust_function(
        "helper",
        "myapp::helper",
        "src/utils.rs",
        10,
    );
    let caller1 = fixtures::create_rust_function(
        "main_fn",
        "myapp::main_fn",
        "src/main.rs",
        5,
    );
    let caller2 = fixtures::create_rust_function(
        "process",
        "myapp::process",
        "src/lib.rs",
        20,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller1_id = fixture.store_unit(&caller1).await.unwrap();
    let caller2_id = fixture.store_unit(&caller2).await.unwrap();

    // Create dependencies (callers reference target)
    let dep1 = fixtures::create_call_dependency(caller1_id, target_id);
    let dep2 = fixtures::create_call_dependency(caller2_id, target_id);
    fixture.store_dependency(&dep1).await.unwrap();
    fixture.store_dependency(&dep2).await.unwrap();

    // Find references by unit_id
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find references");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 2);
        let refs = data["references"].as_array().unwrap();
        assert_eq!(refs.len(), 2);

        // Verify both callers are in the results
        let ref_names: Vec<&str> = refs
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert!(ref_names.contains(&"main_fn"));
        assert!(ref_names.contains(&"process"));
    }
}

#[tokio::test]
async fn test_find_references_by_qualified_name() {
    let fixture = CodeNavTestFixture::new().await;

    // Create target and caller
    let target = fixtures::create_rust_function(
        "utility_fn",
        "myapp::utils::utility_fn",
        "src/utils.rs",
        15,
    );
    let caller = fixtures::create_rust_function(
        "caller_fn",
        "myapp::caller_fn",
        "src/main.rs",
        10,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller_id = fixture.store_unit(&caller).await.unwrap();

    // Create dependency
    let dep = fixtures::create_call_dependency(caller_id, target_id);
    fixture.store_dependency(&dep).await.unwrap();

    // Find by qualified name
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "qualified_name": "myapp::utils::utility_fn",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find references by qualified name");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 1);
        let refs = data["references"].as_array().unwrap();
        assert_eq!(refs[0]["name"], "caller_fn");
    }
}

#[tokio::test]
async fn test_find_references_no_references() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function with no references
    let lonely_func = fixtures::create_rust_function(
        "unused_fn",
        "myapp::unused_fn",
        "src/lib.rs",
        50,
    );
    let func_id = fixture.store_unit(&lonely_func).await.unwrap();

    // Find references (should be empty)
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Should succeed even with no references");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 0);
        assert_eq!(data["references"].as_array().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn test_find_references_struct() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a struct and functions that use it
    let struct_unit = fixtures::create_rust_struct(
        "Config",
        "myapp::Config",
        "src/config.rs",
        5,
    );
    let user1 = fixtures::create_rust_function(
        "load_config",
        "myapp::load_config",
        "src/loader.rs",
        10,
    );
    let user2 = fixtures::create_rust_function(
        "save_config",
        "myapp::save_config",
        "src/saver.rs",
        15,
    );

    let struct_id = fixture.store_unit(&struct_unit).await.unwrap();
    let user1_id = fixture.store_unit(&user1).await.unwrap();
    let user2_id = fixture.store_unit(&user2).await.unwrap();

    // Create uses dependencies
    let dep1 = fixtures::create_call_dependency(user1_id, struct_id);
    let dep2 = fixtures::create_call_dependency(user2_id, struct_id);
    fixture.store_dependency(&dep1).await.unwrap();
    fixture.store_dependency(&dep2).await.unwrap();

    // Find references to struct
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": struct_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 2);
    }
}

#[tokio::test]
async fn test_find_references_cross_file() {
    let fixture = CodeNavTestFixture::new().await;

    // Create target in one file
    let target = fixtures::create_rust_function(
        "api_call",
        "myapp::api::api_call",
        "src/api.rs",
        10,
    );

    // Create callers in different files
    let caller1 = fixtures::create_rust_function(
        "handler1",
        "myapp::handlers::handler1",
        "src/handlers/mod.rs",
        5,
    );
    let caller2 = fixtures::create_rust_function(
        "service_fn",
        "myapp::services::service_fn",
        "src/services.rs",
        20,
    );
    let caller3 = fixtures::create_rust_function(
        "main",
        "myapp::main",
        "src/main.rs",
        1,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller1_id = fixture.store_unit(&caller1).await.unwrap();
    let caller2_id = fixture.store_unit(&caller2).await.unwrap();
    let caller3_id = fixture.store_unit(&caller3).await.unwrap();

    // Create cross-file dependencies
    fixture.store_dependency(&fixtures::create_call_dependency(caller1_id, target_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(caller2_id, target_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(caller3_id, target_id)).await.unwrap();

    // Find all references
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 3);

        let refs = data["references"].as_array().unwrap();
        let files: Vec<&str> = refs
            .iter()
            .map(|r| r["location"]["file"].as_str().unwrap())
            .collect();

        // Verify references from different files
        assert!(files.iter().any(|f| f.contains("handlers")));
        assert!(files.iter().any(|f| f.contains("services")));
        assert!(files.iter().any(|f| f.contains("main")));
    }
}

#[tokio::test]
async fn test_find_references_with_location_info() {
    let fixture = CodeNavTestFixture::new().await;

    // Create target and caller
    let target = fixtures::create_rust_function(
        "target_fn",
        "myapp::target_fn",
        "src/target.rs",
        10,
    );
    let caller = fixtures::create_rust_function(
        "caller_fn",
        "myapp::caller_fn",
        "src/caller.rs",
        25,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller_id = fixture.store_unit(&caller).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, target_id)).await.unwrap();

    // Find references and verify location info
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let refs = data["references"].as_array().unwrap();
        let ref_obj = &refs[0];

        // Verify location info is present
        assert_eq!(ref_obj["location"]["file"], "src/caller.rs");
        assert_eq!(ref_obj["location"]["start_line"], 25);
        assert_eq!(ref_obj["location"]["end_line"], 35);
        assert_eq!(ref_obj["qualified_name"], "myapp::caller_fn");
    }
}

#[tokio::test]
async fn test_find_references_typescript_class() {
    let fixture = CodeNavTestFixture::new().await;

    // Create TypeScript class and methods that use it
    let class = fixtures::create_typescript_class(
        "UserModel",
        "app.models.UserModel",
        "src/models/user.ts",
        10,
    );
    let method = fixtures::create_typescript_method(
        "getUserData",
        "app.services.getUserData",
        "src/services/user.ts",
        20,
    );

    let class_id = fixture.store_unit(&class).await.unwrap();
    let method_id = fixture.store_unit(&method).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(method_id, class_id)).await.unwrap();

    // Find references
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": class_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 1);
        let refs = data["references"].as_array().unwrap();
        assert_eq!(refs[0]["unit_type"], "Method");
    }
}

#[tokio::test]
async fn test_find_references_many_references() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a utility function
    let util_fn = fixtures::create_rust_function(
        "log",
        "myapp::utils::log",
        "src/utils.rs",
        5,
    );
    let util_id = fixture.store_unit(&util_fn).await.unwrap();

    // Create many callers
    for i in 0..50 {
        let caller = fixtures::create_rust_function(
            &format!("fn_{}", i),
            &format!("myapp::fn_{}", i),
            "src/lib.rs",
            i * 10,
        );
        let caller_id = fixture.store_unit(&caller).await.unwrap();
        fixture.store_dependency(&fixtures::create_call_dependency(caller_id, util_id)).await.unwrap();
    }

    // Find all references
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": util_id.to_string(),
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 500, "Should handle many references efficiently, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 50);
    }
}

#[tokio::test]
async fn test_find_references_error_invalid_unit_id() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with invalid unit_id format
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "not-a-valid-id",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail with invalid unit_id");
}

#[tokio::test]
async fn test_find_references_error_unit_not_found() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with non-existent unit_id
    let fake_id = cortex_core::id::CortexId::new();
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": fake_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail when unit not found");
}

#[tokio::test]
async fn test_find_references_error_no_identifier() {
    let fixture = CodeNavTestFixture::new().await;

    // Try without unit_id or qualified_name
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({});

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail without identifier");

    if let Err(ToolError::ExecutionFailed(msg)) = result {
        assert!(msg.contains("unit_id") || msg.contains("qualified_name"));
    }
}

#[tokio::test]
async fn test_find_references_includes_metadata() {
    let fixture = CodeNavTestFixture::new().await;

    // Create target and caller
    let target = fixtures::create_rust_function(
        "target",
        "myapp::target",
        "src/target.rs",
        10,
    );
    let caller = fixtures::create_rust_function(
        "caller",
        "myapp::caller",
        "src/caller.rs",
        20,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller_id = fixture.store_unit(&caller).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, target_id)).await.unwrap();

    // Find references and verify metadata is included
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let refs = data["references"].as_array().unwrap();
        let ref_obj = &refs[0];

        // Verify required fields
        assert!(!ref_obj["unit_id"].is_null());
        assert!(!ref_obj["name"].is_null());
        assert!(!ref_obj["qualified_name"].is_null());
        assert!(!ref_obj["unit_type"].is_null());
        assert!(!ref_obj["location"].is_null());
    }
}

#[tokio::test]
async fn test_find_references_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a complex dependency graph
    let target = fixtures::create_rust_function(
        "core_fn",
        "myapp::core_fn",
        "src/core.rs",
        10,
    );
    let target_id = fixture.store_unit(&target).await.unwrap();

    // Create 100 functions and some that reference the target
    for i in 0..100 {
        let func = fixtures::create_rust_function(
            &format!("func_{}", i),
            &format!("myapp::func_{}", i),
            "src/lib.rs",
            i * 10,
        );
        let func_id = fixture.store_unit(&func).await.unwrap();

        // Every 10th function references the target
        if i % 10 == 0 {
            fixture.store_dependency(&fixtures::create_call_dependency(func_id, target_id)).await.unwrap();
        }
    }

    // Measure lookup performance
    let tool = CodeFindReferencesTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 200, "Should be fast even with many units, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 10);
    }
}
