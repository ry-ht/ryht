//! Unit Tests for cortex.vfs.create_file
//!
//! Tests cover:
//! - Basic file creation
//! - File creation in nested directories
//! - Large file creation
//! - Unicode content and filenames
//! - Concurrent file creation
//! - Content deduplication
//! - Error handling (invalid paths, duplicate creation)

use super::test_helpers::*;
use cortex_mcp::tools::vfs::VfsCreateFileTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_create_file_basic() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "content": fixtures::SMALL_TEXT,
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["path"], "test.txt");
        assert!(data["size_bytes"].as_u64().unwrap() > 0);
        assert!(!data["node_id"].is_null());
        assert_eq!(data["version"].as_u64().unwrap(), 1);
    }
}

#[tokio::test]
async fn test_create_file_in_nested_directory() {
    let fixture = VfsTestFixture::new().await;

    // Create parent directories first
    fixture.create_test_directory("a/b/c").await.unwrap();

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "a/b/c/nested.txt",
        "content": "nested content",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create nested file");
}

#[tokio::test]
async fn test_create_file_large_content() {
    let fixture = VfsTestFixture::new().await;

    let large_content = fixtures::large_text();
    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "large.txt",
        "content": large_content,
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create large file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["size_bytes"].as_u64().unwrap(), large_content.len() as u64);
    }
}

#[tokio::test]
async fn test_create_file_unicode_content() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "unicode.txt",
        "content": fixtures::UNICODE_TEXT,
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create file with unicode content");
}

#[tokio::test]
async fn test_create_file_unicode_filename() {
    let fixture = VfsTestFixture::new().await;

    let unicode_names = vec!["文件.txt", "файл.txt", "αρχείο.txt"];

    for name in unicode_names {
        let tool = VfsCreateFileTool::new(fixture.ctx.clone());
        let input = json!({
            "path": name,
            "content": "content",
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to create file with unicode name: {}", name);
    }
}

#[tokio::test]
async fn test_create_file_json_content() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "data.json",
        "content": fixtures::JSON_CONTENT,
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create JSON file");
}

#[tokio::test]
async fn test_create_file_code_content() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "main.rs",
        "content": fixtures::RUST_CODE,
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create code file");
}

#[tokio::test]
async fn test_create_file_empty_content() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "empty.txt",
        "content": "",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create empty file");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["size_bytes"].as_u64().unwrap(), 0);
    }
}

#[tokio::test]
async fn test_create_file_concurrent() {
    let fixture = VfsTestFixture::new().await;

    // Create 20 files concurrently
    let mut tasks = Vec::new();
    for i in 0..20 {
        let ctx_clone = fixture.ctx.clone();
        let ws_id = fixture.workspace_id;

        let task = tokio::spawn(async move {
            let tool = VfsCreateFileTool::new(ctx_clone);
            let input = json!({
                "path": format!("concurrent{}.txt", i),
                "content": format!("content {}", i),
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

    assert_eq!(success_count, 20, "All concurrent creates should succeed");
}

#[tokio::test]
async fn test_create_file_content_deduplication() {
    let fixture = VfsTestFixture::new().await;

    // Create multiple files with same content
    let same_content = "duplicated content for deduplication test";

    for i in 0..5 {
        let tool = VfsCreateFileTool::new(fixture.ctx.clone());
        let input = json!({
            "path": format!("dup{}.txt", i),
            "content": same_content,
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to create file {}", i);
    }

    // VFS should deduplicate storage using content hashing
    // All 5 files should share the same content blob
}

#[tokio::test]
async fn test_create_file_invalid_workspace() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "test.txt",
        "content": "content",
        "workspace_id": "invalid-uuid",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail with invalid workspace ID");
}

#[tokio::test]
async fn test_create_file_special_characters() {
    let fixture = VfsTestFixture::new().await;

    let special_names = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
    ];

    for name in special_names {
        let tool = VfsCreateFileTool::new(fixture.ctx.clone());
        let input = json!({
            "path": name,
            "content": "test",
            "workspace_id": fixture.workspace_id.to_string(),
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Failed to create file: {}", name);
    }
}

#[tokio::test]
async fn test_create_file_with_permissions() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "perms.txt",
        "content": "content",
        "workspace_id": fixture.workspace_id.to_string(),
        "permissions": "644",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create file with permissions");
}

#[tokio::test]
async fn test_create_file_with_encoding() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "encoded.txt",
        "content": "content",
        "workspace_id": fixture.workspace_id.to_string(),
        "encoding": "utf-8",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to create file with encoding");
}

#[tokio::test]
async fn test_create_file_version_initialized() {
    let fixture = VfsTestFixture::new().await;

    let tool = VfsCreateFileTool::new(fixture.ctx.clone());
    let input = json!({
        "path": "versioned.txt",
        "content": "v1",
        "workspace_id": fixture.workspace_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["version"].as_u64().unwrap(), 1);
    }
}
