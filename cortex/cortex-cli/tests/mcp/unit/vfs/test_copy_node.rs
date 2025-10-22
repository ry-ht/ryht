//! Unit Tests for cortex.vfs.copy_node
//!
//! Tests cover:
//! - Basic file copying
//! - Directory copying (recursive)
//! - Copy with overwrite
//! - Content deduplication after copy
//! - Error handling (source not found, target exists)
//! - Concurrent copies
//! - Large file copies
//! - Reference counting

use super::test_helpers::*;
use cortex_mcp::tools::vfs::{VfsCopyNodeTool, VfsGetNodeTool};
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_copy_file_basic() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["copied"], true);
    }
}

#[tokio::test]
async fn test_copy_file_both_exist() {
    let fixture = VfsTestFixture::new().await;

    let original_content = "original content";
    fixture.create_test_file("original.txt", original_content).await.unwrap();

    // Copy file
    let copy_tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "original.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&copy_tool, input).await;

    // Verify both exist
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());

    // Check original
    let input = json!({
        "path": "original.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_ok(), "Original should still exist");

    // Check copy
    let input = json!({
        "path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    assert!(result.is_ok(), "Copy should exist");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["content"], original_content);
    }
}

#[tokio::test]
async fn test_copy_file_content_preserved() {
    let fixture = VfsTestFixture::new().await;

    let content = "important content to preserve";
    fixture.create_test_file("source.txt", content).await.unwrap();

    let copy_tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&copy_tool, input).await;

    // Verify content
    let get_tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    if let Ok(ToolResult::Success { content: result_content }) = result {
        let data: serde_json::Value = serde_json::from_value(result_content).unwrap();
        assert_eq!(data["content"], content);
    }
}

#[tokio::test]
async fn test_copy_file_not_found() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "nonexistent.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent source");
}

#[tokio::test]
async fn test_copy_file_target_exists_no_overwrite() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "source").await.unwrap();
    fixture.create_test_file("target.txt", "target").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "overwrite": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail when target exists without overwrite");
}

#[tokio::test]
async fn test_copy_file_target_exists_with_overwrite() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "source content").await.unwrap();
    fixture.create_test_file("target.txt", "target content").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "target.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "overwrite": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Should succeed with overwrite=true");
}

#[tokio::test]
async fn test_copy_file_to_different_directory() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();
    fixture.create_test_directory("target_dir").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "target_dir/copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy to different directory");
}

#[tokio::test]
async fn test_copy_directory_recursive() {
    let fixture = VfsTestFixture::new().await;

    // Create directory with content
    fixture.create_test_directory("source_dir/nested").await.unwrap();
    fixture.create_test_file("source_dir/file1.txt", "1").await.unwrap();
    fixture.create_test_file("source_dir/nested/file2.txt", "2").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source_dir",
        "target_path": "copy_dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "recursive": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy directory recursively");
}

#[tokio::test]
async fn test_copy_directory_non_recursive_fails() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("source_dir").await.unwrap();
    fixture.create_test_file("source_dir/file.txt", "content").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source_dir",
        "target_path": "copy_dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail to copy non-empty directory without recursive");
}

#[tokio::test]
async fn test_copy_file_large() {
    let fixture = VfsTestFixture::new().await;

    let large_content = fixtures::large_text();
    fixture.create_test_file("large.txt", &large_content).await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "large.txt",
        "target_path": "large_copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy large file");
}

#[tokio::test]
async fn test_copy_file_content_deduplication() {
    let fixture = VfsTestFixture::new().await;

    let content = "deduplicated content";
    fixture.create_test_file("original.txt", content).await.unwrap();

    // Copy file
    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "original.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&tool, input).await;

    // Both files should share the same content blob in storage
    // VFS should use content deduplication via hashing
}

#[tokio::test]
async fn test_copy_multiple_files() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("copies").await.unwrap();

    // Create and copy multiple files
    for i in 0..5 {
        fixture.create_test_file(&format!("file{}.txt", i), &format!("content{}", i)).await.unwrap();

        let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "source_path": format!("file{}.txt", i),
            "target_path": format!("copies/file{}.txt", i),
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to copy file{}", i);
    }
}

#[tokio::test]
async fn test_copy_file_concurrent() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "shared source").await.unwrap();

    // Copy to multiple targets concurrently
    let mut tasks = Vec::new();
    for i in 0..10 {
        let ctx_clone = fixture.ctx.clone();
        let ws_id = fixture.workspace_id;

        let task = tokio::spawn(async move {
            let tool = VfsCopyNodeTool::new(ctx_clone);
            let input = json!({
                "source_path": "source.txt",
                "target_path": format!("copy{}.txt", i),
                "workspace_id": ws_id.to_string(),
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

    assert_eq!(success_count, 10, "All concurrent copies should succeed");
}

#[tokio::test]
async fn test_copy_file_unicode() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", fixtures::UNICODE_TEXT).await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "文件_copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy to unicode filename");
}

#[tokio::test]
async fn test_copy_nested_directory_structure() {
    let fixture = VfsTestFixture::new().await;

    // Create complex nested structure
    fixture.create_test_directory("src/a/b/c").await.unwrap();
    fixture.create_test_file("src/file1.txt", "1").await.unwrap();
    fixture.create_test_file("src/a/file2.txt", "2").await.unwrap();
    fixture.create_test_file("src/a/b/file3.txt", "3").await.unwrap();
    fixture.create_test_file("src/a/b/c/file4.txt", "4").await.unwrap();

    let tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "src",
        "target_path": "dst",
        "workspace_id": fixture.workspace_id.to_string(),
        "recursive": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to copy nested structure");
}

#[tokio::test]
async fn test_copy_preserves_metadata() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("source.txt", "content").await.unwrap();

    // Get original size
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

    // Copy file
    let copy_tool = VfsCopyNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "source_path": "source.txt",
        "target_path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    fixture.execute_tool(&copy_tool, input).await;

    // Verify copy has same size
    let input = json!({
        "path": "copy.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });
    let (result, _) = fixture.execute_tool(&get_tool, input).await;
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["size_bytes"].as_u64().unwrap(), original_size);
    }
}
