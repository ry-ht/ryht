//! Materialization Tools (8 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use cortex_vfs::{MaterializationEngine, VirtualFileSystem};
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

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
        Self { storage, vfs, engine }
    }
}

macro_rules! impl_mat_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: MaterializationContext,
        }

        impl $name {
            pub fn new(ctx: MaterializationContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
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

impl_mat_tool!(FlushPreviewTool, "cortex.flush.preview", "Preview changes to be flushed", FlushPreviewInput, FlushPreviewOutput);

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

impl_mat_tool!(FlushExecuteTool, "cortex.flush.execute", "Flush changes to filesystem", FlushExecuteInput, FlushExecuteOutput);

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

impl_mat_tool!(FlushSelectiveTool, "cortex.flush.selective", "Flush specific changes only", FlushSelectiveInput, FlushSelectiveOutput);

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

impl_mat_tool!(SyncFromDiskTool, "cortex.sync.from_disk", "Sync changes from filesystem", SyncFromDiskInput, SyncFromDiskOutput);

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

impl_mat_tool!(SyncStatusTool, "cortex.sync.status", "Get sync status", SyncStatusInput, SyncStatusOutput);

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

impl_mat_tool!(SyncResolveConflictTool, "cortex.sync.resolve_conflict", "Resolve sync conflict", SyncResolveConflictInput, SyncResolveConflictOutput);

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

impl_mat_tool!(WatchStartTool, "cortex.watch.start", "Start filesystem watcher", WatchStartInput, WatchStartOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchStopInput {
    watcher_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct WatchStopOutput {
    watcher_id: String,
    stopped: bool,
}

impl_mat_tool!(WatchStopTool, "cortex.watch.stop", "Stop filesystem watcher", WatchStopInput, WatchStopOutput);

fn default_true() -> bool { true }
fn default_memory_resolution() -> String { "memory".to_string() }
