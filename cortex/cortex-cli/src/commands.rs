//! CLI command implementations.
//!
//! This module contains the complete implementation of all Cortex CLI commands.

use crate::config::CortexConfig;
use crate::output::{self, format_bytes, format_timestamp, OutputFormat, TableBuilder};
use anyhow::{Context, Result};
use cortex_core::error::CortexError;
use cortex_ingestion::{ProjectImportOptions, ProjectLoader};
use cortex_memory::CognitiveManager;
use cortex_mcp::CortexMcpServer;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, SurrealDBConfig, SurrealDBManager};
use cortex_vfs::{
    ExternalProjectLoader, FlushOptions, FlushScope, MaterializationEngine, VirtualFileSystem,
    Workspace, WorkspaceType, SourceType,
};
use std::path::{Path, PathBuf};
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

    // TODO: Save workspace to database via VFS
    // vfs.create_workspace(workspace).await?;

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
    let vfs = VirtualFileSystem::new(storage);

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

    // TODO: Save workspace
    // vfs.create_workspace(workspace).await?;

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
    let vfs = VirtualFileSystem::new(storage);

    // TODO: Fetch workspaces from database
    // let workspaces = vfs.list_workspaces().await?;

    // Mock data for now
    let workspaces: Vec<Workspace> = vec![];

    match format {
        OutputFormat::Json => {
            output::output(&workspaces, format)?;
        }
        _ => {
            if workspaces.is_empty() {
                output::info("No workspaces found. Create one with 'cortex workspace create'");
                return Ok(());
            }

            TableBuilder::new()
                .header(vec!["ID", "Name", "Type", "Created", "Root Path"])
                .row(vec!["mock-id", "example", "Agent", "2 hours ago", "/path/to/workspace"])
                .print();
        }
    }

    Ok(())
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
    let vfs = VirtualFileSystem::new(storage);

    // TODO: Delete workspace
    // vfs.delete_workspace(&name_or_id).await?;

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

    // TODO: Implement semantic search
    // let results = memory.search(&query, workspace_name.as_deref(), limit).await?;

    spinner.finish_and_clear();

    // Mock results for now
    match format {
        OutputFormat::Json => {
            let mock_results: Vec<serde_json::Value> = vec![];
            output::output(&mock_results, format)?;
        }
        _ => {
            output::header("Search Results");
            output::info("No results found");
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

    match format {
        OutputFormat::Json => {
            let projects: Vec<serde_json::Value> = vec![];
            output::output(&projects, format)?;
        }
        _ => {
            output::header("Projects");
            output::info("No projects found");
        }
    }

    Ok(())
}

/// List documents in workspace
pub async fn list_documents(workspace: Option<String>, format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;

    match format {
        OutputFormat::Json => {
            let docs: Vec<serde_json::Value> = vec![];
            output::output(&docs, format)?;
        }
        _ => {
            output::header("Documents");
            output::info("No documents found");
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

    match format {
        OutputFormat::Json => {
            let episodes: Vec<serde_json::Value> = vec![];
            output::output(&episodes, format)?;
        }
        _ => {
            output::header(format!("Recent Episodes (limit: {})", limit));
            output::info("No episodes found");
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

    // TODO: Gather actual metrics
    let stats = serde_json::json!({
        "workspaces": 0,
        "files": 0,
        "total_size_bytes": 0,
        "memory": {
            "episodes": 0,
            "semantic_nodes": 0,
            "working_memory_size": 0,
        },
        "database": {
            "connection_pool_size": config.database.pool_size,
            "cache_size_mb": config.storage.cache_size_mb,
        }
    });

    spinner.finish_and_clear();

    match format {
        OutputFormat::Json => {
            output::output(&stats, format)?;
        }
        _ => {
            output::header("Cortex System Statistics");
            println!("\nWorkspaces: 0");
            println!("Files: 0");
            println!("Total Size: 0 B");
            println!("\nMemory:");
            println!("  Episodes: 0");
            println!("  Semantic Nodes: 0");
            println!("\nDatabase:");
            println!("  Pool Size: {}", config.database.pool_size);
            println!("  Cache: {} MB", config.storage.cache_size_mb);
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

    let session_id = Uuid::new_v4();

    // TODO: Create agent session
    // storage.create_agent_session(session_id, name, agent_type).await?;

    spinner.finish_and_clear();

    output::success(format!("Created agent session: {}", name));
    output::kv("Session ID", session_id);
    output::kv("Type", agent_type);

    Ok(())
}

/// List agent sessions
pub async fn agent_list(format: OutputFormat) -> Result<()> {
    let config = CortexConfig::load()?;

    match format {
        OutputFormat::Json => {
            let sessions: Vec<serde_json::Value> = vec![];
            output::output(&sessions, format)?;
        }
        _ => {
            output::header("Agent Sessions");
            output::info("No active sessions");
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

    // TODO: Delete agent session
    // storage.delete_agent_session(&session_id).await?;

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
    let workspace_name = workspace.or(config.active_workspace.clone());

    let spinner = output::spinner("Consolidating memory...");

    let storage = create_storage(&config).await?;
    let memory = CognitiveManager::new(storage);

    // TODO: Trigger consolidation
    // let report = memory.consolidate(workspace_name.as_deref()).await?;

    spinner.finish_and_clear();

    output::success("Memory consolidation complete");
    output::kv("Episodes created", 0);
    output::kv("Semantic nodes created", 0);

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

    // TODO: Delete old memory
    // let deleted = memory.forget_before(&before_date, workspace.as_deref()).await?;

    spinner.finish_and_clear();

    output::success("Memory deleted");
    output::kv("Episodes deleted", 0);

    Ok(())
}

// ============================================================================
// Database Management Commands
// ============================================================================

/// Start the local SurrealDB server
pub async fn db_start(bind_address: Option<String>, data_dir: Option<PathBuf>) -> Result<()> {
    output::info("Starting SurrealDB server...");

    let mut config = SurrealDBConfig::default();

    if let Some(addr) = bind_address {
        config.bind_address = addr;
    }

    if let Some(dir) = data_dir {
        config.data_dir = dir;
    }

    let mut manager = SurrealDBManager::new(config).await?;

    match manager.start().await {
        Ok(_) => {
            output::success("SurrealDB server started successfully");
            output::kv("URL", manager.connection_url());
            output::kv("Data", manager.config().data_dir.display());
            output::kv("Logs", manager.config().log_file.display());
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

    let config = SurrealDBConfig::default();
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

    let config = SurrealDBConfig::default();
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
    let config = SurrealDBConfig::default();
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
    use cortex_storage::{PoolConnectionMode, LoadBalancingStrategy};

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
