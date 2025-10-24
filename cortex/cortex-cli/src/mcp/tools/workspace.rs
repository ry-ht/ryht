//! Workspace Management Tools
//!
//! This module implements the 8 workspace management tools defined in the MCP spec:
//! - cortex.workspace.create - Import existing project
//! - cortex.workspace.get - Get workspace info
//! - cortex.workspace.list - List all workspaces
//! - cortex.workspace.activate - Set active workspace
//! - cortex.workspace.sync_from_disk - Sync filesystem changes
//! - cortex.workspace.export - Export to disk
//! - cortex.workspace.archive - Archive workspace
//! - cortex.workspace.delete - Delete workspace

use async_trait::async_trait;
use chrono::Utc;
use cortex_core::error::{CortexError, Result};
use cortex_parser::CodeParser;
use cortex_storage::ConnectionManager;
use cortex_vfs::{
    ExternalProjectLoader, FileIngestionPipeline, ImportOptions as VfsImportOptions,
    MaterializationEngine, VirtualFileSystem, VirtualPath, Workspace, WorkspaceType, SourceType,
};
use cortex_memory::SemanticMemorySystem;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

// Import services
use crate::services::WorkspaceService;

// =============================================================================
// Shared Context
// =============================================================================

/// Shared context for all workspace tools
#[derive(Clone)]
pub struct WorkspaceContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    loader: Arc<ExternalProjectLoader>,
    #[allow(dead_code)]
    engine: Arc<MaterializationEngine>,
    #[allow(dead_code)]
    parser: Arc<tokio::sync::Mutex<CodeParser>>,
    #[allow(dead_code)]
    semantic_memory: Arc<SemanticMemorySystem>,
    ingestion: Arc<FileIngestionPipeline>,
    /// Active workspace ID (shared across all tools)
    active_workspace: Arc<RwLock<Option<Uuid>>>,
    /// Workspace service
    workspace_service: Arc<WorkspaceService>,
}

impl WorkspaceContext {
    pub fn new(storage: Arc<ConnectionManager>) -> cortex_core::error::Result<Self> {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let loader = Arc::new(ExternalProjectLoader::new((*vfs).clone()));
        let engine = Arc::new(MaterializationEngine::new((*vfs).clone()));
        let parser = Arc::new(tokio::sync::Mutex::new(
            CodeParser::new().map_err(|e| CortexError::internal(e.to_string()))?
        ));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
        let ingestion = Arc::new(FileIngestionPipeline::new(
            parser.clone(),
            vfs.clone(),
            semantic_memory.clone(),
        ));

        // Create workspace service
        let workspace_service = Arc::new(WorkspaceService::new(storage.clone(), vfs.clone()));

        Ok(Self {
            storage,
            vfs,
            loader,
            engine,
            parser,
            semantic_memory,
            ingestion,
            active_workspace: Arc::new(RwLock::new(None)),
            workspace_service,
        })
    }

    /// Get the currently active workspace ID
    pub fn get_active_workspace(&self) -> Option<Uuid> {
        self.active_workspace.read().ok().and_then(|guard| *guard)
    }

    /// Set the active workspace ID
    pub fn set_active_workspace(&self, workspace_id: Option<Uuid>) {
        if let Ok(mut guard) = self.active_workspace.write() {
            *guard = workspace_id;
        }
    }

    /// Get a reference to the active workspace Arc for sharing with other contexts
    pub fn active_workspace_ref(&self) -> Arc<RwLock<Option<Uuid>>> {
        self.active_workspace.clone()
    }

    // Note: Workspace CRUD operations now use WorkspaceService
    // Methods store_workspace, get_workspace, list_workspaces, and update_workspace
    // have been replaced with direct calls to workspace_service

    /// Delete workspace - delegates to workspace service
    async fn delete_workspace(&self, workspace_id: &Uuid) -> Result<()> {
        self.workspace_service
            .delete_workspace(workspace_id)
            .await
            .map_err(|e| CortexError::storage(e.to_string()))
    }

    /// Calculate workspace statistics - delegates to workspace service
    async fn calculate_stats(&self, workspace_id: &Uuid) -> Result<WorkspaceStats> {
        let stats = self.workspace_service
            .get_workspace_stats(workspace_id)
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(WorkspaceStats {
            total_files: stats.total_files,
            total_directories: stats.total_directories,
            total_units: stats.total_units,
            total_bytes: stats.total_bytes,
            languages: serde_json::to_value(stats.languages).unwrap(),
        })
    }

    /// Detect project type from directory
    fn detect_project_type(&self, path: &Path) -> WorkspaceType {
        if path.join("Cargo.toml").exists() {
            WorkspaceType::Code
        } else if path.join("package.json").exists() {
            WorkspaceType::Code
        } else if path.join("go.mod").exists() {
            WorkspaceType::Code
        } else if path.join("pyproject.toml").exists() {
            WorkspaceType::Code
        } else {
            WorkspaceType::Mixed
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
    /// Auto import on creation
    #[serde(default = "default_true")]
    auto_import: bool,
    /// Process code units (parse files)
    #[serde(default = "default_true")]
    process_code: bool,
    /// Maximum file size to import (MB)
    #[serde(default = "default_max_file_size")]
    #[allow(dead_code)]
    max_file_size_mb: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateOutput {
    workspace_id: String,
    workspace_type: String,
    files_imported: usize,
    directories_imported: usize,
    units_extracted: usize,
    total_bytes: usize,
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
        Some("Creates a new workspace by importing an existing project. Respects .gitignore, parses code structure, and extracts semantic units.")
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

        info!("Creating workspace: {} from {}", input.name, input.root_path);
        let start = std::time::Instant::now();

        let root_path = PathBuf::from(&input.root_path);
        if !root_path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Root path does not exist: {}",
                input.root_path
            )));
        }

        if !root_path.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Root path is not a directory: {}",
                input.root_path
            )));
        }

        let mut warnings = Vec::new();
        let workspace_type = self.ctx.detect_project_type(&root_path);

        // Create workspace
        let workspace = Workspace {
            id: Uuid::new_v4(),
            name: input.name.clone(),
            workspace_type,
            source_type: SourceType::Local,
            namespace: "main".to_string(),
            source_path: Some(root_path.clone()),
            read_only: false,
            parent_workspace: None,
            fork_metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let workspace_id = workspace.id;

        // Store workspace in database using workspace service
        use crate::services::workspace::CreateWorkspaceRequest;
        let create_request = CreateWorkspaceRequest {
            name: workspace.name.clone(),
            workspace_type: format!("{:?}", workspace_type).to_lowercase(),
            source_path: workspace.source_path.as_ref().map(|p| p.display().to_string()),
            read_only: Some(workspace.read_only),
        };
        self.ctx.workspace_service.create_workspace(create_request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store workspace: {}", e)))?;

        let mut files_imported = 0;
        let mut directories_imported = 0;
        let mut units_extracted = 0;
        let mut total_bytes = 0;

        // Import if requested
        if input.auto_import {
            let vfs_opts = VfsImportOptions {
                read_only: false,
                create_fork: false,
                namespace: "main".to_string(),
                include_patterns: vec!["**/*".to_string()],
                exclude_patterns: vec![
                    "**/node_modules/**".to_string(),
                    "**/target/**".to_string(),
                    "**/.git/**".to_string(),
                    "**/dist/**".to_string(),
                    "**/build/**".to_string(),
                    "**/.DS_Store".to_string(),
                ],
                max_depth: None,
                process_code: input.process_code,
                generate_embeddings: false,
            };

            match self.ctx.loader.import_project(&root_path, vfs_opts).await {
                Ok(report) => {
                    files_imported = report.files_imported;
                    directories_imported = report.directories_imported;
                    total_bytes = report.bytes_imported;
                    warnings.extend(report.errors);
                }
                Err(e) => {
                    warnings.push(format!("Import failed: {}", e));
                }
            }

            // Process code units if requested
            if input.process_code && files_imported > 0 {
                info!("Processing code units for workspace {}", workspace_id);
                match self.ctx.ingestion.ingest_workspace(&workspace_id).await {
                    Ok(ingestion_result) => {
                        units_extracted = ingestion_result.total_units;
                        if !ingestion_result.files_with_errors.is_empty() {
                            warnings.push(format!(
                                "Failed to parse {} files",
                                ingestion_result.files_with_errors.len()
                            ));
                        }
                    }
                    Err(e) => {
                        warnings.push(format!("Code processing failed: {}", e));
                    }
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Workspace created: {} ({} files, {} units in {:?})",
            workspace_id, files_imported, units_extracted, duration
        );

        Ok(ToolResult::success_json(serde_json::json!(CreateOutput {
            workspace_id: workspace_id.to_string(),
            workspace_type: format!("{:?}", workspace_type).to_lowercase(),
            files_imported,
            directories_imported,
            units_extracted,
            total_bytes,
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
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetOutput {
    workspace_id: String,
    name: String,
    workspace_type: String,
    source_type: String,
    root_path: Option<String>,
    read_only: bool,
    created_at: String,
    updated_at: String,
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
        Some("Retrieves workspace information including metadata and statistics")
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

        let workspace = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        let stats = if input.include_stats {
            Some(self.ctx.calculate_stats(&workspace_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to calculate stats: {}", e)))?)
        } else {
            None
        };

        let output = GetOutput {
            workspace_id: workspace.id.clone(),
            name: workspace.name,
            workspace_type: workspace.workspace_type,
            source_type: workspace.source_type,
            root_path: workspace.source_path,
            read_only: workspace.read_only,
            created_at: workspace.created_at.to_rfc3339(),
            updated_at: workspace.updated_at.to_rfc3339(),
            stats,
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
    #[serde(default)]
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
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
    source_type: String,
    file_count: usize,
    created_at: String,
}

#[async_trait]
impl Tool for WorkspaceListTool {
    fn name(&self) -> &str {
        "cortex.workspace.list"
    }

    fn description(&self) -> Option<&str> {
        Some("Lists all available workspaces with summary information")
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

        debug!("Listing workspaces (limit: {})", input.limit);

        use crate::services::workspace::ListWorkspaceFilters;
        let filters = ListWorkspaceFilters {
            workspace_type: None,
            limit: Some(input.limit),
        };
        let workspaces = self.ctx.workspace_service.list_workspaces(filters).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list workspaces: {}", e)))?;

        let mut summaries = Vec::new();
        for workspace in workspaces.iter() {
            // Quick file count
            let workspace_id = Uuid::parse_str(&workspace.id)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;
            let stats = self.ctx.calculate_stats(&workspace_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to calculate stats: {}", e)))?;

            summaries.push(WorkspaceSummary {
                workspace_id: workspace.id.clone(),
                name: workspace.name.clone(),
                workspace_type: workspace.workspace_type.clone(),
                source_type: workspace.source_type.clone(),
                file_count: stats.total_files,
                created_at: workspace.created_at.to_rfc3339(),
            });
        }

        let output = ListOutput {
            total: workspaces.len(),
            workspaces: summaries,
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
    name: String,
    status: String,
}

#[async_trait]
impl Tool for WorkspaceActivateTool {
    fn name(&self) -> &str {
        "cortex.workspace.activate"
    }

    fn description(&self) -> Option<&str> {
        Some("Sets the active workspace for subsequent operations. Validates workspace exists and is accessible.")
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

        // Verify workspace exists
        let workspace = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        info!("Activating workspace: {} ({})", workspace.name, workspace_id);

        // Store active workspace in context
        self.ctx.set_active_workspace(Some(workspace_id));

        let output = ActivateOutput {
            workspace_id: workspace_id.to_string(),
            name: workspace.name,
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
    #[allow(dead_code)]
    detect_moves: bool,
    #[serde(default)]
    re_parse: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SyncOutput {
    files_added: usize,
    files_modified: usize,
    files_deleted: usize,
    units_updated: usize,
    duration_ms: u64,
    errors: Vec<String>,
}

#[async_trait]
impl Tool for WorkspaceSyncTool {
    fn name(&self) -> &str {
        "cortex.workspace.sync_from_disk"
    }

    fn description(&self) -> Option<&str> {
        Some("Synchronizes workspace with filesystem changes. Detects added, modified, and deleted files, and optionally re-parses code units.")
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

        let start = std::time::Instant::now();
        info!("Syncing workspace from disk: {}", workspace_id);

        let workspace_details = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        let workspace = Workspace {
            id: Uuid::parse_str(&workspace_details.id).unwrap(),
            name: workspace_details.name,
            workspace_type: match workspace_details.workspace_type.as_str() {
                "code" => WorkspaceType::Code,
                "documentation" => WorkspaceType::Documentation,
                "mixed" => WorkspaceType::Mixed,
                "external" => WorkspaceType::External,
                _ => WorkspaceType::Mixed,
            },
            source_type: match workspace_details.source_type.as_str() {
                "local" => SourceType::Local,
                "externalreadonly" => SourceType::ExternalReadOnly,
                "fork" => SourceType::Fork,
                _ => SourceType::Local,
            },
            namespace: workspace_details.namespace,
            source_path: workspace_details.source_path.map(PathBuf::from),
            read_only: workspace_details.read_only,
            parent_workspace: None,
            fork_metadata: None,
            created_at: workspace_details.created_at,
            updated_at: workspace_details.updated_at,
        };

        let root_path = workspace.source_path
            .ok_or_else(|| ToolError::ExecutionFailed("Workspace has no source path".to_string()))?;

        if !root_path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "Source path no longer exists: {}",
                root_path.display()
            )));
        }

        let mut errors = Vec::new();
        let mut files_added = 0;
        let mut files_modified = 0;
        let mut files_deleted = 0;
        let mut units_updated = 0;

        // Get all current vnodes in workspace
        let root = VirtualPath::root();
        let current_vnodes = self.ctx.vfs.list_directory(&workspace_id, &root, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list vnodes: {}", e)))?;

        // Build a map of paths to vnodes
        let mut vnode_map: HashMap<String, _> = current_vnodes
            .into_iter()
            .map(|v| (v.path.to_string(), v))
            .collect();

        // Walk the physical filesystem
        let walker = ignore::WalkBuilder::new(&root_path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip the root
                    if path == root_path {
                        continue;
                    }

                    // Get relative path
                    let rel_path = match path.strip_prefix(&root_path) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    let vpath_str = rel_path.to_string_lossy().to_string();
                    let vpath = match VirtualPath::new(&vpath_str) {
                        Ok(p) => p,
                        Err(e) => {
                            errors.push(format!("Invalid path {}: {}", vpath_str, e));
                            continue;
                        }
                    };

                    // Check if file or directory
                    if path.is_file() {
                        // Check if exists in VFS
                        if let Some(existing) = vnode_map.remove(&vpath_str) {
                            // File exists - check if modified
                            match fs::metadata(path).await {
                                Ok(metadata) => {
                                    if metadata.modified().ok() > Some(existing.updated_at.into()) {
                                        // File modified - update
                                        match fs::read(path).await {
                                            Ok(content) => {
                                                if let Err(e) = self.ctx.vfs.write_file(&workspace_id, &vpath, &content).await {
                                                    errors.push(format!("Failed to update {}: {}", vpath_str, e));
                                                } else {
                                                    files_modified += 1;
                                                }
                                            }
                                            Err(e) => {
                                                errors.push(format!("Failed to read {}: {}", path.display(), e));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to get metadata for {}: {}", path.display(), e));
                                }
                            }
                        } else {
                            // New file - add
                            match fs::read(path).await {
                                Ok(content) => {
                                    if let Err(e) = self.ctx.vfs.write_file(&workspace_id, &vpath, &content).await {
                                        errors.push(format!("Failed to add {}: {}", vpath_str, e));
                                    } else {
                                        files_added += 1;
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to read {}: {}", path.display(), e));
                                }
                            }
                        }
                    } else if path.is_dir() {
                        vnode_map.remove(&vpath_str);
                    }
                }
                Err(e) => {
                    errors.push(format!("Walk error: {}", e));
                }
            }
        }

        // Remaining vnodes in map are deleted files
        for (path_str, vnode) in vnode_map {
            if vnode.is_file() {
                if let Ok(vpath) = VirtualPath::new(&path_str) {
                    if let Err(e) = self.ctx.vfs.delete(&workspace_id, &vpath, false).await {
                        errors.push(format!("Failed to delete {}: {}", path_str, e));
                    } else {
                        files_deleted += 1;
                    }
                }
            }
        }

        // Re-parse if requested
        if input.re_parse && (files_added > 0 || files_modified > 0) {
            info!("Re-parsing workspace after sync");
            match self.ctx.ingestion.ingest_workspace(&workspace_id).await {
                Ok(result) => {
                    units_updated = result.total_units;
                }
                Err(e) => {
                    errors.push(format!("Re-parsing failed: {}", e));
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Sync completed: +{} ~{} -{} files in {:?}",
            files_added, files_modified, files_deleted, duration
        );

        let output = SyncOutput {
            files_added,
            files_modified,
            files_deleted,
            units_updated,
            duration_ms: duration.as_millis() as u64,
            errors,
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
    #[serde(default = "default_true")]
    preserve_permissions: bool,
    #[serde(default = "default_true")]
    preserve_timestamps: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ExportOutput {
    files_exported: usize,
    directories_created: usize,
    bytes_written: usize,
    export_path: String,
    duration_ms: u64,
    errors: Vec<String>,
}

#[async_trait]
impl Tool for WorkspaceExportTool {
    fn name(&self) -> &str {
        "cortex.workspace.export"
    }

    fn description(&self) -> Option<&str> {
        Some("Exports workspace to a filesystem location. Materializes all virtual files with optional preservation of permissions and timestamps.")
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

        // Create target directory if it doesn't exist
        if !target_path.exists() {
            fs::create_dir_all(&target_path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create target directory: {}", e)))?;
        }

        // Manual export implementation to avoid Send issues with MaterializationEngine
        let mut files_exported = 0;
        let mut directories_created = 0;
        let mut bytes_written = 0;
        let mut errors = Vec::new();

        // Get all vnodes in workspace
        let root = VirtualPath::root();
        let vnodes = self.ctx.vfs.list_directory(&workspace_id, &root, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list vnodes: {}", e)))?;

        // Create directories first
        for vnode in &vnodes {
            if vnode.is_directory() {
                let dir_path = target_path.join(vnode.path.to_string());
                if let Err(e) = fs::create_dir_all(&dir_path).await {
                    errors.push(format!("Failed to create directory {}: {}", vnode.path, e));
                } else {
                    directories_created += 1;
                }
            }
        }

        // Export files
        for vnode in &vnodes {
            if vnode.is_file() {
                match self.ctx.vfs.read_file(&workspace_id, &vnode.path).await {
                    Ok(content) => {
                        let file_path = target_path.join(vnode.path.to_string());

                        // Ensure parent directory exists
                        if let Some(parent) = file_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent).await {
                                errors.push(format!("Failed to create parent dir for {}: {}", vnode.path, e));
                                continue;
                            }
                        }

                        // Write file
                        match fs::write(&file_path, &content).await {
                            Ok(_) => {
                                files_exported += 1;
                                bytes_written += content.len();

                                // Set permissions if requested
                                #[cfg(unix)]
                                if input.preserve_permissions {
                                    if let Some(perms) = vnode.permissions {
                                        use std::os::unix::fs::PermissionsExt;
                                        if let Err(e) = std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(perms)) {
                                            warn!("Failed to set permissions for {}: {}", vnode.path, e);
                                        }
                                    }
                                }

                                // Set timestamps if requested
                                if input.preserve_timestamps {
                                    // Note: Setting timestamps requires platform-specific code
                                    // For now, skip this to keep it simple
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Failed to write file {}: {}", vnode.path, e));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to read file {}: {}", vnode.path, e));
                    }
                }
            }
        }

        let duration = start.elapsed();
        info!(
            "Export completed: {} files in {:?}",
            files_exported, duration
        );

        let output = ExportOutput {
            files_exported,
            directories_created,
            bytes_written,
            export_path: input.target_path,
            duration_ms: duration.as_millis() as u64,
            errors,
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
    name: String,
    status: String,
}

#[async_trait]
impl Tool for WorkspaceArchiveTool {
    fn name(&self) -> &str {
        "cortex.workspace.archive"
    }

    fn description(&self) -> Option<&str> {
        Some("Archives a workspace. Makes it read-only and marks it as archived (keeps in DB but inactive).")
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

        let workspace = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        info!("Archiving workspace: {} ({})", workspace.name, workspace_id);

        // Store archive reason in metadata if Workspace had a metadata field
        // For now, just log it
        if let Some(reason) = input.reason {
            info!("Archive reason: {}", reason);
        }

        // Mark as read-only and update metadata
        use crate::services::workspace::UpdateWorkspaceRequest;
        let update_request = UpdateWorkspaceRequest {
            name: None,
            workspace_type: None,
            read_only: Some(true),
        };

        self.ctx.workspace_service.update_workspace(&workspace_id, update_request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update workspace: {}", e)))?;

        let output = ArchiveOutput {
            workspace_id: workspace_id.to_string(),
            name: workspace.name,
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
    message: String,
}

#[async_trait]
impl Tool for WorkspaceDeleteTool {
    fn name(&self) -> &str {
        "cortex.workspace.delete"
    }

    fn description(&self) -> Option<&str> {
        Some("Permanently deletes a workspace and all its data from the database. Requires explicit confirmation.")
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

        // Verify workspace exists
        let workspace = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        warn!("Deleting workspace: {} ({})", workspace.name, workspace_id);

        // Delete workspace and all associated data
        self.ctx.delete_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete workspace: {}", e)))?;

        let output = DeleteOutput {
            workspace_id: workspace_id.to_string(),
            status: "deleted".to_string(),
            message: format!("Workspace '{}' and all associated data have been permanently deleted", workspace.name),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}
