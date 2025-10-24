//! Virtual Filesystem Tools
//!
//! This module implements the 12 VFS tools defined in the MCP spec:
//! - cortex.vfs.get_node
//! - cortex.vfs.list_directory
//! - cortex.vfs.create_file
//! - cortex.vfs.update_file
//! - cortex.vfs.delete_node
//! - cortex.vfs.move_node
//! - cortex.vfs.copy_node
//! - cortex.vfs.create_directory
//! - cortex.vfs.get_tree
//! - cortex.vfs.search_files
//! - cortex.vfs.get_file_history
//! - cortex.vfs.restore_file_version

use async_trait::async_trait;
use cortex_vfs::{NodeType, VirtualFileSystem, VirtualPath};
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

// Import VFS service
use crate::services::VfsService;

// =============================================================================
// Shared Context
// =============================================================================

#[derive(Clone)]
pub struct VfsContext {
    vfs: Arc<VirtualFileSystem>,
    vfs_service: Arc<VfsService>,
}

impl VfsContext {
    pub fn new(vfs: Arc<VirtualFileSystem>) -> Self {
        let vfs_service = Arc::new(VfsService::new(vfs.clone()));
        Self { vfs, vfs_service }
    }
}

// =============================================================================
// cortex.vfs.get_node
// =============================================================================

pub struct VfsGetNodeTool {
    ctx: VfsContext,
}

impl VfsGetNodeTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetNodeInput {
    path: String,
    workspace_id: Option<String>,
    #[serde(default = "default_true")]
    include_content: bool,
    #[serde(default)]
    include_metadata: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetNodeOutput {
    node_id: String,
    node_type: String,
    name: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    size_bytes: u64,
    permissions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    version: u64,
}

#[async_trait]
impl Tool for VfsGetNodeTool {
    fn name(&self) -> &str {
        "cortex.vfs.get_node"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieves a virtual node (file or directory)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetNodeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetNodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        debug!("Getting node: {} in workspace {}", path, workspace_id);

        // Use VFS service to get metadata
        let details = self
            .ctx
            .vfs_service
            .get_metadata(&workspace_id, input.path.as_str())
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get node: {}", e)))?;

        let content = if input.include_content && details.node_type == "file" {
            match self.ctx.vfs_service.read_file(&workspace_id, input.path.as_str()).await {
                Ok(bytes) => Some(String::from_utf8_lossy(&bytes).to_string()),
                Err(_) => None,
            }
        } else {
            None
        };

        // Convert service response to MCP output
        let node = self.ctx.vfs.metadata(&workspace_id, &path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get node: {}", e)))?;

        let output = GetNodeOutput {
            node_id: node.id.to_string(),
            node_type: match node.node_type {
                NodeType::File => "file",
                NodeType::Directory => "directory",
                NodeType::SymLink => "symlink",
                NodeType::Document => "document",
            }
            .to_string(),
            name: node.path.file_name().unwrap_or("").to_string(),
            path: node.path.to_string(),
            content,
            size_bytes: node.size_bytes as u64,
            permissions: node.permissions
                .map(|p| format!("{:o}", p))
                .unwrap_or_else(|| "644".to_string()),
            metadata: if input.include_metadata {
                Some(serde_json::json!({
                    "created_at": node.created_at,
                    "updated_at": node.updated_at,
                }))
            } else {
                None
            },
            version: node.version as u64,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.list_directory
// =============================================================================

pub struct VfsListDirectoryTool {
    ctx: VfsContext,
}

impl VfsListDirectoryTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListDirectoryInput {
    path: String,
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(default)]
    recursive: bool,
    #[serde(default)]
    include_hidden: bool,
    #[serde(default)]
    filter: Option<ListFilter>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilter {
    node_type: Option<String>,
    #[allow(dead_code)]
    language: Option<String>,
    #[allow(dead_code)]
    pattern: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ListDirectoryOutput {
    entries: Vec<DirectoryEntry>,
    total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DirectoryEntry {
    name: String,
    path: String,
    node_type: String,
    size_bytes: u64,
}

#[async_trait]
impl Tool for VfsListDirectoryTool {
    fn name(&self) -> &str {
        "cortex.vfs.list_directory"
    }

    fn description(&self) -> Option<&str> {
        Some("Lists contents of a virtual directory")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ListDirectoryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ListDirectoryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        debug!("Listing directory: {} in workspace {}", path, workspace_id);

        let nodes = self
            .ctx
            .vfs
            .list_directory(&workspace_id, &path, input.recursive)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

        let entries: Vec<DirectoryEntry> = nodes
            .into_iter()
            .filter(|node| {
                // Apply filters
                if !input.include_hidden && node.path.to_string().contains("/.") {
                    return false;
                }

                if let Some(ref filter) = input.filter {
                    if let Some(ref node_type_filter) = filter.node_type {
                        let matches = match node_type_filter.as_str() {
                            "file" => node.is_file(),
                            "directory" => node.is_directory(),
                            _ => true,
                        };
                        if !matches {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|node| DirectoryEntry {
                name: node.path.file_name().unwrap_or("").to_string(),
                path: node.path.to_string(),
                node_type: match node.node_type {
                    NodeType::File => "file",
                    NodeType::Directory => "directory",
                    NodeType::SymLink => "symlink",
                    NodeType::Document => "document",
                }
                .to_string(),
                size_bytes: node.size_bytes as u64,
            })
            .collect();

        let total = entries.len();

        let output = ListDirectoryOutput { entries, total };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.create_file
// =============================================================================

pub struct VfsCreateFileTool {
    ctx: VfsContext,
}

impl VfsCreateFileTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateFileInput {
    path: String,
    content: String,
    workspace_id: Option<String>,
    #[serde(default = "default_encoding")]
    #[allow(dead_code)]
    encoding: String,
    #[serde(default = "default_permissions")]
    #[allow(dead_code)]
    permissions: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    parse: bool,
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

fn default_permissions() -> String {
    "644".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateFileOutput {
    node_id: String,
    path: String,
    size_bytes: u64,
    version: u64,
}

#[async_trait]
impl Tool for VfsCreateFileTool {
    fn name(&self) -> &str {
        "cortex.vfs.create_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new file in the virtual filesystem")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CreateFileInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateFileInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        info!("Creating file: {} in workspace {}", path, workspace_id);

        let content_bytes = input.content.as_bytes();
        self.ctx
            .vfs
            .write_file(&workspace_id, &path, content_bytes)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create file: {}", e)))?;

        let node = self
            .ctx
            .vfs
            .metadata(&workspace_id, &path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get created node: {}", e)))?;

        let output = CreateFileOutput {
            node_id: node.id.to_string(),
            path: path.to_string(),
            size_bytes: node.size_bytes as u64,
            version: node.version as u64,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.update_file
// =============================================================================

pub struct VfsUpdateFileTool {
    ctx: VfsContext,
}

impl VfsUpdateFileTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateFileInput {
    path: String,
    content: String,
    #[allow(dead_code)]
    expected_version: u64,
    workspace_id: Option<String>,
    #[serde(default = "default_encoding")]
    #[allow(dead_code)]
    encoding: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    reparse: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UpdateFileOutput {
    node_id: String,
    path: String,
    version: u64,
    size_bytes: u64,
}

#[async_trait]
impl Tool for VfsUpdateFileTool {
    fn name(&self) -> &str {
        "cortex.vfs.update_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Updates file content with automatic parsing")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(UpdateFileInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: UpdateFileInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        // Check expected_version for optimistic locking
        let current_node = self.ctx.vfs.metadata(&workspace_id, &path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current node: {}", e)))?;

        if current_node.version as u64 != input.expected_version {
            return Err(ToolError::ExecutionFailed(format!(
                "Version mismatch: expected version {}, but current version is {}. File was modified by another process.",
                input.expected_version, current_node.version
            )));
        }

        info!("Updating file: {} in workspace {}", path, workspace_id);

        let content_bytes = input.content.as_bytes();
        self.ctx
            .vfs
            .write_file(&workspace_id, &path, content_bytes)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update file: {}", e)))?;

        let node = self
            .ctx
            .vfs
            .metadata(&workspace_id, &path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get updated node: {}", e)))?;

        let output = UpdateFileOutput {
            node_id: node.id.to_string(),
            path: path.to_string(),
            version: node.version as u64,
            size_bytes: node.size_bytes as u64,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.delete_node
// =============================================================================

pub struct VfsDeleteNodeTool {
    ctx: VfsContext,
}

impl VfsDeleteNodeTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteNodeInput {
    path: String,
    workspace_id: Option<String>,
    #[serde(default)]
    recursive: bool,
    #[allow(dead_code)]
    expected_version: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeleteNodeOutput {
    path: String,
    deleted: bool,
}

#[async_trait]
impl Tool for VfsDeleteNodeTool {
    fn name(&self) -> &str {
        "cortex.vfs.delete_node"
    }

    fn description(&self) -> Option<&str> {
        Some("Deletes a file or directory")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(DeleteNodeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: DeleteNodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        info!("Deleting node: {} in workspace {}", path, workspace_id);

        self.ctx
            .vfs
            .delete(&workspace_id, &path, input.recursive)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete node: {}", e)))?;

        let output = DeleteNodeOutput {
            path: path.to_string(),
            deleted: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.move_node
// =============================================================================

pub struct VfsMoveNodeTool {
    ctx: VfsContext,
}

impl VfsMoveNodeTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MoveNodeInput {
    source_path: String,
    target_path: String,
    workspace_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    overwrite: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct MoveNodeOutput {
    source_path: String,
    target_path: String,
    moved: bool,
}

#[async_trait]
impl Tool for VfsMoveNodeTool {
    fn name(&self) -> &str {
        "cortex.vfs.move_node"
    }

    fn description(&self) -> Option<&str> {
        Some("Moves or renames a node")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(MoveNodeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: MoveNodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let source_path = VirtualPath::new(&input.source_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid source path: {}", e)))?;
        let target_path = VirtualPath::new(&input.target_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid target path: {}", e)))?;

        info!(
            "Moving node: {} -> {} in workspace {}",
            source_path, target_path, workspace_id
        );

        // VFS doesn't have move_node yet, so we implement it as copy+delete
        let content = self.ctx.vfs.read_file(&workspace_id, &source_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read source: {}", e)))?;

        self.ctx.vfs.write_file(&workspace_id, &target_path, &content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write target: {}", e)))?;

        self.ctx.vfs.delete(&workspace_id, &source_path, false).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete source: {}", e)))?;

        let output = MoveNodeOutput {
            source_path: source_path.to_string(),
            target_path: target_path.to_string(),
            moved: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.copy_node
// =============================================================================

pub struct VfsCopyNodeTool {
    ctx: VfsContext,
}

impl VfsCopyNodeTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CopyNodeInput {
    source_path: String,
    target_path: String,
    workspace_id: Option<String>,
    #[serde(default = "default_true")]
    recursive: bool,
    #[serde(default)]
    overwrite: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CopyNodeOutput {
    source_path: String,
    target_path: String,
    copied: bool,
}

#[async_trait]
impl Tool for VfsCopyNodeTool {
    fn name(&self) -> &str {
        "cortex.vfs.copy_node"
    }

    fn description(&self) -> Option<&str> {
        Some("Copies a node to a new location")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CopyNodeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CopyNodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let source_path = VirtualPath::new(&input.source_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid source path: {}", e)))?;
        let target_path = VirtualPath::new(&input.target_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid target path: {}", e)))?;

        info!(
            "Copying node: {} -> {} in workspace {}",
            source_path, target_path, workspace_id
        );

        // Get source node
        let source_node = self.ctx.vfs.metadata(&workspace_id, &source_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get source node: {}", e)))?;

        // Check if target exists and handle overwrite
        if self.ctx.vfs.exists(&workspace_id, &target_path).await.unwrap_or(false) {
            if !input.overwrite {
                return Err(ToolError::ExecutionFailed(
                    "Target already exists (use overwrite=true)".to_string(),
                ));
            }
        }

        // Copy based on node type
        if source_node.is_file() {
            // Copy file content
            let content = self.ctx.vfs.read_file(&workspace_id, &source_path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read source file: {}", e)))?;

            self.ctx.vfs.write_file(&workspace_id, &target_path, &content).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write target file: {}", e)))?;
        } else if source_node.is_directory() {
            if !input.recursive {
                return Err(ToolError::ExecutionFailed(
                    "Cannot copy directory without recursive=true".to_string(),
                ));
            }

            // Create target directory
            self.ctx.vfs.create_directory(&workspace_id, &target_path, true).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create target directory: {}", e)))?;

            // Recursively copy contents
            let children = self.ctx.vfs.list_directory(&workspace_id, &source_path, true).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list source directory: {}", e)))?;

            for child in children {
                if child.is_file() {
                    // Get relative path from source
                    let child_path_str = child.path.to_string();
                    let source_path_str = source_path.to_string();
                    let rel_path = child_path_str
                        .strip_prefix(&source_path_str)
                        .unwrap_or("")
                        .trim_start_matches('/');

                    if !rel_path.is_empty() {
                        let child_target = target_path.join(rel_path)
                            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid target path: {}", e)))?;

                        // Copy file
                        let content = self.ctx.vfs.read_file(&workspace_id, &child.path).await
                            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read child file: {}", e)))?;

                        self.ctx.vfs.write_file(&workspace_id, &child_target, &content).await
                            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write child file: {}", e)))?;
                    }
                }
            }
        }

        let output = CopyNodeOutput {
            source_path: source_path.to_string(),
            target_path: target_path.to_string(),
            copied: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.create_directory
// =============================================================================

pub struct VfsCreateDirectoryTool {
    ctx: VfsContext,
}

impl VfsCreateDirectoryTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateDirectoryInput {
    path: String,
    workspace_id: Option<String>,
    #[serde(default = "default_permissions")]
    #[allow(dead_code)]
    permissions: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    create_parents: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateDirectoryOutput {
    node_id: String,
    path: String,
}

#[async_trait]
impl Tool for VfsCreateDirectoryTool {
    fn name(&self) -> &str {
        "cortex.vfs.create_directory"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new directory")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CreateDirectoryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateDirectoryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        info!("Creating directory: {} in workspace {}", path, workspace_id);

        self.ctx
            .vfs
            .create_directory(&workspace_id, &path, true)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create directory: {}", e)))?;

        let node = self
            .ctx
            .vfs
            .metadata(&workspace_id, &path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get created node: {}", e)))?;

        let output = CreateDirectoryOutput {
            node_id: node.id.to_string(),
            path: path.to_string(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.get_tree
// =============================================================================

pub struct VfsGetTreeTool {
    ctx: VfsContext,
}

impl VfsGetTreeTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetTreeInput {
    #[serde(default = "default_root")]
    path: String,
    workspace_id: Option<String>,
    #[serde(default = "default_max_depth")]
    max_depth: usize,
    #[serde(default = "default_true")]
    include_files: bool,
}

fn default_root() -> String {
    "/".to_string()
}

fn default_max_depth() -> usize {
    3
}

#[derive(Debug, Serialize, JsonSchema)]
struct TreeNode {
    name: String,
    path: String,
    node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetTreeOutput {
    root: TreeNode,
    total_nodes: usize,
}

#[async_trait]
impl Tool for VfsGetTreeTool {
    fn name(&self) -> &str {
        "cortex.vfs.get_tree"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets directory tree structure")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetTreeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetTreeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        debug!("Getting tree for: {} in workspace {}", path, workspace_id);

        // Get root node
        let root_node = self.ctx.vfs.metadata(&workspace_id, &path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get root node: {}", e)))?;

        let mut total_nodes = 0;
        let tree = self.build_tree_node(&workspace_id, &root_node, 0, input.max_depth, input.include_files, &mut total_nodes).await?;

        let output = GetTreeOutput {
            root: tree,
            total_nodes,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

impl VfsGetTreeTool {
    fn build_tree_node<'a>(
        &'a self,
        workspace_id: &'a Uuid,
        vnode: &'a cortex_vfs::VNode,
        current_depth: usize,
        max_depth: usize,
        include_files: bool,
        total_nodes: &'a mut usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<TreeNode, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            *total_nodes += 1;

            let mut tree_node = TreeNode {
                name: vnode.path.file_name().unwrap_or("/").to_string(),
                path: vnode.path.to_string(),
                node_type: match vnode.node_type {
                    NodeType::File => "file",
                    NodeType::Directory => "directory",
                    NodeType::SymLink => "symlink",
                    NodeType::Document => "document",
                }.to_string(),
                size_bytes: if vnode.is_file() { Some(vnode.size_bytes as u64) } else { None },
                children: None,
            };

            // If it's a directory and we haven't reached max depth, get children
            if vnode.is_directory() && current_depth < max_depth {
                let children_nodes = self.ctx.vfs.list_directory(workspace_id, &vnode.path, false).await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

                let mut children = Vec::new();
                for child in children_nodes {
                    // Filter files if requested
                    if !include_files && child.is_file() {
                        continue;
                    }

                    let child_tree = self.build_tree_node(
                        workspace_id,
                        &child,
                        current_depth + 1,
                        max_depth,
                        include_files,
                        total_nodes,
                    ).await?;
                    children.push(child_tree);
                }

                if !children.is_empty() {
                    tree_node.children = Some(children);
                }
            }

            Ok(tree_node)
        })
    }
}

// =============================================================================
// cortex.vfs.search_files
// =============================================================================

pub struct VfsSearchFilesTool {
    ctx: VfsContext,
}

impl VfsSearchFilesTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchFilesInput {
    pattern: String,
    workspace_id: Option<String>,
    #[serde(default = "default_root")]
    base_path: String,
    #[serde(default)]
    search_content: bool,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    max_results: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchMatch {
    path: String,
    node_type: String,
    size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchFilesOutput {
    matches: Vec<SearchMatch>,
    total: usize,
    truncated: bool,
}

#[async_trait]
impl Tool for VfsSearchFilesTool {
    fn name(&self) -> &str {
        "cortex.vfs.search_files"
    }

    fn description(&self) -> Option<&str> {
        Some("Searches for files by pattern or content")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SearchFilesInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchFilesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let base_path = VirtualPath::new(&input.base_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid base path: {}", e)))?;

        debug!("Searching files with pattern: {} in workspace {}", input.pattern, workspace_id);

        // List all files recursively from base path
        let all_nodes = self.ctx.vfs.list_directory(&workspace_id, &base_path, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

        let mut matches = Vec::new();
        let pattern_lower = input.pattern.to_lowercase();

        for node in all_nodes {
            let mut matched = false;
            let mut match_type = None;

            // Check filename pattern (glob-like matching)
            let filename = node.path.file_name().unwrap_or("");
            let _filename_to_check = if input.case_sensitive {
                filename.to_string()
            } else {
                filename.to_lowercase()
            };

            // Simple glob pattern matching
            if Self::glob_match(&input.pattern, filename, input.case_sensitive) {
                matched = true;
                match_type = Some("filename".to_string());
            }

            // Search content if requested and is a file
            if !matched && input.search_content && node.is_file() {
                if let Ok(content) = self.ctx.vfs.read_file(&workspace_id, &node.path).await {
                    let content_str = String::from_utf8_lossy(&content);
                    let content_to_check = if input.case_sensitive {
                        content_str.to_string()
                    } else {
                        content_str.to_lowercase()
                    };

                    if content_to_check.contains(&pattern_lower) {
                        matched = true;
                        match_type = Some("content".to_string());
                    }
                }
            }

            if matched {
                matches.push(SearchMatch {
                    path: node.path.to_string(),
                    node_type: match node.node_type {
                        NodeType::File => "file",
                        NodeType::Directory => "directory",
                        NodeType::SymLink => "symlink",
                        NodeType::Document => "document",
                    }.to_string(),
                    size_bytes: node.size_bytes as u64,
                    match_type,
                });

                // Check max results
                if let Some(max) = input.max_results {
                    if matches.len() >= max {
                        break;
                    }
                }
            }
        }

        let total = matches.len();
        let truncated = input.max_results.map(|max| total >= max).unwrap_or(false);

        let output = SearchFilesOutput {
            matches,
            total,
            truncated,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

impl VfsSearchFilesTool {
    /// Simple glob pattern matching (supports * and ?)
    fn glob_match(pattern: &str, text: &str, case_sensitive: bool) -> bool {
        let pattern = if case_sensitive { pattern.to_string() } else { pattern.to_lowercase() };
        let text = if case_sensitive { text.to_string() } else { text.to_lowercase() };

        // Simple wildcard matching
        if pattern.contains('*') {
            // Split by * and check each part
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut last_pos = 0;

            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                if i == 0 && !text.starts_with(part) {
                    return false;
                }

                if i == parts.len() - 1 && !text.ends_with(part) {
                    return false;
                }

                if let Some(pos) = text[last_pos..].find(part) {
                    last_pos += pos + part.len();
                } else {
                    return false;
                }
            }
            true
        } else if pattern.contains('?') {
            // Simple single character wildcard
            if pattern.len() != text.len() {
                return false;
            }
            pattern.chars().zip(text.chars()).all(|(p, t)| p == '?' || p == t)
        } else {
            // Exact match or contains
            text.contains(&pattern)
        }
    }
}

// =============================================================================
// cortex.vfs.get_file_history
// =============================================================================

pub struct VfsGetFileHistoryTool {
    ctx: VfsContext,
}

impl VfsGetFileHistoryTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetFileHistoryInput {
    path: String,
    workspace_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    max_versions: Option<usize>,
    #[serde(default)]
    include_content: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FileVersion {
    version: u32,
    content_hash: String,
    size_bytes: u64,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetFileHistoryOutput {
    path: String,
    current_version: u32,
    versions: Vec<FileVersion>,
    total_versions: usize,
}

#[async_trait]
impl Tool for VfsGetFileHistoryTool {
    fn name(&self) -> &str {
        "cortex.vfs.get_file_history"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieves version history of a file")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetFileHistoryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetFileHistoryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        debug!("Getting file history for: {} in workspace {}", path, workspace_id);

        // Get current file node
        let current_node = self.ctx.vfs.metadata(&workspace_id, &path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get file: {}", e)))?;

        if !current_node.is_file() {
            return Err(ToolError::ExecutionFailed("Not a file".to_string()));
        }

        // Query database for all versions of this file
        // Note: This is a simplified implementation. In production, you'd have a separate
        // version history table. For now, we'll return just the current version.
        let mut versions = Vec::new();

        let content = if input.include_content {
            self.ctx.vfs.read_file(&workspace_id, &path).await
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        } else {
            None
        };

        versions.push(FileVersion {
            version: current_node.version,
            content_hash: current_node.content_hash.unwrap_or_default(),
            size_bytes: current_node.size_bytes as u64,
            created_at: current_node.created_at.to_rfc3339(),
            updated_at: current_node.updated_at.to_rfc3339(),
            content,
        });

        let output = GetFileHistoryOutput {
            path: path.to_string(),
            current_version: current_node.version,
            versions,
            total_versions: 1, // In a full implementation, this would query the version count
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.vfs.restore_file_version
// =============================================================================

pub struct VfsRestoreFileVersionTool {
    ctx: VfsContext,
}

impl VfsRestoreFileVersionTool {
    pub fn new(ctx: VfsContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RestoreFileVersionInput {
    path: String,
    version: u32,
    workspace_id: Option<String>,
    #[serde(default)]
    create_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RestoreFileVersionOutput {
    path: String,
    restored_version: u32,
    new_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_path: Option<String>,
}

#[async_trait]
impl Tool for VfsRestoreFileVersionTool {
    fn name(&self) -> &str {
        "cortex.vfs.restore_file_version"
    }

    fn description(&self) -> Option<&str> {
        Some("Restores a file to a previous version")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(RestoreFileVersionInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: RestoreFileVersionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = input
            .workspace_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("workspace_id is required".to_string()))?;

        let path = VirtualPath::new(&input.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;

        debug!("Restoring file {} to version {} in workspace {}", path, input.version, workspace_id);

        // Get current file node
        let current_node = self.ctx.vfs.metadata(&workspace_id, &path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get file: {}", e)))?;

        if !current_node.is_file() {
            return Err(ToolError::ExecutionFailed("Not a file".to_string()));
        }

        // Check if requested version exists (simplified - in production, query version history)
        if input.version > current_node.version {
            return Err(ToolError::ExecutionFailed(
                format!("Version {} does not exist (current version is {})", input.version, current_node.version)
            ));
        }

        // Create backup if requested
        let backup_path = if input.create_backup {
            let backup_path_str = format!("{}.backup.v{}", path.to_string(), current_node.version);
            let backup_path = VirtualPath::new(&backup_path_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid backup path: {}", e)))?;

            // Copy current content to backup
            let current_content = self.ctx.vfs.read_file(&workspace_id, &path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read current file: {}", e)))?;

            self.ctx.vfs.write_file(&workspace_id, &backup_path, &current_content).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create backup: {}", e)))?;

            Some(backup_path.to_string())
        } else {
            None
        };

        // Note: In a simplified implementation, we can't actually restore to a previous version
        // because we don't have a version history table. In production, you would:
        // 1. Query the version history table for the content_hash at the requested version
        // 2. Write that content to the file
        //
        // For now, we'll return a message indicating the operation would succeed
        // but with the current implementation limitation noted.

        // If we're "restoring" to the current version, just return success
        if input.version == current_node.version {
            let output = RestoreFileVersionOutput {
                path: path.to_string(),
                restored_version: input.version,
                new_version: current_node.version,
                backup_path,
            };
            return Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()));
        }

        // In a full implementation with version history, we would:
        // 1. Look up the content_hash for version `input.version`
        // 2. Read that content from the database
        // 3. Write it to the file (which creates a new version)
        //
        // For now, return an informative error
        Err(ToolError::ExecutionFailed(
            format!(
                "Version history restore requires version tracking. Current implementation only supports current version ({}). \
                To enable full version history, a separate version_history table would need to be implemented.",
                current_node.version
            )
        ))
    }
}
