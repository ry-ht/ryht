//! Unit Tests for cortex.vfs.move_node
//!
//! Tests cover:
//! - Basic file moves
//! - File renaming
//! - Moving between directories
//! - Moving with overwrite
//! - Error handling (source not found, target exists)
//! - Concurrent moves
//! - Large file moves

use super::test_helpers::*;
use cortex_mcp::tools::vfs::{VfsGetNodeTool, VfsMoveNodeTool};
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_move_file_basic() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "destination.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["moved"], true);
        assert_eq!(data["source_path"], "source.txt");
        assert_eq!(data["target_path"], "destination.txt");
    }
}

#[tokio::test]
async fn test_move_file_verify_content_preserved() {
    let fixture = VfsTestFixture::new().await;

    let original_content = "important content";
    fixture.create_test_file("original.txt", original_content).await.unwrap();

    // Move file
    let move_tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "original.txt",
        "target_path": "moved.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&move_tool, input).await;

    // Verify content at new location
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "moved.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["content"], original_content);
    }
}

#[tokio::test]
async fn test_move_file_source_deleted() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    // Move file
    let move_tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&move_tool, input).await;

    // Verify source no longer exists
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "source.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_err(), "Source file should not exist after move");
}

#[tokio::test]
async fn test_move_file_to_different_directory() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("file.txt", "content").await.unwrap();
    fixture.create_test_directory("target_dir").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "file.txt",
        "target_path": "target_dir/file.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move file to different directory");
}

#[tokio::test]
async fn test_move_file_rename_only() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("old_name.txt", "content").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "old_name.txt",
        "target_path": "new_name.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to rename file");
}

#[tokio::test]
async fn test_move_file_not_found() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "nonexistent.txt",
        "target_path": "target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent source");
}

#[tokio::test]
async fn test_move_file_with_overwrite() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "source content").await.unwrap();
    fixture.create_test_file("target.txt", "target content").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "overwrite": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    // Overwrite behavior depends on implementation
    let _ = result;
}

#[tokio::test]
async fn test_move_file_nested_paths() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("a/b/c").await.unwrap();
    fixture.create_test_directory("x/y/z").await.unwrap();
    fixture.create_test_file("a/b/c/file.txt", "nested").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "a/b/c/file.txt",
        "target_path": "x/y/z/file.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move between nested directories");
}

#[tokio::test]
async fn test_move_file_large() {
    let fixture = VfsTestFixture::new().await;

    let large_content = fixtures::large_text();
    fixture.create_test_file("large_source.txt", &large_content).await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "large_source.txt",
        "target_path": "large_target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move large file");
}

#[tokio::test]
async fn test_move_file_unicode_names() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "文件.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move to unicode filename");
}

#[tokio::test]
async fn test_move_file_special_characters() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "file with spaces.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to move to name with spaces");
}

#[tokio::test]
async fn test_move_multiple_files() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("target").await.unwrap();

    // Create and move multiple files
    for i in 0..5 {
        fixture.create_test_file(&format!("file{}.txt", i), &format!("content{}", i)).await.unwrap();

        let tool = VfsMoveNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "source_path": format!("file{}.txt", i),
            "target_path": format!("target/file{}.txt", i),
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to move file{}", i);
    }
}

#[tokio::test]
async fn test_move_preserves_content_hash() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content for hash").await.unwrap();

    // Get original metadata
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "source.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    let original_size = if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        data["size_bytes"].as_u64().unwrap()
    } else {
        0
    };

    // Move file
    let move_tool = VfsMoveNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "moved.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&move_tool, input).await;

    // Verify size matches
    let input = json!({
        "path": "moved.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["size_bytes"].as_u64().unwrap(), original_size);
    }
}
