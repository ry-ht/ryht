//! Cortex CLI - Command-line interface for the Cortex cognitive memory system.
//!
//! # Usage
//!
//! ```bash
//! # Initialize a workspace
//! cortex init my-project
//!
//! # Ingest files
//! cortex ingest ./src
//!
//! # Search memory
//! cortex search "authentication logic"
//!
//! # Start MCP server
//! cortex mcp http
//!
//! # Manage database
//! cortex db start
//! cortex db status
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use cortex_cli::{commands, output, OutputFormat};
use cortex_cli::qdrant_commands;
use cortex_vfs::{FlushScope, WorkspaceType};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "cortex")]
#[command(about = "Cortex - Cognitive memory system for code", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output format (human, json, plain)
    #[arg(long, global = true, default_value = "human")]
    format: OutputFormatArg,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormatArg {
    Human,
    Json,
    Plain,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Human => OutputFormat::Human,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Plain => OutputFormat::Plain,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum WorkspaceTypeArg {
    Code,
    Documentation,
    Mixed,
    External,
}

impl From<WorkspaceTypeArg> for WorkspaceType {
    fn from(arg: WorkspaceTypeArg) -> Self {
        match arg {
            WorkspaceTypeArg::Code => WorkspaceType::Code,
            WorkspaceTypeArg::Documentation => WorkspaceType::Documentation,
            WorkspaceTypeArg::Mixed => WorkspaceType::Mixed,
            WorkspaceTypeArg::External => WorkspaceType::External,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Cortex workspace
    Init {
        /// Workspace name
        name: String,

        /// Workspace path (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Workspace type
        #[arg(short = 't', long, default_value = "code")]
        workspace_type: WorkspaceTypeArg,
    },

    /// Workspace management
    #[command(subcommand)]
    Workspace(WorkspaceCommands),

    /// Ingest files or directories into Cortex
    Ingest {
        /// Path to ingest
        path: PathBuf,

        /// Target workspace (uses active workspace if not specified)
        #[arg(short, long)]
        workspace: Option<String>,

        /// Recursively ingest directories
        #[arg(short, long, default_value = "true")]
        recursive: bool,
    },

    /// Search across Cortex memory
    Search {
        /// Search query
        query: String,

        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Search in specific workspace
        #[arg(short, long)]
        workspace: Option<String>,
    },

    /// List entities
    #[command(subcommand)]
    List(ListCommands),

    /// Flush VFS to disk
    Flush {
        /// Workspace name
        workspace: String,

        /// Target path
        target: PathBuf,

        /// Flush scope (all, workspace, project)
        #[arg(short, long, default_value = "workspace")]
        scope: FlushScopeArg,
    },

    /// Show system statistics
    Stats,

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Agent session management
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Memory operations
    #[command(subcommand)]
    Memory(MemoryCommands),

    /// Database management
    #[command(subcommand)]
    Db(DbCommands),

    /// System diagnostics and health checks
    #[command(subcommand)]
    Doctor(DoctorCommands),

    /// Run system tests
    #[command(subcommand)]
    Test(TestCommands),

    /// Export data to various formats
    #[command(subcommand)]
    Export(ExportCommands),

    /// Model Context Protocol operations
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Qdrant vector database operations
    #[command(subcommand)]
    Qdrant(QdrantCommands),

    /// Interactive mode
    Interactive {
        /// Interactive mode to enter (wizard, search, menu)
        #[arg(short, long, default_value = "menu")]
        mode: String,
    },

    /// REST API server management
    #[command(subcommand)]
    Server(ServerCommands),

    /// Internal command to run server (hidden)
    #[command(hide = true)]
    #[command(name = "internal-server-run")]
    InternalServerRun {
        #[arg(long)]
        host: String,

        #[arg(long)]
        port: u16,

        #[arg(long)]
        workers: Option<usize>,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommands {
    /// Create a new workspace
    Create {
        /// Workspace name
        name: String,

        /// Workspace type
        #[arg(short = 't', long, default_value = "code")]
        workspace_type: WorkspaceTypeArg,
    },

    /// List all workspaces
    List,

    /// Delete a workspace
    Delete {
        /// Workspace name or ID
        name_or_id: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Switch active workspace
    Switch {
        /// Workspace name
        name: String,
    },
}

#[derive(Subcommand)]
enum ListCommands {
    /// List projects
    Projects {
        /// Workspace to list from
        #[arg(short, long)]
        workspace: Option<String>,
    },

    /// List documents
    Documents {
        /// Workspace to list from
        #[arg(short, long)]
        workspace: Option<String>,
    },

    /// List memory episodes
    Episodes {
        /// Workspace to list from
        #[arg(short, long)]
        workspace: Option<String>,

        /// Limit results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., "database.namespace")
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,

        /// Set globally (system-wide) instead of project-local
        #[arg(short, long)]
        global: bool,
    },

    /// List all configuration values
    List,
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Create a new agent session
    Create {
        /// Session name
        name: String,

        /// Agent type
        #[arg(short = 't', long, default_value = "general")]
        agent_type: String,
    },

    /// List agent sessions
    List,

    /// Delete an agent session
    Delete {
        /// Session ID
        session_id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Consolidate memory (working -> episodic/semantic)
    Consolidate {
        /// Workspace to consolidate
        #[arg(short, long)]
        workspace: Option<String>,
    },

    /// Forget (delete) old memory
    Forget {
        /// Delete memory before this date (YYYY-MM-DD)
        before: String,

        /// Workspace to forget from
        #[arg(short, long)]
        workspace: Option<String>,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// Start the local SurrealDB server
    Start {
        /// Bind address (default: 127.0.0.1:8000)
        #[arg(short, long)]
        bind: Option<String>,

        /// Data directory
        #[arg(short, long)]
        data_dir: Option<PathBuf>,
    },

    /// Stop the local SurrealDB server
    Stop,

    /// Restart the local SurrealDB server
    Restart,

    /// Check server status
    Status,

    /// Install SurrealDB
    Install,
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the REST API server
    Start {
        /// Server host address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Number of worker threads
        #[arg(long)]
        workers: Option<usize>,
    },

    /// Stop the REST API server
    Stop,

    /// Check server status
    Status,
}

#[derive(Subcommand)]
enum DoctorCommands {
    /// Run all diagnostic checks
    Check {
        /// Automatically fix issues
        #[arg(short, long)]
        fix: bool,
    },

    /// Quick health check
    Health,
}

#[derive(Subcommand)]
enum TestCommands {
    /// Run all system tests
    All,

    /// Run performance benchmarks
    Benchmark,

    /// Test specific component
    Component {
        /// Component to test (database, storage, vfs, memory, mcp)
        component: String,
    },
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export workspace data
    Workspace {
        /// Workspace name
        workspace: String,

        /// Output file path
        #[arg(short, long)]
        output: std::path::PathBuf,

        /// Export format (json, csv, yaml, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Export memory episodes
    Episodes {
        /// Workspace name
        #[arg(short, long)]
        workspace: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: std::path::PathBuf,

        /// Export format (json, csv, yaml, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Limit number of episodes
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Export system statistics
    Stats {
        /// Output file path
        #[arg(short, long)]
        output: std::path::PathBuf,

        /// Export format (json, csv, yaml, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Start MCP server in stdio mode
    Stdio,

    /// Start MCP server in HTTP mode
    Http {
        /// Server address
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Server port
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Show information about available MCP tools
    Info {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum FlushScopeArg {
    All,
    Workspace,
    Project,
}

// Note: FlushScopeArg needs to be converted in context where we have access to config/workspace
// This simple From impl provides a default mapping.
// For workspace-specific flushing, use FlushScope::Workspace(uuid) directly with the active workspace ID.
impl From<FlushScopeArg> for FlushScope {
    fn from(arg: FlushScopeArg) -> Self {
        match arg {
            FlushScopeArg::All => FlushScope::All,
            // Workspace and Project scopes fall back to All here.
            // The actual workspace-specific flush should be done by:
            // 1. Loading the active workspace from config
            // 2. Using FlushScope::Workspace(workspace_id)
            FlushScopeArg::Workspace => FlushScope::All,
            FlushScopeArg::Project => FlushScope::All,
        }
    }
}

#[derive(Subcommand)]
enum QdrantCommands {
    /// Initialize Qdrant collections
    Init {
        /// Force recreate existing collections
        #[arg(short, long)]
        force: bool,

        /// Skip verification after creation
        #[arg(long)]
        skip_verify: bool,
    },

    /// Check Qdrant health and statistics
    Status {
        /// Show detailed collection information
        #[arg(short, long)]
        detailed: bool,

        /// Filter by collection name
        #[arg(short, long)]
        collection: Option<String>,
    },

    /// Migrate data from HNSW to Qdrant
    Migrate {
        /// Source collection or path
        source: String,

        /// Target Qdrant collection
        #[arg(short, long)]
        target: String,

        /// Batch size for migration
        #[arg(short, long, default_value = "500")]
        batch_size: usize,

        /// Enable dry-run mode
        #[arg(long)]
        dry_run: bool,
    },

    /// Verify Qdrant data consistency
    Verify {
        /// Collection to verify (all if not specified)
        #[arg(short, long)]
        collection: Option<String>,

        /// Fix inconsistencies automatically
        #[arg(short, long)]
        fix: bool,
    },

    /// Run performance benchmarks
    Benchmark {
        /// Collection to benchmark
        #[arg(short, long)]
        collection: Option<String>,

        /// Number of queries to run
        #[arg(short, long, default_value = "100")]
        num_queries: usize,

        /// Vector dimensionality for test data
        #[arg(short, long, default_value = "1536")]
        dimensions: usize,
    },

    /// Create a snapshot of Qdrant data
    Snapshot {
        /// Collection to snapshot (all if not specified)
        #[arg(short, long)]
        collection: Option<String>,

        /// Output directory for snapshot
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Restore from a snapshot
    Restore {
        /// Snapshot file or directory
        snapshot: PathBuf,

        /// Target collection (required if snapshot contains single collection)
        #[arg(short, long)]
        collection: Option<String>,
    },

    /// Optimize collection (trigger segment optimization)
    Optimize {
        /// Collection to optimize
        collection: String,

        /// Wait for optimization to complete
        #[arg(short, long)]
        wait: bool,
    },

    /// List all collections
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        output::error(format!("{:#}", e));
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Skip default logging for MCP stdio mode (it will use file-only logging)
    let is_mcp_stdio = matches!(&cli.command, Commands::Mcp(McpCommands::Stdio));

    if !is_mcp_stdio {
        init_logging(cli.verbose);
    }

    let format = OutputFormat::from(cli.format);

    match cli.command {
        Commands::Init {
            name,
            path,
            workspace_type,
        } => {
            commands::init_workspace(name, path, workspace_type.into()).await?;
        }

        Commands::Workspace(workspace_cmd) => match workspace_cmd {
            WorkspaceCommands::Create {
                name,
                workspace_type,
            } => {
                commands::workspace_create(name, workspace_type.into()).await?;
            }
            WorkspaceCommands::List => {
                commands::workspace_list(format).await?;
            }
            WorkspaceCommands::Delete { name_or_id, force } => {
                commands::workspace_delete(name_or_id, force).await?;
            }
            WorkspaceCommands::Switch { name } => {
                commands::workspace_switch(name).await?;
            }
        },

        Commands::Ingest {
            path,
            workspace,
            recursive,
        } => {
            commands::ingest_path(path, workspace, recursive).await?;
        }

        Commands::Search {
            query,
            limit,
            workspace,
        } => {
            commands::search_memory(query, workspace, limit, format).await?;
        }

        Commands::List(list_cmd) => match list_cmd {
            ListCommands::Projects { workspace } => {
                commands::list_projects(workspace, format).await?;
            }
            ListCommands::Documents { workspace } => {
                commands::list_documents(workspace, format).await?;
            }
            ListCommands::Episodes { workspace, limit } => {
                commands::list_episodes(workspace, limit, format).await?;
            }
        },

        Commands::Flush {
            workspace,
            target,
            scope,
        } => {
            commands::flush_vfs(workspace, target, scope.into()).await?;
        }

        Commands::Stats => {
            commands::show_stats(format).await?;
        }

        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Get { key } => {
                commands::config_get(key).await?;
            }
            ConfigCommands::Set { key, value, global } => {
                commands::config_set(key, value, global).await?;
            }
            ConfigCommands::List => {
                commands::config_list().await?;
            }
        },

        Commands::Agent(agent_cmd) => match agent_cmd {
            AgentCommands::Create { name, agent_type } => {
                commands::agent_create(name, agent_type).await?;
            }
            AgentCommands::List => {
                commands::agent_list(format).await?;
            }
            AgentCommands::Delete { session_id } => {
                commands::agent_delete(session_id).await?;
            }
        },

        Commands::Memory(memory_cmd) => match memory_cmd {
            MemoryCommands::Consolidate { workspace } => {
                commands::memory_consolidate(workspace).await?;
            }
            MemoryCommands::Forget { before, workspace } => {
                commands::memory_forget(before, workspace).await?;
            }
        },

        Commands::Db(db_cmd) => match db_cmd {
            DbCommands::Start { bind, data_dir } => {
                commands::db_start(bind, data_dir).await?;
            }
            DbCommands::Stop => {
                commands::db_stop().await?;
            }
            DbCommands::Restart => {
                commands::db_restart().await?;
            }
            DbCommands::Status => {
                commands::db_status().await?;
            }
            DbCommands::Install => {
                commands::db_install().await?;
            }
        },

        Commands::Doctor(doctor_cmd) => match doctor_cmd {
            DoctorCommands::Check { fix } => {
                use cortex_cli::doctor;
                let results = doctor::run_diagnostics(fix).await?;

                // Exit with error code if there are failures
                let has_failures = results.iter().any(|r| r.status == cortex_cli::doctor::DiagnosticStatus::Fail);
                if has_failures {
                    std::process::exit(1);
                }
            }
            DoctorCommands::Health => {
                use cortex_cli::doctor;
                let healthy = doctor::quick_health_check().await?;
                if !healthy {
                    std::process::exit(1);
                }
            }
        },

        Commands::Test(test_cmd) => match test_cmd {
            TestCommands::All => {
                use cortex_cli::testing;
                let results = testing::run_all_tests().await?;
                testing::print_test_results(&results, format)?;

                if results.failed > 0 {
                    std::process::exit(1);
                }
            }
            TestCommands::Benchmark => {
                use cortex_cli::testing;
                let results = testing::run_benchmarks().await?;
                testing::print_benchmark_results(&results, format)?;
            }
            TestCommands::Component { component } => {
                output::info(format!("Testing {} component...", component));
                output::warning("Component-specific tests not yet implemented");
            }
        },

        Commands::Export(export_cmd) => match export_cmd {
            ExportCommands::Workspace { workspace, output, format: fmt } => {
                use cortex_cli::export;
                let export_format = export::ExportFormat::from_extension(&fmt)
                    .unwrap_or(export::ExportFormat::Json);
                export::export_workspace(&workspace, &output, export_format).await?;
            }
            ExportCommands::Episodes { workspace, output, format: fmt, limit } => {
                use cortex_cli::export;
                let export_format = export::ExportFormat::from_extension(&fmt)
                    .unwrap_or(export::ExportFormat::Json);
                export::export_episodes(workspace, &output, export_format, limit).await?;
            }
            ExportCommands::Stats { output, format: fmt } => {
                use cortex_cli::export;
                let export_format = export::ExportFormat::from_extension(&fmt)
                    .unwrap_or(export::ExportFormat::Json);
                export::export_stats(&output, export_format).await?;
            }
        },

        Commands::Mcp(mcp_cmd) => match mcp_cmd {
            McpCommands::Stdio => {
                commands::mcp_stdio().await?;
            }
            McpCommands::Http { address, port } => {
                commands::mcp_http(address, port).await?;
            }
            McpCommands::Info { detailed, category } => {
                commands::mcp_info(detailed, category).await?;
            }
        },

        Commands::Qdrant(qdrant_cmd) => match qdrant_cmd {
            QdrantCommands::Init { force, skip_verify } => {
                qdrant_commands::qdrant_init(force, skip_verify).await?;
            }
            QdrantCommands::Status { detailed, collection } => {
                qdrant_commands::qdrant_status(detailed, collection, format).await?;
            }
            QdrantCommands::Migrate { source, target, batch_size, dry_run } => {
                qdrant_commands::qdrant_migrate(source, target, batch_size, dry_run).await?;
            }
            QdrantCommands::Verify { collection, fix } => {
                qdrant_commands::qdrant_verify(collection, fix).await?;
            }
            QdrantCommands::Benchmark { collection, num_queries, dimensions } => {
                qdrant_commands::qdrant_benchmark(collection, num_queries, dimensions, format).await?;
            }
            QdrantCommands::Snapshot { collection, output } => {
                qdrant_commands::qdrant_snapshot(collection, output).await?;
            }
            QdrantCommands::Restore { snapshot, collection } => {
                qdrant_commands::qdrant_restore(snapshot, collection).await?;
            }
            QdrantCommands::Optimize { collection, wait } => {
                qdrant_commands::qdrant_optimize(collection, wait).await?;
            }
            QdrantCommands::List { detailed } => {
                qdrant_commands::qdrant_list(detailed, format).await?;
            }
        },

        Commands::Interactive { mode } => {
            match mode.as_str() {
                "wizard" => {
                    use cortex_cli::interactive;
                    let config = interactive::workspace_setup_wizard().await?;
                    commands::workspace_create(config.name, config.workspace_type).await?;
                }
                "search" => {
                    use cortex_cli::interactive;
                    interactive::interactive_search().await?;
                }
                "health" => {
                    use cortex_cli::interactive;
                    interactive::interactive_health_check().await?;
                }
                "menu" | _ => {
                    use cortex_cli::interactive;
                    let menu = interactive::Menu::new("Cortex Main Menu")
                        .add_item("Create Workspace", Some("Set up a new workspace".to_string()))
                        .add_item("Run Diagnostics", Some("Check system health".to_string()))
                        .add_item("Interactive Search", Some("Search interactively".to_string()))
                        .add_item("View Statistics", Some("Show system stats".to_string()))
                        .add_item("Exit", None);

                    let choice = menu.show()?;
                    match choice {
                        0 => {
                            let config = interactive::workspace_setup_wizard().await?;
                            commands::workspace_create(config.name, config.workspace_type).await?;
                        }
                        1 => {
                            interactive::interactive_health_check().await?;
                        }
                        2 => {
                            interactive::interactive_search().await?;
                        }
                        3 => {
                            commands::show_stats(format).await?;
                        }
                        _ => {}
                    }
                }
            }
        },

        Commands::Server(server_cmd) => match server_cmd {
            ServerCommands::Start { host, port, workers } => {
                commands::server_start(host, port, workers).await?;
            }
            ServerCommands::Stop => {
                commands::server_stop().await?;
            }
            ServerCommands::Status => {
                commands::server_status().await?;
            }
        },

        Commands::InternalServerRun { host, port, workers } => {
            // This is the internal blocking server run command
            commands::server_run_blocking(host, port, workers).await?;
        }
    }

    Ok(())
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: bool) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = if verbose {
        EnvFilter::new("cortex=debug,cortex_cli=debug,info")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("cortex=info,cortex_cli=info,warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
