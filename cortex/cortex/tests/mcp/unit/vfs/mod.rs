//! Unit Tests for VFS MCP Tools
//!
//! This module contains comprehensive unit tests for all VFS MCP tools:
//! - cortex.vfs.get_node
//! - cortex.vfs.list_directory
//! - cortex.vfs.create_file
//! - cortex.vfs.update_file
//! - cortex.vfs.delete_node
//! - cortex.vfs.move_node
//! - cortex.vfs.copy_node
//!
//! Each test module covers:
//! - Basic operations
//! - Concurrent operations
//! - Error handling (not found, permission denied, version conflicts)
//! - Edge cases (large files, special characters, nested directories)
//! - Content deduplication
//! - Reference counting

mod test_get_node;
mod test_list_directory;
mod test_create_file;
mod test_update_file;
mod test_delete_node;
mod test_move_node;
mod test_copy_node;

// Re-export test helpers
pub use test_helpers::*;

/// Common test helpers and fixtures
mod test_helpers {
    use cortex_mcp::tools::vfs::VfsContext;
    use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig};
    use cortex_vfs::VirtualFileSystem;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

    /// Test fixture for VFS testing
    pub struct VfsTestFixture {
        pub vfs: Arc<VirtualFileSystem>,
        pub storage: Arc<ConnectionManager>,
        pub workspace_id: Uuid,
        pub ctx: VfsContext,
    }

    impl VfsTestFixture {
        /// Create a new test fixture with in-memory database
        pub async fn new() -> Self {
            use cortex_storage::connection_pool::{ConnectionMode, PoolConfig};

            let database_config = DatabaseConfig {
                connection_mode: ConnectionMode::Local {
                    endpoint: "mem://".to_string(),
                },
                credentials: Credentials {
                    username: Some("root".to_string()),
                    password: Some("root".to_string()),
                },
                pool_config: PoolConfig::default(),
                namespace: format!("test_{}", Uuid::new_v4().to_string().replace("-", "")),
                database: "cortex_vfs_test".to_string(),
            };

            let storage = Arc::new(
                ConnectionManager::new(database_config)
                    .await
                    .expect("Failed to create test storage"),
            );

            let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
            let workspace_id = Uuid::new_v4();
            let ctx = VfsContext::new(vfs.clone());

            Self {
                vfs,
                storage,
                workspace_id,
                ctx,
            }
        }

        /// Helper to execute a tool and measure performance
        pub async fn execute_tool(
            &self,
            tool: &dyn Tool,
            input: serde_json::Value,
        ) -> (Result<ToolResult, ToolError>, u128) {
            let start = Instant::now();
            let result = tool.execute(input, &ToolContext::default()).await;
            let duration = start.elapsed().as_millis();
            (result, duration)
        }

        /// Create a test file with given path and content
        pub async fn create_test_file(&self, path: &str, content: &str) -> Result<serde_json::Value, String> {
            use cortex_mcp::tools::vfs::VfsCreateFileTool;

            let tool = VfsCreateFileTool::new(self.ctx.clone());
            let input = json!({
                "path": path,
                "content": content,
                "workspace_id": self.workspace_id.to_string(),
            });

            let (result, _) = self.execute_tool(&tool, input).await;
            result
                .map(|r| {
                    if let ToolResult::Success { content } = r {
                        content
                    } else {
                        json!(null)
                    }
                })
                .map_err(|e| format!("{:?}", e))
        }

        /// Create a test directory
        pub async fn create_test_directory(&self, path: &str) -> Result<serde_json::Value, String> {
            use cortex_mcp::tools::vfs::VfsCreateDirectoryTool;

            let tool = VfsCreateDirectoryTool::new(self.ctx.clone());
            let input = json!({
                "path": path,
                "workspace_id": self.workspace_id.to_string(),
            });

            let (result, _) = self.execute_tool(&tool, input).await;
            result
                .map(|r| {
                    if let ToolResult::Success { content } = r {
                        content
                    } else {
                        json!(null)
                    }
                })
                .map_err(|e| format!("{:?}", e))
        }
    }

    /// Sample file content fixtures
    pub mod fixtures {
        /// Small text file
        pub const SMALL_TEXT: &str = "Hello, VFS!";

        /// Medium text file (1KB)
        pub fn medium_text() -> String {
            "Lorem ipsum ".repeat(100)
        }

        /// Large text file (1MB)
        pub fn large_text() -> String {
            "X".repeat(1024 * 1024)
        }

        /// Very large text file (10MB)
        pub fn very_large_text() -> String {
            "Y".repeat(10 * 1024 * 1024)
        }

        /// Unicode content
        pub const UNICODE_TEXT: &str = "Hello 世界! Привет мир! مرحبا 🌍";

        /// JSON content
        pub const JSON_CONTENT: &str = r#"{"name": "test", "value": 42}"#;

        /// Code content
        pub const RUST_CODE: &str = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    }
}
