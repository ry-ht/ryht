use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

use meridian::{MeridianServer, Config};

#[derive(Parser)]
#[command(name = "meridian")]
#[command(about = "Cognitive memory system for LLM codebase interaction", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize global configuration
    InitConfig,

    /// Start the MCP server
    Serve {
        /// Use stdio transport (default)
        #[arg(long)]
        stdio: bool,

        /// Use HTTP/SSE transport with optional port
        #[arg(long)]
        http: bool,

        /// HTTP port (default: 3000)
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Project management
    Projects {
        #[command(subcommand)]
        command: ProjectsCommands,
    },

    /// Index a project
    Index {
        /// Project root directory
        path: PathBuf,

        /// Force full reindex
        #[arg(short, long)]
        force: bool,
    },

    /// Query the index
    Query {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show index statistics
    Stats {
        /// Detailed statistics
        #[arg(short, long)]
        detailed: bool,
    },

    /// Initialize a new index
    Init {
        /// Project root directory
        path: PathBuf,
    },
}


#[derive(Subcommand)]
enum ProjectsCommands {
    /// Add a monorepo/project
    Add {
        /// Path to monorepo or project
        path: PathBuf,
    },

    /// List all registered projects
    List {
        /// Filter by monorepo ID
        #[arg(long)]
        monorepo: Option<String>,
    },

    /// Show project information
    Info {
        /// Project ID
        project_id: String,
    },

    /// Relocate a project
    Relocate {
        /// Project ID (full_id like "@scope/name@version")
        project_id: String,

        /// New path
        new_path: PathBuf,
    },

    /// Remove a project
    Remove {
        /// Project ID
        project_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if we're in stdio mode - if so, disable console logging
    let is_stdio = matches!(cli.command, Commands::Serve { http: false, .. });

    // Initialize logging
    if is_stdio {
        // In STDIO mode, redirect logs to file to avoid interfering with JSON-RPC protocol
        use tracing_subscriber::fmt::writer::MakeWriterExt;
        use std::fs::{create_dir_all, OpenOptions};
        use meridian::config::get_meridian_home;

        // Create log directory
        let log_dir = get_meridian_home().join("logs");
        create_dir_all(&log_dir).ok();

        // Create log file
        if let Ok(log_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("meridian.log"))
        {
            tracing_subscriber::fmt()
                .with_writer(log_file.with_max_level(tracing::Level::DEBUG))
                .with_ansi(false)
                .with_env_filter("meridian=info")
                .init();
        } else {
            // Fallback: disable logging if we can't create log file
            tracing_subscriber::fmt()
                .with_writer(std::io::sink)
                .init();
        }
    } else {
        // For non-STDIO modes, log to stderr as usual
        if cli.verbose {
            tracing_subscriber::fmt()
                .with_env_filter("meridian=debug")
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter("meridian=info")
                .init();
        }
    }

    info!("Meridian cognitive memory system starting...");

    // Load configuration from global location
    let config = Config::load()?;

    match cli.command {
        Commands::InitConfig => {
            Config::init_global()?;
            println!("✓ Global configuration initialized at {:?}", Config::global_config_path());
            println!("Edit this file to customize Meridian settings.");
        }
        Commands::Serve { stdio, http, port } => {
            info!("Starting MCP server...");
            serve_mcp(config, stdio, http, port).await?;
        }
        Commands::Projects { command } => {
            handle_projects_command(command).await?;
        }
        Commands::Index { path, force } => {
            info!("Indexing project at {:?}", path);
            index_project(config, path, force).await?;
        }
        Commands::Query { query, limit } => {
            info!("Executing query: {}", query);
            execute_query(config, query, limit).await?;
        }
        Commands::Stats { detailed } => {
            show_stats(config, detailed).await?;
        }
        Commands::Init { path } => {
            info!("Initializing index at {:?}", path);
            initialize_index(config, path).await?;
        }
    }

    Ok(())
}


async fn handle_projects_command(command: ProjectsCommands) -> Result<()> {
    use meridian::global::{GlobalStorage, ProjectRegistryManager};
    use meridian::config::get_meridian_home;
    use std::sync::Arc;

    // Get global storage path
    let data_dir = get_meridian_home().join("data");

    std::fs::create_dir_all(&data_dir)?;

    let storage = Arc::new(GlobalStorage::new(&data_dir).await?);
    let manager = Arc::new(ProjectRegistryManager::new(storage));

    match command {
        ProjectsCommands::Add { path } => {
            info!("Adding project at {:?}", path);
            let registry = manager.register(path.clone()).await?;
            println!("Project registered:");
            println!("  ID: {}", registry.identity.full_id);
            println!("  Name: {}", registry.identity.id);
            println!("  Version: {}", registry.identity.version);
            println!("  Type: {:?}", registry.identity.project_type);
            println!("  Path: {:?}", path);
        }
        ProjectsCommands::List { monorepo } => {
            let projects = manager.list_all().await?;

            if projects.is_empty() {
                println!("No projects registered.");
                println!("Use 'meridian projects add <path>' to register a project.");
                return Ok(());
            }

            println!("Registered projects ({}):", projects.len());
            println!();

            for project in projects {
                if let Some(ref filter) = monorepo {
                    if let Some(ref mono) = project.monorepo {
                        if &mono.id != filter {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                println!("  {} ({})", project.identity.id, project.identity.version);
                println!("    Full ID: {}", project.identity.full_id);
                println!("    Type: {:?}", project.identity.project_type);
                println!("    Path: {:?}", project.current_path);
                println!("    Status: {:?}", project.status);
                if let Some(ref mono) = project.monorepo {
                    println!("    Monorepo: {}", mono.id);
                }
                println!();
            }
        }
        ProjectsCommands::Info { project_id } => {
            match manager.get(&project_id).await? {
                Some(project) => {
                    println!("Project: {}", project.identity.id);
                    println!("  Full ID: {}", project.identity.full_id);
                    println!("  Version: {}", project.identity.version);
                    println!("  Type: {:?}", project.identity.project_type);
                    println!("  Current Path: {:?}", project.current_path);
                    println!("  Status: {:?}", project.status);
                    println!("  Created: {}", project.created_at);
                    println!("  Updated: {}", project.updated_at);
                    println!();
                    println!("Path History:");
                    for (i, entry) in project.path_history.iter().enumerate() {
                        println!(
                            "  {}. {} - {} ({})",
                            i + 1,
                            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            entry.path,
                            entry.reason
                        );
                    }
                }
                None => {
                    println!("Project not found: {}", project_id);
                }
            }
        }
        ProjectsCommands::Relocate {
            project_id,
            new_path,
        } => {
            manager
                .relocate_project(&project_id, new_path.clone(), "user-initiated".to_string())
                .await?;
            println!("Project relocated to {:?}", new_path);
        }
        ProjectsCommands::Remove { project_id } => {
            manager.delete(&project_id).await?;
            println!("Project marked as deleted: {}", project_id);
        }
    }

    Ok(())
}

async fn serve_mcp(mut config: Config, stdio: bool, http: bool, port: u16) -> Result<()> {
    if http {
        // Update config with HTTP port
        if config.mcp.http.is_none() {
            config.mcp.http = Some(meridian::config::HttpConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port,
                max_connections: 10,
                cors_origins: vec![],
            });
        } else if let Some(ref mut http_config) = config.mcp.http {
            http_config.enabled = true;
            http_config.port = port;
        }

        // Create Meridian server in multi-project mode for HTTP
        info!("Starting MCP server with HTTP/SSE transport on port {}", port);
        let mut server = MeridianServer::new_for_http(config)?;
        server.serve_http().await?;
    } else {
        // Create Meridian server in single-project mode for stdio (default)
        info!("Starting MCP server with stdio transport");
        let mut server = MeridianServer::new(config).await?;
        server.serve_stdio().await?;
    }

    Ok(())
}

async fn index_project(config: Config, path: PathBuf, force: bool) -> Result<()> {
    // Convert to absolute path first
    let absolute_path = if path.is_absolute() {
        path
    } else {
        // Try to canonicalize first (resolves symlinks and makes absolute)
        path.canonicalize()
            .or_else(|_| {
                // Fallback: manually construct absolute path
                std::env::current_dir()
                    .map(|cwd| cwd.join(&path))
            })?
    };

    let mut server = MeridianServer::new(config).await?;

    if force {
        info!("Forcing full reindex");
    }

    server.index_project(absolute_path.clone(), force).await?;
    info!("Indexing completed successfully");

    // Register project in global registry
    use meridian::global::{GlobalStorage, ProjectRegistryManager};
    use meridian::config::get_meridian_home;
    use std::sync::Arc;

    let data_dir = get_meridian_home().join("data");
    std::fs::create_dir_all(&data_dir)?;

    let storage = Arc::new(GlobalStorage::new(&data_dir).await?);
    let manager = Arc::new(ProjectRegistryManager::new(storage));

    // Register the project with absolute path
    let registry = manager.register(absolute_path).await?;
    info!("Project registered in global registry: {}", registry.identity.full_id);

    // Set as current project
    manager.set_current_project(&registry.identity.full_id).await?;
    info!("Set as current project");

    if let Some(specs_path) = &registry.specs_path {
        info!("Specs directory detected: {:?}", specs_path);
    } else {
        info!("No specs directory found at project root");
    }

    Ok(())
}

async fn execute_query(config: Config, query: String, limit: usize) -> Result<()> {
    let server = MeridianServer::new(config).await?;

    let results = server.query(&query, limit).await?;

    println!("Query results ({} found):", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result);
    }

    Ok(())
}

async fn show_stats(config: Config, detailed: bool) -> Result<()> {
    let server = MeridianServer::new(config).await?;

    let stats = server.get_stats().await?;

    println!("Meridian Index Statistics");
    println!("========================");
    println!("Total symbols: {}", stats.total_symbols);
    println!("Total files: {}", stats.total_files);
    println!("Total projects: {}", stats.total_projects);
    println!("Index size: {} MB", stats.index_size_mb);

    if detailed {
        println!("\nDetailed Statistics:");
        println!("-------------------");
        println!("Episodes: {}", stats.episodes_count);
        println!("Working memory size: {}", stats.working_memory_size);
        println!("Semantic patterns: {}", stats.semantic_patterns);
        println!("Procedural knowledge: {}", stats.procedures_count);
    }

    Ok(())
}

async fn initialize_index(config: Config, path: PathBuf) -> Result<()> {
    info!("Initializing new index at {:?}", path);

    let server = MeridianServer::new(config).await?;
    let path_display = path.display().to_string();
    server.initialize(path).await?;

    info!("Index initialized successfully");
    println!("Meridian index initialized at {:?}", path_display);
    println!("Run 'meridian index {}' to start indexing", path_display);

    Ok(())
}
