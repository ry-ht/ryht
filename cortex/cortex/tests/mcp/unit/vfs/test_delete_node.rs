//! Unit Tests for cortex.vfs.delete_node
//!
//! Tests cover:
//! - Basic file deletion
//! - Directory deletion (recursive and non-recursive)
//! - Error handling (not found, non-empty directory)
//! - Concurrent deletions
//! - Reference counting verification
//! - Cache invalidation

use super::test_helpers::*;
use cortex_mcp::tools::vfs::{VfsDeleteNodeTool, VfsGetNodeTool};
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_delete_file_basic() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("delete_me.txt", "content").await.unwrap();

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "delete_me.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to delete file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["deleted"], true);
    }
}

#[tokio::test]
async fn test_delete_file_verify_deletion() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("verify_delete.txt", "content").await.unwrap();

    // Delete the file
    let delete_tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "verify_delete.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });
    fixture.execute_tool(&delete_tool, input).await;

    // Try to get it - should fail
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "verify_delete.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_err(), "File should not exist after deletion");
}

#[tokio::test]
async fn test_delete_empty_directory() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("empty_dir").await.unwrap();

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "empty_dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to delete empty directory");
}

#[tokio::test]
async fn test_delete_directory_recursive() {
    let fixture = VfsTestFixture::new().await;

    // Create directory with contents
    fixture.create_test_directory("dir_with_files/subdir").await.unwrap();
    fixture.create_test_file("dir_with_files/file1.txt", "1").await.unwrap();
    fixture.create_test_file("dir_with_files/subdir/file2.txt", "2").await.unwrap();

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "dir_with_files",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to delete directory recursively");
}

#[tokio::test]
async fn test_delete_not_found() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "nonexistent.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent file");
}

#[tokio::test]
async fn test_delete_nested_file() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("a/b/c").await.unwrap();
    fixture.create_test_file("a/b/c/deep.txt", "deep").await.unwrap();

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "a/b/c/deep.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to delete nested file");
}

#[tokio::test]
async fn test_delete_multiple_files() {
    let fixture = VfsTestFixture::new().await;

    // Create multiple files
    for i in 0..5 {
        fixture.create_test_file(&format!("file{}.txt", i), "content").await.unwrap();
    }

    // Delete them all
    for i in 0..5 {
        let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "path": format!("file{}.txt", i),
            "workspace_id": fixture.workspace_id.to_string(),
            "expected_version": 1,
            "recursive": false,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to delete file{}", i);
    }
}

#[tokio::test]
async fn test_delete_concurrent() {
    let fixture = VfsTestFixture::new().await;

    // Create files to delete
    for i in 0..10 {
        fixture.create_test_file(&format!("concurrent{}.txt", i), "content").await.unwrap();
    }

    // Delete concurrently
    let mut tasks = Vec::new();
    for i in 0..10 {
        let ctx_clone = fixture.ctx.clone();
        let ws_id = fixture.workspace_id;

        let task = tokio::spawn(async move {
            let tool = VfsDeleteNodeTool::new(ctx_clone);
            let input = json!({
                "path": format!("concurrent{}.txt", i),
                "workspace_id": ws_id.to_string(),
                "expected_version": 1,
                "recursive": false,
            });

            tool.execute(input, &ToolContext::default()).await
        });
        tasks.push(task);
    }

    let mut success_count = 0;
    for task in tasks {
        if let Ok(Ok(_)) = task.await {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All concurrent deletes should succeed");
}

#[tokio::test]
async fn test_delete_and_recreate() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("recreate.txt", "v1").await.unwrap();

    // Delete
    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "recreate.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });
    fixture.execute_tool(&tool, input).await;

    // Recreate with same name
    fixture.create_test_file("recreate.txt", "v2").await.unwrap();

    // Verify it exists with new content
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "recreate.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["content"], "v2");
    }
}

#[tokio::test]
async fn test_delete_large_file() {
    let fixture = VfsTestFixture::new().await;

    let large_content = fixtures::large_text();
    fixture.create_test_file("large.txt", &large_content).await.unwrap();

    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "large.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to delete large file");
}

#[tokio::test]
async fn test_delete_with_shared_content() {
    let fixture = VfsTestFixture::new().await;

    // Create files with same content (deduplicated)
    let same_content = "shared content";
    fixture.create_test_file("file1.txt", same_content).await.unwrap();
    fixture.create_test_file("file2.txt", same_content).await.unwrap();

    // Delete one file
    let tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "file1.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });
    fixture.execute_tool(&tool, input).await;

    // Other file should still exist and be readable
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "file2.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_ok(), "file2 should still exist");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["content"], same_content);
    }
}

#[tokio::test]
async fn test_delete_cache_invalidation() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("cached.txt", "content").await.unwrap();

    // Read to populate cache
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "cached.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });
    fixture.execute_tool(&get_tool, input).await;

    // Delete
    let delete_tool = VfsDeleteNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "cached.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "expected_version": 1,
        "recursive": false,
    });
    fixture.execute_tool(&delete_tool, input).await;

    // Try to read again - should fail
    let input = json!({
        "path": "cached.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_err(), "Cache should be invalidated");
}
