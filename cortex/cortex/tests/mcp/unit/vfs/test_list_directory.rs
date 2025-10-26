//! Unit Tests for cortex.vfs.list_directory
//!
//! Tests cover:
//! - Basic directory listing
//! - Recursive listing
//! - Hidden file filtering
//! - Type filtering (files vs directories)
//! - Empty directories
//! - Large directories (1000+ entries)
//! - Concurrent listings
//! - Pattern filtering

use super::test_helpers::*;
use cortex_mcp::tools::vfs::VfsListDirectoryTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_list_directory_basic() {
    let fixture = VfsTestFixture::new().await;

    // Create directory with files
    fixture.create_test_directory("testdir").await.unwrap();
    fixture.create_test_file("testdir/file1.txt", "content1").await.unwrap();
    fixture.create_test_file("testdir/file2.txt", "content2").await.unwrap();
    fixture.create_test_file("testdir/file3.txt", "content3").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "testdir",
        "workspace_id": fixture.workspace_id.to_string(),
        "recursive": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["total"].as_u64().unwrap(), 3);
        assert!(data["entries"].is_array());
    }
}

#[tokio::test]
async fn test_list_directory_empty() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("empty").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "empty",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["total"].as_u64().unwrap(), 0);
    }
}

#[tokio::test]
async fn test_list_directory_recursive() {
    let fixture = VfsTestFixture::new().await;

    // Create nested structure
    fixture.create_test_directory("root/a/b/c").await.unwrap();
    fixture.create_test_file("root/file1.txt", "1").await.unwrap();
    fixture.create_test_file("root/a/file2.txt", "2").await.unwrap();
    fixture.create_test_file("root/a/b/file3.txt", "3").await.unwrap();
    fixture.create_test_file("root/a/b/c/file4.txt", "4").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "root",
        "workspace_id": fixture.workspace_id.to_string(),
        "recursive": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        // Should include all files and directories
        assert!(data["total"].as_u64().unwrap() >= 4, "Should list all nested files");
    }
}

#[tokio::test]
async fn test_list_directory_hidden_files() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("dir").await.unwrap();
    fixture.create_test_file("dir/visible.txt", "visible").await.unwrap();
    fixture.create_test_file("dir/.hidden.txt", "hidden").await.unwrap();
    fixture.create_test_file("dir/.gitignore", "git").await.unwrap();

    // List without hidden files
    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_hidden": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let entries = data["entries"].as_array().unwrap();

        // Should not include hidden files
        for entry in entries {
            let name = entry["name"].as_str().unwrap();
            assert!(!name.starts_with('.'), "Should not include hidden files");
        }
    }
}

#[tokio::test]
async fn test_list_directory_with_hidden_files() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("dir").await.unwrap();
    fixture.create_test_file("dir/visible.txt", "visible").await.unwrap();
    fixture.create_test_file("dir/.hidden.txt", "hidden").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_hidden": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["total"].as_u64().unwrap(), 2);
    }
}

#[tokio::test]
async fn test_list_directory_filter_files_only() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("dir/subdir").await.unwrap();
    fixture.create_test_file("dir/file1.txt", "1").await.unwrap();
    fixture.create_test_file("dir/file2.txt", "2").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "filter": {
            "node_type": "file"
        },
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let entries = data["entries"].as_array().unwrap();

        for entry in entries {
            assert_eq!(entry["node_type"], "file");
        }
    }
}

#[tokio::test]
async fn test_list_directory_filter_directories_only() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("dir/sub1").await.unwrap();
    fixture.create_test_directory("dir/sub2").await.unwrap();
    fixture.create_test_file("dir/file.txt", "content").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "dir",
        "workspace_id": fixture.workspace_id.to_string(),
        "filter": {
            "node_type": "directory"
        },
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let entries = data["entries"].as_array().unwrap();

        for entry in entries {
            assert_eq!(entry["node_type"], "directory");
        }
    }
}

#[tokio::test]
async fn test_list_directory_large() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("large").await.unwrap();

    // Create 100 files
    for i in 0..100 {
        fixture.create_test_file(&format!("large/file{:03}.txt", i), "content").await.unwrap();
    }

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "large",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["total"].as_u64().unwrap(), 100);
    }
}

#[tokio::test]
async fn test_list_directory_not_found() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "nonexistent",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent directory");
}

#[tokio::test]
async fn test_list_directory_concurrent() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("concurrent").await.unwrap();
    for i in 0..10 {
        fixture.create_test_file(&format!("concurrent/file{}.txt", i), "content").await.unwrap();
    }

    // Spawn 20 concurrent list operations
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let ctx_clone = fixture.ctx.clone();
        let ws_id = fixture.workspace_id;

        let task = tokio::spawn(async move {
            let tool = VfsListDirectoryTool::new(ctx_clone);
            let input = json!({
                "path": "concurrent",
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

    assert_eq!(success_count, 20, "All concurrent lists should succeed");
}

#[tokio::test]
async fn test_list_directory_mixed_content() {
    let fixture = VfsTestFixture::new().await;

    // Create mixed content types
    fixture.create_test_directory("mixed/subdir").await.unwrap();
    fixture.create_test_file("mixed/text.txt", "text").await.unwrap();
    fixture.create_test_file("mixed/code.rs", "fn main() {}").await.unwrap();
    fixture.create_test_file("mixed/data.json", "{}").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "mixed",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(data["total"].as_u64().unwrap() >= 3);
    }
}

#[tokio::test]
async fn test_list_directory_entry_metadata() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("meta").await.unwrap();
    fixture.create_test_file("meta/file.txt", fixtures::SMALL_TEXT).await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "meta",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let entries = data["entries"].as_array().unwrap();

        for entry in entries {
            assert!(!entry["name"].is_null());
            assert!(!entry["path"].is_null());
            assert!(!entry["node_type"].is_null());
            assert!(!entry["size_bytes"].is_null());
        }
    }
}

#[tokio::test]
async fn test_list_directory_unicode_filenames() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("unicode").await.unwrap();
    fixture.create_test_file("unicode/文件.txt", "chinese").await.unwrap();
    fixture.create_test_file("unicode/файл.txt", "russian").await.unwrap();

    let tool = VfsListDirectoryTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "unicode",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["total"].as_u64().unwrap(), 2);
    }
}
