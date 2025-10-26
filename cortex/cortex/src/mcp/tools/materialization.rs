//! Materialization Tools (8 tools)

use async_trait::async_trait;
use cortex_core::error::Result as CortexResult;
use cortex_storage::ConnectionManager;
use cortex_vfs::{
    FlushOptions, FlushScope, MaterializationEngine, VirtualFileSystem,
    VirtualPath, VNode,
};
// FileEvent and WatcherConfig are public in cortex_vfs::watcher but not re-exported
use cortex_vfs::watcher::{FileEvent, FileWatcher, WatcherConfig};
use dashmap::DashMap;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Global watcher registry to track active watchers
static WATCHER_REGISTRY: once_cell::sync::Lazy<Arc<DashMap<String, WatcherHandle>>> =
    once_cell::sync::Lazy::new(|| Arc::new(DashMap::new()));

/// Handle for an active file watcher
struct WatcherHandle {
    watcher: Arc<Mutex<FileWatcher>>,
    paths: Vec<PathBuf>,
    auto_sync: bool,
    workspace_id: Uuid,
    vfs: Arc<VirtualFileSystem>,
}

#[derive(Clone)]
pub struct MaterializationContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    engine: Arc<MaterializationEngine>,
}

impl MaterializationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let engine = Arc::new(MaterializationEngine::new((*vfs).clone()));
        Self {
            storage,
            vfs,
            engine,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlushPreviewInput {
    scope_paths: Option<Vec<String>>,
    #[serde(default = "default_true")]
    include_diffs: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FlushPreviewOutput {
    changes: Vec<FileChange>,
    total_files: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FileChange {
    path: String,
    change_type: String,
    diff: Option<String>,
}

pub struct FlushPreviewTool {
    ctx: MaterializationContext,
}

impl FlushPreviewTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }

    async fn query_vnodes(&self, scope_paths: Option<Vec<String>>) -> std::result::Result<Vec<VNode>, ToolError> {
        let query = if let Some(paths) = scope_paths {
            if paths.is_empty() {
                "SELECT * FROM vnode WHERE status IN ['modified', 'created', 'deleted']".to_string()
            } else {
                let path_conditions = paths
                    .iter()
                    .map(|p| format!("path LIKE '{}%'", p.trim_start_matches('/')))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                format!(
                    "SELECT * FROM vnode WHERE status IN ['modified', 'created', 'deleted'] AND ({})",
                    path_conditions
                )
            }
        } else {
            "SELECT * FROM vnode WHERE status IN ['modified', 'created', 'deleted']".to_string()
        };

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Storage error: {}", e)))?;

        let mut response = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;

        let vnodes: Vec<VNode> = response.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Parse error: {}", e)))?;

        Ok(vnodes)
    }

    async fn compute_diff(&self, vnode: &VNode, target_path: &Path) -> Option<String> {
        let physical_path = target_path.join(vnode.path.to_string().trim_start_matches('/'));

        // Read VFS content
        let vfs_content = match self.ctx.vfs.read_file(&vnode.workspace_id, &vnode.path).await {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read VFS content: {}", e);
                return None;
            }
        };

        // Read physical file if it exists
        let physical_content = match fs::read(&physical_path).await {
            Ok(content) => content,
            Err(_) => return Some(format!("+++ NEW FILE ({})", vnode.path)),
        };

        // Simple diff
        if vfs_content == physical_content {
            None
        } else {
            let vfs_str = String::from_utf8_lossy(&vfs_content);
            let phys_str = String::from_utf8_lossy(&physical_content);
            Some(format!(
                "--- {}\n+++ {}\n@@ Lines differ: VFS={} bytes, Disk={} bytes @@",
                vnode.path,
                vnode.path,
                vfs_str.len(),
                phys_str.len()
            ))
        }
    }
}

#[async_trait]
impl Tool for FlushPreviewTool {
    fn name(&self) -> &str {
        "cortex.flush.preview"
    }

    fn description(&self) -> Option<&str> {
        Some("Preview changes to be flushed from VFS to filesystem")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FlushPreviewInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FlushPreviewInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Previewing flush for paths: {:?}", input.scope_paths);

        // Query vnodes
        let vnodes = self.query_vnodes(input.scope_paths).await?;
        let total_files = vnodes.len() as i32;

        // Get current directory for diffs
        let target_path = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;

        // Build changes
        let mut changes = Vec::new();
        for vnode in &vnodes {
            let change_type = match vnode.status {
                cortex_vfs::SyncStatus::Created => "created",
                cortex_vfs::SyncStatus::Modified => "modified",
                cortex_vfs::SyncStatus::Deleted => "deleted",
                _ => "unknown",
            };

            let diff = if input.include_diffs && change_type != "deleted" {
                self.compute_diff(vnode, &target_path).await
            } else {
                None
            };

            changes.push(FileChange {
                path: vnode.path.to_string(),
                change_type: change_type.to_string(),
                diff,
            });
        }

        let output = FlushPreviewOutput {
            changes,
            total_files,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlushExecuteInput {
    scope_paths: Option<Vec<String>>,
    #[serde(default = "default_true")]
    format_code: bool,
    #[serde(default = "default_true")]
    create_backup: bool,
    #[serde(default = "default_true")]
    atomic: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FlushExecuteOutput {
    files_written: i32,
    bytes_written: i64,
    duration_ms: i64,
}

pub struct FlushExecuteTool {
    ctx: MaterializationContext,
}

impl FlushExecuteTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for FlushExecuteTool {
    fn name(&self) -> &str {
        "cortex.flush.execute"
    }

    fn description(&self) -> Option<&str> {
        Some("Execute flush of VFS changes to filesystem")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FlushExecuteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FlushExecuteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Executing flush for paths: {:?}", input.scope_paths);

        let target_path = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;

        // Determine scope
        let scope = if let Some(ref paths) = input.scope_paths {
            if paths.len() == 1 {
                let vpath = VirtualPath::new(&paths[0])
                    .map_err(|e| ToolError::ExecutionFailed(format!("Invalid path: {}", e)))?;
                FlushScope::Path(vpath)
            } else {
                FlushScope::All
            }
        } else {
            FlushScope::All
        };

        // Configure options
        let options = FlushOptions {
            preserve_permissions: true,
            preserve_timestamps: true,
            create_backup: input.create_backup,
            atomic: input.atomic,
            parallel: true,
            max_workers: num_cpus::get(),
        };

        // Execute flush
        let report = self.ctx.engine.flush(scope, &target_path, options).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Flush failed: {}", e)))?;

        let output = FlushExecuteOutput {
            files_written: report.files_written as i32,
            bytes_written: report.bytes_written as i64,
            duration_ms: report.duration_ms as i64,
        };

        info!("Flush completed: {} files, {} bytes in {}ms",
            output.files_written, output.bytes_written, output.duration_ms);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlushSelectiveInput {
    entity_ids: Vec<String>,
    #[serde(default)]
    skip_dependencies: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FlushSelectiveOutput {
    files_written: i32,
    entity_ids: Vec<String>,
}

pub struct FlushSelectiveTool {
    ctx: MaterializationContext,
}

impl FlushSelectiveTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for FlushSelectiveTool {
    fn name(&self) -> &str {
        "cortex.flush.selective"
    }

    fn description(&self) -> Option<&str> {
        Some("Flush specific VFS entities by ID to filesystem")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FlushSelectiveInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FlushSelectiveInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Selective flush for {} entities", input.entity_ids.len());

        // Parse UUIDs
        let mut ids = Vec::new();
        for id_str in &input.entity_ids {
            let id = Uuid::parse_str(id_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid UUID {}: {}", id_str, e)))?;
            ids.push(id);
        }

        let target_path = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;

        let scope = FlushScope::Specific(ids);
        let options = FlushOptions::default();

        // Execute flush
        let report = self.ctx.engine.flush(scope, &target_path, options).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Selective flush failed: {}", e)))?;

        let output = FlushSelectiveOutput {
            files_written: report.files_written as i32,
            entity_ids: input.entity_ids,
        };

        info!("Selective flush completed: {} files", output.files_written);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncFromDiskInput {
    paths: Option<Vec<String>>,
    #[serde(default = "default_true")]
    detect_moves: bool,
    #[serde(default)]
    auto_merge: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SyncFromDiskOutput {
    files_synced: i32,
    conflicts: Vec<String>,
}

pub struct SyncFromDiskTool {
    ctx: MaterializationContext,
}

impl SyncFromDiskTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }

    async fn sync_file_from_disk(
        &self,
        workspace_id: &Uuid,
        physical_path: &Path,
        virtual_path: &VirtualPath,
    ) -> CortexResult<bool> {
        // Read file content from disk
        let content = fs::read(physical_path).await
            .map_err(|e| cortex_core::error::CortexError::vfs(format!("Failed to read {}: {}", physical_path.display(), e)))?;

        // Check if it exists in VFS
        let vnode_opt = self.ctx.vfs.get_vnode(workspace_id, virtual_path).await?;

        if let Some(vnode) = vnode_opt {
            // Check for conflicts (both modified)
            if vnode.status == cortex_vfs::SyncStatus::Modified {
                // Both VFS and disk modified - conflict!
                return Ok(false);
            }

            // Update VFS with disk content
            self.ctx.vfs.write_file(workspace_id, virtual_path, &content).await?;
        } else {
            // New file on disk, add to VFS
            self.ctx.vfs.write_file(workspace_id, virtual_path, &content).await?;
        }

        Ok(true)
    }

    async fn scan_directory(&self, base_path: &Path) -> CortexResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![base_path.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut entries = fs::read_dir(&dir).await
                .map_err(|e| cortex_core::error::CortexError::vfs(format!("Failed to read dir: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| cortex_core::error::CortexError::vfs(format!("Failed to read entry: {}", e)))? {
                let path = entry.path();
                let metadata = entry.metadata().await
                    .map_err(|e| cortex_core::error::CortexError::vfs(format!("Failed to get metadata: {}", e)))?;

                if metadata.is_dir() {
                    // Skip common ignored directories
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str == "node_modules" || name_str == "target"
                            || name_str == ".git" || name_str == "dist" {
                            continue;
                        }
                    }
                    stack.push(path);
                } else if metadata.is_file() {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }
}

#[async_trait]
impl Tool for SyncFromDiskTool {
    fn name(&self) -> &str {
        "cortex.sync.from_disk"
    }

    fn description(&self) -> Option<&str> {
        Some("Sync changes from physical filesystem to VFS")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SyncFromDiskInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SyncFromDiskInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Syncing from disk for paths: {:?}", input.paths);

        let base_path = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;

        // For now, use a default workspace ID (in production, this should come from context)
        let workspace_id = Uuid::new_v4();

        let paths_to_sync = if let Some(ref paths) = input.paths {
            paths.iter()
                .map(|p| base_path.join(p))
                .collect::<Vec<_>>()
        } else {
            vec![base_path.clone()]
        };

        let mut files_synced = 0;
        let mut conflicts = Vec::new();

        for path in paths_to_sync {
            let files = if path.is_dir() {
                self.scan_directory(&path).await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            } else if path.is_file() {
                vec![path.clone()]
            } else {
                continue;
            };

            for file_path in files {
                // Convert to virtual path
                let relative_path = file_path.strip_prefix(&base_path)
                    .unwrap_or(&file_path);

                let virtual_path = match VirtualPath::new(relative_path.to_string_lossy().as_ref()) {
                    Ok(vp) => vp,
                    Err(e) => {
                        warn!("Invalid path {}: {}", relative_path.display(), e);
                        continue;
                    }
                };

                match self.sync_file_from_disk(&workspace_id, &file_path, &virtual_path).await {
                    Ok(true) => files_synced += 1,
                    Ok(false) => {
                        // Conflict detected
                        conflicts.push(virtual_path.to_string());
                    }
                    Err(e) => {
                        error!("Failed to sync {}: {}", virtual_path, e);
                    }
                }
            }
        }

        let output = SyncFromDiskOutput {
            files_synced,
            conflicts,
        };

        info!("Sync from disk completed: {} files, {} conflicts",
            output.files_synced, output.conflicts.len());

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncStatusInput {
    #[serde(default)]
    detailed: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SyncStatusOutput {
    in_sync: bool,
    pending_writes: i32,
    pending_reads: i32,
    conflicts: i32,
}

pub struct SyncStatusTool {
    ctx: MaterializationContext,
}

impl SyncStatusTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SyncStatusTool {
    fn name(&self) -> &str {
        "cortex.sync.status"
    }

    fn description(&self) -> Option<&str> {
        Some("Get synchronization status between VFS and filesystem")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SyncStatusInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let _input: SyncStatusInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Getting sync status");

        // Query VFS for pending changes
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Storage error: {}", e)))?;

        // Count pending writes (modified/created)
        let write_query = "SELECT count() as count FROM vnode WHERE status IN ['modified', 'created'] GROUP ALL";
        let mut write_response = conn.connection()
            .query(write_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;

        let write_counts: Vec<serde_json::Value> = write_response.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Parse error: {}", e)))?;
        let pending_writes = write_counts.first()
            .and_then(|v| v.get("count").and_then(|c| c.as_i64()))
            .unwrap_or(0) as i32;

        // Count pending deletes
        let delete_query = "SELECT count() as count FROM vnode WHERE status = 'deleted' GROUP ALL";
        let mut delete_response = conn.connection()
            .query(delete_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;

        let delete_counts: Vec<serde_json::Value> = delete_response.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Parse error: {}", e)))?;
        let pending_reads = delete_counts.first()
            .and_then(|v| v.get("count").and_then(|c| c.as_i64()))
            .unwrap_or(0) as i32;

        // Count conflicts
        let conflict_query = "SELECT count() as count FROM vnode WHERE status = 'conflict' GROUP ALL";
        let mut conflict_response = conn.connection()
            .query(conflict_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;

        let conflict_counts: Vec<serde_json::Value> = conflict_response.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Parse error: {}", e)))?;
        let conflicts = conflict_counts.first()
            .and_then(|v| v.get("count").and_then(|c| c.as_i64()))
            .unwrap_or(0) as i32;

        let in_sync = pending_writes == 0 && pending_reads == 0 && conflicts == 0;

        let output = SyncStatusOutput {
            in_sync,
            pending_writes,
            pending_reads,
            conflicts,
        };

        info!("Sync status: in_sync={}, pending_writes={}, pending_reads={}, conflicts={}",
            output.in_sync, output.pending_writes, output.pending_reads, output.conflicts);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncResolveConflictInput {
    conflict_id: String,
    #[serde(default = "default_memory_resolution")]
    resolution: String,
    merge_content: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SyncResolveConflictOutput {
    conflict_id: String,
    resolved: bool,
}

pub struct SyncResolveConflictTool {
    ctx: MaterializationContext,
}

impl SyncResolveConflictTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SyncResolveConflictTool {
    fn name(&self) -> &str {
        "cortex.sync.resolve_conflict"
    }

    fn description(&self) -> Option<&str> {
        Some("Resolve a synchronization conflict between VFS and filesystem")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SyncResolveConflictInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SyncResolveConflictInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Resolving conflict: {} with strategy: {}", input.conflict_id, input.resolution);

        // Parse the conflict ID as a VNode UUID
        let vnode_id = Uuid::parse_str(&input.conflict_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid conflict ID: {}", e)))?;

        // Get the VNode
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Storage error: {}", e)))?;

        let query = "SELECT * FROM vnode WHERE id = $id AND status = 'conflict'";
        let mut response = conn.connection()
            .query(query)
            .bind(("id", vnode_id.to_string()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;

        let vnodes: Vec<VNode> = response.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Parse error: {}", e)))?;

        if vnodes.is_empty() {
            return Err(ToolError::ExecutionFailed(format!("Conflict not found: {}", input.conflict_id)));
        }

        let mut vnode = vnodes[0].clone();

        // Resolve based on strategy
        match input.resolution.as_str() {
            "memory" | "vfs" => {
                // Keep VFS version - just mark as synchronized
                vnode.mark_synchronized();
            }
            "disk" | "filesystem" => {
                // Load disk version into VFS
                let target_path = std::env::current_dir()
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;
                let physical_path = target_path.join(vnode.path.to_string().trim_start_matches('/'));

                let content = fs::read(&physical_path).await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read disk file: {}", e)))?;

                self.ctx.vfs.write_file(&vnode.workspace_id, &vnode.path, &content).await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write to VFS: {}", e)))?;

                vnode.mark_synchronized();
            }
            "manual" | "merge" => {
                // Use provided merge content
                if let Some(ref content) = input.merge_content {
                    self.ctx.vfs.write_file(&vnode.workspace_id, &vnode.path, content.as_bytes()).await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write merge content: {}", e)))?;
                    vnode.mark_synchronized();
                } else {
                    return Err(ToolError::ExecutionFailed("Manual resolution requires merge_content".to_string()));
                }
            }
            _ => {
                return Err(ToolError::ExecutionFailed(format!("Unknown resolution strategy: {}", input.resolution)));
            }
        }

        // Save the resolved VNode
        self.ctx.vfs.save_vnode(&vnode).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save vnode: {}", e)))?;

        let output = SyncResolveConflictOutput {
            conflict_id: input.conflict_id,
            resolved: true,
        };

        info!("Conflict resolved: {}", output.conflict_id);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchStartInput {
    paths: Vec<String>,
    #[serde(default)]
    auto_sync: bool,
    ignore_patterns: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct WatchStartOutput {
    watcher_id: String,
    paths_watched: Vec<String>,
}

pub struct WatchStartTool {
    ctx: MaterializationContext,
}

impl WatchStartTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for WatchStartTool {
    fn name(&self) -> &str {
        "cortex.watch.start"
    }

    fn description(&self) -> Option<&str> {
        Some("Start watching filesystem for changes and optionally auto-sync to VFS")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(WatchStartInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: WatchStartInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Starting filesystem watcher for paths: {:?}", input.paths);

        if input.paths.is_empty() {
            return Err(ToolError::ExecutionFailed("At least one path is required".to_string()));
        }

        // Convert paths to absolute PathBuf
        let base_path = std::env::current_dir()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get current dir: {}", e)))?;

        let watch_paths: Vec<PathBuf> = input.paths.iter()
            .map(|p| {
                let path = PathBuf::from(p);
                if path.is_absolute() {
                    path
                } else {
                    base_path.join(path)
                }
            })
            .collect();

        // Verify paths exist
        for path in &watch_paths {
            if !path.exists() {
                return Err(ToolError::ExecutionFailed(format!("Path does not exist: {}", path.display())));
            }
        }

        // Create watcher with custom config
        let config = WatcherConfig {
            debounce_duration: Duration::from_millis(200),
            batch_interval: Duration::from_millis(500),
            max_batch_size: 100,
            coalesce_events: true,
        };

        // Watch the first path (for multiple paths, we'd need multiple watchers or a common parent)
        let watcher = FileWatcher::with_config(&watch_paths[0], config)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create watcher: {}", e)))?;

        // Generate watcher ID
        let watcher_id = Uuid::new_v4().to_string();

        // Create workspace for this watcher
        let workspace_id = Uuid::new_v4();

        // Store watcher in global registry
        let handle = WatcherHandle {
            watcher: Arc::new(Mutex::new(watcher)),
            paths: watch_paths.clone(),
            auto_sync: input.auto_sync,
            workspace_id,
            vfs: self.ctx.vfs.clone(),
        };

        WATCHER_REGISTRY.insert(watcher_id.clone(), handle);

        // If auto_sync is enabled, spawn a background task to handle events
        if input.auto_sync {
            let watcher_id_clone = watcher_id.clone();
            tokio::spawn(async move {
                Self::auto_sync_task(watcher_id_clone).await;
            });
        }

        let output = WatchStartOutput {
            watcher_id,
            paths_watched: watch_paths.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        };

        info!("Filesystem watcher started: {}", output.watcher_id);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

impl WatchStartTool {
    async fn auto_sync_task(watcher_id: String) {
        loop {
            // Check if watcher still exists
            let handle_opt = WATCHER_REGISTRY.get(&watcher_id);
            if handle_opt.is_none() {
                debug!("Watcher {} removed, stopping auto-sync task", watcher_id);
                break;
            }

            let handle = handle_opt.unwrap();
            let mut watcher = handle.watcher.lock().await;

            // Wait for events
            if let Some(events) = watcher.recv().await {
                for event in events {
                    match event {
                        FileEvent::Created(path) | FileEvent::Modified(path) => {
                            // Sync file to VFS
                            if let Err(e) = Self::sync_file_to_vfs(&handle, &path).await {
                                error!("Failed to sync {}: {}", path.display(), e);
                            }
                        }
                        FileEvent::Deleted(path) => {
                            // Mark as deleted in VFS
                            if let Err(e) = Self::mark_deleted_in_vfs(&handle, &path).await {
                                error!("Failed to mark deleted {}: {}", path.display(), e);
                            }
                        }
                        FileEvent::Renamed { from, to } => {
                            // Handle rename
                            if let Err(e) = Self::handle_rename_in_vfs(&handle, &from, &to).await {
                                error!("Failed to handle rename {} -> {}: {}", from.display(), to.display(), e);
                            }
                        }
                    }
                }
            } else {
                // Receiver closed
                break;
            }
        }
    }

    async fn sync_file_to_vfs(handle: &WatcherHandle, path: &Path) -> CortexResult<()> {
        let content = fs::read(path).await
            .map_err(|e| cortex_core::error::CortexError::vfs(format!("Failed to read file: {}", e)))?;

        // Convert to virtual path (relative to watched path)
        let base = &handle.paths[0];
        let relative = path.strip_prefix(base)
            .map_err(|e| cortex_core::error::CortexError::invalid_input(format!("Path not under watch base: {}", e)))?;

        let vpath = VirtualPath::new(relative.to_string_lossy().as_ref())?;

        handle.vfs.write_file(&handle.workspace_id, &vpath, &content).await?;

        debug!("Auto-synced to VFS: {}", vpath);
        Ok(())
    }

    async fn mark_deleted_in_vfs(handle: &WatcherHandle, path: &Path) -> CortexResult<()> {
        let base = &handle.paths[0];
        let relative = path.strip_prefix(base)
            .map_err(|e| cortex_core::error::CortexError::invalid_input(format!("Path not under watch base: {}", e)))?;

        let vpath = VirtualPath::new(relative.to_string_lossy().as_ref())?;

        // Get the vnode and mark as deleted
        if let Some(mut vnode) = handle.vfs.get_vnode(&handle.workspace_id, &vpath).await? {
            vnode.status = cortex_vfs::SyncStatus::Deleted;
            handle.vfs.save_vnode(&vnode).await?;
            debug!("Marked as deleted in VFS: {}", vpath);
        }

        Ok(())
    }

    async fn handle_rename_in_vfs(handle: &WatcherHandle, from: &Path, to: &Path) -> CortexResult<()> {
        // For simplicity, treat as delete + create
        Self::mark_deleted_in_vfs(handle, from).await?;
        Self::sync_file_to_vfs(handle, to).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchStopInput {
    watcher_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct WatchStopOutput {
    watcher_id: String,
    stopped: bool,
}

pub struct WatchStopTool {
    ctx: MaterializationContext,
}

impl WatchStopTool {
    pub fn new(ctx: MaterializationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for WatchStopTool {
    fn name(&self) -> &str {
        "cortex.watch.stop"
    }

    fn description(&self) -> Option<&str> {
        Some("Stop a running filesystem watcher")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(WatchStopInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: WatchStopInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Stopping filesystem watcher: {}", input.watcher_id);

        // Remove from registry (this will stop the auto-sync task)
        let stopped = WATCHER_REGISTRY.remove(&input.watcher_id).is_some();

        if !stopped {
            warn!("Watcher not found: {}", input.watcher_id);
        }

        let output = WatchStopOutput {
            watcher_id: input.watcher_id,
            stopped,
        };

        info!("Filesystem watcher stopped: {} (success: {})", output.watcher_id, output.stopped);

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

fn default_true() -> bool {
    true
}
fn default_memory_resolution() -> String {
    "memory".to_string()
}
