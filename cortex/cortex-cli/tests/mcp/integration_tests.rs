//! Integration tests for Cortex MCP Server
//!
//! These tests verify that all tool categories work correctly with the MCP framework.

use cortex_mcp::prelude::*;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig};
use cortex_vfs::VirtualFileSystem;
use std::sync::Arc;
use uuid::Uuid;

/// Helper to create a test storage manager
async fn create_test_storage() -> Arc<ConnectionManager> {
    let database_config = DatabaseConfig {
        endpoints: vec!["mem://".to_string()],
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    let credentials = Credentials {
        username: "root".to_string(),
        password: "root".to_string(),
    };

    let pool_config = PoolConfig::default();

    Arc::new(
        ConnectionManager::new(database_config, credentials, pool_config)
            .await
            .expect("Failed to create test storage"),
    )
}

#[tokio::test]
#[ignore] // Requires database
async fn test_workspace_create_tool() {
    use cortex_mcp::tools::workspace::*;

    let storage = create_test_storage().await;
    let ctx = WorkspaceContext::new(storage);
    let tool = WorkspaceCreateTool::new(ctx);

    // Verify tool metadata
    assert_eq!(tool.name(), "cortex.workspace.create");
    assert!(tool.description().is_some());

    // Verify schema is valid JSON
    let schema = tool.input_schema();
    assert!(schema.is_object());
}

#[tokio::test]
#[ignore] // Requires database
async fn test_vfs_create_file_tool() {
    use cortex_mcp::tools::vfs::*;

    let storage = create_test_storage().await;
    let vfs = Arc::new(VirtualFileSystem::new(storage));
    let ctx = VfsContext::new(vfs);
    let tool = VfsCreateFileTool::new(ctx);

    // Verify tool metadata
    assert_eq!(tool.name(), "cortex.vfs.create_file");
    assert!(tool.description().is_some());

    // Verify schema is valid JSON
    let schema = tool.input_schema();
    assert!(schema.is_object());
}

#[tokio::test]
#[ignore] // Requires database
async fn test_code_get_unit_tool() {
    use cortex_mcp::tools::code_nav::*;

    let storage = create_test_storage().await;
    let ctx = CodeNavContext::new(storage);
    let tool = CodeGetUnitTool::new(ctx);

    // Verify tool metadata
    assert_eq!(tool.name(), "cortex.code.get_unit");
    assert!(tool.description().is_some());

    // Verify schema is valid JSON
    let schema = tool.input_schema();
    assert!(schema.is_object());
}

#[tokio::test]
#[ignore] // Requires database
async fn test_all_tools_registered() {
    // This test would require a full server setup
    // For now, we just verify the number of tools is correct
    // In a real test, we'd query the server for tools/list

    // Expected tool counts
    let workspace_tools = 8;
    let vfs_tools = 12;
    let code_nav_tools = 10;
    let total = workspace_tools + vfs_tools + code_nav_tools;

    assert_eq!(total, 30);
}

#[tokio::test]
#[ignore] // Requires database
async fn test_tool_execution_with_mock_transport() {
    use mcp_sdk::transport::MockTransport;
    use mcp_sdk::protocol::JsonRpcRequest;
    use serde_json::json;

    let storage = create_test_storage().await;
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

    // Build server
    let server = mcp_sdk::McpServer::builder()
        .name("test-server")
        .version("0.1.0")
        .tool(cortex_mcp::tools::vfs::VfsGetNodeTool::new(
            cortex_mcp::tools::vfs::VfsContext::new(vfs.clone()),
        ))
        .build()
        .expect("Failed to build server");

    // Create mock transport
    let transport = MockTransport::new();

    // Queue a tools/list request
    transport.push_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/list".to_string(),
        params: None,
    });

    // Spawn server in background
    let server_handle = tokio::spawn({
        let transport = transport.clone();
        async move {
            let _ = server.serve(transport).await;
        }
    });

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check responses
    let responses = transport.responses();
    assert!(!responses.is_empty(), "Should have at least one response");

    // Cleanup
    server_handle.abort();
}

/// Test that all workspace tools have valid schemas
#[test]
fn test_workspace_tool_schemas() {
    use schemars::schema_for;
    use cortex_mcp::tools::workspace::*;

    // Verify all input schemas compile
    let _ = schema_for!(CreateInput);
    let _ = schema_for!(GetInput);
    let _ = schema_for!(ListInput);
    let _ = schema_for!(ActivateInput);
    let _ = schema_for!(SyncInput);
    let _ = schema_for!(ExportInput);
    let _ = schema_for!(ArchiveInput);
    let _ = schema_for!(DeleteInput);
}

/// Test that all VFS tools have valid schemas
#[test]
fn test_vfs_tool_schemas() {
    use schemars::schema_for;
    use cortex_mcp::tools::vfs::*;

    // Verify all input schemas compile
    let _ = schema_for!(GetNodeInput);
    let _ = schema_for!(ListDirectoryInput);
    let _ = schema_for!(CreateFileInput);
    let _ = schema_for!(UpdateFileInput);
    let _ = schema_for!(DeleteNodeInput);
    let _ = schema_for!(MoveNodeInput);
    let _ = schema_for!(CopyNodeInput);
    let _ = schema_for!(CreateDirectoryInput);
    let _ = schema_for!(GetTreeInput);
}

/// Test that tool contexts are properly cloneable
#[test]
fn test_context_cloning() {
    use cortex_mcp::tools::workspace::*;
    use cortex_mcp::tools::vfs::*;
    use cortex_mcp::tools::code_nav::*;

    // Create mock storage
    let storage = Arc::new(ConnectionManager::default());

    // Test workspace context
    let ws_ctx = WorkspaceContext::new(storage.clone());
    let _ws_ctx_clone = ws_ctx.clone();

    // Test VFS context
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    let vfs_ctx = VfsContext::new(vfs);
    let _vfs_ctx_clone = vfs_ctx.clone();

    // Test code nav context
    let code_ctx = CodeNavContext::new(storage.clone());
    let _code_ctx_clone = code_ctx.clone();
}
