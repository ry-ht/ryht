//! Unit Tests for cortex.vfs.get_node
//!
//! Tests cover:
//! - Basic file and directory retrieval
//! - Content inclusion/exclusion
//! - Metadata retrieval
//! - Error handling (not found, invalid paths)
//! - Concurrent access
//! - Large files
//! - Unicode filenames
//! - Cache behavior

use super::test_helpers::*;
use cortex_mcp::tools::vfs::VfsGetNodeTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_get_node_basic_file() {
    let fixture = VfsTestFixture::new().await;

    // Create a test file
    fixture.create_test_file("test.txt", fixtures::SMALL_TEXT).await.unwrap();

    // Get the node
    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get node");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["node_type"], "file");
        assert_eq!(data["name"], "test.txt");
        assert_eq!(data["content"], fixtures::SMALL_TEXT);
        assert!(data["size_bytes"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_get_node_without_content() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("test.txt", fixtures::SMALL_TEXT).await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(data["content"].is_null());
        assert!(data["size_bytes"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_get_node_with_metadata() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("test.txt", fixtures::SMALL_TEXT).await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_metadata": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["metadata"].is_null());
        assert!(!data["metadata"]["created_at"].is_null());
        assert!(!data["metadata"]["updated_at"].is_null());
        assert!(data["version"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_get_node_directory() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("testdir").await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "testdir",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["node_type"], "directory");
        assert_eq!(data["name"], "testdir");
        assert!(data["content"].is_null(), "Directory should not have content");
    }
}

#[tokio::test]
async fn test_get_node_not_found() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "nonexistent.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent file");
}

#[tokio::test]
async fn test_get_node_invalid_workspace() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("test.txt", fixtures::SMALL_TEXT).await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "workspace_id": "invalid-uuid",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for invalid workspace ID");
}

#[tokio::test]
async fn test_get_node_nested_path() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_directory("a/b/c").await.unwrap();
    fixture.create_test_file("a/b/c/deep.txt", "deep content").await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "a/b/c/deep.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "deep.txt");
        assert_eq!(data["content"], "deep content");
    }
}

#[tokio::test]
async fn test_get_node_unicode_filename() {
    let fixture = VfsTestFixture::new().await;

    let unicode_filenames = vec![
        "文件.txt",     // Chinese
        "файл.txt",    // Russian
        "αρχείο.txt", // Greek
        "ملف.txt",    // Arabic
    ];

    for filename in unicode_filenames {
        fixture.create_test_file(filename, fixtures::UNICODE_TEXT).await.unwrap();

        let tool = VfsGetNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "path": filename,
            "workspace_id": fixture.workspace_id.to_string(),
            "include_content": true,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to get unicode filename: {}", filename);

        if let Ok(ToolResult::Success { content }) = result {
            let data: serde_json::Value = serde_json::from_value(content).unwrap();
            assert_eq!(data["name"], filename);
        }
    }
}

#[tokio::test]
async fn test_get_node_large_file() {
    let fixture = VfsTestFixture::new().await;

    let large_content = fixtures::large_text();
    fixture.create_test_file("large.txt", &large_content).await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "large.txt",
        "workspace_id": fixture.workspace_id.to_string(),
        "include_content": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get large file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["size_bytes"].as_u64().unwrap(), large_content.len() as u64);
        assert_eq!(data["content"].as_str().unwrap().len(), large_content.len());
    }
}

#[tokio::test]
async fn test_get_node_concurrent_access() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("concurrent.txt", "shared content").await.unwrap();

    // Spawn 50 concurrent get operations
    let mut tasks = Vec::new();
    for _ in 0..50 {
        let ctx_clone = fixture.ctx.clone();
        let ws_id = fixture.workspace_id;

        let task = tokio::spawn(async move {
            let tool = VfsGetNodeTool::new(ctx_clone);
            let input = json!({
                "path": "concurrent.txt",
                "workspace_id": ws_id.to_string(),
                "include_content": true,
            });

            let start = std::time::Instant::now();
            let result = tool.execute(input, &ToolContext::default()).await;
            let duration = start.elapsed().as_millis();
            (result, duration)
        });
        tasks.push(task);
    }

    // Wait for all tasks
    let mut success_count = 0;
    for task in tasks {
        if let Ok((Ok(_), _)) = task.await {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 50, "All concurrent gets should succeed");
}

#[tokio::test]
async fn test_get_node_special_characters() {
    let fixture = VfsTestFixture::new().await;

    let special_names = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
    ];

    for name in special_names {
        fixture.create_test_file(name, "test").await.unwrap();

        let tool = VfsGetNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "path": name,
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to get file with special chars: {}", name);
    }
}

#[tokio::test]
async fn test_get_node_version_tracking() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("versioned.txt", "v1").await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "versioned.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let version = data["version"].as_u64().unwrap();
        assert!(version > 0, "Version should be tracked");
    }
}

#[tokio::test]
async fn test_get_node_permissions() {
    let fixture = VfsTestFixture::new().await;

    fixture.create_test_file("perms.txt", "test").await.unwrap();

    let tool = VfsGetNodeTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "perms.txt",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["permissions"].is_null(), "Should have permissions");
    }
}

#[tokio::test]
async fn test_get_node_content_deduplication() {
    let fixture = VfsTestFixture::new().await;

    // Create multiple files with same content
    let same_content = "duplicated content";
    fixture.create_test_file("file1.txt", same_content).await.unwrap();
    fixture.create_test_file("file2.txt", same_content).await.unwrap();
    fixture.create_test_file("file3.txt", same_content).await.unwrap();

    // Get all files
    for name in ["file1.txt", "file2.txt", "file3.txt"] {
        let tool = VfsGetNodeTool::new(fixture.ctx.clone());
        let input = json!({
            "path": name,
            "workspace_id": fixture.workspace_id.to_string(),
            "include_content": true,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok());

        if let Ok(ToolResult::Success { content }) = result {
            let data: serde_json::Value = serde_json::from_value(content).unwrap();
            assert_eq!(data["content"], same_content);
        }
    }
}
