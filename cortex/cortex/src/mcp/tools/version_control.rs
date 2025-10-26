//! Version Control Tools (10 tools)

use async_trait::async_trait;
use chrono::Utc;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use mcp_sdk::error::{McpError, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

type McpResult<T> = std::result::Result<T, McpError>;

#[derive(Clone)]
pub struct VersionControlContext {
    storage: Arc<ConnectionManager>,
}

impl VersionControlContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Store a version in the version history
    async fn store_version(
        &self,
        entity_id: &str,
        entity_type: &str,
        version: i32,
        content_hash: &str,
        content: Option<&str>,
        size_bytes: i32,
        author: &str,
        message: &str,
        parent_version: Option<i32>,
    ) -> McpResult<()> {
        let conn = self.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get database connection: {}", e)))?;

        let metadata: HashMap<String, serde_json::Value> = HashMap::new();

        let record = serde_json::json!({
            "entity_id": entity_id,
            "entity_type": entity_type,
            "version": version,
            "content_hash": content_hash,
            "content": content,
            "size_bytes": size_bytes,
            "author": author,
            "message": message,
            "timestamp": Utc::now(),
            "parent_version": parent_version,
            "metadata": metadata,
        });

        let query = format!("CREATE version_history CONTENT {}", serde_json::to_string(&record).unwrap());
        conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store version: {}", e)))?;

        Ok(())
    }
}

// =============================================================================
// Helper structures for database queries
// =============================================================================

#[derive(Debug, Deserialize)]
struct VersionRecord {
    #[allow(dead_code)]
    id: serde_json::Value,
    entity_id: String,
    #[allow(dead_code)]
    entity_type: String,
    version: i32,
    content_hash: String,
    content: Option<String>,
    size_bytes: i32,
    author: String,
    message: String,
    timestamp: chrono::DateTime<Utc>,
    #[allow(dead_code)]
    parent_version: Option<i32>,
    #[allow(dead_code)]
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SnapshotRecord {
    id: serde_json::Value,
    name: String,
    description: Option<String>,
    #[allow(dead_code)]
    workspace_id: Option<String>,
    #[allow(dead_code)]
    scope_paths: Option<Vec<String>>,
    created_at: chrono::DateTime<Utc>,
    #[allow(dead_code)]
    author: String,
    #[allow(dead_code)]
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SnapshotEntryRecord {
    #[allow(dead_code)]
    id: serde_json::Value,
    #[allow(dead_code)]
    snapshot_id: String,
    entity_id: String,
    #[allow(dead_code)]
    entity_type: String,
    version: i32,
    content_hash: String,
    path: String,
    size_bytes: i32,
}

#[derive(Debug, Deserialize)]
struct TagRecord {
    #[allow(dead_code)]
    id: serde_json::Value,
    tag_name: String,
    message: Option<String>,
    snapshot_id: Option<String>,
    created_at: chrono::DateTime<Utc>,
    #[allow(dead_code)]
    author: String,
    #[allow(dead_code)]
    metadata: HashMap<String, serde_json::Value>,
}

// =============================================================================
// 1. cortex.version.get_history
// =============================================================================

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

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct VersionEntry {
    version: i32,
    timestamp: String,
    author: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

pub struct VersionGetHistoryTool {
    ctx: VersionControlContext,
}

impl VersionGetHistoryTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionGetHistoryTool {
    fn name(&self) -> &str {
        "cortex.version.get_history"
    }

    fn description(&self) -> Option<&str> {
        Some("Get version history of entity")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetHistoryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetHistoryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Getting version history for entity: {} (type: {})",
            input.entity_id, input.entity_type
        );

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Query version history
        let query = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' ORDER BY version DESC LIMIT {}",
            input.entity_id, input.limit
        );

        let mut result = conn
            .connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let versions: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse versions: {}", e)))?;

        let total_versions = versions.len() as i32;

        let mut history = Vec::new();
        let mut prev_content: Option<String> = None;

        for (idx, record) in versions.iter().enumerate() {
            let mut entry = VersionEntry {
                version: record.version,
                timestamp: record.timestamp.to_rfc3339(),
                author: record.author.clone(),
                message: record.message.clone(),
                content_hash: Some(record.content_hash.clone()),
                size_bytes: Some(record.size_bytes),
                diff: None,
            };

            // Generate diff if requested and content is available
            if input.include_diffs && idx > 0 {
                if let (Some(current_content), Some(previous_content)) =
                    (&record.content, &prev_content)
                {
                    let diff = TextDiff::from_lines(previous_content, current_content);
                    entry.diff = Some(Self::format_unified_diff(&diff));
                }
            }

            prev_content = record.content.clone();
            history.push(entry);
        }

        let output = GetHistoryOutput {
            history,
            total_versions,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

impl VersionGetHistoryTool {
    fn format_unified_diff<'a>(diff: &TextDiff<'a, 'a, 'a, str>) -> String {
        let mut output = String::new();
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            output.push_str(&format!("{}{}", sign, change));
        }
        output
    }
}

// =============================================================================
// 2. cortex.version.compare
// =============================================================================

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

pub struct VersionCompareTool {
    ctx: VersionControlContext,
}

impl VersionCompareTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionCompareTool {
    fn name(&self) -> &str {
        "cortex.version.compare"
    }

    fn description(&self) -> Option<&str> {
        Some("Compare two versions")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CompareInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CompareInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Comparing versions {} and {} for entity {}",
            input.version_a, input.version_b, input.entity_id
        );

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Fetch both versions
        let query_a = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' AND version = {} LIMIT 1",
            input.entity_id, input.version_a
        );
        let query_b = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' AND version = {} LIMIT 1",
            input.entity_id, input.version_b
        );

        let mut result_a = conn.connection()
            .query(&query_a)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query A failed: {}", e)))?;
        let mut result_b = conn.connection()
            .query(&query_b)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query B failed: {}", e)))?;

        let versions_a: Vec<VersionRecord> = result_a.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse version A: {}", e))
        })?;
        let versions_b: Vec<VersionRecord> = result_b.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse version B: {}", e))
        })?;

        if versions_a.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "Version {} not found",
                input.version_a
            )));
        }
        if versions_b.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "Version {} not found",
                input.version_b
            )));
        }

        let version_a = &versions_a[0];
        let version_b = &versions_b[0];

        // Get content for both versions
        let content_a = version_a
            .content
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("Version A content not stored".to_string()))?;
        let content_b = version_b
            .content
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("Version B content not stored".to_string()))?;

        // Generate diff
        let text_diff = TextDiff::from_lines(content_a, content_b);

        let mut additions = 0;
        let mut deletions = 0;
        let mut diff_output = String::new();

        if input.diff_format == "unified" {
            diff_output.push_str(&format!(
                "--- Version {}\n+++ Version {}\n",
                input.version_a, input.version_b
            ));

            for change in text_diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Delete => {
                        deletions += 1;
                        diff_output.push_str(&format!("-{}", change));
                    }
                    ChangeTag::Insert => {
                        additions += 1;
                        diff_output.push_str(&format!("+{}", change));
                    }
                    ChangeTag::Equal => {
                        diff_output.push_str(&format!(" {}", change));
                    }
                }
            }
        } else {
            // Simple format
            for change in text_diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Delete => {
                        deletions += 1;
                        diff_output.push_str(&format!("- {}", change));
                    }
                    ChangeTag::Insert => {
                        additions += 1;
                        diff_output.push_str(&format!("+ {}", change));
                    }
                    ChangeTag::Equal => {}
                }
            }
        }

        let output = CompareOutput {
            diff: diff_output,
            additions,
            deletions,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 3. cortex.version.restore
// =============================================================================

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

pub struct VersionRestoreTool {
    ctx: VersionControlContext,
}

impl VersionRestoreTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionRestoreTool {
    fn name(&self) -> &str {
        "cortex.version.restore"
    }

    fn description(&self) -> Option<&str> {
        Some("Restore to previous version")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RestoreInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: RestoreInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!(
            "Restoring entity {} to version {}",
            input.entity_id, input.target_version
        );

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Get the target version
        let query = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' AND version = {} LIMIT 1",
            input.entity_id, input.target_version
        );

        let mut result = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let versions: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse version: {}", e)))?;

        if versions.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "Version {} not found",
                input.target_version
            )));
        }

        let target_version = &versions[0];

        // Get current latest version to determine new version number
        let latest_query = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' ORDER BY version DESC LIMIT 1",
            input.entity_id
        );

        let mut latest_result = conn.connection()
            .query(&latest_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Latest query failed: {}", e)))?;

        let latest_versions: Vec<VersionRecord> = latest_result.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse latest version: {}", e))
        })?;

        let current_version = if !latest_versions.is_empty() {
            latest_versions[0].version
        } else {
            0
        };

        // Create backup if requested
        if input.create_backup && !latest_versions.is_empty() {
            let current = &latest_versions[0];
            self.ctx
                .store_version(
                    &input.entity_id,
                    &current.entity_type,
                    current.version,
                    &current.content_hash,
                    current.content.as_deref(),
                    current.size_bytes,
                    &current.author,
                    &format!("Backup before restore to v{}", input.target_version),
                    Some(current.version - 1),
                )
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create backup: {}", e)))?;
        }

        // Create new version with restored content
        let new_version = current_version + 1;

        self.ctx
            .store_version(
                &input.entity_id,
                &target_version.entity_type,
                new_version,
                &target_version.content_hash,
                target_version.content.as_deref(),
                target_version.size_bytes,
                "system",
                &format!("Restored from version {}", input.target_version),
                Some(current_version),
            )
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to restore version: {}", e)))?;

        let output = RestoreOutput {
            entity_id: input.entity_id,
            restored_version: input.target_version,
            new_version,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 4. cortex.version.create_snapshot
// =============================================================================

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

pub struct VersionCreateSnapshotTool {
    ctx: VersionControlContext,
}

impl VersionCreateSnapshotTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionCreateSnapshotTool {
    fn name(&self) -> &str {
        "cortex.version.create_snapshot"
    }

    fn description(&self) -> Option<&str> {
        Some("Create named snapshot")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CreateSnapshotInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateSnapshotInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Creating snapshot: {}", input.name);

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        let snapshot_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Create snapshot record
        let metadata: HashMap<String, serde_json::Value> = HashMap::new();

        let snapshot_record = serde_json::json!({
            "name": input.name,
            "description": input.description,
            "workspace_id": None::<String>,
            "scope_paths": input.scope_paths,
            "created_at": now,
            "author": "system",
            "metadata": metadata,
        });

        let create_query = format!("CREATE snapshots CONTENT {}", serde_json::to_string(&snapshot_record).unwrap());
        let mut result = conn.connection()
            .query(&create_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create snapshot: {}", e)))?;

        let created: Vec<SnapshotRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse created snapshot: {}", e)))?;

        if created.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "Failed to create snapshot".to_string(),
            ));
        }

        let created_snapshot = &created[0];
        let snapshot_db_id = created_snapshot.id.to_string();

        // Get all latest versions for entities (simplified: get all version history)
        let query = "SELECT * FROM version_history ORDER BY entity_id, version DESC";
        let mut result = conn.connection()
            .query(query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let all_versions: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse versions: {}", e)))?;

        // Group by entity_id and take the latest version
        let mut latest_versions: HashMap<String, &VersionRecord> = HashMap::new();
        for version in &all_versions {
            latest_versions
                .entry(version.entity_id.clone())
                .or_insert(version);
        }

        // Create snapshot entries
        let mut entities_count = 0;
        for version in latest_versions.values() {
            let entry_record = serde_json::json!({
                "snapshot_id": snapshot_db_id,
                "entity_id": version.entity_id,
                "entity_type": version.entity_type,
                "version": version.version,
                "content_hash": version.content_hash,
                "path": format!("/{}", version.entity_id), // Simplified path
                "size_bytes": version.size_bytes,
            });

            let entry_query = format!("CREATE snapshot_entries CONTENT {}", serde_json::to_string(&entry_record).unwrap());
            conn.connection()
                .query(&entry_query)
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to create snapshot entry: {}", e))
                })?;

            entities_count += 1;
        }

        let output = CreateSnapshotOutput {
            snapshot_id,
            name: input.name,
            timestamp: now.to_rfc3339(),
            entities_count,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 5. cortex.version.list_snapshots
// =============================================================================

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

pub struct VersionListSnapshotsTool {
    ctx: VersionControlContext,
}

impl VersionListSnapshotsTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionListSnapshotsTool {
    fn name(&self) -> &str {
        "cortex.version.list_snapshots"
    }

    fn description(&self) -> Option<&str> {
        Some("List available snapshots")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ListSnapshotsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ListSnapshotsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Listing snapshots (limit: {})", input.limit);

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Query snapshots
        let query = format!(
            "SELECT * FROM snapshots ORDER BY created_at DESC LIMIT {}",
            input.limit
        );

        let mut result = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let snapshot_records: Vec<SnapshotRecord> = result.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse snapshots: {}", e))
        })?;

        let total_count = snapshot_records.len() as i32;

        let mut snapshots = Vec::new();

        for snapshot in snapshot_records {
            let snapshot_id = snapshot.id.to_string();

            // Count entries in this snapshot
            let count_query = format!(
                "SELECT count() FROM snapshot_entries WHERE snapshot_id = '{}' GROUP ALL",
                snapshot_id
            );

            let mut count_result = conn.connection()
                .query(&count_query)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Count query failed: {}", e)))?;

            let counts: Vec<serde_json::Value> = count_result.take(0).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to parse count: {}", e))
            })?;

            let entities_count = counts
                .first()
                .and_then(|v| v.get("count"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            snapshots.push(SnapshotInfo {
                snapshot_id,
                name: snapshot.name,
                timestamp: snapshot.created_at.to_rfc3339(),
                entities_count,
            });
        }

        let output = ListSnapshotsOutput {
            snapshots,
            total_count,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 6. cortex.version.restore_snapshot
// =============================================================================

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

pub struct VersionRestoreSnapshotTool {
    ctx: VersionControlContext,
}

impl VersionRestoreSnapshotTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionRestoreSnapshotTool {
    fn name(&self) -> &str {
        "cortex.version.restore_snapshot"
    }

    fn description(&self) -> Option<&str> {
        Some("Restore from snapshot")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RestoreSnapshotInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: RestoreSnapshotInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Restoring snapshot: {}", input.snapshot_id);

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Get snapshot entries
        let query = format!(
            "SELECT * FROM snapshot_entries WHERE snapshot_id = '{}'",
            input.snapshot_id
        );

        let mut result = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let entries: Vec<SnapshotEntryRecord> = result.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse snapshot entries: {}", e))
        })?;

        if entries.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "Snapshot not found or empty".to_string(),
            ));
        }

        let mut entities_restored = 0;

        // For each entry, get the version content and create a new version
        for entry in &entries {
            // Get the version content
            let version_query = format!(
                "SELECT * FROM version_history WHERE entity_id = '{}' AND version = {} LIMIT 1",
                entry.entity_id, entry.version
            );

            let mut version_result = conn.connection().query(&version_query).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Version query failed: {}", e))
            })?;

            let versions: Vec<VersionRecord> = version_result.take(0).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to parse version: {}", e))
            })?;

            if let Some(version) = versions.first() {
                // Get current latest version
                let latest_query = format!(
                    "SELECT * FROM version_history WHERE entity_id = '{}' ORDER BY version DESC LIMIT 1",
                    entry.entity_id
                );

                let mut latest_result = conn.connection().query(&latest_query).await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Latest query failed: {}", e))
                })?;

                let latest_versions: Vec<VersionRecord> = latest_result.take(0).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to parse latest version: {}", e))
                })?;

                let new_version = if !latest_versions.is_empty() {
                    latest_versions[0].version + 1
                } else {
                    1
                };

                // Create new version with snapshot content
                self.ctx
                    .store_version(
                        &entry.entity_id,
                        &entry.entity_type,
                        new_version,
                        &entry.content_hash,
                        version.content.as_deref(),
                        entry.size_bytes,
                        "system",
                        &format!("Restored from snapshot {}", input.snapshot_id),
                        if !latest_versions.is_empty() {
                            Some(latest_versions[0].version)
                        } else {
                            None
                        },
                    )
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Failed to restore from snapshot: {}", e)))?;

                entities_restored += 1;
            }
        }

        let workspace_id = input
            .target_workspace
            .unwrap_or_else(|| "default".to_string());

        let output = RestoreSnapshotOutput {
            workspace_id,
            entities_restored,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 7. cortex.version.diff_snapshots
// =============================================================================

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

pub struct VersionDiffSnapshotsTool {
    ctx: VersionControlContext,
}

impl VersionDiffSnapshotsTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionDiffSnapshotsTool {
    fn name(&self) -> &str {
        "cortex.version.diff_snapshots"
    }

    fn description(&self) -> Option<&str> {
        Some("Compare two snapshots")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DiffSnapshotsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: DiffSnapshotsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Diffing snapshots: {} vs {}",
            input.snapshot_a, input.snapshot_b
        );

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Get entries for both snapshots
        let query_a = format!(
            "SELECT * FROM snapshot_entries WHERE snapshot_id = '{}'",
            input.snapshot_a
        );
        let query_b = format!(
            "SELECT * FROM snapshot_entries WHERE snapshot_id = '{}'",
            input.snapshot_b
        );

        let mut result_a = conn.connection()
            .query(&query_a)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query A failed: {}", e)))?;
        let mut result_b = conn.connection()
            .query(&query_b)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query B failed: {}", e)))?;

        let entries_a: Vec<SnapshotEntryRecord> = result_a.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse snapshot A entries: {}", e))
        })?;
        let entries_b: Vec<SnapshotEntryRecord> = result_b.take(0).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse snapshot B entries: {}", e))
        })?;

        // Build maps for comparison
        let mut map_a: HashMap<String, &SnapshotEntryRecord> = HashMap::new();
        let mut map_b: HashMap<String, &SnapshotEntryRecord> = HashMap::new();

        for entry in &entries_a {
            map_a.insert(entry.entity_id.clone(), entry);
        }
        for entry in &entries_b {
            map_b.insert(entry.entity_id.clone(), entry);
        }

        let mut added_files = Vec::new();
        let mut deleted_files = Vec::new();
        let mut modified_files = Vec::new();

        // Find added and modified files
        for (entity_id, entry_b) in &map_b {
            if let Some(entry_a) = map_a.get(entity_id) {
                // File exists in both - check if modified
                if entry_a.content_hash != entry_b.content_hash {
                    modified_files.push(entry_b.path.clone());
                }
            } else {
                // File only in B - added
                added_files.push(entry_b.path.clone());
            }
        }

        // Find deleted files
        for (entity_id, entry_a) in &map_a {
            if !map_b.contains_key(entity_id) {
                deleted_files.push(entry_a.path.clone());
            }
        }

        let total_changes =
            (added_files.len() + deleted_files.len() + modified_files.len()) as i32;

        let output = DiffSnapshotsOutput {
            added_files,
            deleted_files,
            modified_files,
            total_changes,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 8. cortex.version.blame
// =============================================================================

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
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

pub struct VersionBlameTool {
    ctx: VersionControlContext,
}

impl VersionBlameTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionBlameTool {
    fn name(&self) -> &str {
        "cortex.version.blame"
    }

    fn description(&self) -> Option<&str> {
        Some("Get blame information")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(BlameInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: BlameInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Getting blame for file: {}", input.file_path);

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Get the latest version for this file
        let query = format!(
            "SELECT * FROM version_history WHERE entity_id = '{}' ORDER BY version DESC LIMIT 1",
            input.file_path
        );

        let mut result = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let versions: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse versions: {}", e)))?;

        if versions.is_empty() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                input.file_path
            )));
        }

        let latest_version = &versions[0];

        // Get content
        let content = latest_version
            .content
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("File content not available".to_string()))?;

        let lines_vec: Vec<&str> = content.lines().collect();

        let start = input.start_line.unwrap_or(1).max(1) as usize;
        let end = input
            .end_line
            .map(|e| e as usize)
            .unwrap_or(lines_vec.len())
            .min(lines_vec.len());

        let mut blame_lines = Vec::new();

        // For each line, we'll attribute it to the latest version
        // In a full implementation, you'd track line-by-line changes through history
        for (idx, line_content) in lines_vec.iter().enumerate() {
            let line_number = (idx + 1) as i32;

            if (line_number as usize) < start || (line_number as usize) > end {
                continue;
            }

            blame_lines.push(BlameLine {
                line_number,
                version: latest_version.version,
                author: latest_version.author.clone(),
                timestamp: latest_version.timestamp.to_rfc3339(),
                content: Some(line_content.to_string()),
            });
        }

        let output = BlameOutput { lines: blame_lines };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 9. cortex.version.get_changelog
// =============================================================================

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

pub struct VersionGetChangelogTool {
    ctx: VersionControlContext,
}

impl VersionGetChangelogTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionGetChangelogTool {
    fn name(&self) -> &str {
        "cortex.version.get_changelog"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate changelog")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetChangelogInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetChangelogInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Generating changelog");

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        // Query all version history ordered by timestamp
        let query = "SELECT * FROM version_history ORDER BY timestamp DESC LIMIT 100";

        let mut result = conn.connection()
            .query(query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

        let versions: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse versions: {}", e)))?;

        // Group by entity_id
        let mut changes_by_entity: HashMap<String, Vec<&VersionRecord>> = HashMap::new();
        for version in &versions {
            changes_by_entity
                .entry(version.entity_id.clone())
                .or_default()
                .push(version);
        }

        // Generate changelog
        let mut changelog = String::new();

        if input.format == "markdown" {
            changelog.push_str("# Changelog\n\n");

            for (entity_id, entity_versions) in &changes_by_entity {
                changelog.push_str(&format!("## {}\n\n", entity_id));

                for version in entity_versions {
                    changelog.push_str(&format!(
                        "### Version {} - {} ({})\n\n",
                        version.version,
                        version.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        version.author
                    ));
                    changelog.push_str(&format!("{}\n\n", version.message));
                }
            }
        } else {
            // Plain text format
            changelog.push_str("CHANGELOG\n");
            changelog.push_str("=========\n\n");

            for (entity_id, entity_versions) in &changes_by_entity {
                changelog.push_str(&format!("{}\n", entity_id));
                changelog.push_str(&format!("{}\n\n", "-".repeat(entity_id.len())));

                for version in entity_versions {
                    changelog.push_str(&format!(
                        "Version {}: {} by {} - {}\n",
                        version.version,
                        version.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        version.author,
                        version.message
                    ));
                }
                changelog.push('\n');
            }
        }

        let output = GetChangelogOutput {
            changelog,
            format: input.format,
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// 10. cortex.version.tag
// =============================================================================

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

pub struct VersionTagTool {
    ctx: VersionControlContext,
}

impl VersionTagTool {
    pub fn new(ctx: VersionControlContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionTagTool {
    fn name(&self) -> &str {
        "cortex.version.tag"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a version tag")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TagInput)).unwrap()
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: TagInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Creating version tag: {}", input.tag_name);

        let conn = self.ctx.storage
            .acquire()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get connection: {}", e)))?;

        let now = Utc::now();
        let mut snapshot_id = None;

        // Create snapshot if requested
        if input.snapshot {
            let snapshot_name = format!("tag-{}", input.tag_name);

            // Create snapshot
            let metadata: HashMap<String, serde_json::Value> = HashMap::new();

            let snapshot_record = serde_json::json!({
                "name": snapshot_name,
                "description": input.message.clone().or(Some(format!("Snapshot for tag {}", input.tag_name))),
                "workspace_id": None::<String>,
                "scope_paths": None::<Vec<String>>,
                "created_at": now,
                "author": "system",
                "metadata": metadata,
            });

            let create_query = format!("CREATE snapshots CONTENT {}", serde_json::to_string(&snapshot_record).unwrap());
            let mut result = conn.connection()
                .query(&create_query)
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to execute create snapshot query: {}", e))
                })?;

            let created: Vec<SnapshotRecord> = result
                .take(0)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse created snapshot: {}", e)))?;

            if !created.is_empty() {
                let snapshot_db_id = created[0].id.to_string();
                snapshot_id = Some(snapshot_db_id.clone());

                // Get all latest versions
                let query = "SELECT * FROM version_history ORDER BY entity_id, version DESC";
                let mut result = conn.connection()
                    .query(query)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("Query failed: {}", e)))?;

                let all_versions: Vec<VersionRecord> = result.take(0).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to parse versions: {}", e))
                })?;

                // Group by entity_id and take the latest version
                let mut latest_versions: HashMap<String, &VersionRecord> = HashMap::new();
                for version in &all_versions {
                    latest_versions
                        .entry(version.entity_id.clone())
                        .or_insert(version);
                }

                // Create snapshot entries
                for version in latest_versions.values() {
                    let entry_record = serde_json::json!({
                        "snapshot_id": snapshot_db_id,
                        "entity_id": version.entity_id,
                        "entity_type": version.entity_type,
                        "version": version.version,
                        "content_hash": version.content_hash,
                        "path": format!("/{}", version.entity_id),
                        "size_bytes": version.size_bytes,
                    });

                    let entry_query = format!("CREATE snapshot_entries CONTENT {}", serde_json::to_string(&entry_record).unwrap());
                    conn.connection()
                        .query(&entry_query)
                        .await
                        .map_err(|e| {
                            ToolError::ExecutionFailed(format!(
                                "Failed to execute create snapshot query entry: {}",
                                e
                            ))
                        })?;
                }
            } else {
                warn!("Failed to execute create snapshot query for tag");
            }
        }

        // Create tag record
        let metadata: HashMap<String, serde_json::Value> = HashMap::new();

        let tag_record = serde_json::json!({
            "tag_name": input.tag_name,
            "message": input.message,
            "snapshot_id": snapshot_id,
            "created_at": now,
            "author": "system",
            "metadata": metadata,
        });

        let tag_query = format!("CREATE version_tags CONTENT {}", serde_json::to_string(&tag_record).unwrap());
        conn.connection()
            .query(&tag_query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create tag: {}", e)))?;

        let output = TagOutput {
            tag_name: input.tag_name,
            snapshot_id,
            timestamp: now.to_rfc3339(),
        };

        Ok(ToolResult::success_json(
            serde_json::to_value(output).unwrap(),
        ))
    }
}

// =============================================================================
// Default value functions
// =============================================================================

fn default_file_type() -> String {
    "file".to_string()
}
fn default_history_limit() -> i32 {
    20
}
fn default_unified() -> String {
    "unified".to_string()
}
fn default_true() -> bool {
    true
}
fn default_snapshot_limit() -> i32 {
    50
}
fn default_markdown() -> String {
    "markdown".to_string()
}
