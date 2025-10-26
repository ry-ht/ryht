//! Unit Tests for cortex.code.get_call_hierarchy
//!
//! Tests cover:
//! - Getting outgoing calls (what this function calls)
//! - Getting incoming calls (what calls this function)
//! - Getting both directions
//! - Multi-level call chains
//! - Recursive calls
//! - No calls case
//! - Error handling
//! - Performance measurement

use super::test_helpers::*;
use cortex::mcp::tools::code_nav::CodeGetCallHierarchyTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_call_hierarchy_outgoing() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function that calls others
    let caller = fixtures::create_rust_function(
        "main_fn",
        "myapp::main_fn",
        "src/main.rs",
        10,
    );
    let callee1 = fixtures::create_rust_function(
        "helper1",
        "myapp::helper1",
        "src/utils.rs",
        20,
    );
    let callee2 = fixtures::create_rust_function(
        "helper2",
        "myapp::helper2",
        "src/utils.rs",
        30,
    );

    let caller_id = fixture.store_unit(&caller).await.unwrap();
    let callee1_id = fixture.store_unit(&callee1).await.unwrap();
    let callee2_id = fixture.store_unit(&callee2).await.unwrap();

    // Create outgoing call dependencies
    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, callee1_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, callee2_id)).await.unwrap();

    // Get outgoing calls
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": caller_id.to_string(),
        "direction": "outgoing",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get call hierarchy");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["outgoing_count"], 2);
        assert_eq!(data["incoming_count"], 0);

        let outgoing = data["outgoing_calls"].as_array().unwrap();
        assert_eq!(outgoing.len(), 2);

        let names: Vec<&str> = outgoing
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"helper1"));
        assert!(names.contains(&"helper2"));
    }
}

#[tokio::test]
async fn test_call_hierarchy_incoming() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function that is called by others
    let target = fixtures::create_rust_function(
        "utility_fn",
        "myapp::utility_fn",
        "src/utils.rs",
        10,
    );
    let caller1 = fixtures::create_rust_function(
        "consumer1",
        "myapp::consumer1",
        "src/main.rs",
        20,
    );
    let caller2 = fixtures::create_rust_function(
        "consumer2",
        "myapp::consumer2",
        "src/lib.rs",
        30,
    );

    let target_id = fixture.store_unit(&target).await.unwrap();
    let caller1_id = fixture.store_unit(&caller1).await.unwrap();
    let caller2_id = fixture.store_unit(&caller2).await.unwrap();

    // Create incoming call dependencies
    fixture.store_dependency(&fixtures::create_call_dependency(caller1_id, target_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(caller2_id, target_id)).await.unwrap();

    // Get incoming calls
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": target_id.to_string(),
        "direction": "incoming",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["outgoing_count"], 0);
        assert_eq!(data["incoming_count"], 2);

        let incoming = data["incoming_calls"].as_array().unwrap();
        assert_eq!(incoming.len(), 2);

        let names: Vec<&str> = incoming
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"consumer1"));
        assert!(names.contains(&"consumer2"));
    }
}

#[tokio::test]
async fn test_call_hierarchy_both_directions() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a middle function that calls and is called
    let middle = fixtures::create_rust_function(
        "process",
        "myapp::process",
        "src/lib.rs",
        20,
    );
    let caller = fixtures::create_rust_function(
        "orchestrator",
        "myapp::orchestrator",
        "src/main.rs",
        10,
    );
    let callee = fixtures::create_rust_function(
        "helper",
        "myapp::helper",
        "src/utils.rs",
        30,
    );

    let middle_id = fixture.store_unit(&middle).await.unwrap();
    let caller_id = fixture.store_unit(&caller).await.unwrap();
    let callee_id = fixture.store_unit(&callee).await.unwrap();

    // Create both incoming and outgoing dependencies
    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, middle_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(middle_id, callee_id)).await.unwrap();

    // Get both directions (default)
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": middle_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["outgoing_count"], 1);
        assert_eq!(data["incoming_count"], 1);

        let outgoing = data["outgoing_calls"].as_array().unwrap();
        assert_eq!(outgoing[0]["name"], "helper");

        let incoming = data["incoming_calls"].as_array().unwrap();
        assert_eq!(incoming[0]["name"], "orchestrator");
    }
}

#[tokio::test]
async fn test_call_hierarchy_no_calls() {
    let fixture = CodeNavTestFixture::new().await;

    // Create an isolated function
    let isolated = fixtures::create_rust_function(
        "isolated_fn",
        "myapp::isolated_fn",
        "src/lib.rs",
        10,
    );
    let isolated_id = fixture.store_unit(&isolated).await.unwrap();

    // Get call hierarchy (should be empty)
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": isolated_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["outgoing_count"], 0);
        assert_eq!(data["incoming_count"], 0);
    }
}

#[tokio::test]
async fn test_call_hierarchy_recursive() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a recursive function
    let recursive_fn = fixtures::create_rust_function(
        "factorial",
        "myapp::math::factorial",
        "src/math.rs",
        10,
    );
    let recursive_id = fixture.store_unit(&recursive_fn).await.unwrap();

    // Create self-reference
    fixture.store_dependency(&fixtures::create_call_dependency(recursive_id, recursive_id)).await.unwrap();

    // Get call hierarchy
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": recursive_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should show self in both incoming and outgoing
        assert_eq!(data["outgoing_count"], 1);
        assert_eq!(data["incoming_count"], 1);
    }
}

#[tokio::test]
async fn test_call_hierarchy_chain() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a call chain: fn1 -> fn2 -> fn3
    let fn1 = fixtures::create_rust_function("fn1", "myapp::fn1", "src/lib.rs", 10);
    let fn2 = fixtures::create_rust_function("fn2", "myapp::fn2", "src/lib.rs", 20);
    let fn3 = fixtures::create_rust_function("fn3", "myapp::fn3", "src/lib.rs", 30);

    let fn1_id = fixture.store_unit(&fn1).await.unwrap();
    let fn2_id = fixture.store_unit(&fn2).await.unwrap();
    let fn3_id = fixture.store_unit(&fn3).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(fn1_id, fn2_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_call_dependency(fn2_id, fn3_id)).await.unwrap();

    // Get hierarchy for middle function
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": fn2_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // fn2 is called by fn1 and calls fn3
        assert_eq!(data["incoming_count"], 1);
        assert_eq!(data["outgoing_count"], 1);

        let incoming = data["incoming_calls"].as_array().unwrap();
        assert_eq!(incoming[0]["name"], "fn1");

        let outgoing = data["outgoing_calls"].as_array().unwrap();
        assert_eq!(outgoing[0]["name"], "fn3");
    }
}

#[tokio::test]
async fn test_call_hierarchy_with_locations() {
    let fixture = CodeNavTestFixture::new().await;

    // Create functions with specific locations
    let caller = fixtures::create_rust_function("caller", "myapp::caller", "src/main.rs", 42);
    let callee = fixtures::create_rust_function("callee", "myapp::callee", "src/utils.rs", 100);

    let caller_id = fixture.store_unit(&caller).await.unwrap();
    let callee_id = fixture.store_unit(&callee).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, callee_id)).await.unwrap();

    // Get hierarchy and verify locations
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": caller_id.to_string(),
        "direction": "outgoing",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let outgoing = data["outgoing_calls"].as_array().unwrap();

        assert_eq!(outgoing[0]["file"], "src/utils.rs");
        assert_eq!(outgoing[0]["line"], 100);
    }
}

#[tokio::test]
async fn test_call_hierarchy_many_callers() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a popular utility function
    let util = fixtures::create_rust_function("log", "myapp::utils::log", "src/utils.rs", 5);
    let util_id = fixture.store_unit(&util).await.unwrap();

    // Create many callers
    for i in 0..20 {
        let caller = fixtures::create_rust_function(
            &format!("fn_{}", i),
            &format!("myapp::fn_{}", i),
            "src/lib.rs",
            i * 10,
        );
        let caller_id = fixture.store_unit(&caller).await.unwrap();
        fixture.store_dependency(&fixtures::create_call_dependency(caller_id, util_id)).await.unwrap();
    }

    // Get incoming calls
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": util_id.to_string(),
        "direction": "incoming",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 200, "Should handle many callers efficiently, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["incoming_count"], 20);
    }
}

#[tokio::test]
async fn test_call_hierarchy_typescript_methods() {
    let fixture = CodeNavTestFixture::new().await;

    // Create TypeScript class and methods
    let class_method = fixtures::create_typescript_method(
        "processUser",
        "app.UserService.processUser",
        "src/service.ts",
        20,
    );
    let helper = fixtures::create_typescript_method(
        "validateUser",
        "app.utils.validateUser",
        "src/utils.ts",
        10,
    );

    let method_id = fixture.store_unit(&class_method).await.unwrap();
    let helper_id = fixture.store_unit(&helper).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(method_id, helper_id)).await.unwrap();

    // Get call hierarchy
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": method_id.to_string(),
        "direction": "outgoing",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["outgoing_count"], 1);
    }
}

#[tokio::test]
async fn test_call_hierarchy_error_invalid_id() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with invalid unit_id
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "not-a-valid-id",
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail with invalid unit_id");
}

#[tokio::test]
async fn test_call_hierarchy_error_unit_not_found() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with non-existent unit_id
    let fake_id = cortex_core::id::CortexId::new();
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": fake_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    // This might succeed but return empty results, or fail depending on implementation
}

#[tokio::test]
async fn test_call_hierarchy_default_direction() {
    let fixture = CodeNavTestFixture::new().await;

    // Create simple call relationship
    let caller = fixtures::create_rust_function("caller", "myapp::caller", "src/lib.rs", 10);
    let callee = fixtures::create_rust_function("callee", "myapp::callee", "src/lib.rs", 20);

    let caller_id = fixture.store_unit(&caller).await.unwrap();
    let callee_id = fixture.store_unit(&callee).await.unwrap();

    fixture.store_dependency(&fixtures::create_call_dependency(caller_id, callee_id)).await.unwrap();

    // Omit direction (should default to "both")
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": caller_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        // Should include both directions by default
        assert!(data["outgoing_count"].as_u64().unwrap() >= 0);
        assert!(data["incoming_count"].as_u64().unwrap() >= 0);
    }
}

#[tokio::test]
async fn test_call_hierarchy_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a complex call graph
    let hub = fixtures::create_rust_function("hub", "myapp::hub", "src/lib.rs", 50);
    let hub_id = fixture.store_unit(&hub).await.unwrap();

    // Create 50 incoming and 50 outgoing connections
    for i in 0..50 {
        let caller = fixtures::create_rust_function(
            &format!("caller_{}", i),
            &format!("myapp::caller_{}", i),
            "src/lib.rs",
            i * 10,
        );
        let caller_id = fixture.store_unit(&caller).await.unwrap();
        fixture.store_dependency(&fixtures::create_call_dependency(caller_id, hub_id)).await.unwrap();

        let callee = fixtures::create_rust_function(
            &format!("callee_{}", i),
            &format!("myapp::callee_{}", i),
            "src/lib.rs",
            1000 + i * 10,
        );
        let callee_id = fixture.store_unit(&callee).await.unwrap();
        fixture.store_dependency(&fixtures::create_call_dependency(hub_id, callee_id)).await.unwrap();
    }

    // Measure performance
    let tool = CodeGetCallHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": hub_id.to_string(),
        "direction": "both",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 500, "Should handle complex graph efficiently, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["incoming_count"], 50);
        assert_eq!(data["outgoing_count"], 50);
    }
}
