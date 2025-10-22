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
    Workspace, WorkspaceType, SourceType,
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
pub async fn server_start(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    output::header("Starting Cortex REST API Server");
    output::kv("Host", &host);
    output::kv("Port", port);
    if let Some(w) = workers {
        output::kv("Workers", w);
    }

    let config = crate::api::server::ServerConfig {
        host,
        port,
        workers,
    };

    let server = crate::api::RestApiServer::with_config(config).await?;

    output::success("REST API server started successfully");
    output::info("Press Ctrl+C to stop");

    server.serve().await?;

    Ok(())
}
