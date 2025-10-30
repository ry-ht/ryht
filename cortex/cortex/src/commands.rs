//! CLI command implementations.
//!
//! This module contains the complete implementation of all Cortex CLI commands.

use crate::config::CortexConfig;
use crate::mcp::CortexMcpServer;
use crate::output::{self, format_bytes, OutputFormat, TableBuilder};
use anyhow::{Context, Result};
use cortex_memory::CognitiveManager;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, SurrealDBManager};
use cortex_storage::session::SessionManager;
use cortex_vfs::{
    ExternalProjectLoader, FlushOptions, FlushScope, MaterializationEngine, VirtualFileSystem,
    VirtualPath, VNode, Workspace, SyncSource, SyncSourceType,
    SyncSourceStatus,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, error};

// ============================================================================
// Logging Utilities
// ============================================================================

/// Initialize file-based logging (for MCP stdio mode - no stdout/stderr!)
fn init_file_logging(log_file: &str, log_level: &str) -> Result<()> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    // Create log directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(log_file).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open log file for appending
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    // Create filter from log level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Initialize file-only subscriber (NO stdout/stderr!)
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(Arc::new(file)))
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}

// ============================================================================
// Init Command
// ============================================================================

/// Initialize a new Cortex workspace
pub async fn init_workspace(
    name: String,
    path: Option<PathBuf>,
) -> Result<()> {
    let spinner = output::spinner("Initializing Cortex workspace...");

    let config = CortexConfig::load()?;
    let workspace_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Create workspace directory
    std::fs::create_dir_all(&workspace_path)
        .context("Failed to create workspace directory")?;

    // Create .cortex directory
    let cortex_dir = workspace_path.join(".cortex");
    std::fs::create_dir_all(&cortex_dir)
        .context("Failed to create .cortex directory")?;

    // Create workspace config
    let mut workspace_config = config.clone();
    workspace_config.default_workspace = Some(name.clone());
    workspace_config.save_project()?;

    // Initialize storage
    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage.clone());

    // Create workspace in VFS
    let workspace_id = Uuid::new_v4();

    // Create metadata
    let metadata: HashMap<String, serde_json::Value> = HashMap::new();

    // Create sync source for local path
    let sync_sources = vec![SyncSource {
        id: Uuid::new_v4(),
        source: SyncSourceType::LocalPath {
            path: workspace_path.display().to_string(),
            watch: false,
        },
        read_only: false,
        priority: 100,
        last_sync: Some(chrono::Utc::now()),
        status: SyncSourceStatus::Synced,
        metadata: HashMap::new(),
    }];

    // Save workspace metadata to database using raw query to avoid serialization issues
    let conn = storage.acquire().await?;
    let query = format!(r#"
        CREATE workspace:`{}` CONTENT {{
            name: $name,
            namespace: $namespace,
            sync_sources: $sync_sources,
            metadata: $metadata,
            read_only: $read_only,
            parent_workspace: $parent_workspace,
            fork_metadata: $fork_metadata,
            dependencies: $dependencies,
            created_at: <datetime> $created_at,
            updated_at: <datetime> $updated_at
        }}
    "#, workspace_id);

    conn.connection()
        .query(&query)
        .bind(("name", name.clone()))
        .bind(("namespace", format!("workspace_{}", workspace_id)))
        .bind(("sync_sources", sync_sources.clone()))
        .bind(("metadata", metadata.clone()))
        .bind(("read_only", false))
        .bind(("parent_workspace", None::<Uuid>))
        .bind(("fork_metadata", None::<String>))
        .bind(("dependencies", Vec::<String>::new()))
        .bind(("created_at", chrono::Utc::now().to_rfc3339()))
        .bind(("updated_at", chrono::Utc::now().to_rfc3339()))
        .await
        .context("Failed to create workspace in database")?;

    // Initialize root directory in VFS
    let root_path = VirtualPath::new(".")?;
    vfs.create_directory(&workspace_id, &root_path, true).await
        .context("Failed to create root directory in VFS")?;

    spinner.finish_and_clear();

    output::success(format!("Initialized Cortex workspace: {}", name));
    output::kv("Workspace ID", workspace_id);
    output::kv("Path", workspace_path.display());
    // Config is now at ~/.ryht/config.toml (unified config)
    if let Ok(config_path) = cortex_core::config::GlobalConfig::config_path() {
        output::kv("Config", config_path.display());
    }

    Ok(())
}

// ============================================================================
// Workspace Management Commands
// ============================================================================

/// Create a new workspace
pub async fn workspace_create(
    name: String,
    root_path: Option<PathBuf>,
    auto_import: bool,
    process_code: bool,
    max_file_size_mb: u64,
) -> Result<()> {
    let spinner = output::spinner("Creating workspace...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage.clone());

    let workspace_id = Uuid::new_v4();

    // Create sync source for local path if provided
    let sync_sources = if let Some(ref path) = root_path {
        vec![SyncSource {
            id: Uuid::new_v4(),
            source: SyncSourceType::LocalPath {
                path: path.display().to_string(),
                watch: false,
            },
            read_only: false,
            priority: 100,
            last_sync: None,
            status: SyncSourceStatus::Unsynced,
            metadata: HashMap::new(),
        }]
    } else {
        vec![]
    };

    // Create metadata
    let metadata = HashMap::new();

    let workspace = Workspace {
        id: workspace_id,
        name: name.clone(),
        namespace: format!("workspace_{}", workspace_id),
        sync_sources,
        metadata,
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        dependencies: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Save workspace to database using VFS method which handles serialization properly
    vfs.create_workspace(&workspace).await
        .context("Failed to create workspace in database")?;

    // Initialize root directory in VFS
    let root = VirtualPath::new(".")?;
    vfs.create_directory(&workspace_id, &root, true).await
        .context("Failed to create root directory in VFS")?;

    // Import if requested and root_path is provided
    if auto_import && root_path.is_some() {
        let path = root_path.as_ref().unwrap();
        let loader = ExternalProjectLoader::new(vfs.clone());

        let vfs_opts = cortex_vfs::ImportOptions {
            read_only: false,
            create_fork: false,
            namespace: workspace.namespace.clone(),
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
            process_code,
            generate_embeddings: false,
            max_file_size_bytes: Some((max_file_size_mb * 1024 * 1024) as usize),
        };

        spinner.finish_and_clear();
        let import_spinner = output::spinner("Importing project...");

        // Use import_into_workspace to import into the already created workspace
        match loader.import_into_workspace(&workspace_id, path, vfs_opts).await {
            Ok(report) => {
                import_spinner.finish_and_clear();
                output::success(format!("Created workspace: {}", name));
                output::kv("ID", workspace_id);
                output::kv("Files imported", report.files_imported);
                output::kv("Directories imported", report.directories_imported);
                output::kv("Total size", format_bytes(report.bytes_imported as u64));

                if !report.errors.is_empty() {
                    output::warning(format!("{} errors occurred during import", report.errors.len()));
                }
            }
            Err(e) => {
                import_spinner.finish_and_clear();
                output::warning(format!("Import failed: {}", e));
                output::success(format!("Created workspace: {}", name));
                output::kv("ID", workspace_id);
            }
        }
    } else {
        spinner.finish_and_clear();
        output::success(format!("Created workspace: {}", name));
        output::kv("ID", workspace_id);
    }

    Ok(())
}

/// List all workspaces
pub async fn workspace_list(status: Option<String>, limit: usize, format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let _vfs = VirtualFileSystem::new(storage.clone());

    // Fetch workspaces from database
    let conn = storage.acquire().await?;

    debug!("Fetching workspaces using query with meta::id pattern");

    // Select only specific fields to avoid serialization issues with complex nested types
    let query = if let Some(ref status_filter) = status {
        format!("SELECT <string>meta::id(id) as id, name, namespace, sync_sources, metadata, read_only, parent_workspace, fork_metadata, dependencies, created_at, updated_at FROM workspace WHERE metadata.archived = $archived LIMIT {}", limit)
    } else {
        format!("SELECT <string>meta::id(id) as id, name, namespace, sync_sources, metadata, read_only, parent_workspace, fork_metadata, dependencies, created_at, updated_at FROM workspace LIMIT {}", limit)
    };

    // Intermediate struct to deserialize Thing IDs as strings
    #[derive(serde::Deserialize)]
    struct WorkspaceWithStringId {
        id: String,
        #[serde(flatten)]
        rest: serde_json::Value,
    }

    let mut response = if let Some(ref status_filter) = status {
        let archived = status_filter == "archived";
        conn.connection()
            .query(&query)
            .bind(("archived", archived))
            .await
            .context("Failed to fetch workspaces from database")?
    } else {
        conn.connection()
            .query(&query)
            .await
            .context("Failed to fetch workspaces from database")?
    };

    let workspaces_raw: Vec<WorkspaceWithStringId> = match response.take::<Vec<WorkspaceWithStringId>>(0) {
        Ok(records) => {
            debug!("Successfully deserialized {} workspace records", records.len());
            records
        }
        Err(e) => {
            error!("Failed to deserialize workspaces: {}", e);
            error!("Error details: {:?}", e);
            Vec::new()
        }
    };
    debug!("Fetched {} workspace records", workspaces_raw.len());

    // Now convert to Workspace structs using the proven pattern
    let workspaces: Vec<Workspace> = workspaces_raw
        .into_iter()
        .filter_map(|w| {
            // Extract UUID from "workspace:uuid" format
            let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
            let mut workspace_json = w.rest;

            // Insert the cleaned UUID
            if let Some(obj) = workspace_json.as_object_mut() {
                obj.insert("id".to_string(), serde_json::Value::String(uuid_str.to_string()));
            }

            serde_json::from_value::<Workspace>(workspace_json).ok()
        })
        .collect();

    match format {
        OutputFormat::Json => {
            output::output(&workspaces, format)?;
        }
        _ => {
            if workspaces.is_empty() {
                output::info("No workspaces found. Create one with 'cortex workspace create'");
                return Ok(());
            }

            let table = TableBuilder::new()
                .header(vec!["ID", "Name", "Created", "Root Path"]);

            let mut table_with_rows = table;
            for ws in &workspaces {
                let path_str = ws.source_path()
                    .unwrap_or_else(|| "N/A".to_string());

                let created = format_relative_time(&ws.created_at);

                table_with_rows = table_with_rows.row(vec![
                    ws.id.to_string(),
                    ws.name.clone(),
                    created,
                    path_str,
                ]);
            }

            table_with_rows.print();
        }
    }

    Ok(())
}

/// Format a timestamp as relative time (e.g., "2 hours ago")
fn format_relative_time(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Delete a workspace
pub async fn workspace_delete(workspace_id: String, confirm: bool) -> Result<()> {
    if !confirm {
        output::error("Deletion requires confirmation. Use --confirm flag to proceed.");
        return Err(anyhow::anyhow!("Confirmation required"));
    }

    let spinner = output::spinner("Deleting workspace...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let _vfs = VirtualFileSystem::new(storage.clone());

    // Find workspace by name or ID
    let conn = storage.acquire().await?;

    // Try to parse as UUID first
    let ws_id = if let Ok(uuid) = Uuid::parse_str(&workspace_id) {
        uuid
    } else {
        // Search by name - use workspace list command which handles IDs correctly
        let mut response = conn.connection()
            .query("SELECT meta::id(id) as id_str FROM workspace WHERE name = $name LIMIT 1")
            .bind(("name", workspace_id.clone()))
            .await
            .context("Failed to query workspace")?;

        #[derive(serde::Deserialize)]
        struct WorkspaceIdResult {
            id_str: String,
        }

        let results: Vec<WorkspaceIdResult> = response.take(0)?;

        if results.is_empty() {
            return Err(anyhow::anyhow!("Workspace not found: {}", workspace_id));
        }

        // Extract UUID from "workspace:uuid" format
        let id_str = &results[0].id_str;
        let uuid_part = id_str.strip_prefix("workspace:").unwrap_or(id_str);
        Uuid::parse_str(uuid_part)?
    };

    // Delete all VNodes in this workspace
    let mut response = conn.connection()
        .query("DELETE FROM vnode WHERE workspace_id = $workspace_id")
        .bind(("workspace_id", ws_id.to_string()))
        .await
        .context("Failed to delete workspace vnodes")?;

    // Get count of deleted vnodes
    let deleted_vnodes: Vec<serde_json::Value> = response.take(0)?;
    let vnode_count = deleted_vnodes.len();

    // Delete the workspace using the SDK delete method (same pattern as other services)
    // Note: We ignore the return value to avoid deserialization issues with Thing IDs
    let _: Option<Workspace> = conn.connection()
        .delete(("workspace", ws_id.to_string()))
        .await
        .context("Failed to delete workspace from database")?;

    spinner.finish_and_clear();
    output::success(format!("Deleted workspace: {} ({} vnodes deleted)", workspace_id, vnode_count));

    Ok(())
}

// ============================================================================
// Ingestion Commands
// ============================================================================

/// Ingest files or directories into Cortex
pub async fn ingest_path(
    path: PathBuf,
    workspace: Option<String>,
    recursive: bool,
) -> Result<()> {
    let config = CortexConfig::load()?;

    let storage = create_storage(&config).await?;

    // Create a temporary session for this ingestion
    let (session_id, workspace_id, workspace_name) = create_temp_session(storage.clone(), workspace, &config).await
        .context("Failed to create session for ingestion")?;

    output::header(format!("Ingesting: {}", path.display()));
    output::kv("Workspace", &workspace_name);
    output::kv("Session", &session_id.to_string());
    output::kv("Recursive", recursive);

    let spinner = output::spinner("Loading project...");

    let vfs = VirtualFileSystem::new(storage.clone());
    let loader = ExternalProjectLoader::new(vfs.clone());

    // Import project into existing workspace
    let options = cortex_vfs::ImportOptions {
        read_only: false,
        create_fork: false,
        namespace: workspace_name.clone(),
        include_patterns: vec!["**/*".to_string()],
        exclude_patterns: vec![
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/.git/**".to_string(),
            "**/dist/**".to_string(),
            "**/build/**".to_string(),
        ],
        max_depth: None,
        process_code: true,
        generate_embeddings: false,
        max_file_size_bytes: Some(10 * 1024 * 1024), // 10 MB default
    };

    let report = loader.import_into_workspace(&workspace_id, &path, options).await?;

    spinner.finish_and_clear();

    output::success("Ingestion complete");
    output::kv("Files imported", report.files_imported);
    output::kv("Directories imported", report.directories_imported);
    output::kv("Total size", format_bytes(report.bytes_imported as u64));
    output::kv("Duration", format!("{:.2}s", report.duration_ms as f64 / 1000.0));

    if !report.errors.is_empty() {
        output::warning(format!("{} errors occurred:", report.errors.len()));
        for error in report.errors.iter().take(5) {
            eprintln!("  - {}", error);
        }
        if report.errors.len() > 5 {
            eprintln!("  ... and {} more", report.errors.len() - 5);
        }
    }

    Ok(())
}

// ============================================================================
// Search Commands
// ============================================================================

/// Search across Cortex memory
pub async fn search_memory(
    query: String,
    workspace: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> Result<()> {
    let config = CortexConfig::load()?;
    let workspace_name = workspace.or(config.default_workspace.clone());

    let spinner = output::spinner("Searching...");

    let storage = create_storage(&config).await?;
    let memory = CognitiveManager::new(storage);

    // Create a memory query
    let memory_query = cortex_memory::types::MemoryQuery::new(query.clone())
        .with_limit(limit)
        .with_threshold(0.6);

    // Generate a simple term-based embedding for the query
    // In production, use a proper embedding model (OpenAI, sentence-transformers, etc.)
    // For now, use a simple keyword-based approach with normalized term frequencies
    let query_terms: Vec<&str> = query.split_whitespace().collect();
    let embedding = if query_terms.is_empty() {
        vec![0.0f32; 384] // Fallback empty embedding
    } else {
        // Create a simple one-hot-like embedding based on term positions
        // This is a placeholder - real embeddings would use semantic models
        let mut emb = vec![0.0f32; 384];
        for (i, term) in query_terms.iter().enumerate().take(384) {
            // Simple hash-based position assignment
            let pos = (term.len() * 17 + i * 31) % 384;
            emb[pos] = 1.0 / (query_terms.len() as f32).sqrt(); // Normalized
        }
        emb
    };

    // Perform cross-memory search
    let cross_query = cortex_memory::CrossMemoryQuery::new(std::sync::Arc::new(memory));
    let results = cross_query.search_all(&memory_query, &embedding).await
        .context("Failed to search memory")?;

    spinner.finish_and_clear();

    // Filter by workspace if specified
    let filtered_results: Vec<_> = if let Some(ref ws_name) = workspace_name {
        results.into_iter()
            .filter(|r| match r {
                cortex_memory::query::UnifiedMemoryResult::Episode(ep) => {
                    ep.item.agent_id.contains(ws_name)
                }
                cortex_memory::query::UnifiedMemoryResult::SemanticUnit(unit) => {
                    unit.item.file_path.contains(ws_name)
                }
                cortex_memory::query::UnifiedMemoryResult::Pattern(pattern) => {
                    pattern.item.context.contains(ws_name)
                }
            })
            .collect()
    } else {
        results
    };

    match format {
        OutputFormat::Json => {
            let json_results: Vec<serde_json::Value> = filtered_results.iter().map(|r| {
                serde_json::json!({
                    "type": match r {
                        cortex_memory::query::UnifiedMemoryResult::Episode(_) => "episode",
                        cortex_memory::query::UnifiedMemoryResult::SemanticUnit(_) => "code_unit",
                        cortex_memory::query::UnifiedMemoryResult::Pattern(_) => "pattern",
                    },
                    "relevance": r.relevance(),
                    "similarity": r.similarity(),
                })
            }).collect();
            output::output(&json_results, format)?;
        }
        _ => {
            output::header(format!("Search Results for '{}'", query));

            if filtered_results.is_empty() {
                output::info("No results found");
            } else {
                for (idx, result) in filtered_results.iter().enumerate() {
                    println!("\n{}. [Score: {:.2}]", idx + 1, result.combined_score());
                    match result {
                        cortex_memory::query::UnifiedMemoryResult::Episode(ep) => {
                            println!("   Type: Episode");
                            println!("   Task: {}", ep.item.task_description);
                            println!("   Agent: {}", ep.item.agent_id);
                        }
                        cortex_memory::query::UnifiedMemoryResult::SemanticUnit(unit) => {
                            println!("   Type: Code Unit");
                            println!("   Name: {}", unit.item.name);
                            println!("   File: {}", unit.item.file_path);
                        }
                        cortex_memory::query::UnifiedMemoryResult::Pattern(pattern) => {
                            println!("   Type: Pattern");
                            println!("   Name: {}", pattern.item.name);
                            println!("   Pattern Type: {:?}", pattern.item.pattern_type);
                        }
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}

// ============================================================================
// List Commands
// ============================================================================

/// List projects in workspace
pub async fn list_projects(workspace: Option<String>, format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let workspace_name = workspace.or(config.default_workspace.clone());

    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Get workspaces (projects are represented as workspaces in our system)
    // Use raw query to avoid serialization issues with Thing IDs
    let query = "SELECT *, <string>meta::id(id) as id FROM workspace";
    let mut response = conn.connection().query(query).await
        .context("Failed to fetch workspaces from database")?;

    #[derive(serde::Deserialize)]
    struct WorkspaceWithStringId {
        id: String,
        #[serde(flatten)]
        rest: serde_json::Value,
    }

    let workspaces_raw: Vec<WorkspaceWithStringId> = response.take(0)?;
    let mut workspaces: Vec<Workspace> = workspaces_raw
        .into_iter()
        .filter_map(|w| {
            // Extract UUID from "workspace:uuid" format
            let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
            let mut workspace_json = w.rest;
            if let Some(obj) = workspace_json.as_object_mut() {
                obj.insert("id".to_string(), serde_json::Value::String(uuid_str.to_string()));
            }
            serde_json::from_value::<Workspace>(workspace_json).ok()
        })
        .collect();

    // Filter by workspace name if specified
    if let Some(ref ws_name) = workspace_name {
        workspaces.retain(|w| w.name == *ws_name || w.id.to_string() == *ws_name);
    }

    match format {
        OutputFormat::Json => {
            output::output(&workspaces, format)?;
        }
        _ => {
            output::header("Projects");

            if workspaces.is_empty() {
                output::info("No projects found");
            } else {
                for ws in &workspaces {
                    println!("  {} - {}", ws.name, ws.id);
                    if let Some(path) = ws.source_path() {
                        println!("    Path: {}", path);
                    }
                    println!("    Namespace: {}", ws.namespace);
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// List documents in workspace
pub async fn list_documents(workspace: Option<String>, format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let workspace_name = workspace.or(config.default_workspace.clone());

    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Find workspace ID if workspace name is provided
    let workspace_id = if let Some(ref ws_name) = workspace_name {
        let mut response = conn.connection()
            .query("SELECT *, <string>meta::id(id) as id FROM workspace WHERE name = $name LIMIT 1")
            .bind(("name", ws_name.clone()))
            .await
            .context("Failed to query workspace")?;

        // Parse with string IDs and convert to UUIDs
        #[derive(serde::Deserialize)]
        struct WorkspaceWithStringId {
            id: String,
            #[serde(flatten)]
            rest: serde_json::Value,
        }

        let workspaces_raw: Vec<WorkspaceWithStringId> = response.take(0)?;

        if workspaces_raw.is_empty() {
            return Err(anyhow::anyhow!("Workspace not found: {}", ws_name));
        }

        let w = &workspaces_raw[0];
        // Extract UUID from "workspace:uuid" format
        let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
        Some(Uuid::parse_str(uuid_str)?)
    } else {
        None
    };

    // Query vnodes of type Document
    let mut response = if let Some(ws_id) = workspace_id {
        conn.connection().query("SELECT * FROM vnode WHERE workspace_id = $workspace_id AND node_type = 'document'")
            .bind(("workspace_id", ws_id.to_string()))
            .await
            .context("Failed to fetch documents from database")?
    } else {
        conn.connection().query("SELECT * FROM vnode WHERE node_type = 'document'")
            .await
            .context("Failed to fetch documents from database")?
    };
    let documents: Vec<VNode> = response.take(0)?;

    match format {
        OutputFormat::Json => {
            output::output(&documents, format)?;
        }
        _ => {
            output::header("Documents");

            if documents.is_empty() {
                output::info("No documents found");
            } else {
                for doc in &documents {
                    println!("  {} ({})", doc.path, format_bytes(doc.size_bytes as u64));
                    println!("    Workspace: {}", doc.workspace_id);
                    if let Some(ref lang) = doc.language {
                        println!("    Language: {:?}", lang);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// List memory episodes
pub async fn list_episodes(
    workspace: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> Result<()> {
    let config = CortexConfig::load()?;
    let workspace_name = workspace.or(config.default_workspace.clone());

    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Query episodes from database
    let mut response = if let Some(ref ws_name) = workspace_name {
        conn.connection().query("SELECT * FROM episodic_memory WHERE agent_id = $workspace ORDER BY created_at DESC LIMIT $limit")
            .bind(("workspace", ws_name.clone()))
            .bind(("limit", limit))
            .await
            .context("Failed to fetch episodes from database")?
    } else {
        conn.connection().query("SELECT * FROM episodic_memory ORDER BY created_at DESC LIMIT $limit")
            .bind(("limit", limit))
            .await
            .context("Failed to fetch episodes from database")?
    };
    let mut episodes: Vec<cortex_memory::types::EpisodicMemory> = response.take(0)?;

    // Limit results
    episodes.truncate(limit);

    match format {
        OutputFormat::Json => {
            output::output(&episodes, format)?;
        }
        _ => {
            output::header(format!("Recent Episodes (limit: {})", limit));

            if episodes.is_empty() {
                output::info("No episodes found");
            } else {
                for (idx, episode) in episodes.iter().enumerate() {
                    println!("\n{}. {} [{}]",
                        idx + 1,
                        episode.task_description,
                        format_relative_time(&episode.created_at)
                    );
                    println!("   ID: {}", episode.id);
                    println!("   Agent: {}", episode.agent_id);
                    println!("   Type: {:?}", episode.episode_type);
                    println!("   Outcome: {:?}", episode.outcome);
                    println!("   Duration: {}s", episode.duration_seconds);
                    println!("   Files touched: {}", episode.files_touched.len());
                }
                println!();
            }
        }
    }

    Ok(())
}

// ============================================================================
// MCP Server Commands
// ============================================================================

/// Start the MCP server
pub async fn serve_mcp(address: String, port: u16) -> Result<()> {
    output::header("Starting Cortex MCP Server");
    output::kv("Address", &address);
    output::kv("Port", port);

    let config = CortexConfig::load()?;

    // Start MCP server
    let server = CortexMcpServer::new().await?;

    output::success("MCP server started successfully");
    output::info(format!("Listening on {}:{}", address, port));
    output::info("Press Ctrl+C to stop");

    // Serve over stdio
    server.serve_stdio().await?;

    Ok(())
}

// ============================================================================
// VFS Flush Commands
// ============================================================================

/// Flush VFS to disk
pub async fn flush_vfs(
    workspace: String,
    target_path: PathBuf,
    scope: FlushScope,
) -> Result<()> {
    let config = CortexConfig::load()?;

    output::header("Flushing VFS to disk");
    output::kv("Workspace", &workspace);
    output::kv("Target", target_path.display());
    output::kv("Scope", format!("{:?}", scope));

    let spinner = output::spinner("Materializing files...");

    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage);
    let engine = MaterializationEngine::new(vfs);

    let options = FlushOptions {
        preserve_permissions: true,
        preserve_timestamps: true,
        create_backup: false,
        atomic: true,
        parallel: true,
        max_workers: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
    };

    let report = engine.flush(scope, &target_path, options).await?;

    spinner.finish_and_clear();

    output::success("Flush complete");
    output::kv("Files written", report.files_written);
    output::kv("Directories created", report.directories_created);
    output::kv("Total size", format_bytes(report.bytes_written as u64));
    output::kv("Duration", format!("{:.2}s", report.duration_ms as f64 / 1000.0));

    if !report.errors.is_empty() {
        output::warning(format!("{} errors occurred:", report.errors.len()));
        for error in report.errors.iter().take(5) {
            eprintln!("  - {}", error);
        }
    }

    Ok(())
}

// ============================================================================
// Stats Commands
// ============================================================================

/// Show system statistics
pub async fn show_stats(format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;

    let spinner = output::spinner("Gathering statistics...");

    // Gather actual metrics from storage
    let system_stats = storage.health_status();

    let stats = serde_json::json!({
        "workspaces": 0, // VFS workspaces not tracked by core storage
        "projects": 0,  // Would need separate query
        "documents": 0, // Would need separate query
        "chunks": 0,    // Would need separate query
        "total_size_bytes": 0, // Would need separate query
        "memory": {
            "episodes": 0,    // Would need separate query
            "embeddings": 0,  // Would need separate query
        },
        "database": {
            "healthy": system_stats.healthy,
            "pool_size": system_stats.pool_size,
            "available_connections": system_stats.available_connections,
            "total_connections": system_stats.total_connections,
            "failed_connections": system_stats.failed_connections,
        },
    });

    spinner.finish_and_clear();

    match format {
        OutputFormat::Json => {
            output::output(&stats, format)?;
        }
        _ => {
            output::header("Cortex System Statistics");
            println!("\nDatabase Health: {}", if system_stats.healthy { "OK" } else { "ERROR" });
            println!("Pool Size: {}/{}", system_stats.available_connections, system_stats.pool_size);
            println!("Total Connections: {}", system_stats.total_connections);
            println!("Failed Connections: {}", system_stats.failed_connections);
            println!("\nCircuit Breaker: {:?}", system_stats.circuit_breaker_state);
        }
    }

    Ok(())
}

// ============================================================================
// Config Commands
// ============================================================================

/// Get a configuration value
pub async fn config_get(key: String) -> Result<()> {
    let config = CortexConfig::load()?;

    match config.get(&key) {
        Some(value) => {
            println!("{}", value);
            Ok(())
        }
        None => {
            output::error(format!("Unknown configuration key: {}", key));
            Err(anyhow::anyhow!("Unknown key"))
        }
    }
}

/// Set a configuration value
pub async fn config_set(key: String, value: String, global: bool) -> Result<()> {
    let mut config = CortexConfig::load()?;

    config.set(&key, &value)?;

    if global {
        config.save_default()?;
        output::success(format!("Set {} = {} (global)", key, value));
    } else {
        config.save_project()?;
        output::success(format!("Set {} = {} (project)", key, value));
    }

    Ok(())
}

/// List all configuration values
pub async fn config_list() -> Result<()> {
    let config = CortexConfig::load()?;

    output::header("Configuration");

    println!("\nDatabase:");
    println!("  connection_string: {}", config.database.connection_string);
    println!("  namespace: {}", config.database.namespace);
    println!("  database: {}", config.database.database);
    println!("  pool_size: {}", config.database.pool_size);

    println!("\nStorage:");
    println!("  data_dir: {}", config.storage.data_dir.display());
    println!("  cache_size_mb: {}", config.storage.cache_size_mb);
    println!("  compression_enabled: {}", config.storage.compression_enabled);

    println!("\nMCP:");
    println!("  enabled: {}", config.mcp.enabled);
    println!("  address: {}", config.mcp.address);
    println!("  port: {}", config.mcp.port);

    if let Some(workspace) = &config.default_workspace {
        println!("\nDefault Workspace: {}", workspace);
    }

    Ok(())
}

// ============================================================================
// Agent Session Commands
// ============================================================================

/// Create a new agent session
pub async fn agent_create(name: String, agent_type: String) -> Result<()> {
    let spinner = output::spinner("Creating agent session...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    let session_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Create agent session in database
    let _: Option<serde_json::Value> = conn.connection()
        .create(("agent_sessions", session_id.clone()))
        .content(serde_json::json!({
            "id": session_id,
            "name": name,
            "agent_type": agent_type,
            "created_at": now,
            "last_active": now,
            "metadata": serde_json::json!({}),
        }))
        .await
        .context("Failed to create agent session")?;

    spinner.finish_and_clear();

    output::success(format!("Created agent session: {}", name));
    output::kv("Session ID", &session_id);
    output::kv("Type", &agent_type);

    Ok(())
}

/// List agent sessions
pub async fn agent_list(format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Query agent sessions from database
    let sessions: Vec<cortex_core::types::AgentSession> = conn.connection()
        .select("agent_sessions")
        .await
        .context("Failed to fetch agent sessions from database")?;

    match format {
        OutputFormat::Json => {
            output::output(&sessions, format)?;
        }
        _ => {
            output::header("Agent Sessions");
            if sessions.is_empty() {
                output::info("No active sessions");
            } else {
                let table = TableBuilder::new()
                    .header(vec!["Session ID", "Name", "Type", "Created", "Last Active"]);

                let mut table_with_rows = table;
                for session in sessions {
                    table_with_rows = table_with_rows.row(vec![
                        session.id,
                        session.name,
                        session.agent_type,
                        session.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        session.last_active.format("%Y-%m-%d %H:%M:%S").to_string(),
                    ]);
                }

                table_with_rows.print();
            }
        }
    }

    Ok(())
}

/// Delete an agent session
pub async fn agent_delete(session_id: String) -> Result<()> {
    if !output::confirm(format!("Delete agent session '{}'?", session_id))? {
        output::info("Cancelled");
        return Ok(());
    }

    let spinner = output::spinner("Deleting agent session...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Delete agent session from database
    let _: Option<cortex_core::types::AgentSession> = conn.connection()
        .delete(("agent_sessions", &session_id))
        .await
        .context("Failed to delete agent session")?;

    spinner.finish_and_clear();
    output::success(format!("Deleted agent session: {}", session_id));

    Ok(())
}

// ============================================================================
// Memory Operation Commands
// ============================================================================

/// Consolidate memory (move from working to episodic/semantic)
pub async fn memory_consolidate(
    workspace: Option<String>,
    _merge_similar: bool,
    _archive_old: bool,
    _threshold_days: i32,
) -> Result<()> {
    let config = CortexConfig::load()?;
    let _workspace_name = workspace.or(config.default_workspace.clone());

    let spinner = output::spinner("Consolidating memory...");

    let storage = create_storage(&config).await?;
    let memory = CognitiveManager::new(storage);

    // Trigger consolidation
    let report = memory.consolidate().await?;

    spinner.finish_and_clear();

    output::success("Memory consolidation complete");
    output::kv("Episodes processed", report.episodes_processed as i64);
    output::kv("Patterns extracted", report.patterns_extracted as i64);
    output::kv("Memories decayed", report.memories_decayed as i64);
    output::kv("Duplicates merged", report.duplicates_merged as i64);
    output::kv("Knowledge links created", report.knowledge_links_created as i64);
    output::kv("Duration (ms)", report.duration_ms as i64);

    Ok(())
}

/// Forget (delete) old memory
pub async fn memory_forget(before_date: String, workspace: Option<String>) -> Result<()> {
    let config = CortexConfig::load()?;

    if !output::confirm(format!("Delete all memory before {}?", before_date))? {
        output::info("Cancelled");
        return Ok(());
    }

    let spinner = output::spinner("Forgetting memory...");

    let storage = create_storage(&config).await?;
    let memory = CognitiveManager::new(storage);

    // Parse the date string to DateTime<Utc>
    let before = chrono::DateTime::parse_from_rfc3339(&before_date)
        .map_err(|e| cortex_core::error::CortexError::invalid_input(format!("Invalid date format: {}. Expected RFC3339 format (e.g., 2024-01-01T00:00:00Z)", e)))?
        .with_timezone(&chrono::Utc);

    // Delete old memory
    let deleted = memory.forget_before(&before, workspace.as_deref()).await?;

    spinner.finish_and_clear();

    output::success("Memory deleted");
    output::kv("Total memories deleted", deleted as i64);

    Ok(())
}

// ============================================================================
// Database Management Commands
// ============================================================================

/// Start both SurrealDB and Qdrant databases
pub async fn db_start(
    surreal_bind: Option<String>,
    surreal_data: Option<PathBuf>,
    qdrant_port: Option<u16>,
    qdrant_grpc_port: Option<u16>,
    qdrant_data: Option<PathBuf>,
    _use_docker: bool,
) -> Result<()> {
    output::header("Starting Database Infrastructure");
    output::info("Managing both SurrealDB (metadata) and Qdrant (vectors)");
    println!();

    // Check if native Qdrant binary is installed
    let qdrant_binary_installed = is_qdrant_binary_installed().await;

    if !qdrant_binary_installed {
        output::error("Native Qdrant binary not found at ~/.cortex/bin/qdrant");
        output::info("Please install Qdrant:");
        output::info("  cortex db install --database qdrant");
        return Err(anyhow::anyhow!("Qdrant not installed"));
    }

    output::info("Native Qdrant binary detected at ~/.cortex/bin/qdrant");
    output::info("Starting databases in native mode");
    println!();

    // Load base configuration
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    // Build SurrealDB configuration with overrides
    let mut surrealdb_config = cortex_storage::SurrealDBConfig::default();
    surrealdb_config.username = db_config.username.clone();
    surrealdb_config.password = db_config.password.clone();
    surrealdb_config.bind_address = surreal_bind.unwrap_or_else(|| db_config.local_bind.clone());
    if let Some(data_dir) = surreal_data {
        surrealdb_config.data_dir = data_dir;
    }

    // Build Qdrant configuration with overrides
    let qdrant_config = cortex_storage::QdrantConfig {
        host: std::env::var("QDRANT_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: qdrant_port.unwrap_or_else(|| {
            std::env::var("QDRANT_HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(6333)
        }),
        grpc_port: qdrant_grpc_port.or_else(|| {
            std::env::var("QDRANT_GRPC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
        }),
        api_key: std::env::var("QDRANT_API_KEY").ok(),
        use_https: std::env::var("QDRANT_USE_HTTPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false),
        timeout: std::time::Duration::from_secs(10),
        request_timeout: std::time::Duration::from_secs(60),
    };

    // Build database manager configuration - always use native mode
    let manager_config = crate::db_manager::DatabaseManagerConfig::default();
    // defaults are already set to native mode

    // Store qdrant_data for later use if needed
    if let Some(_data_dir) = qdrant_data {
        // Note: Current QdrantConfig doesn't have a data_dir field
        // This would require extending QdrantConfig or passing it to docker volumes
        output::warning("Custom Qdrant data directory is not yet supported in this version");
    }

    // Create unified database manager
    let manager = crate::db_manager::DatabaseManager::new(
        manager_config,
        surrealdb_config,
        qdrant_config,
    ).await?;

    // Start both databases in sequence
    match manager.start().await {
        Ok(_) => {
            println!();
            output::success("Database infrastructure started successfully");
            output::info("Both SurrealDB and Qdrant are running and healthy");
            println!();

            // Show connection information
            let status = manager.status().await?;
            output::kv("SurrealDB URL", &status.surrealdb.url);
            output::kv("Qdrant URL", &status.qdrant.url);
            println!();

            output::info("Use 'cortex db stop' to stop all databases");
            output::info("Use 'cortex db status' to check health and metrics");
            println!();
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to start database infrastructure: {}", e));
            output::warning("Some databases may have started - run 'cortex db status' to check");
            Err(e)
        }
    }
}

/// Stop both SurrealDB and Qdrant databases
pub async fn db_stop() -> Result<()> {
    output::header("Stopping Database Infrastructure");
    output::info("Stopping Qdrant and SurrealDB in reverse order");
    println!();

    // Create unified database manager
    let manager = crate::db_manager::create_from_global_config().await?;

    // Stop both databases in reverse order
    match manager.stop().await {
        Ok(_) => {
            println!();
            output::success("Database infrastructure stopped successfully");
            output::info("All databases have been shut down gracefully");
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to stop all databases: {}", e));
            output::warning("Some databases may still be running - check 'cortex db status'");
            Err(e)
        }
    }
}

/// Restart both SurrealDB and Qdrant databases
pub async fn db_restart() -> Result<()> {
    output::header("Restarting Database Infrastructure");
    output::info("Gracefully restarting all databases");
    println!();

    // Create unified database manager
    let manager = crate::db_manager::create_from_global_config().await?;

    // Restart both databases
    match manager.restart().await {
        Ok(_) => {
            println!();
            output::success("Database infrastructure restarted successfully");

            // Show connection information
            let status = manager.status().await?;
            output::kv("SurrealDB URL", &status.surrealdb.url);
            output::kv("Qdrant URL", &status.qdrant.url);
            println!();
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to restart database infrastructure: {}", e));
            Err(e)
        }
    }
}

// ============================================================================
// Database Installation Detection Helpers
// ============================================================================

/// Check if SurrealDB is installed
async fn is_surrealdb_installed() -> bool {
    cortex_storage::SurrealDBManager::find_surreal_binary().await.is_ok()
}

/// Check if Qdrant native binary is installed
async fn is_qdrant_binary_installed() -> bool {
    use std::path::PathBuf;
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let qdrant_path = PathBuf::from(home).join(".cortex").join("bin").join("qdrant");
    qdrant_path.exists()
}

/// Check if Docker is available
async fn is_docker_available() -> bool {
    use tokio::process::Command;
    Command::new("docker")
        .arg("--version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if Qdrant Docker image is present
async fn is_qdrant_docker_installed() -> bool {
    use tokio::process::Command;
    let output = Command::new("docker")
        .args(&["images", "-q", "qdrant/qdrant"])
        .output()
        .await;

    match output {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Check the status of both SurrealDB and Qdrant databases
pub async fn db_status(detailed: bool) -> Result<()> {
    output::header("Database Infrastructure Status");
    println!();

    // Create unified database manager
    let manager = crate::db_manager::create_from_global_config().await?;

    // Get combined status
    let spinner = output::spinner("Checking database health...");
    let status = manager.status().await?;
    spinner.finish_and_clear();

    // Overall status
    if status.overall_healthy {
        output::success("Overall Status: Healthy");
    } else {
        output::error("Overall Status: Degraded");
    }
    println!();

    // SurrealDB Status
    output::header("SurrealDB (Metadata & Relational)");
    output::kv("URL", &status.surrealdb.url);
    output::kv("Running", if status.surrealdb.running { "Yes" } else { "No" });
    output::kv("Healthy", if status.surrealdb.healthy { "Yes" } else { "No" });

    // Check installation status
    let surreal_installed = is_surrealdb_installed().await;
    output::kv("Installed", if surreal_installed { "Yes" } else { "No" });

    if !surreal_installed {
        output::warning("SurrealDB binary not found");
    }

    if let Some(ref error) = status.surrealdb.error {
        output::error(format!("Error: {}", error));
    }

    if let Some(ref metrics) = status.surrealdb.metrics {
        if let Some(uptime) = metrics.uptime_seconds {
            output::kv("Uptime", format!("{}s", uptime));
        }
        if let Some(memory) = metrics.memory_mb {
            output::kv("Memory", format!("{} MB", memory));
        }
        if let Some(connections) = metrics.connections {
            output::kv("Connections", connections);
        }
    }

    // Show additional details in detailed mode
    if detailed && status.surrealdb.healthy {
        output::info("Additional SurrealDB Information:");
        output::info("  - Process Management: Active");
        output::info("  - Data Persistence: Enabled");
        output::info("  - Connection Mode: Local");
    }
    println!();

    // Qdrant Status
    output::header("Qdrant (Vector Database)");
    output::kv("URL", &status.qdrant.url);
    output::kv("Running", if status.qdrant.running { "Yes" } else { "No" });
    output::kv("Healthy", if status.qdrant.healthy { "Yes" } else { "No" });

    // Check installation status
    let qdrant_binary = is_qdrant_binary_installed().await;
    let docker_available = is_docker_available().await;
    let qdrant_docker = is_qdrant_docker_installed().await;

    if qdrant_binary {
        output::kv("Installed", "Yes (Native Binary)");
        output::kv("Binary Path", "~/.cortex/bin/qdrant");
    } else if docker_available && qdrant_docker {
        output::kv("Installed", "Yes (Docker)");
        output::kv("Docker Image", "qdrant/qdrant");
    } else {
        output::kv("Installed", "No");
        if !qdrant_binary && !qdrant_docker {
            output::warning("Qdrant not found (neither binary nor Docker)");
        }
    }

    if let Some(ref error) = status.qdrant.error {
        output::error(format!("Error: {}", error));
    }

    if let Some(ref metrics) = status.qdrant.metrics {
        if let Some(uptime) = metrics.uptime_seconds {
            output::kv("Uptime", format!("{}s", uptime));
        }
        if let Some(memory) = metrics.memory_mb {
            output::kv("Memory", format!("{} MB", memory));
        }
        if let Some(rps) = metrics.requests_per_sec {
            output::kv("Requests/sec", format!("{:.2}", rps));
        }
    }

    // Show additional details in detailed mode
    if detailed && status.qdrant.healthy {
        output::info("Additional Qdrant Information:");

        // Try to fetch collection statistics
        match cortex_storage::QdrantClient::new(cortex_storage::QdrantConfig {
            host: "localhost".to_string(),
            port: 6333,
            grpc_port: None,
            api_key: None,
            use_https: false,
            timeout: std::time::Duration::from_secs(5),
            request_timeout: std::time::Duration::from_secs(10),
        }).await {
            Ok(client) => {
                match client.list_collections().await {
                    Ok(collections) => {
                        output::info(format!("  - Collections: {}", collections.len()));
                        if !collections.is_empty() {
                            output::info("  - Active Collections:");
                            for collection in collections.iter().take(5) {
                                output::info(format!("    * {}", collection));
                            }
                            if collections.len() > 5 {
                                output::info(format!("    ... and {} more", collections.len() - 5));
                            }
                        }
                    }
                    Err(_) => {
                        output::warning("  - Could not fetch collection details");
                    }
                }
            }
            Err(_) => {
                output::warning("  - Could not connect to Qdrant for detailed info");
            }
        }
    }
    println!();

    // Summary and recommendations
    if !status.overall_healthy {
        output::warning("Some components are unhealthy");
        println!();

        if !status.surrealdb.healthy {
            let installed = is_surrealdb_installed().await;
            if !installed {
                output::info("SurrealDB Recommendations:");
                output::info("  - Install: cortex db install --database surrealdb");
                output::info("  - Then start: cortex db start");
            } else {
                output::info("SurrealDB Recommendations:");
                output::info("  - Binary is installed but not running");
                output::info("  - Start: cortex db start");
            }
            println!();
        }

        if !status.qdrant.healthy {
            let binary_installed = is_qdrant_binary_installed().await;
            let docker_available = is_docker_available().await;
            let docker_installed = is_qdrant_docker_installed().await;

            output::info("Qdrant Recommendations:");

            if binary_installed {
                output::info("  - Native binary is installed but not running");
                output::info("  - Start: cortex db start");
            } else if docker_available && docker_installed {
                output::info("  - Docker image is installed but not running");
                output::info("  - Start: cortex db start --use-docker");
            } else if docker_available && !docker_installed {
                output::info("  - Docker is available but Qdrant image not pulled");
                output::info("  - Install: cortex db install --database qdrant-docker");
                output::info("  - Or install native: cortex db install --database qdrant");
            } else {
                output::info("  - Docker not available");
                output::info("  - Install native binary: cortex db install --database qdrant");
                output::info("  - Or install Docker and then: cortex db install --database qdrant-docker");
            }
            println!();
        }
    }

    if detailed {
        println!();
        output::info("Tip: Use 'cortex db start --help' to see available configuration options");
    }

    Ok(())
}

/// Install database binaries (SurrealDB and/or Qdrant)
pub async fn db_install(database: String) -> Result<()> {
    let database_lower = database.to_lowercase();

    match database_lower.as_str() {
        "surrealdb" => {
            install_surrealdb().await?;
        }
        "qdrant" => {
            install_qdrant_binary().await?;
        }
        "qdrant-docker" => {
            install_qdrant_docker().await?;
        }
        "both" => {
            output::header("Installing Database Infrastructure");
            println!();

            output::info("Installing SurrealDB...");
            if let Err(e) = install_surrealdb().await {
                output::warning(format!("SurrealDB installation failed: {}", e));
            }
            println!();

            output::info("Installing Qdrant native binary...");
            if let Err(e) = install_qdrant_binary().await {
                output::warning(format!("Qdrant binary installation failed: {}", e));
            }
            println!();

            output::info("Installation complete. Run 'cortex db status' to verify.");
        }
        _ => {
            output::error(format!("Unknown database: {}", database));
            output::info("Valid options: surrealdb, qdrant, qdrant-docker, both");
            return Err(anyhow::anyhow!("Invalid database option"));
        }
    }

    Ok(())
}

/// Install SurrealDB
async fn install_surrealdb() -> Result<()> {
    output::info("Checking for SurrealDB installation...");

    match SurrealDBManager::find_surreal_binary().await {
        Ok(path) => {
            output::success(format!("SurrealDB is already installed at: {}", path.display()));

            // Try to get version
            if let Ok(output) = std::process::Command::new(&path)
                .arg("version")
                .output()
            {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    output::kv("Version", version.trim());
                }
            }

            Ok(())
        }
        Err(_) => {
            output::info("SurrealDB not found. Installing...");

            match SurrealDBManager::install_surrealdb().await {
                Ok(path) => {
                    output::success(format!("SurrealDB installed successfully at: {}", path.display()));
                    Ok(())
                }
                Err(e) => {
                    output::error(format!("Failed to install SurrealDB: {}", e));
                    output::info("Please install manually from: https://surrealdb.com/install");
                    Err(e.into())
                }
            }
        }
    }
}

/// Install Qdrant native binary from GitHub releases
async fn install_qdrant_binary() -> Result<()> {
    output::info("Installing Qdrant binary from GitHub releases...");

    // Detect platform
    let (os, arch) = detect_platform()?;
    output::info(format!("Detected platform: {} {}", arch, os));

    // Get latest release info
    let release_info = get_latest_qdrant_release().await?;
    let version = release_info["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get version from release"))?;
    output::info(format!("Latest Qdrant version: {}", version));

    // Find appropriate asset for platform
    let download_url = find_qdrant_asset(&release_info, &os, &arch)?;

    // Download and install
    download_and_install_qdrant(&download_url, version).await?;

    output::success("Qdrant binary installed successfully");
    Ok(())
}

/// Detect the current platform (OS and architecture)
fn detect_platform() -> Result<(String, String)> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let os_name = match os {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-musl",
        "windows" => "pc-windows-msvc",
        _ => return Err(anyhow::anyhow!("Unsupported OS: {}", os)),
    };

    let arch_name = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(anyhow::anyhow!("Unsupported architecture: {}", arch)),
    };

    Ok((os_name.to_string(), arch_name.to_string()))
}

/// Fetch latest Qdrant release information from GitHub API
async fn get_latest_qdrant_release() -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/qdrant/qdrant/releases/latest")
        .header("User-Agent", "cortex")
        .send()
        .await
        .context("Failed to fetch Qdrant release information")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "GitHub API returned error: {}",
            response.status()
        ));
    }

    let release_info: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse release information")?;

    Ok(release_info)
}

/// Find the appropriate Qdrant binary asset for the given platform
fn find_qdrant_asset(release: &serde_json::Value, os: &str, arch: &str) -> Result<String> {
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No assets found in release"))?;

    // Build target string
    let target = format!("{}-{}", arch, os);
    output::info(format!("Looking for binary matching: {}", target));

    // Look for matching asset name pattern
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");

        // Skip checksum files
        if name.contains(".sha256") || name.contains(".md5") {
            continue;
        }

        // Check if this is the right binary for our platform
        if name.contains(&target) {
            let download_url = asset["browser_download_url"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("No download URL found for asset"))?;

            output::info(format!("Found matching binary: {}", name));
            return Ok(download_url.to_string());
        }
    }

    Err(anyhow::anyhow!(
        "No Qdrant binary found for platform: {} {}. Available assets: {}",
        arch,
        os,
        assets
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Download and install the Qdrant binary
async fn download_and_install_qdrant(url: &str, version: &str) -> Result<()> {
    use std::path::PathBuf;
    use flate2::read::GzDecoder;
    use tar::Archive;

    // Create installation directory
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory")?;
    let install_dir = PathBuf::from(home).join(".cortex").join("bin");
    std::fs::create_dir_all(&install_dir)
        .context("Failed to create installation directory")?;

    let binary_path = install_dir.join("qdrant");

    output::info(format!("Downloading Qdrant {} from GitHub...", version));
    let spinner = output::spinner("Downloading...");

    // Download file
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to download Qdrant binary")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download binary: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read binary data")?;

    spinner.finish_and_clear();

    output::info("Extracting archive...");

    // Decompress gzip and extract tar
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);

    // Extract to temp directory
    let temp_dir = install_dir.join("qdrant_temp");
    std::fs::create_dir_all(&temp_dir)?;

    archive.unpack(&temp_dir)
        .context("Failed to extract archive")?;

    // Find qdrant binary in extracted files
    let mut qdrant_binary = None;
    for entry in std::fs::read_dir(&temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("qdrant") {
            qdrant_binary = Some(path);
            break;
        }
    }

    let qdrant_binary = qdrant_binary
        .ok_or_else(|| anyhow::anyhow!("Qdrant binary not found in archive"))?;

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&qdrant_binary)?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&qdrant_binary, perms)
            .context("Failed to set executable permissions")?;
    }

    // Move to final location
    if binary_path.exists() {
        std::fs::remove_file(&binary_path)?;
    }
    std::fs::rename(&qdrant_binary, &binary_path)
        .context("Failed to install binary")?;

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    output::success(format!("Installed Qdrant to: {}", binary_path.display()));
    output::info(format!("Version: {}", version));
    output::info("Add ~/.cortex/bin to your PATH to use 'qdrant' command");

    // Try to get version to verify installation
    if let Ok(output) = std::process::Command::new(&binary_path)
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout);
            output::info(format!("Verified: {}", version_output.trim()));
        }
    }

    Ok(())
}

/// Install Qdrant (via Docker)
async fn install_qdrant_docker() -> Result<()> {
    use tokio::process::Command;

    output::info("Checking for Qdrant...");

    // Check if Docker is available
    let docker_check = Command::new("docker")
        .arg("--version")
        .output()
        .await;

    if docker_check.is_err() || !docker_check.unwrap().status.success() {
        output::error("Docker is not installed or not running");
        output::info("Qdrant requires Docker. Please install Docker first:");
        output::info("  - macOS/Windows: https://www.docker.com/products/docker-desktop");
        output::info("  - Linux: https://docs.docker.com/engine/install/");
        return Err(anyhow::anyhow!("Docker not available"));
    }

    output::success("Docker is available");

    // Check if Qdrant image exists
    output::info("Pulling Qdrant Docker image...");
    let pull_result = Command::new("docker")
        .arg("pull")
        .arg("qdrant/qdrant:v1.12.5")
        .output()
        .await;

    match pull_result {
        Ok(output_result) => {
            if output_result.status.success() {
                output::success("Qdrant image pulled successfully");
                output::kv("Image", "qdrant/qdrant:v1.12.5");
                output::info("Use 'cortex db start' to start Qdrant");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output_result.stderr);
                output::error(format!("Failed to pull Qdrant image: {}", stderr));
                Err(anyhow::anyhow!("Failed to pull Qdrant image"))
            }
        }
        Err(e) => {
            output::error(format!("Failed to execute docker pull: {}", e));
            Err(e.into())
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a storage connection manager from config
async fn create_storage(config: &CortexConfig) -> Result<Arc<ConnectionManager>> {
    use cortex_storage::connection_pool::ConnectionMode;
    use std::time::Duration;

    let db_config = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: config.database.connection_string.clone(),
        },
        credentials: Credentials {
            username: config.database.username.clone(),
            password: config.database.password.clone(),
        },
        pool_config: PoolConfig {
            max_connections: config.database.pool_size,
            min_connections: 2,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(1800)),
            retry_policy: cortex_storage::connection_pool::RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(50),
                max_backoff: Duration::from_secs(5),
                multiplier: 1.5,
            },
            warm_connections: true,
            validate_on_checkout: false,
            recycle_after_uses: Some(10000),
            shutdown_grace_period: Duration::from_secs(30),
        },
        namespace: config.database.namespace.clone(),
        database: config.database.database.clone(),
    };

    let manager = ConnectionManager::new(db_config)
        .await
        .context("Failed to create storage connection")?;

    Ok(Arc::new(manager))
}

/// Create a temporary session for a workspace
async fn create_temp_session(
    storage: Arc<ConnectionManager>,
    workspace_name: Option<String>,
    config: &CortexConfig,
) -> Result<(cortex_storage::session::SessionId, Uuid, String)> {
    use cortex_storage::session::{AgentSession, SessionManager, SessionState};

    // Determine which workspace to use
    let workspace_to_use = workspace_name
        .or_else(|| config.default_workspace.clone())
        .ok_or_else(|| anyhow::anyhow!("No workspace specified and no default workspace configured"))?;

    // Get workspace ID from name using raw query to avoid serialization issues
    let conn = storage.acquire().await?;
    let query = "SELECT *, <string>meta::id(id) as id FROM workspace";
    let mut response = conn.connection().query(query).await
        .context("Failed to fetch workspaces")?;

    #[derive(serde::Deserialize)]
    struct WorkspaceWithStringId {
        id: String,
        #[serde(flatten)]
        rest: serde_json::Value,
    }

    let workspaces_raw: Vec<WorkspaceWithStringId> = response.take(0)?;
    let workspaces: Vec<Workspace> = workspaces_raw
        .into_iter()
        .filter_map(|w| {
            // Extract UUID from "workspace:uuid" format
            let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
            let mut workspace_json = w.rest;
            if let Some(obj) = workspace_json.as_object_mut() {
                obj.insert("id".to_string(), serde_json::Value::String(uuid_str.to_string()));
            }
            serde_json::from_value::<Workspace>(workspace_json).ok()
        })
        .collect();

    let workspace = workspaces.iter()
        .find(|w| w.name == workspace_to_use)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_to_use))?;

    // Create a temporary session for CLI operations
    // Create session manager using the storage connection manager
    let session_manager = SessionManager::from_connection_manager(&*storage).await?;

    // Create metadata for the session
    use cortex_storage::session::{SessionMetadata, IsolationLevel, SessionScope};
    use std::collections::HashMap;

    let metadata = SessionMetadata {
        description: "Temporary CLI session".to_string(),
        tags: vec!["cli".to_string(), "temporary".to_string()],
        isolation_level: IsolationLevel::Serializable,
        scope: SessionScope {
            paths: vec!["*".to_string()],  // Full access to all paths
            read_only_paths: vec![],
            units: vec![],
            allow_create: true,
            allow_delete: true,
        },
        custom: HashMap::new(),
    };

    let session = session_manager.create_session(
        "cli-temp".to_string(),
        workspace.id.into(), // Convert Uuid to CortexId
        metadata,
        None, // Default TTL
    ).await?;

    Ok((session.id, workspace.id, workspace.name.clone()))
}

/// Get workspace name to use for command
fn get_workspace_name(
    specified: Option<String>,
    config: &CortexConfig,
) -> Result<String> {
    specified
        .or_else(|| config.default_workspace.clone())
        .ok_or_else(|| anyhow::anyhow!("No workspace specified and no default workspace configured"))
}

// ============================================================================
// MCP Commands
// ============================================================================

/// Start MCP server in stdio mode
pub async fn mcp_stdio() -> Result<()> {
    // Load config to get log file path
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let mcp_config = global_config.mcp();

    // Initialize file logging for stdio mode (NO stdout/stderr output!)
    init_file_logging(&mcp_config.log_file_stdio, &mcp_config.log_level)?;

    tracing::info!("Starting Cortex MCP Server (stdio mode)");
    tracing::info!("Log file: {}", mcp_config.log_file_stdio);

    let server = CortexMcpServer::new().await
        .context("Failed to initialize MCP server")?;

    tracing::info!("MCP server started successfully, listening on stdio");

    server.serve_stdio().await?;
    Ok(())
}

/// Start MCP server in HTTP mode
pub async fn mcp_http(address: String, port: u16) -> Result<()> {
    // Load config to get log file path
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let mcp_config = global_config.mcp();

    // Initialize file logging for HTTP mode
    init_file_logging(&mcp_config.log_file_http, &mcp_config.log_level)?;

    output::header("Starting Cortex MCP Server (HTTP mode)");
    output::kv("Address", &address);
    output::kv("Port", port);
    output::info("Initializing server...");

    let server = CortexMcpServer::new().await
        .context("Failed to initialize MCP server")?;

    let bind_addr = format!("{}:{}", address, port);

    output::success("MCP server started successfully");
    output::info(format!("Listening on http://{}", bind_addr));
    output::info("Press Ctrl+C to stop");

    tracing::info!("MCP HTTP server started on {}", bind_addr);
    tracing::info!("Log file: {}", mcp_config.log_file_http);

    server.serve_http(&bind_addr).await?;
    Ok(())
}

/// Show information about available MCP tools
pub async fn mcp_info(detailed: bool, category: Option<String>) -> Result<()> {
    output::header("Cortex MCP Server - Tools Information");

    // Tool categories and counts
    let categories = vec![
        ("Workspace Management", 8, vec![
            "cortex.workspace.create",
            "cortex.workspace.get",
            "cortex.workspace.list",
            "cortex.workspace.activate",
            "cortex.workspace.sync_from_disk",
            "cortex.workspace.export",
            "cortex.workspace.archive",
            "cortex.workspace.delete",
        ]),
        ("Virtual Filesystem", 12, vec![
            "cortex.vfs.get_node",
            "cortex.vfs.list_directory",
            "cortex.vfs.create_file",
            "cortex.vfs.update_file",
            "cortex.vfs.delete_node",
            "cortex.vfs.move_node",
            "cortex.vfs.copy_node",
            "cortex.vfs.create_directory",
            "cortex.vfs.get_tree",
            "cortex.vfs.search_files",
            "cortex.vfs.get_file_history",
            "cortex.vfs.restore_file_version",
        ]),
        ("Code Navigation", 10, vec![
            "cortex.code.get_unit",
            "cortex.code.list_units",
            "cortex.code.get_symbols",
            "cortex.code.find_definition",
            "cortex.code.find_references",
            "cortex.code.get_signature",
            "cortex.code.get_call_hierarchy",
            "cortex.code.get_type_hierarchy",
            "cortex.code.get_imports",
            "cortex.code.get_exports",
        ]),
        ("Code Manipulation", 15, vec![
            "cortex.code.extract_method",
            "cortex.code.inline_variable",
            "cortex.code.rename_symbol",
            "cortex.code.move_code",
            "cortex.code.change_signature",
            "cortex.code.extract_constant",
            "cortex.code.inline_method",
            "cortex.code.convert_to_function",
            "cortex.code.extract_interface",
            "cortex.code.pull_up_method",
            "cortex.code.push_down_method",
            "cortex.code.introduce_parameter",
            "cortex.code.replace_temp_with_query",
            "cortex.code.split_temporary_variable",
            "cortex.code.remove_assignments_to_parameters",
        ]),
        ("Semantic Search", 8, vec![
            "cortex.search.semantic",
            "cortex.search.similar_code",
            "cortex.search.by_pattern",
            "cortex.search.by_type",
            "cortex.search.cross_reference",
            "cortex.search.usage_examples",
            "cortex.search.api_discovery",
            "cortex.search.query_expansion",
        ]),
        ("Dependency Analysis", 10, vec![
            "cortex.deps.find_dependencies",
            "cortex.deps.find_dependents",
            "cortex.deps.shortest_path",
            "cortex.deps.all_paths",
            "cortex.deps.detect_cycles",
            "cortex.deps.impact_analysis",
            "cortex.deps.architectural_layers",
            "cortex.deps.detect_hubs",
            "cortex.deps.check_constraints",
            "cortex.deps.visualize_graph",
        ]),
        ("Code Quality", 8, vec![
            "cortex.quality.analyze_complexity",
            "cortex.quality.detect_code_smells",
            "cortex.quality.suggest_improvements",
            "cortex.quality.calculate_metrics",
            "cortex.quality.check_standards",
            "cortex.quality.find_duplicates",
            "cortex.quality.analyze_maintainability",
            "cortex.quality.generate_report",
        ]),
        ("Version Control", 10, vec![
            "cortex.vcs.get_history",
            "cortex.vcs.get_diff",
            "cortex.vcs.get_blame",
            "cortex.vcs.find_commits",
            "cortex.vcs.analyze_churn",
            "cortex.vcs.get_contributors",
            "cortex.vcs.get_branches",
            "cortex.vcs.get_tags",
            "cortex.vcs.compare_branches",
            "cortex.vcs.get_merge_conflicts",
        ]),
        ("Cognitive Memory", 12, vec![
            "cortex.memory.store_episode",
            "cortex.memory.retrieve_episode",
            "cortex.memory.consolidate",
            "cortex.memory.forget",
            "cortex.memory.associate",
            "cortex.memory.recall_pattern",
            "cortex.memory.get_context",
            "cortex.memory.update_weights",
            "cortex.memory.prune",
            "cortex.memory.get_stats",
            "cortex.memory.export",
            "cortex.memory.import",
        ]),
        ("Multi-Agent Coordination", 10, vec![
            "cortex.agent.create_session",
            "cortex.agent.get_session",
            "cortex.agent.list_sessions",
            "cortex.agent.delete_session",
            "cortex.agent.send_message",
            "cortex.agent.receive_messages",
            "cortex.agent.broadcast",
            "cortex.agent.request_capability",
            "cortex.agent.register_capability",
            "cortex.agent.coordinate_task",
        ]),
        ("Materialization", 8, vec![
            "cortex.mat.generate_code",
            "cortex.mat.generate_tests",
            "cortex.mat.generate_docs",
            "cortex.mat.generate_schema",
            "cortex.mat.apply_template",
            "cortex.mat.scaffold_project",
            "cortex.mat.generate_migration",
            "cortex.mat.preview_changes",
        ]),
        ("Testing & Validation", 10, vec![
            "cortex.test.generate_unit_tests",
            "cortex.test.generate_integration_tests",
            "cortex.test.run_tests",
            "cortex.test.analyze_coverage",
            "cortex.test.suggest_test_cases",
            "cortex.test.validate_contracts",
            "cortex.test.check_invariants",
            "cortex.test.verify_properties",
            "cortex.test.generate_mocks",
            "cortex.test.analyze_assertions",
        ]),
        ("Documentation", 8, vec![
            "cortex.doc.generate",
            "cortex.doc.extract_examples",
            "cortex.doc.generate_api_spec",
            "cortex.doc.update_readme",
            "cortex.doc.generate_changelog",
            "cortex.doc.check_coverage",
            "cortex.doc.validate_links",
            "cortex.doc.generate_diagrams",
        ]),
        ("Build & Execution", 8, vec![
            "cortex.build.compile",
            "cortex.build.run",
            "cortex.build.watch",
            "cortex.build.clean",
            "cortex.build.analyze_artifacts",
            "cortex.build.optimize",
            "cortex.build.profile",
            "cortex.build.debug",
        ]),
        ("Monitoring & Analytics", 10, vec![
            "cortex.monitor.track_metric",
            "cortex.monitor.get_metrics",
            "cortex.monitor.create_dashboard",
            "cortex.monitor.set_threshold",
            "cortex.monitor.get_health",
            "cortex.monitor.analyze_performance",
            "cortex.monitor.analyze_errors",
            "cortex.monitor.analyze_productivity",
            "cortex.monitor.quality_trends",
            "cortex.monitor.export_metrics",
        ]),
        ("Security Analysis", 4, vec![
            "cortex.security.scan",
            "cortex.security.check_dependencies",
            "cortex.security.analyze_secrets",
            "cortex.security.generate_report",
        ]),
        ("Type Analysis", 4, vec![
            "cortex.code.infer_types",
            "cortex.code.check_types",
            "cortex.code.suggest_type_annotations",
            "cortex.code.analyze_type_coverage",
        ]),
        ("AI-Assisted Development", 6, vec![
            "cortex.ai.suggest_refactoring",
            "cortex.ai.explain_code",
            "cortex.ai.suggest_optimization",
            "cortex.ai.suggest_fix",
            "cortex.ai.generate_docstring",
            "cortex.ai.review_code",
        ]),
        ("Advanced Testing", 6, vec![
            "cortex.test.generate_property",
            "cortex.test.generate_mutation",
            "cortex.test.generate_benchmarks",
            "cortex.test.generate_fuzzing",
            "cortex.test.analyze_flaky",
            "cortex.test.suggest_edge_cases",
        ]),
        ("Architecture Analysis", 5, vec![
            "cortex.arch.visualize",
            "cortex.arch.detect_patterns",
            "cortex.arch.suggest_boundaries",
            "cortex.arch.check_violations",
            "cortex.arch.analyze_drift",
        ]),
    ];

    // Filter by category if specified
    let filtered_categories: Vec<_> = if let Some(ref filter) = category {
        categories.into_iter()
            .filter(|(name, _, _)| name.to_lowercase().contains(&filter.to_lowercase()))
            .collect()
    } else {
        categories
    };

    if filtered_categories.is_empty() {
        output::warning(format!("No categories found matching '{}'", category.unwrap_or_default()));
        return Ok(());
    }

    // Display categories
    for (cat_name, count, tools) in &filtered_categories {
        println!("\n{}: {} tools", cat_name, count);

        if detailed {
            for tool in tools {
                println!("  - {}", tool);
            }
        }
    }

    // Total count
    let total: usize = filtered_categories.iter().map(|(_, count, _)| count).sum();
    println!("\nTotal: {} tools across {} categories", total, filtered_categories.len());

    if !detailed {
        output::info("Use --detailed to see all tool names");
    }

    if category.is_none() {
        output::info("Use --category <name> to filter by category");
    }

    Ok(())
}

// ============================================================================
// REST API Server Commands
// ============================================================================

/// Start the REST API server
/// Run server in blocking mode (used internally by background process)
pub async fn server_run_blocking(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    let config = crate::api::server::ServerConfig {
        host,
        port,
        workers,
    };

    let server = crate::api::RestApiServer::with_config(config).await?;
    server.serve().await?;

    Ok(())
}

/// Start server in background
pub async fn server_start(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    use crate::server_manager::{ServerManager, ServerConfig};

    output::info("Starting Cortex REST API Server...");

    let config = ServerConfig {
        host,
        port,
        workers,
        ..Default::default()
    };

    let manager = ServerManager::new(config);

    manager.start().await?;

    output::success("REST API server started successfully in background");
    output::kv("URL", format!("http://{}:{}", manager.config().host, manager.config().port));
    output::kv("Logs", manager.config().log_file.display());
    output::kv("PID file", manager.config().pid_file.display());
    output::info("Use 'cortex server stop' to stop the server");
    output::info("Use 'cortex server status' to check server status");
    println!();

    Ok(())
}

/// Stop the REST API server
pub async fn server_stop() -> Result<()> {
    use crate::server_manager::{ServerManager, ServerConfig};

    output::info("Stopping Cortex REST API Server...");

    let config = ServerConfig::default();
    let manager = ServerManager::new(config);

    manager.stop().await?;

    output::success("REST API server stopped successfully");

    Ok(())
}

/// Check REST API server status
pub async fn server_status() -> Result<()> {
    use crate::server_manager::{ServerManager, ServerConfig, ServerStatus};

    output::header("Cortex REST API Server Status");

    let config = ServerConfig::default();
    output::kv("URL", format!("http://{}:{}", config.host, config.port));
    output::kv("Logs", config.log_file.display());
    output::kv("PID file", config.pid_file.display());
    println!();

    let manager = ServerManager::new(config);

    match manager.status().await {
        ServerStatus::Running => {
            output::success("Status: Running");
        }
        ServerStatus::Stopped => {
            output::info("Status: Stopped");
        }
        ServerStatus::Unknown => {
            output::warning("Status: Unknown");
        }
    }

    Ok(())
}

// ============================================================================
// Qdrant Commands (re-exported from qdrant_commands module)
// ============================================================================
// ============================================================================
// VFS Commands
// ============================================================================

/// Helper to get workspace ID from name or use active workspace
async fn resolve_workspace_id(storage: &Arc<ConnectionManager>, workspace: Option<String>) -> Result<Uuid> {
    if let Some(name) = workspace {
        // Try parsing as UUID first
        if let Ok(uuid) = Uuid::parse_str(&name) {
            return Ok(uuid);
        }
        // Search by name
        let conn = storage.acquire().await?;
        let mut response = conn.connection()
            .query("SELECT *, <string>meta::id(id) as id FROM workspace WHERE name = $name")
            .bind(("name", name.clone()))
            .await?;

        // Parse with string IDs and convert to UUIDs
        #[derive(serde::Deserialize)]
        struct WorkspaceWithStringId {
            id: String,
            #[serde(flatten)]
            rest: serde_json::Value,
        }

        let workspaces_raw: Vec<WorkspaceWithStringId> = response.take(0)?;
        workspaces_raw.first()
            .map(|w| {
                // Extract UUID from "workspace:uuid" format
                let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
                Uuid::parse_str(uuid_str)
            })
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Workspace not found: {}", name))
    } else {
        // Use default workspace from config
        let config = CortexConfig::load()?;
        config.default_workspace
            .ok_or_else(|| anyhow::anyhow!("No workspace specified. Use --workspace flag or set default_workspace in config"))
            .and_then(|name| {
                // Resolve default workspace name to ID
                futures::executor::block_on(async {
                    let conn = storage.acquire().await?;
                    let mut response = conn.connection()
                        .query("SELECT *, <string>meta::id(id) as id FROM workspace WHERE name = $name")
                        .bind(("name", name.clone()))
                        .await?;

                    // Parse with string IDs and convert to UUIDs
                    #[derive(serde::Deserialize)]
                    struct WorkspaceWithStringId {
                        id: String,
                        #[serde(flatten)]
                        rest: serde_json::Value,
                    }

                    let workspaces_raw: Vec<WorkspaceWithStringId> = response.take(0)?;
                    workspaces_raw.first()
                        .map(|w| {
                            // Extract UUID from "workspace:uuid" format
                            let uuid_str = w.id.split(':').nth(1).unwrap_or(&w.id);
                            Uuid::parse_str(uuid_str)
                        })
                        .transpose()?
                        .ok_or_else(|| anyhow::anyhow!("Default workspace not found: {}", name))
                })
            })
    }
}

pub async fn vfs_ls(
    path: String,
    workspace: Option<String>,
    recursive: bool,
    hidden: bool,
    format: OutputFormat,
) -> Result<()> {
    let spinner = output::spinner("Listing directory...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage.clone());
    let vpath = VirtualPath::new(&path)?;
    let nodes = vfs.list_directory(&workspace_id, &vpath, recursive).await?;

    spinner.finish_and_clear();

    match format {
        OutputFormat::Json => {
            output::output(&nodes, format)?;
        }
        _ => {
            for node in nodes {
                if !hidden && node.path.to_string().contains("/.") {
                    continue;
                }
                let type_str = if node.is_directory() { "DIR " } else { "FILE" };
                let size_str = if node.is_file() {
                    format_bytes(node.size_bytes as u64)
                } else {
                    "-".to_string()
                };
                println!("{} {:>10} {}", type_str, size_str, node.path);
            }
        }
    }

    Ok(())
}

pub async fn vfs_cat(path: String, workspace: Option<String>) -> Result<()> {
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let vpath = VirtualPath::new(&path)?;
    let content = vfs.read_file(&workspace_id, &vpath).await?;

    print!("{}", String::from_utf8_lossy(&content));
    Ok(())
}

pub async fn vfs_tree(
    path: String,
    workspace: Option<String>,
    max_depth: usize,
    files: bool,
    format: OutputFormat,
) -> Result<()> {
    let spinner = output::spinner("Building tree...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let vpath = VirtualPath::new(&path)?;
    let nodes = vfs.list_directory(&workspace_id, &vpath, true).await?;

    spinner.finish_and_clear();

    // Build tree structure
    fn print_tree(nodes: &[VNode], prefix: &str, depth: usize, max_depth: usize, show_files: bool) {
        if depth > max_depth {
            return;
        }
        for (i, node) in nodes.iter().enumerate() {
            if !show_files && node.is_file() {
                continue;
            }
            let is_last = i == nodes.len() - 1;
            let connector = if is_last { "└──" } else { "├──" };
            let icon = if node.is_directory() { "📁" } else { "📄" };
            println!("{}{} {} {}", prefix, connector, icon, node.path.file_name().unwrap_or(""));
        }
    }

    match format {
        OutputFormat::Json => output::output(&nodes, format)?,
        _ => print_tree(&nodes, "", 0, max_depth, files),
    }

    Ok(())
}

pub async fn vfs_rm(path: String, workspace: Option<String>, recursive: bool) -> Result<()> {
    let spinner = output::spinner("Deleting...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let vpath = VirtualPath::new(&path)?;
    vfs.delete(&workspace_id, &vpath, recursive).await?;

    spinner.finish_and_clear();
    output::success(format!("Deleted: {}", path));
    Ok(())
}

pub async fn vfs_cp(
    source: String,
    target: String,
    workspace: Option<String>,
    _recursive: bool,
    _overwrite: bool,
) -> Result<()> {
    let spinner = output::spinner("Copying...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let src_path = VirtualPath::new(&source)?;
    let dst_path = VirtualPath::new(&target)?;

    // Copy by reading and writing
    let content = vfs.read_file(&workspace_id, &src_path).await?;
    vfs.write_file(&workspace_id, &dst_path, &content).await?;

    spinner.finish_and_clear();
    output::success(format!("Copied: {} -> {}", source, target));
    Ok(())
}

pub async fn vfs_mv(
    source: String,
    target: String,
    workspace: Option<String>,
    _overwrite: bool,
) -> Result<()> {
    let spinner = output::spinner("Moving...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let src_path = VirtualPath::new(&source)?;
    let dst_path = VirtualPath::new(&target)?;

    // Move by reading, writing, then deleting original
    let content = vfs.read_file(&workspace_id, &src_path).await?;
    vfs.write_file(&workspace_id, &dst_path, &content).await?;
    vfs.delete(&workspace_id, &src_path, false).await?;

    spinner.finish_and_clear();
    output::success(format!("Moved: {} -> {}", source, target));
    Ok(())
}

pub async fn vfs_mkdir(path: String, workspace: Option<String>, parents: bool) -> Result<()> {
    let spinner = output::spinner("Creating directory...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let vpath = VirtualPath::new(&path)?;
    vfs.create_directory(&workspace_id, &vpath, parents).await?;

    spinner.finish_and_clear();
    output::success(format!("Created directory: {}", path));
    Ok(())
}

pub async fn vfs_write(path: String, content: String, workspace: Option<String>) -> Result<()> {
    let spinner = output::spinner("Writing file...");
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let workspace_id = resolve_workspace_id(&storage, workspace).await?;

    let vfs = VirtualFileSystem::new(storage);
    let vpath = VirtualPath::new(&path)?;
    vfs.write_file(&workspace_id, &vpath, content.as_bytes()).await?;

    spinner.finish_and_clear();
    output::success(format!("Written: {}", path));
    Ok(())
}

// ============================================================================
// Code Commands
// ============================================================================

pub async fn code_create(
    file: String,
    unit_type: String,
    name: String,
    _body: String,
    _signature: Option<String>,
    _workspace: Option<String>,
) -> Result<()> {
    output::info(format!("Would create {} '{}' in {}", unit_type, name, file));
    output::warning("This command will be fully implemented with MCP tool integration");
    output::info("Use MCP tool cortex.code.create_unit for now");
    Ok(())
}

pub async fn code_rename(
    unit_id: String,
    name: String,
    _update_refs: bool,
    _workspace: Option<String>,
) -> Result<()> {
    output::info(format!("Would rename unit {} to '{}'", unit_id, name));
    output::warning("This command will be fully implemented with MCP tool integration");
    output::info("Use MCP tool cortex.code.rename_unit for now");
    Ok(())
}

pub async fn code_extract_function(
    unit_id: String,
    start_line: usize,
    end_line: usize,
    name: String,
    _workspace: Option<String>,
) -> Result<()> {
    output::info(format!("Would extract function '{}' from {} lines {}-{}", name, unit_id, start_line, end_line));
    output::warning("This command will be fully implemented with MCP tool integration");
    output::info("Use MCP tool cortex.code.extract_function for now");
    Ok(())
}

pub async fn code_optimize_imports(
    file: String,
    _remove_unused: bool,
    _sort: bool,
    _group: bool,
    _workspace: Option<String>,
) -> Result<()> {
    output::info(format!("Would optimize imports in {}", file));
    output::warning("This command will be fully implemented with MCP tool integration");
    output::info("Use MCP tool cortex.code.optimize_imports for now");
    Ok(())
}

// ============================================================================
// Additional Memory Commands
// ============================================================================

pub async fn memory_search_episodes(
    _query: String,
    _agent: Option<String>,
    _outcome: Option<String>,
    _limit: usize,
    _workspace: Option<String>,
    _format: OutputFormat,
) -> Result<()> {
    Err(anyhow::anyhow!("Memory search_episodes not yet implemented"))
}

pub async fn memory_find_similar(
    _query: String,
    _min_similarity: f32,
    _limit: usize,
    _workspace: Option<String>,
    _format: OutputFormat,
) -> Result<()> {
    Err(anyhow::anyhow!("Memory find_similar_episodes not yet implemented"))
}

pub async fn memory_record_episode(
    _task: String,
    _solution: String,
    _outcome: String,
    _workspace: Option<String>,
) -> Result<()> {
    Err(anyhow::anyhow!("Memory record_episode not yet implemented"))
}

pub async fn memory_get_episode(
    _episode_id: String,
    _include_changes: bool,
    _format: OutputFormat,
) -> Result<()> {
    Err(anyhow::anyhow!("Memory get_episode not yet implemented"))
}

pub use crate::qdrant_commands::{
    qdrant_init,
    qdrant_status,
    qdrant_migrate,
    qdrant_verify,
    qdrant_benchmark,
    qdrant_snapshot,
    qdrant_restore,
    qdrant_optimize,
    qdrant_list,
};
