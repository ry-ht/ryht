//! Workspace Management Tools
//!
//! This module implements 12 workspace management tools:
//!
//! **Core Operations (8):**
//! - cortex.workspace.create - Import existing project with auto-parsing
//! - cortex.workspace.get - Get workspace info and statistics
//! - cortex.workspace.list - List all workspaces with filtering
//! - cortex.workspace.activate - Set active workspace context
//! - cortex.workspace.sync_from_disk - Sync filesystem changes to VFS
//! - cortex.workspace.export - Export/materialize to disk
//! - cortex.workspace.archive - Archive workspace (mark read-only)
//! - cortex.workspace.delete - Permanent deletion with cascade
//!
//! **Advanced Operations (4):**
//! - cortex.workspace.fork - Create editable fork for experimentation
//! - cortex.workspace.search - Search files/content within workspace
//! - cortex.workspace.compare - Compare two workspaces and identify differences
//! - cortex.workspace.merge - Merge workspaces with conflict resolution

use async_trait::async_trait;
use chrono::Utc;
use cortex_core::error::{CortexError, Result};
use cortex_code_analysis::CodeParser;
use cortex_storage::ConnectionManager;
use regex;
use cortex_vfs::{
    ExternalProjectLoader, FileIngestionPipeline, ImportOptions as VfsImportOptions,
    MaterializationEngine, VirtualFileSystem, VirtualPath, Workspace, SyncSource, SyncSourceType,
    SyncSourceStatus, ForkManager, MergeStrategy, FileWatcher, WatcherConfig, AutoReparseHandle,
    AutoReparseConfig,
};
use cortex_memory::SemanticMemorySystem;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;
use tracing::{debug, info, warn, error};
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
    fork_manager: Arc<ForkManager>,
    /// Workspace service
    workspace_service: Arc<WorkspaceService>,
    /// Auto-reparse handle for FileWatcher integration
    auto_reparse: Arc<AutoReparseHandle>,
    /// Active file watchers (workspace_id -> watcher handle)
    active_watchers: Arc<RwLock<HashMap<Uuid, Arc<tokio::sync::Mutex<FileWatcher>>>>>,
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
        let fork_manager = Arc::new(ForkManager::new((*vfs).clone(), storage.clone()));

        // Create workspace service
        let workspace_service = Arc::new(WorkspaceService::new(storage.clone(), vfs.clone()));

        // Create auto-reparse handle
        let auto_reparse_config = AutoReparseConfig {
            enabled: true,
            debounce_ms: 500,
            max_pending_changes: 10,
            background_parsing: true,
        };
        let auto_reparse = Arc::new(AutoReparseHandle::new(auto_reparse_config, Some(ingestion.clone())));

        Ok(Self {
            storage,
            vfs,
            loader,
            engine,
            parser,
            semantic_memory,
            ingestion,
            fork_manager,
            workspace_service,
            auto_reparse,
            active_watchers: Arc::new(RwLock::new(HashMap::new())),
        })
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

    /// Start FileWatcher for a workspace with auto-sync and auto-reparse.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Workspace ID to watch
    /// * `root_path` - Physical filesystem path to watch
    /// * `enable_auto_watch` - Whether to actually start the watcher
    ///
    /// # Returns
    ///
    /// Returns Ok(true) if watcher was started, Ok(false) if auto-watch is disabled
    async fn start_file_watcher(
        &self,
        workspace_id: Uuid,
        root_path: &Path,
        enable_auto_watch: bool,
    ) -> Result<bool> {
        if !enable_auto_watch {
            debug!("Auto-watch disabled for workspace {}", workspace_id);
            return Ok(false);
        }

        // Check if watcher already exists
        {
            let watchers = self.active_watchers.read().unwrap();
            if watchers.contains_key(&workspace_id) {
                info!("FileWatcher already active for workspace {}", workspace_id);
                return Ok(true);
            }
        }

        // Configure watcher
        let mut config = WatcherConfig::default();
        config.enable_auto_sync = true;
        config.enable_auto_reparse = true;
        config.debounce_duration = std::time::Duration::from_millis(100);
        config.batch_interval = std::time::Duration::from_millis(500);

        // Create watcher with VFS integration
        let watcher = FileWatcher::with_integration(
            root_path,
            workspace_id,
            config,
            self.vfs.clone(),
            Some(self.auto_reparse.clone()),
        )
        .map_err(|e| CortexError::vfs(format!("Failed to create FileWatcher: {}", e)))?;

        info!(
            "Started FileWatcher for workspace {} at path: {}",
            workspace_id,
            root_path.display()
        );

        // Store watcher handle
        let watcher = Arc::new(tokio::sync::Mutex::new(watcher));
        {
            let mut watchers = self.active_watchers.write().unwrap();
            watchers.insert(workspace_id, watcher.clone());
        }

        // Spawn background task to process events
        let watcher_clone = watcher.clone();
        let workspace_id_clone = workspace_id;
        tokio::spawn(async move {
            let mut watcher = watcher_clone.lock().await;
            loop {
                match watcher.process_events().await {
                    Some(events) => {
                        if !events.is_empty() {
                            debug!(
                                "Processed {} file events for workspace {}",
                                events.len(),
                                workspace_id_clone
                            );
                        }
                    }
                    None => {
                        info!("FileWatcher closed for workspace {}", workspace_id_clone);
                        break;
                    }
                }
            }
        });

        Ok(true)
    }

    /// Stop FileWatcher for a workspace.
    #[allow(dead_code)]
    async fn stop_file_watcher(&self, workspace_id: &Uuid) -> Result<()> {
        let mut watchers = self.active_watchers.write().unwrap();
        if let Some(_watcher) = watchers.remove(workspace_id) {
            info!("Stopped FileWatcher for workspace {}", workspace_id);
            // Watcher will be dropped and cleaned up automatically
        }
        Ok(())
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
    /// Root path of the project to import (optional for empty workspace)
    #[serde(default)]
    root_path: Option<String>,
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
    /// Enable automatic file watching for this workspace
    #[serde(default = "default_true")]
    enable_auto_watch: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateOutput {
    workspace_id: String,
    files_imported: usize,
    directories_imported: usize,
    units_extracted: usize,
    total_bytes: usize,
    import_duration_ms: u64,
    warnings: Vec<String>,
    watcher_started: bool,
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
        Some("Creates a new workspace. Can import an existing project or create an empty workspace. When importing, respects .gitignore, parses code structure, and extracts semantic units. Empty workspaces allow agents to create projects from scratch.")
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

        info!("Creating workspace: {}", input.name);
        let start = std::time::Instant::now();

        let mut warnings = Vec::new();

        // Handle optional root_path
        let root_path = if let Some(ref path_str) = input.root_path {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Root path does not exist: {}",
                    path_str
                )));
            }

            if !path.is_dir() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Root path is not a directory: {}",
                    path_str
                )));
            }
            Some(path)
        } else {
            None
        };

        // Create workspace with new model
        let workspace_id = Uuid::new_v4();

        // Create sync source for local path if provided
        let sync_sources = if let Some(ref path) = root_path {
            // Canonicalize path to ensure it's absolute
            let canonical_path = path.canonicalize()
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("Failed to canonicalize path {}: {}", path.display(), e)
                ))?;

            vec![SyncSource {
                id: Uuid::new_v4(),
                source: SyncSourceType::LocalPath {
                    path: canonical_path.display().to_string(),
                    watch: false,
                },
                read_only: false,
                priority: 0,
                last_sync: None,
                status: SyncSourceStatus::Unsynced,
                metadata: HashMap::new(),
            }]
        } else {
            Vec::new()
        };

        let metadata = HashMap::new();

        let workspace = Workspace {
            id: workspace_id,
            name: input.name.clone(),
            namespace: "main".to_string(),
            sync_sources,
            metadata,
            read_only: false,
            parent_workspace: None,
            fork_metadata: None,
            dependencies: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Store workspace in database using workspace service
        use crate::services::workspace::CreateWorkspaceRequest;
        let create_request = CreateWorkspaceRequest {
            name: workspace.name.clone(),
            source_path: root_path.as_ref().map(|p| p.display().to_string()),
            sync_sources: None, // Service layer will create from source_path
            metadata: None,
            read_only: Some(workspace.read_only),
        };
        self.ctx.workspace_service.create_workspace(create_request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store workspace: {}", e)))?;

        let mut files_imported = 0;
        let mut directories_imported = 0;
        let mut units_extracted = 0;
        let mut total_bytes = 0;

        // Import if requested and root_path is provided
        if input.auto_import && root_path.is_some() {
            let path = root_path.as_ref().unwrap();
            // Use canonical path for import
            let canonical_path = path.canonicalize()
                .map_err(|e| ToolError::ExecutionFailed(
                    format!("Failed to canonicalize path for import {}: {}", path.display(), e)
                ))?;
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
                max_file_size_bytes: Some((input.max_file_size_mb * 1024 * 1024) as usize),
            };

            // Use import_into_workspace to import into the already created workspace
            match self.ctx.loader.import_into_workspace(&workspace_id, &canonical_path, vfs_opts).await {
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
            // Now uses batch processing to avoid memory issues on large workspaces
            if input.process_code && files_imported > 0 {
                info!("Processing code units (using batch processing for large workspaces)");
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

        // Start FileWatcher if requested and root_path is provided
        let watcher_started = if let Some(ref path) = root_path {
            match self.ctx.start_file_watcher(workspace_id, path, input.enable_auto_watch).await {
                Ok(started) => {
                    if started {
                        info!("FileWatcher auto-started for workspace {}", workspace_id);
                    }
                    started
                }
                Err(e) => {
                    error!("Failed to start FileWatcher: {}", e);
                    warnings.push(format!("Failed to start FileWatcher: {}", e));
                    false
                }
            }
        } else {
            false
        };

        let duration = start.elapsed();
        info!(
            "Workspace created: {} ({} files, {} units in {:?})",
            workspace_id, files_imported, units_extracted, duration
        );

        Ok(ToolResult::success_json(serde_json::json!(CreateOutput {
            workspace_id: workspace_id.to_string(),
            files_imported,
            directories_imported,
            units_extracted,
            total_bytes,
            import_duration_ms: duration.as_millis() as u64,
            warnings,
            watcher_started,
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

        let source_type = workspace.source_type();
        let root_path = workspace.source_path();
        let output = GetOutput {
            workspace_id: workspace.id.clone(),
            name: workspace.name,
            source_type,
            root_path,
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
                source_type: workspace.source_type(),
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
        Some("DEPRECATED: This tool no longer sets a global active workspace. \
              Use workspace_id parameter in individual tool calls, or set workspace_id \
              in session metadata when creating an MCP session. \
              For CLI usage, set default_workspace in the configuration file.")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ActivateInput)).unwrap()
    }

    async fn execute(
        &self,
        _input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        Err(ToolError::ExecutionFailed(
            "This tool is deprecated. The global 'active workspace' concept has been removed \
             in favor of session-based workspace management. \n\n\
             To use a workspace:\n\
             1. Pass 'workspace_id' directly in tool calls that support it\n\
             2. Set 'workspace_id' in session metadata when creating an MCP session\n\
             3. For CLI usage, set 'default_workspace' in the configuration file\n\n\
             Multiple sessions can now work on different workspaces simultaneously without conflicts."
                .to_string()
        ))
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

        // Get the first LocalPath sync source
        let root_path = workspace_details.source_path()
            .map(PathBuf::from)
            .ok_or_else(|| ToolError::ExecutionFailed("Workspace has no source path for syncing".to_string()))?;

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
                                    use std::time::SystemTime;

                                    // Convert DateTime<Utc> to SystemTime
                                    let mtime = SystemTime::UNIX_EPOCH +
                                        std::time::Duration::from_secs(vnode.updated_at.timestamp() as u64);
                                    let atime = SystemTime::UNIX_EPOCH +
                                        std::time::Duration::from_secs(vnode.accessed_at.timestamp() as u64);

                                    // Set file times using filetime crate
                                    if let Err(e) = filetime::set_file_times(
                                        &file_path,
                                        filetime::FileTime::from_system_time(atime),
                                        filetime::FileTime::from_system_time(mtime),
                                    ) {
                                        warn!("Failed to set timestamps for {}: {}", vnode.path, e);
                                    }
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

        // Prepare metadata with archive information
        let mut archive_metadata = std::collections::HashMap::new();
        archive_metadata.insert("archived".to_string(), serde_json::json!(true));
        archive_metadata.insert("archived_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));

        if let Some(reason) = &input.reason {
            info!("Archive reason: {}", reason);
            archive_metadata.insert("archive_reason".to_string(), serde_json::json!(reason));
        }

        // Mark as read-only and update metadata
        use crate::services::workspace::UpdateWorkspaceRequest;
        let update_request = UpdateWorkspaceRequest {
            name: None,
            read_only: Some(true),
            metadata: Some(archive_metadata),
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

        // Stop FileWatcher if running
        if let Err(e) = self.ctx.stop_file_watcher(&workspace_id).await {
            warn!("Failed to stop FileWatcher for workspace {}: {}", workspace_id, e);
        }

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

// =============================================================================
// cortex.workspace.fork
// =============================================================================

pub struct WorkspaceForkTool {
    ctx: WorkspaceContext,
}

impl WorkspaceForkTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ForkInput {
    workspace_id: String,
    fork_name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ForkOutput {
    fork_workspace_id: String,
    fork_name: String,
    source_workspace_id: String,
    source_name: String,
    vnodes_copied: usize,
    fork_point: String,
}

#[async_trait]
impl Tool for WorkspaceForkTool {
    fn name(&self) -> &str {
        "cortex.workspace.fork"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates an editable fork of a workspace for experimentation. Perfect for trying changes without affecting the original workspace.")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ForkInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ForkInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        info!("Forking workspace: {} as {}", workspace_id, input.fork_name);

        // Get source workspace details
        let source = self.ctx.workspace_service.get_workspace(&workspace_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get workspace: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Workspace not found: {}", workspace_id)))?;

        // Create fork using fork manager
        let fork = self.ctx.fork_manager.create_fork(&workspace_id, input.fork_name.clone()).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create fork: {}", e)))?;

        // Count vnodes in fork
        let stats = self.ctx.calculate_stats(&fork.id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to calculate stats: {}", e)))?;

        let output = ForkOutput {
            fork_workspace_id: fork.id.to_string(),
            fork_name: fork.name.clone(),
            source_workspace_id: workspace_id.to_string(),
            source_name: source.name,
            vnodes_copied: stats.total_files + stats.total_directories,
            fork_point: fork.created_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.search
// =============================================================================

pub struct WorkspaceSearchTool {
    ctx: WorkspaceContext,
}

impl WorkspaceSearchTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchInput {
    workspace_id: String,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    content_query: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default = "default_root")]
    base_path: String,
    #[serde(default = "default_limit")]
    max_results: usize,
    #[serde(default)]
    case_sensitive: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchOutput {
    matches: Vec<SearchMatch>,
    total: usize,
    truncated: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchMatch {
    path: String,
    node_type: String,
    match_type: String, // "filename", "content", "both"
    size_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_snippet: Option<String>,
}

fn default_root() -> String {
    "/".to_string()
}

#[async_trait]
impl Tool for WorkspaceSearchTool {
    fn name(&self) -> &str {
        "cortex.workspace.search"
    }

    fn description(&self) -> Option<&str> {
        Some("Searches for files and content within a workspace using patterns and queries.")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SearchInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        debug!("Searching workspace {} with pattern: {:?}, content: {:?}",
            workspace_id, input.pattern, input.content_query);

        let base_path = VirtualPath::new(&input.base_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid base path: {}", e)))?;

        // Get all vnodes in base path
        let vnodes = self.ctx.vfs.list_directory(&workspace_id, &base_path, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

        let mut matches = Vec::new();
        let mut total_found = 0;

        for vnode in vnodes {
            if !vnode.is_file() {
                continue;
            }

            // Filter by language if specified
            if let Some(ref lang) = input.language {
                if let Some(ref node_lang) = vnode.language {
                    let lang_str = format!("{:?}", node_lang).to_lowercase();
                    if !lang_str.contains(&lang.to_lowercase()) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            let mut matched = false;
            let mut match_type = String::new();
            let mut content_snippet = None;

            // Pattern matching on filename
            if let Some(ref pattern) = input.pattern {
                let file_name = vnode.path.to_string();
                let matches_pattern = if pattern.contains('*') || pattern.contains('?') {
                    // Simple glob pattern matching (only * and ?)
                    let pattern_regex = pattern
                        .replace(".", "\\.")
                        .replace("*", ".*")
                        .replace("?", ".");
                    if let Ok(re) = regex::Regex::new(&format!("^{}$", pattern_regex)) {
                        re.is_match(&file_name)
                    } else {
                        false
                    }
                } else {
                    // Simple substring match
                    if input.case_sensitive {
                        file_name.contains(pattern)
                    } else {
                        file_name.to_lowercase().contains(&pattern.to_lowercase())
                    }
                };

                if matches_pattern {
                    matched = true;
                    match_type = "filename".to_string();
                }
            }

            // Content search
            if let Some(ref query) = input.content_query {
                if let Ok(content) = self.ctx.vfs.read_file(&workspace_id, &vnode.path).await {
                    let content_str = String::from_utf8_lossy(&content);
                    let content_matches = if input.case_sensitive {
                        content_str.contains(query)
                    } else {
                        content_str.to_lowercase().contains(&query.to_lowercase())
                    };

                    if content_matches {
                        matched = true;
                        match_type = if match_type.is_empty() {
                            "content".to_string()
                        } else {
                            "both".to_string()
                        };

                        // Extract snippet
                        if let Some(pos) = content_str.to_lowercase().find(&query.to_lowercase()) {
                            let start = pos.saturating_sub(50);
                            let end = (pos + query.len() + 50).min(content_str.len());
                            content_snippet = Some(format!("...{}...", &content_str[start..end]));
                        }
                    }
                }
            }

            // If no pattern or query, match all files
            if input.pattern.is_none() && input.content_query.is_none() {
                matched = true;
                match_type = "all".to_string();
            }

            if matched {
                total_found += 1;
                if matches.len() < input.max_results {
                    matches.push(SearchMatch {
                        path: vnode.path.to_string(),
                        node_type: format!("{:?}", vnode.node_type).to_lowercase(),
                        match_type,
                        size_bytes: vnode.size_bytes,
                        content_snippet,
                    });
                }
            }
        }

        let output = SearchOutput {
            total: total_found,
            truncated: total_found > matches.len(),
            matches,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.compare
// =============================================================================

pub struct WorkspaceCompareTool {
    ctx: WorkspaceContext,
}

impl WorkspaceCompareTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CompareInput {
    workspace_a_id: String,
    workspace_b_id: String,
    #[serde(default)]
    include_content_diff: bool,
    #[serde(default = "default_limit")]
    max_diffs: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CompareOutput {
    workspace_a_id: String,
    workspace_b_id: String,
    files_only_in_a: Vec<String>,
    files_only_in_b: Vec<String>,
    files_modified: Vec<FileDiff>,
    files_identical: usize,
    total_differences: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FileDiff {
    path: String,
    size_a: usize,
    size_b: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_diff: Option<String>,
}

#[async_trait]
impl Tool for WorkspaceCompareTool {
    fn name(&self) -> &str {
        "cortex.workspace.compare"
    }

    fn description(&self) -> Option<&str> {
        Some("Compares two workspaces and identifies differences in files, content, and structure.")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CompareInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CompareInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let workspace_a = Uuid::parse_str(&input.workspace_a_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace A ID: {}", e)))?;
        let workspace_b = Uuid::parse_str(&input.workspace_b_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace B ID: {}", e)))?;

        info!("Comparing workspaces: {} vs {}", workspace_a, workspace_b);

        let root = VirtualPath::root();

        // Get all files from both workspaces
        let vnodes_a = self.ctx.vfs.list_directory(&workspace_a, &root, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list workspace A: {}", e)))?;
        let vnodes_b = self.ctx.vfs.list_directory(&workspace_b, &root, true).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list workspace B: {}", e)))?;

        // Build path maps
        let mut map_a: HashMap<String, _> = vnodes_a
            .into_iter()
            .filter(|v| v.is_file())
            .map(|v| (v.path.to_string(), v))
            .collect();

        let mut map_b: HashMap<String, _> = vnodes_b
            .into_iter()
            .filter(|v| v.is_file())
            .map(|v| (v.path.to_string(), v))
            .collect();

        let mut files_only_in_a = Vec::new();
        let mut files_only_in_b = Vec::new();
        let mut files_modified = Vec::new();
        let mut files_identical = 0;

        // Find files only in A and modified files
        for (path, vnode_a) in &map_a {
            if let Some(vnode_b) = map_b.remove(path) {
                // File exists in both
                if vnode_a.content_hash != vnode_b.content_hash {
                    // Content differs
                    if files_modified.len() < input.max_diffs {
                        let content_diff = if input.include_content_diff {
                            Some(format!("Hash A: {}, Hash B: {}",
                                vnode_a.content_hash.as_deref().unwrap_or("none"),
                                vnode_b.content_hash.as_deref().unwrap_or("none")))
                        } else {
                            None
                        };

                        files_modified.push(FileDiff {
                            path: path.clone(),
                            size_a: vnode_a.size_bytes,
                            size_b: vnode_b.size_bytes,
                            content_diff,
                        });
                    }
                } else {
                    files_identical += 1;
                }
            } else {
                // File only in A
                files_only_in_a.push(path.clone());
            }
        }

        // Remaining files in B are only in B
        files_only_in_b = map_b.keys().cloned().collect();

        let total_differences = files_only_in_a.len() + files_only_in_b.len() + files_modified.len();

        let output = CompareOutput {
            workspace_a_id: workspace_a.to_string(),
            workspace_b_id: workspace_b.to_string(),
            files_only_in_a,
            files_only_in_b,
            files_modified,
            files_identical,
            total_differences,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.workspace.merge
// =============================================================================

pub struct WorkspaceMergeTool {
    ctx: WorkspaceContext,
}

impl WorkspaceMergeTool {
    pub fn new(ctx: WorkspaceContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MergeInput {
    source_workspace_id: String,
    target_workspace_id: String,
    #[serde(default = "default_merge_strategy")]
    strategy: String, // "manual", "auto", "prefer_source", "prefer_target"
}

fn default_merge_strategy() -> String {
    "manual".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
struct MergeOutput {
    changes_applied: usize,
    conflicts_count: usize,
    auto_resolved: usize,
    conflicts: Vec<ConflictInfo>,
    success: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ConflictInfo {
    path: String,
    conflict_type: String,
    source_hash: String,
    target_hash: String,
}

#[async_trait]
impl Tool for WorkspaceMergeTool {
    fn name(&self) -> &str {
        "cortex.workspace.merge"
    }

    fn description(&self) -> Option<&str> {
        Some("Merges changes from one workspace into another with configurable conflict resolution strategies.")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(MergeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: MergeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let source_id = Uuid::parse_str(&input.source_workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid source workspace ID: {}", e)))?;
        let target_id = Uuid::parse_str(&input.target_workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid target workspace ID: {}", e)))?;

        info!("Merging workspace {} into {}", source_id, target_id);

        // Parse merge strategy
        let strategy = match input.strategy.as_str() {
            "manual" => MergeStrategy::Manual,
            "auto" => MergeStrategy::AutoMerge,
            "prefer_source" => MergeStrategy::PreferFork,
            "prefer_target" => MergeStrategy::PreferTarget,
            _ => return Err(ToolError::ExecutionFailed(format!(
                "Invalid merge strategy: {}. Use: manual, auto, prefer_source, prefer_target",
                input.strategy
            ))),
        };

        // Perform merge using fork manager
        let merge_report = self.ctx.fork_manager.merge_fork(&source_id, &target_id, strategy).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Merge failed: {}", e)))?;

        // Convert conflicts to output format
        let conflicts: Vec<ConflictInfo> = merge_report.conflicts.iter().map(|c| {
            // Compute content hashes using blake3
            use blake3::Hasher;
            let mut fork_hasher = Hasher::new();
            fork_hasher.update(c.fork_content.as_bytes());
            let fork_hash = fork_hasher.finalize().to_hex().to_string();

            let mut target_hasher = Hasher::new();
            target_hasher.update(c.target_content.as_bytes());
            let target_hash = target_hasher.finalize().to_hex().to_string();

            // Determine conflict type based on content
            let conflict_type = if c.fork_content.is_empty() && !c.target_content.is_empty() {
                "delete-modify".to_string()
            } else if !c.fork_content.is_empty() && c.target_content.is_empty() {
                "modify-delete".to_string()
            } else if c.fork_content.is_empty() && c.target_content.is_empty() {
                "add-add".to_string()
            } else {
                "modify-modify".to_string()
            };

            ConflictInfo {
                path: c.path.to_string(),
                conflict_type,
                source_hash: fork_hash[..16].to_string(), // First 16 chars of hash
                target_hash: target_hash[..16].to_string(),
            }
        }).collect();

        let output = MergeOutput {
            changes_applied: merge_report.changes_applied,
            conflicts_count: merge_report.conflicts_count,
            auto_resolved: merge_report.auto_resolved,
            conflicts,
            success: merge_report.errors.is_empty() && merge_report.conflicts_count == 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}
