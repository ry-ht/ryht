//! Workspace Management Tools
//!
//! This module implements the 8 workspace management tools defined in the MCP spec:
//! - cortex.workspace.create
//! - cortex.workspace.get
//! - cortex.workspace.list
//! - cortex.workspace.activate
//! - cortex.workspace.sync_from_disk
//! - cortex.workspace.export
//! - cortex.workspace.archive
//! - cortex.workspace.delete

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use cortex_vfs::{
    ExternalProjectLoader, ImportOptions as VfsImportOptions, MaterializationEngine, VirtualFileSystem,
    Workspace, WorkspaceType,
};
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

// =============================================================================
// Shared Context
// =============================================================================

/// Shared context for all workspace tools
#[derive(Clone)]
pub struct WorkspaceContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    loader: Arc<ExternalProjectLoader>,
    engine: Arc<MaterializationEngine>,
}

impl WorkspaceContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let loader = Arc::new(ExternalProjectLoader::new((*vfs).clone()));
        let engine = Arc::new(MaterializationEngine::new((*vfs).clone()));

        Self {
            storage,
            vfs,
            loader,
            engine,
        }
    }
}

// =============================================================================
// cortex.workspace.create
// =============================================================================

pub struct WorkspaceCreateTool {
    ctx: WorkspaceContext,
}

impl WorkspaceCreateTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateInput {
    /// Workspace name
    name: String,
    /// Root path of the project to import
    root_path: String,
    /// Workspace type
    #[serde(default)]
    workspace_type: Option<String>,
    /// Auto import on creation
    #[serde(default = "default_true")]
    auto_import: bool,
    /// Import options
    #[serde(default)]
    import_options: Option<ImportOptions>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
struct ImportOptions {
    #[serde(default)]
    include_git_history: bool,
    #[serde(default)]
    include_node_modules: bool,
    #[serde(default = "default_true")]
    include_hidden: bool,
    #[serde(default = "default_max_file_size")]
    max_file_size_mb: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateOutput {
    workspace_id: String,
    files_imported: usize,
    units_extracted: usize,
    import_duration_ms: u64,
    warnings: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_max_file_size() -> u64 {
    10
}

#[async_trait]
impl Tool for WorkspaceCreateTool {
    fn name(&self) -> &str {
        "cortex.workspace.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new workspace by importing an existing project")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CreateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        debug!("Creating workspace: {}", input.name);
        let start = std::time::Instant::now();

        // Parse workspace type
        let workspace_type = input
            .workspace_type
            .as_deref()
            .and_then(|s| match s {
                "rust_cargo" => Some(WorkspaceType::Code),
                "typescript_turborepo" => Some(WorkspaceType::Code),
                "typescript_nx" => Some(WorkspaceType::Code),
                "python_poetry" => Some(WorkspaceType::Code),
                "go_modules" => Some(WorkspaceType::Code),
                "mixed" => Some(WorkspaceType::Mixed),
                _ => None,
            })
            .unwrap_or(WorkspaceType::Mixed);

        // Create workspace
        let workspace_id = Uuid::new_v4();
        let root_path = PathBuf::from(&input.root_path);

        if !root_path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Root path does not exist: {}",
                input.root_path
            )));
        }

        let mut warnings = Vec::new();
        let mut files_imported = 0;
        let units_extracted = 0;

        // Import if requested
        if input.auto_import {
            let _import_opts = input.import_options.unwrap_or_default();
            let vfs_opts = VfsImportOptions {
                read_only: false,
                create_fork: false,
                namespace: "main".to_string(),
                include_patterns: vec![],
                exclude_patterns: vec![],
                max_depth: None,
                process_code: true,
                generate_embeddings: false,
            };

            match self
                .ctx
                .loader
                .import_project(&root_path, vfs_opts)
                .await
            {
                Ok(report) => {
                    files_imported = report.files_imported;
                    // Note: ImportReport doesn't have warnings field
                }
                Err(e) => {
                    warnings.push(format!("Import failed: {}", e));
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Workspace created: {} ({} files in {:?})",
            workspace_id, files_imported, duration
        );

        Ok(ToolResult::success_json(serde_json::json!(CreateOutput {
            workspace_id: workspace_id.to_string(),
            files_imported,
            units_extracted,
            import_duration_ms: duration.as_millis() as u64,
            warnings,
        })))
    }
}

// =============================================================================
// cortex.workspace.get
// =============================================================================

pub struct WorkspaceGetTool {
    ctx: WorkspaceContext,
}

impl WorkspaceGetTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetInput {
    workspace_id: String,
    #[serde(default = "default_true")]
    include_stats: bool,
    #[serde(default)]
    include_structure: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetOutput {
    workspace_id: String,
    name: String,
    workspace_type: String,
    root_path: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<WorkspaceStats>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct WorkspaceStats {
    total_files: usize,
    total_directories: usize,
    total_units: usize,
    total_bytes: u64,
    languages: serde_json::Value,
}

#[async_trait]
impl Tool for WorkspaceGetTool {
    fn name(&self) -> &str {
        "cortex.workspace.get"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieves workspace information and statistics")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        debug!("Getting workspace: {}", workspace_id);

        // TODO: Query workspace from database
        // For now, return a placeholder
        let output = GetOutput {
            workspace_id: workspace_id.to_string(),
            name: "Workspace".to_string(),
            workspace_type: "mixed".to_string(),
            root_path: "/path/to/workspace".to_string(),
            status: "active".to_string(),
            stats: if input.include_stats {
                Some(WorkspaceStats {
                    total_files: 0,
                    total_directories: 0,
                    total_units: 0,
                    total_bytes: 0,
                    languages: serde_json::json!({}),
                })
            } else {
                None
            },
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.list
// =============================================================================

pub struct WorkspaceListTool {
    ctx: WorkspaceContext,
}

impl WorkspaceListTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListInput {
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_limit() -> usize {
    100
}

#[derive(Debug, Serialize, JsonSchema)]
struct ListOutput {
    workspaces: Vec<WorkspaceSummary>,
    total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct WorkspaceSummary {
    workspace_id: String,
    name: String,
    workspace_type: String,
    status: String,
    file_count: usize,
}

#[async_trait]
impl Tool for WorkspaceListTool {
    fn name(&self) -> &str {
        "cortex.workspace.list"
    }

    fn description(&self) -> Option<&str> {
        Some("Lists all available workspaces")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ListInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        debug!("Listing workspaces with status: {}", input.status);

        // TODO: Query workspaces from database
        let output = ListOutput {
            workspaces: vec![],
            total: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.activate
// =============================================================================

pub struct WorkspaceActivateTool {
    ctx: WorkspaceContext,
}

impl WorkspaceActivateTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ActivateInput {
    workspace_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ActivateOutput {
    workspace_id: String,
    status: String,
}

#[async_trait]
impl Tool for WorkspaceActivateTool {
    fn name(&self) -> &str {
        "cortex.workspace.activate"
    }

    fn description(&self) -> Option<&str> {
        Some("Sets the active workspace for subsequent operations")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ActivateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ActivateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Activating workspace: {}", workspace_id);

        // TODO: Set active workspace in context/session
        let output = ActivateOutput {
            workspace_id: workspace_id.to_string(),
            status: "activated".to_string(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.sync_from_disk
// =============================================================================

pub struct WorkspaceSyncTool {
    ctx: WorkspaceContext,
}

impl WorkspaceSyncTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SyncInput {
    workspace_id: String,
    #[serde(default = "default_true")]
    detect_moves: bool,
    #[serde(default)]
    auto_resolve: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SyncOutput {
    files_added: usize,
    files_modified: usize,
    files_deleted: usize,
    conflicts: usize,
}

#[async_trait]
impl Tool for WorkspaceSyncTool {
    fn name(&self) -> &str {
        "cortex.workspace.sync_from_disk"
    }

    fn description(&self) -> Option<&str> {
        Some("Synchronizes workspace with filesystem changes")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SyncInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: SyncInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Syncing workspace from disk: {}", workspace_id);

        // TODO: Implement sync from disk
        let output = SyncOutput {
            files_added: 0,
            files_modified: 0,
            files_deleted: 0,
            conflicts: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.export
// =============================================================================

pub struct WorkspaceExportTool {
    ctx: WorkspaceContext,
}

impl WorkspaceExportTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExportInput {
    workspace_id: String,
    target_path: String,
    #[serde(default)]
    include_history: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ExportOutput {
    files_exported: usize,
    export_path: String,
    duration_ms: u64,
}

#[async_trait]
impl Tool for WorkspaceExportTool {
    fn name(&self) -> &str {
        "cortex.workspace.export"
    }

    fn description(&self) -> Option<&str> {
        Some("Exports workspace to a new filesystem location")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ExportInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ExportInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Exporting workspace {} to {}", workspace_id, input.target_path);

        let start = std::time::Instant::now();
        let target_path = PathBuf::from(&input.target_path);

        // TODO: Implement export using MaterializationEngine
        let files_exported = 0;

        let duration = start.elapsed();

        let output = ExportOutput {
            files_exported,
            export_path: input.target_path,
            duration_ms: duration.as_millis() as u64,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.archive
// =============================================================================

pub struct WorkspaceArchiveTool {
    ctx: WorkspaceContext,
}

impl WorkspaceArchiveTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ArchiveInput {
    workspace_id: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ArchiveOutput {
    workspace_id: String,
    status: String,
}

#[async_trait]
impl Tool for WorkspaceArchiveTool {
    fn name(&self) -> &str {
        "cortex.workspace.archive"
    }

    fn description(&self) -> Option<&str> {
        Some("Archives a workspace (keeps in DB but marks inactive)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ArchiveInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ArchiveInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Archiving workspace: {}", workspace_id);

        // TODO: Mark workspace as archived in database
        let output = ArchiveOutput {
            workspace_id: workspace_id.to_string(),
            status: "archived".to_string(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.delete
// =============================================================================

pub struct WorkspaceDeleteTool {
    ctx: WorkspaceContext,
}

impl WorkspaceDeleteTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteInput {
    workspace_id: String,
    confirm: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeleteOutput {
    workspace_id: String,
    status: String,
}

#[async_trait]
impl Tool for WorkspaceDeleteTool {
    fn name(&self) -> &str {
        "cortex.workspace.delete"
    }

    fn description(&self) -> Option<&str> {
        Some("Permanently deletes a workspace from the database")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(DeleteInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: DeleteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        if !input.confirm {
            return Err(ToolError::ExecutionFailed(
                "Confirmation required: confirm must be true".to_string(),
            ));
        }

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Deleting workspace: {}", workspace_id);

        // TODO: Delete workspace from database
        let output = DeleteOutput {
            workspace_id: workspace_id.to_string(),
            status: "deleted".to_string(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}
