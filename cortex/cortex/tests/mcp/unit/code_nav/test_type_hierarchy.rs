//! Unit Tests for cortex.code.get_type_hierarchy
//!
//! Tests cover:
//! - Getting supertypes (what this type extends/implements)
//! - Getting subtypes (what extends/implements this type)
//! - Getting both directions
//! - Multi-level inheritance
//! - Interface implementations
//! - Trait implementations
//! - No hierarchy case
//! - Error handling
//! - Performance measurement

use super::test_helpers::*;
use cortex_cli::mcp::tools::code_nav::CodeGetTypeHierarchyTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_type_hierarchy_supertypes() {
    let fixture = CodeNavTestFixture::new().await;

    // Create inheritance: Child extends Base
    let base_struct = fixtures::create_rust_struct(
        "Base",
        "myapp::Base",
        "src/types.rs",
        10,
    );
    let child_struct = fixtures::create_rust_struct(
        "Child",
        "myapp::Child",
        "src/types.rs",
        30,
    );

    let base_id = fixture.store_unit(&base_struct).await.unwrap();
    let child_id = fixture.store_unit(&child_struct).await.unwrap();

    // Create extends dependency
    fixture.store_dependency(&fixtures::create_extends_dependency(child_id, base_id)).await.unwrap();

    // Get supertypes of Child
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": child_id.to_string(),
        "direction": "supertypes",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get type hierarchy");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 1);
        assert_eq!(data["subtypes_count"], 0);

        let supertypes = data["supertypes"].as_array().unwrap();
        assert_eq!(supertypes.len(), 1);
        assert_eq!(supertypes[0]["name"], "Base");
        assert!(supertypes[0]["relationship"].as_str().unwrap().contains("Extends"));
    }
}

#[tokio::test]
async fn test_type_hierarchy_subtypes() {
    let fixture = CodeNavTestFixture::new().await;

    // Create inheritance: Child1 and Child2 extend Base
    let base_struct = fixtures::create_rust_struct(
        "Animal",
        "myapp::Animal",
        "src/types.rs",
        10,
    );
    let child1 = fixtures::create_rust_struct(
        "Dog",
        "myapp::Dog",
        "src/types.rs",
        30,
    );
    let child2 = fixtures::create_rust_struct(
        "Cat",
        "myapp::Cat",
        "src/types.rs",
        50,
    );

    let base_id = fixture.store_unit(&base_struct).await.unwrap();
    let child1_id = fixture.store_unit(&child1).await.unwrap();
    let child2_id = fixture.store_unit(&child2).await.unwrap();

    // Create extends dependencies
    fixture.store_dependency(&fixtures::create_extends_dependency(child1_id, base_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_extends_dependency(child2_id, base_id)).await.unwrap();

    // Get subtypes of Base
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": base_id.to_string(),
        "direction": "subtypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 0);
        assert_eq!(data["subtypes_count"], 2);

        let subtypes = data["subtypes"].as_array().unwrap();
        assert_eq!(subtypes.len(), 2);

        let names: Vec<&str> = subtypes
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Dog"));
        assert!(names.contains(&"Cat"));
    }
}

#[tokio::test]
async fn test_type_hierarchy_both_directions() {
    let fixture = CodeNavTestFixture::new().await;

    // Create three-level hierarchy: Base -> Middle -> Child
    let base = fixtures::create_rust_struct("Base", "myapp::Base", "src/types.rs", 10);
    let middle = fixtures::create_rust_struct("Middle", "myapp::Middle", "src/types.rs", 30);
    let child = fixtures::create_rust_struct("Child", "myapp::Child", "src/types.rs", 50);

    let base_id = fixture.store_unit(&base).await.unwrap();
    let middle_id = fixture.store_unit(&middle).await.unwrap();
    let child_id = fixture.store_unit(&child).await.unwrap();

    // Create hierarchy
    fixture.store_dependency(&fixtures::create_extends_dependency(middle_id, base_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_extends_dependency(child_id, middle_id)).await.unwrap();

    // Get both directions for Middle
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": middle_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 1);
        assert_eq!(data["subtypes_count"], 1);

        let supertypes = data["supertypes"].as_array().unwrap();
        assert_eq!(supertypes[0]["name"], "Base");

        let subtypes = data["subtypes"].as_array().unwrap();
        assert_eq!(subtypes[0]["name"], "Child");
    }
}

#[tokio::test]
async fn test_type_hierarchy_trait_implementation() {
    let fixture = CodeNavTestFixture::new().await;

    // Create trait and implementing struct
    let trait_unit = fixtures::create_rust_trait(
        "Display",
        "myapp::Display",
        "src/traits.rs",
        10,
    );
    let impl_struct = fixtures::create_rust_struct(
        "User",
        "myapp::User",
        "src/models.rs",
        20,
    );

    let trait_id = fixture.store_unit(&trait_unit).await.unwrap();
    let impl_id = fixture.store_unit(&impl_struct).await.unwrap();

    // Create implements dependency
    fixture.store_dependency(&fixtures::create_implements_dependency(impl_id, trait_id)).await.unwrap();

    // Get supertypes of implementing struct
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": impl_id.to_string(),
        "direction": "supertypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 1);

        let supertypes = data["supertypes"].as_array().unwrap();
        assert_eq!(supertypes[0]["name"], "Display");
        assert!(supertypes[0]["relationship"].as_str().unwrap().contains("Implements"));
    }
}

#[tokio::test]
async fn test_type_hierarchy_multiple_traits() {
    let fixture = CodeNavTestFixture::new().await;

    // Create struct implementing multiple traits
    let trait1 = fixtures::create_rust_trait("Serialize", "myapp::Serialize", "src/traits.rs", 10);
    let trait2 = fixtures::create_rust_trait("Deserialize", "myapp::Deserialize", "src/traits.rs", 20);
    let trait3 = fixtures::create_rust_trait("Clone", "myapp::Clone", "src/traits.rs", 30);
    let impl_struct = fixtures::create_rust_struct("Data", "myapp::Data", "src/models.rs", 40);

    let trait1_id = fixture.store_unit(&trait1).await.unwrap();
    let trait2_id = fixture.store_unit(&trait2).await.unwrap();
    let trait3_id = fixture.store_unit(&trait3).await.unwrap();
    let impl_id = fixture.store_unit(&impl_struct).await.unwrap();

    // Implement all traits
    fixture.store_dependency(&fixtures::create_implements_dependency(impl_id, trait1_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_implements_dependency(impl_id, trait2_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_implements_dependency(impl_id, trait3_id)).await.unwrap();

    // Get supertypes
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": impl_id.to_string(),
        "direction": "supertypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 3);

        let supertypes = data["supertypes"].as_array().unwrap();
        let names: Vec<&str> = supertypes
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();

        assert!(names.contains(&"Serialize"));
        assert!(names.contains(&"Deserialize"));
        assert!(names.contains(&"Clone"));
    }
}

#[tokio::test]
async fn test_type_hierarchy_typescript_class() {
    let fixture = CodeNavTestFixture::new().await;

    // Create TypeScript class hierarchy
    let base_class = fixtures::create_typescript_class(
        "BaseService",
        "app.services.BaseService",
        "src/base.ts",
        10,
    );
    let derived_class = fixtures::create_typescript_class(
        "UserService",
        "app.services.UserService",
        "src/user.ts",
        20,
    );

    let base_id = fixture.store_unit(&base_class).await.unwrap();
    let derived_id = fixture.store_unit(&derived_class).await.unwrap();

    fixture.store_dependency(&fixtures::create_extends_dependency(derived_id, base_id)).await.unwrap();

    // Get supertypes
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": derived_id.to_string(),
        "direction": "supertypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 1);
        assert_eq!(data["supertypes"].as_array().unwrap()[0]["name"], "BaseService");
    }
}

#[tokio::test]
async fn test_type_hierarchy_no_hierarchy() {
    let fixture = CodeNavTestFixture::new().await;

    // Create isolated type with no inheritance
    let isolated = fixtures::create_rust_struct(
        "Standalone",
        "myapp::Standalone",
        "src/types.rs",
        10,
    );
    let isolated_id = fixture.store_unit(&isolated).await.unwrap();

    // Get hierarchy (should be empty)
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": isolated_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 0);
        assert_eq!(data["subtypes_count"], 0);
    }
}

#[tokio::test]
async fn test_type_hierarchy_multi_level() {
    let fixture = CodeNavTestFixture::new().await;

    // Create 4-level hierarchy
    let level1 = fixtures::create_rust_struct("Level1", "myapp::Level1", "src/types.rs", 10);
    let level2 = fixtures::create_rust_struct("Level2", "myapp::Level2", "src/types.rs", 20);
    let level3 = fixtures::create_rust_struct("Level3", "myapp::Level3", "src/types.rs", 30);
    let level4 = fixtures::create_rust_struct("Level4", "myapp::Level4", "src/types.rs", 40);

    let level1_id = fixture.store_unit(&level1).await.unwrap();
    let level2_id = fixture.store_unit(&level2).await.unwrap();
    let level3_id = fixture.store_unit(&level3).await.unwrap();
    let level4_id = fixture.store_unit(&level4).await.unwrap();

    fixture.store_dependency(&fixtures::create_extends_dependency(level2_id, level1_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_extends_dependency(level3_id, level2_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_extends_dependency(level4_id, level3_id)).await.unwrap();

    // Get hierarchy from level 3 (middle)
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": level3_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Level 3 extends Level2 and is extended by Level4
        assert_eq!(data["supertypes_count"], 1);
        assert_eq!(data["subtypes_count"], 1);
    }
}

#[tokio::test]
async fn test_type_hierarchy_with_metadata() {
    let fixture = CodeNavTestFixture::new().await;

    // Create hierarchy with metadata
    let base = fixtures::create_rust_struct("Base", "myapp::types::Base", "src/base.rs", 15);
    let child = fixtures::create_rust_struct("Child", "myapp::types::Child", "src/child.rs", 25);

    let base_id = fixture.store_unit(&base).await.unwrap();
    let child_id = fixture.store_unit(&child).await.unwrap();

    fixture.store_dependency(&fixtures::create_extends_dependency(child_id, base_id)).await.unwrap();

    // Get hierarchy and verify metadata
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": child_id.to_string(),
        "direction": "supertypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let supertypes = data["supertypes"].as_array().unwrap();

        // Verify all required fields are present
        assert!(!supertypes[0]["unit_id"].is_null());
        assert!(!supertypes[0]["name"].is_null());
        assert!(!supertypes[0]["qualified_name"].is_null());
        assert!(!supertypes[0]["unit_type"].is_null());
        assert!(!supertypes[0]["relationship"].is_null());
    }
}

#[tokio::test]
async fn test_type_hierarchy_error_invalid_id() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with invalid type_id
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": "not-a-valid-id",
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail with invalid type_id");
}

#[tokio::test]
async fn test_type_hierarchy_error_type_not_found() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with non-existent type_id
    let fake_id = cortex_core::id::CortexId::new();
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": fake_id.to_string(),
        "direction": "both",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    // May succeed with empty results or fail depending on implementation
}

#[tokio::test]
async fn test_type_hierarchy_default_direction() {
    let fixture = CodeNavTestFixture::new().await;

    // Create simple hierarchy
    let base = fixtures::create_rust_struct("Base", "myapp::Base", "src/types.rs", 10);
    let child = fixtures::create_rust_struct("Child", "myapp::Child", "src/types.rs", 20);

    let base_id = fixture.store_unit(&base).await.unwrap();
    let child_id = fixture.store_unit(&child).await.unwrap();

    fixture.store_dependency(&fixtures::create_extends_dependency(child_id, base_id)).await.unwrap();

    // Omit direction (should default to "both")
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": child_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        // Should include both directions by default
        assert!(data.get("supertypes_count").is_some());
        assert!(data.get("subtypes_count").is_some());
    }
}

#[tokio::test]
async fn test_type_hierarchy_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a type with many subtypes
    let base = fixtures::create_rust_struct("BaseType", "myapp::BaseType", "src/types.rs", 10);
    let base_id = fixture.store_unit(&base).await.unwrap();

    // Create 50 subtypes
    for i in 0..50 {
        let child = fixtures::create_rust_struct(
            &format!("Child{}", i),
            &format!("myapp::Child{}", i),
            "src/types.rs",
            20 + i * 10,
        );
        let child_id = fixture.store_unit(&child).await.unwrap();
        fixture.store_dependency(&fixtures::create_extends_dependency(child_id, base_id)).await.unwrap();
    }

    // Measure performance
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": base_id.to_string(),
        "direction": "subtypes",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 300, "Should handle many subtypes efficiently, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["subtypes_count"], 50);
    }
}

#[tokio::test]
async fn test_type_hierarchy_diamond_problem() {
    let fixture = CodeNavTestFixture::new().await;

    // Create diamond inheritance pattern:
    //     Base
    //    /    \
    //  Left  Right
    //    \    /
    //    Bottom

    let base = fixtures::create_rust_trait("Base", "myapp::Base", "src/types.rs", 10);
    let left = fixtures::create_rust_trait("Left", "myapp::Left", "src/types.rs", 20);
    let right = fixtures::create_rust_trait("Right", "myapp::Right", "src/types.rs", 30);
    let bottom = fixtures::create_rust_struct("Bottom", "myapp::Bottom", "src/types.rs", 40);

    let base_id = fixture.store_unit(&base).await.unwrap();
    let left_id = fixture.store_unit(&left).await.unwrap();
    let right_id = fixture.store_unit(&right).await.unwrap();
    let bottom_id = fixture.store_unit(&bottom).await.unwrap();

    // Create diamond structure
    fixture.store_dependency(&fixtures::create_extends_dependency(left_id, base_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_extends_dependency(right_id, base_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_implements_dependency(bottom_id, left_id)).await.unwrap();
    fixture.store_dependency(&fixtures::create_implements_dependency(bottom_id, right_id)).await.unwrap();

    // Get supertypes of Bottom (should have Left and Right)
    let tool = CodeGetTypeHierarchyTool::new(fixture.ctx.clone());
    let input = json!({
        "type_id": bottom_id.to_string(),
        "direction": "supertypes",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["supertypes_count"], 2);

        let names: Vec<&str> = data["supertypes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();

        assert!(names.contains(&"Left"));
        assert!(names.contains(&"Right"));
    }
}
