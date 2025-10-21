//! Version Control Tools (10 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct VersionControlContext {
    #[allow(dead_code)]
    storage: Arc<ConnectionManager>,
}

impl VersionControlContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_version_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            #[allow(dead_code)]
            ctx: VersionControlContext,
        }

        impl $name {
            pub fn new(ctx: VersionControlContext) -> Self {
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
#[allow(dead_code)]
pub struct GetHistoryInput {
    entity_id: String,
    #[serde(default = "default_file_type")]
    entity_type: String,
    #[serde(default = "default_history_limit")]
    limit: i32,
    #[serde(default)]
    include_diffs: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetHistoryOutput {
    history: Vec<VersionEntry>,
    total_versions: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct VersionEntry {
    version: i32,
    timestamp: String,
    author: String,
    message: String,
}

impl_version_tool!(VersionGetHistoryTool, "cortex.version.get_history", "Get version history of entity", GetHistoryInput, GetHistoryOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct CompareInput {
    entity_id: String,
    version_a: i32,
    version_b: i32,
    #[serde(default = "default_unified")]
    diff_format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CompareOutput {
    diff: String,
    additions: i32,
    deletions: i32,
}

impl_version_tool!(VersionCompareTool, "cortex.version.compare", "Compare two versions", CompareInput, CompareOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RestoreInput {
    entity_id: String,
    target_version: i32,
    #[serde(default = "default_true")]
    create_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RestoreOutput {
    entity_id: String,
    restored_version: i32,
    new_version: i32,
}

impl_version_tool!(VersionRestoreTool, "cortex.version.restore", "Restore to previous version", RestoreInput, RestoreOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct CreateSnapshotInput {
    name: String,
    description: Option<String>,
    scope_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CreateSnapshotOutput {
    snapshot_id: String,
    name: String,
    timestamp: String,
    entities_count: i32,
}

impl_version_tool!(VersionCreateSnapshotTool, "cortex.version.create_snapshot", "Create named snapshot", CreateSnapshotInput, CreateSnapshotOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ListSnapshotsInput {
    workspace_id: Option<String>,
    #[serde(default = "default_snapshot_limit")]
    limit: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ListSnapshotsOutput {
    snapshots: Vec<SnapshotInfo>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SnapshotInfo {
    snapshot_id: String,
    name: String,
    timestamp: String,
    entities_count: i32,
}

impl_version_tool!(VersionListSnapshotsTool, "cortex.version.list_snapshots", "List available snapshots", ListSnapshotsInput, ListSnapshotsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RestoreSnapshotInput {
    snapshot_id: String,
    target_workspace: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RestoreSnapshotOutput {
    workspace_id: String,
    entities_restored: i32,
}

impl_version_tool!(VersionRestoreSnapshotTool, "cortex.version.restore_snapshot", "Restore from snapshot", RestoreSnapshotInput, RestoreSnapshotOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct DiffSnapshotsInput {
    snapshot_a: String,
    snapshot_b: String,
    #[serde(default)]
    include_file_diffs: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct DiffSnapshotsOutput {
    added_files: Vec<String>,
    deleted_files: Vec<String>,
    modified_files: Vec<String>,
    total_changes: i32,
}

impl_version_tool!(VersionDiffSnapshotsTool, "cortex.version.diff_snapshots", "Compare two snapshots", DiffSnapshotsInput, DiffSnapshotsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct BlameInput {
    file_path: String,
    start_line: Option<i32>,
    end_line: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct BlameOutput {
    lines: Vec<BlameLine>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct BlameLine {
    line_number: i32,
    version: i32,
    author: String,
    timestamp: String,
}

impl_version_tool!(VersionBlameTool, "cortex.version.blame", "Get blame information", BlameInput, BlameOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetChangelogInput {
    from_version: Option<String>,
    to_version: Option<String>,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetChangelogOutput {
    changelog: String,
    format: String,
}

impl_version_tool!(VersionGetChangelogTool, "cortex.version.get_changelog", "Generate changelog", GetChangelogInput, GetChangelogOutput);

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct TagInput {
    tag_name: String,
    message: Option<String>,
    #[serde(default = "default_true")]
    snapshot: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TagOutput {
    tag_name: String,
    snapshot_id: Option<String>,
    timestamp: String,
}

impl_version_tool!(VersionTagTool, "cortex.version.tag", "Create a version tag", TagInput, TagOutput);

fn default_file_type() -> String { "file".to_string() }
fn default_history_limit() -> i32 { 20 }
fn default_unified() -> String { "unified".to_string() }
fn default_true() -> bool { true }
fn default_snapshot_limit() -> i32 { 50 }
fn default_markdown() -> String { "markdown".to_string() }
