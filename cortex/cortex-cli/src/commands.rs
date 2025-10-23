//! CLI command implementations.
//!
//! This module contains the complete implementation of all Cortex CLI commands.

use crate::config::CortexConfig;
use crate::mcp::CortexMcpServer;
use crate::output::{self, format_bytes, OutputFormat, TableBuilder};
use anyhow::{Context, Result};
use cortex_memory::CognitiveManager;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, SurrealDBConfig, SurrealDBManager};
use cortex_vfs::{
    ExternalProjectLoader, FlushOptions, FlushScope, MaterializationEngine, VirtualFileSystem,
    VirtualPath, VNode, Workspace, WorkspaceType, SourceType,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Init Command
// ============================================================================

/// Initialize a new Cortex workspace
pub async fn init_workspace(
    name: String,
    path: Option<PathBuf>,
    workspace_type: WorkspaceType,
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
    workspace_config.active_workspace = Some(name.clone());
    workspace_config.save_project()?;

    // Initialize storage
    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage.clone());

    // Create workspace in VFS
    let workspace_id = Uuid::new_v4();
    let workspace = Workspace {
        id: workspace_id,
        name: name.clone(),
        workspace_type,
        source_type: SourceType::Local,
        namespace: format!("workspace_{}", workspace_id),
        source_path: Some(workspace_path.clone()),
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Save workspace metadata to database
    let conn = storage.acquire().await?;
    let _: Option<Workspace> = conn.connection().create("workspace")
        .content(workspace)
        .await
        .context("Failed to create workspace in database")?;

    // Initialize root directory in VFS
    let root_path = VirtualPath::new(".")?;
    vfs.create_directory(&workspace_id, &root_path, true).await
        .context("Failed to create root directory in VFS")?;

    spinner.finish_and_clear();

    output::success(format!("Initialized Cortex workspace: {}", name));
    output::kv("Workspace ID", workspace_id);
    output::kv("Type", format!("{:?}", workspace_type));
    output::kv("Path", workspace_path.display());
    output::kv("Config", cortex_dir.join("config.toml").display());

    Ok(())
}

// ============================================================================
// Workspace Management Commands
// ============================================================================

/// Create a new workspace
pub async fn workspace_create(name: String, workspace_type: WorkspaceType) -> Result<()> {
    let spinner = output::spinner("Creating workspace...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage.clone());

    let workspace_id = Uuid::new_v4();
    let workspace = Workspace {
        id: workspace_id,
        name: name.clone(),
        workspace_type,
        source_type: SourceType::Local,
        namespace: format!("workspace_{}", workspace_id),
        source_path: None,
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Save workspace to database
    let conn = storage.acquire().await?;
    let _: Option<Workspace> = conn.connection().create("workspace")
        .content(workspace)
        .await
        .context("Failed to create workspace in database")?;

    // Initialize root directory in VFS
    let root_path = VirtualPath::new(".")?;
    vfs.create_directory(&workspace_id, &root_path, true).await
        .context("Failed to create root directory in VFS")?;

    spinner.finish_and_clear();
    output::success(format!("Created workspace: {}", name));
    output::kv("ID", workspace_id);
    output::kv("Type", format!("{:?}", workspace_type));

    Ok(())
}

/// List all workspaces
pub async fn workspace_list(format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let _vfs = VirtualFileSystem::new(storage.clone());

    // Fetch workspaces from database
    let conn = storage.acquire().await?;
    let workspaces: Vec<Workspace> = conn.connection().select("workspace")
        .await
        .context("Failed to fetch workspaces from database")?;

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
                .header(vec!["ID", "Name", "Type", "Created", "Root Path"]);

            let mut table_with_rows = table;
            for ws in &workspaces {
                let path_str = ws.source_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "N/A".to_string());

                let created = format_relative_time(&ws.created_at);

                table_with_rows = table_with_rows.row(vec![
                    ws.id.to_string(),
                    ws.name.clone(),
                    format!("{:?}", ws.workspace_type),
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
pub async fn workspace_delete(name_or_id: String, force: bool) -> Result<()> {
    if !force && !output::confirm(format!("Delete workspace '{}'?", name_or_id))? {
        output::info("Cancelled");
        return Ok(());
    }

    let spinner = output::spinner("Deleting workspace...");

    let config = CortexConfig::load()?;
    let storage = create_storage(&config).await?;
    let _vfs = VirtualFileSystem::new(storage.clone());

    // Find workspace by name or ID
    let conn = storage.acquire().await?;

    // Try to parse as UUID first
    let workspace_id = if let Ok(uuid) = Uuid::parse_str(&name_or_id) {
        uuid
    } else {
        // Search by name
        let mut response = conn.connection()
            .query("SELECT * FROM workspace WHERE name = $name")
            .bind(("name", name_or_id.clone()))
            .await
            .context("Failed to query workspace")?;
        let workspaces: Vec<Workspace> = response.take(0)?;

        if workspaces.is_empty() {
            return Err(anyhow::anyhow!("Workspace not found: {}", name_or_id));
        }

        workspaces[0].id
    };

    // Delete all VNodes in this workspace
    let mut _response = conn.connection()
        .query("DELETE FROM vnode WHERE workspace_id = $workspace_id")
        .bind(("workspace_id", workspace_id))
        .await
        .context("Failed to delete workspace vnodes")?;

    // Delete the workspace
    let _: Option<Workspace> = conn.connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .context("Failed to delete workspace from database")?;

    spinner.finish_and_clear();
    output::success(format!("Deleted workspace: {}", name_or_id));

    Ok(())
}

/// Switch active workspace
pub async fn workspace_switch(name: String) -> Result<()> {
    let mut config = CortexConfig::load()?;
    config.active_workspace = Some(name.clone());
    config.save_project()?;

    output::success(format!("Switched to workspace: {}", name));
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
    let workspace_name = workspace.or(config.active_workspace.clone())
        .ok_or_else(|| anyhow::anyhow!("No active workspace. Use --workspace or 'cortex workspace switch'"))?;

    output::header(format!("Ingesting: {}", path.display()));
    output::kv("Workspace", &workspace_name);
    output::kv("Recursive", recursive);

    let spinner = output::spinner("Loading project...");

    let storage = create_storage(&config).await?;
    let vfs = VirtualFileSystem::new(storage.clone());
    let loader = ExternalProjectLoader::new(vfs.clone());

    // Import project
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
    };

    let report = loader.import_project(&path, options).await?;

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
    let workspace_name = workspace.or(config.active_workspace.clone());

    let spinner = output::spinner("Searching...");

    let storage = create_storage(&config).await?;
    let memory = CognitiveManager::new(storage);

    // Create a memory query
    let memory_query = cortex_memory::types::MemoryQuery::new(query.clone())
        .with_limit(limit)
        .with_threshold(0.6);

    // Generate a simple embedding for the query (placeholder - in production use a proper embedding model)
    // For now, create a zero vector as we don't have an embedding service configured
    let embedding = vec![0.0f32; 384]; // Standard embedding dimension

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
    let workspace_name = workspace.or(config.active_workspace.clone());

    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Get workspaces (projects are represented as workspaces in our system)
    let mut workspaces: Vec<Workspace> = conn.connection().select("workspace")
        .await
        .context("Failed to fetch workspaces from database")?;

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
                    println!("  {} - {} ({:?})", ws.name, ws.id, ws.workspace_type);
                    if let Some(path) = &ws.source_path {
                        println!("    Path: {}", path.display());
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
    let workspace_name = workspace.or(config.active_workspace.clone());

    let storage = create_storage(&config).await?;
    let conn = storage.acquire().await?;

    // Find workspace ID if workspace name is provided
    let workspace_id = if let Some(ref ws_name) = workspace_name {
        let mut response = conn.connection()
            .query("SELECT * FROM workspace WHERE name = $name LIMIT 1")
            .bind(("name", ws_name.clone()))
            .await
            .context("Failed to query workspace")?;
        let workspaces: Vec<Workspace> = response.take(0)?;

        if workspaces.is_empty() {
            return Err(anyhow::anyhow!("Workspace not found: {}", ws_name));
        }

        Some(workspaces[0].id)
    } else {
        None
    };

    // Query vnodes of type Document
    let mut response = if let Some(ws_id) = workspace_id {
        conn.connection().query("SELECT * FROM vnode WHERE workspace_id = $workspace_id AND node_type = 'document'")
            .bind(("workspace_id", ws_id))
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
    let workspace_name = workspace.or(config.active_workspace.clone());

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
    use cortex_core::traits::Storage;

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

    if let Some(workspace) = &config.active_workspace {
        println!("\nActive Workspace: {}", workspace);
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
pub async fn memory_consolidate(workspace: Option<String>) -> Result<()> {
    let config = CortexConfig::load()?;
    let _workspace_name = workspace.or(config.active_workspace.clone());

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

/// Start the local SurrealDB server in background
pub async fn db_start(bind_address: Option<String>, data_dir: Option<PathBuf>) -> Result<()> {
    output::info("Starting SurrealDB server...");

    // Load global config to get database credentials
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    let mut config = SurrealDBConfig::default();

    // Use credentials from global config
    config.username = db_config.username.clone();
    config.password = db_config.password.clone();

    // Override bind address from global config if not provided as argument
    if let Some(addr) = bind_address {
        config.bind_address = addr;
    } else {
        // Use local_bind from config
        config.bind_address = db_config.local_bind.clone();
    }

    if let Some(dir) = data_dir {
        config.data_dir = dir;
    }

    let mut manager = SurrealDBManager::new(config).await?;

    match manager.start().await {
        Ok(_) => {
            output::success("SurrealDB server started successfully in background");
            output::kv("URL", manager.connection_url());
            output::kv("Data", manager.config().data_dir.display());
            output::kv("Logs", manager.config().log_file.display());
            output::kv("PID file", manager.config().pid_file.display());
            output::info("Use 'cortex db stop' to stop the server");
            output::info("Use 'cortex db status' to check server status");
            println!();

            // Don't wait for shutdown - server runs in background
            // The process is configured with kill_on_drop(false) so it continues running
            std::mem::forget(manager); // Prevent Drop from being called
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to start SurrealDB server: {}", e));
            Err(e.into())
        }
    }
}

/// Stop the local SurrealDB server
pub async fn db_stop() -> Result<()> {
    output::info("Stopping SurrealDB server...");

    // Load global config to ensure consistent configuration
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    let mut config = SurrealDBConfig::default();
    config.username = db_config.username.clone();
    config.password = db_config.password.clone();
    config.bind_address = db_config.local_bind.clone();

    let mut manager = SurrealDBManager::new(config).await?;

    match manager.stop().await {
        Ok(_) => {
            output::success("SurrealDB server stopped successfully");
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to stop SurrealDB server: {}", e));
            Err(e.into())
        }
    }
}

/// Restart the local SurrealDB server
pub async fn db_restart() -> Result<()> {
    output::info("Restarting SurrealDB server...");

    // Load global config to ensure consistent configuration
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    let mut config = SurrealDBConfig::default();
    config.username = db_config.username.clone();
    config.password = db_config.password.clone();
    config.bind_address = db_config.local_bind.clone();

    let mut manager = SurrealDBManager::new(config).await?;

    match manager.restart().await {
        Ok(_) => {
            output::success("SurrealDB server restarted successfully");
            output::kv("URL", manager.connection_url());
            Ok(())
        }
        Err(e) => {
            output::error(format!("Failed to restart SurrealDB server: {}", e));
            Err(e.into())
        }
    }
}

/// Check the status of the local SurrealDB server
pub async fn db_status() -> Result<()> {
    // Load global config to ensure consistent configuration
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    let mut config = SurrealDBConfig::default();
    config.username = db_config.username.clone();
    config.password = db_config.password.clone();
    config.bind_address = db_config.local_bind.clone();

    let manager = SurrealDBManager::new(config).await?;

    output::header("SurrealDB Server Status");
    output::kv("URL", manager.connection_url());
    output::kv("Data", manager.config().data_dir.display());
    output::kv("Logs", manager.config().log_file.display());
    output::kv("PID file", manager.config().pid_file.display());
    println!();

    let is_running = manager.is_running().await;

    if is_running {
        output::success("Status: Running");

        match manager.health_check().await {
            Ok(_) => {
                output::success("Health: Healthy");
            }
            Err(e) => {
                output::error(format!("Health: Unhealthy ({})", e));
            }
        }
    } else {
        output::warning("Status: Stopped");
    }

    Ok(())
}

/// Install SurrealDB if not already installed
pub async fn db_install() -> Result<()> {
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a storage connection manager from config
async fn create_storage(config: &CortexConfig) -> Result<Arc<ConnectionManager>> {
    use cortex_storage::PoolConnectionMode;

    let db_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: config.database.connection_string.clone(),
        },
        credentials: Credentials {
            username: config.database.username.clone(),
            password: config.database.password.clone(),
        },
        pool_config: PoolConfig {
            max_connections: config.database.pool_size,
            ..Default::default()
        },
        namespace: config.database.namespace.clone(),
        database: config.database.database.clone(),
    };

    let manager = ConnectionManager::new(db_config)
        .await
        .context("Failed to create storage connection")?;

    Ok(Arc::new(manager))
}

// ============================================================================
// MCP Commands
// ============================================================================

/// Start MCP server in stdio mode
pub async fn mcp_stdio() -> Result<()> {
    output::header("Starting Cortex MCP Server (stdio mode)");
    output::info("Initializing server...");

    let server = CortexMcpServer::new().await
        .context("Failed to initialize MCP server")?;

    output::success("MCP server started successfully");
    output::info("Listening on stdio");
    output::info("Press Ctrl+C to stop");

    server.serve_stdio().await?;
    Ok(())
}

/// Start MCP server in HTTP mode
pub async fn mcp_http(address: String, port: u16) -> Result<()> {
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
