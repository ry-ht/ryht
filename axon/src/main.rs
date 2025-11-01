//! Axon CLI - Command-line interface for the Axon multi-agent system.
//!
//! # Usage
//!
//! ```bash
//! # Initialize multi-agent workspace
//! axon init my-agents
//!
//! # Start agents
//! axon agent start orchestrator --name main-orchestrator
//! axon agent start worker --name worker-1 --capabilities coding,testing
//!
//! # List running agents
//! axon agent list
//!
//! # Start MCP server
//! axon mcp stdio
//! axon mcp http
//!
//! # Start REST API server for dashboard
//! axon server start
//!
//! # Execute a workflow
//! axon workflow run my-workflow.yaml
//!
//! # Check system status
//! axon status
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "axon")]
#[command(about = "Axon - Multi-Agent System Framework", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AgentTypeArg {
    Orchestrator,
    Developer,
    Reviewer,
    Tester,
    Documenter,
    Architect,
    Researcher,
    Optimizer,
}

impl From<AgentTypeArg> for axon::agents::AgentType {
    fn from(arg: AgentTypeArg) -> Self {
        match arg {
            AgentTypeArg::Orchestrator => axon::agents::AgentType::Orchestrator,
            AgentTypeArg::Developer => axon::agents::AgentType::Developer,
            AgentTypeArg::Reviewer => axon::agents::AgentType::Reviewer,
            AgentTypeArg::Tester => axon::agents::AgentType::Tester,
            AgentTypeArg::Documenter => axon::agents::AgentType::Documenter,
            AgentTypeArg::Architect => axon::agents::AgentType::Architect,
            AgentTypeArg::Researcher => axon::agents::AgentType::Researcher,
            AgentTypeArg::Optimizer => axon::agents::AgentType::Optimizer,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Axon workspace
    Init {
        /// Workspace name
        name: String,

        /// Workspace path (default: current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Agent management
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Workflow orchestration
    #[command(subcommand)]
    Workflow(WorkflowCommands),

    /// REST API server management
    #[command(subcommand)]
    Server(ServerCommands),

    /// System status and health
    Status {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Monitor agents and workflows
    #[command(subcommand)]
    Monitor(MonitorCommands),

    /// System diagnostics
    #[command(subcommand)]
    Doctor(DoctorCommands),

    /// Export metrics and reports
    #[command(subcommand)]
    Export(ExportCommands),

    /// Model Context Protocol operations
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Interactive mode
    Interactive {
        /// Interactive mode to enter (wizard, dashboard)
        #[arg(short, long, default_value = "dashboard")]
        mode: String,
    },

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
enum AgentCommands {
    /// Start an agent
    Start {
        /// Agent type
        #[arg(value_enum)]
        agent_type: AgentTypeArg,

        /// Agent name
        #[arg(short, long)]
        name: String,

        /// Capabilities (comma-separated)
        #[arg(short, long)]
        capabilities: Option<String>,

        /// Model to use
        #[arg(short, long)]
        model: Option<String>,

        /// Maximum concurrent tasks
        #[arg(long, default_value = "1")]
        max_tasks: usize,
    },

    /// Stop an agent
    Stop {
        /// Agent ID or name
        agent_id: String,

        /// Force stop without graceful shutdown
        #[arg(short, long)]
        force: bool,
    },

    /// List running agents
    List {
        /// Filter by agent type
        #[arg(short, long)]
        agent_type: Option<AgentTypeArg>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Get agent information
    Info {
        /// Agent ID or name
        agent_id: String,
    },

    /// Pause an agent
    Pause {
        /// Agent ID or name
        agent_id: String,
    },

    /// Resume a paused agent
    Resume {
        /// Agent ID or name
        agent_id: String,
    },

    /// View agent logs
    Logs {
        /// Agent ID or name
        agent_id: String,

        /// Follow logs
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: usize,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Run a workflow
    Run {
        /// Workflow file path
        workflow: PathBuf,

        /// Input parameters (JSON)
        #[arg(short, long)]
        input: Option<String>,

        /// Dry run (validate only)
        #[arg(long)]
        dry_run: bool,
    },

    /// List workflows
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Get workflow status
    Status {
        /// Workflow ID
        workflow_id: String,
    },

    /// Cancel a running workflow
    Cancel {
        /// Workflow ID
        workflow_id: String,
    },

    /// Validate a workflow definition
    Validate {
        /// Workflow file path
        workflow: PathBuf,
    },
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the REST API server
    Start {
        /// Server host address
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port
        #[arg(long, default_value = "3000")]
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
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,

        /// Set globally
        #[arg(short, long)]
        global: bool,
    },

    /// List all configuration values
    List,
}

#[derive(Subcommand)]
enum MonitorCommands {
    /// Show dashboard
    Dashboard {
        /// Refresh interval in seconds
        #[arg(short, long, default_value = "5")]
        refresh: u64,
    },

    /// Show metrics
    Metrics {
        /// Agent ID (show all if not specified)
        #[arg(short, long)]
        agent_id: Option<String>,
    },

    /// Show telemetry data
    Telemetry {
        /// Time range in minutes
        #[arg(short, long, default_value = "60")]
        range: u64,
    },
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
enum ExportCommands {
    /// Export agent metrics
    Metrics {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, csv, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Export workflow results
    Workflows {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, csv, yaml)
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

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {:#}", e);
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

    // Use command functions from the commands module
    use axon::commands::*;

    // Convert OutputFormatArg from this file to the one in commands module
    let output_format = match cli.format {
        OutputFormatArg::Human => axon::commands::output::OutputFormatArg::Human,
        OutputFormatArg::Json => axon::commands::output::OutputFormatArg::Json,
        OutputFormatArg::Plain => axon::commands::output::OutputFormatArg::Plain,
    };

    match cli.command {
        Commands::Init { name, path } => {
            init_workspace(name, path).await?;
        }

        Commands::Agent(agent_cmd) => match agent_cmd {
            AgentCommands::Start {
                agent_type,
                name,
                capabilities,
                model,
                max_tasks,
            } => {
                agent_start(
                    agent_type.into(),
                    name,
                    capabilities,
                    model,
                    max_tasks,
                ).await?;
            }
            AgentCommands::Stop { agent_id, force } => {
                agent_stop(agent_id, force).await?;
            }
            AgentCommands::List { agent_type, detailed } => {
                agent_list(agent_type.map(Into::into), detailed, output_format).await?;
            }
            AgentCommands::Info { agent_id } => {
                agent_info(agent_id, output_format).await?;
            }
            AgentCommands::Pause { agent_id } => {
                agent_pause(agent_id).await?;
            }
            AgentCommands::Resume { agent_id } => {
                agent_resume(agent_id).await?;
            }
            AgentCommands::Logs { agent_id, follow, lines } => {
                agent_logs(agent_id, follow, lines).await?;
            }
        },

        Commands::Workflow(workflow_cmd) => match workflow_cmd {
            WorkflowCommands::Run { workflow, input, dry_run } => {
                workflow_run(workflow, input, dry_run).await?;
            }
            WorkflowCommands::List { status } => {
                workflow_list(status, output_format).await?;
            }
            WorkflowCommands::Status { workflow_id } => {
                workflow_status(workflow_id, output_format).await?;
            }
            WorkflowCommands::Cancel { workflow_id } => {
                workflow_cancel(workflow_id).await?;
            }
            WorkflowCommands::Validate { workflow } => {
                workflow_validate(workflow).await?;
            }
        },

        Commands::Server(server_cmd) => match server_cmd {
            ServerCommands::Start { host, port, workers } => {
                server_start(host, port, workers).await?;
            }
            ServerCommands::Stop => {
                server_stop().await?;
            }
            ServerCommands::Status => {
                server_status().await?;
            }
        },

        Commands::Status { detailed } => {
            show_status(detailed, output_format).await?;
        }

        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Get { key } => {
                config_get(key).await?;
            }
            ConfigCommands::Set { key, value, global } => {
                config_set(key, value, global).await?;
            }
            ConfigCommands::List => {
                config_list(output_format).await?;
            }
        },

        Commands::Monitor(monitor_cmd) => match monitor_cmd {
            MonitorCommands::Dashboard { refresh } => {
                monitor_dashboard(refresh).await?;
            }
            MonitorCommands::Metrics { agent_id } => {
                monitor_metrics(agent_id, output_format).await?;
            }
            MonitorCommands::Telemetry { range } => {
                monitor_telemetry(range, output_format).await?;
            }
        },

        Commands::Doctor(doctor_cmd) => match doctor_cmd {
            DoctorCommands::Check { fix } => {
                doctor_check(fix).await?;
            }
            DoctorCommands::Health => {
                doctor_health().await?;
            }
        },

        Commands::Export(export_cmd) => match export_cmd {
            ExportCommands::Metrics { output, format } => {
                export_metrics(output, format).await?;
            }
            ExportCommands::Workflows { output, format } => {
                export_workflows(output, format).await?;
            }
        },

        Commands::Mcp(mcp_cmd) => match mcp_cmd {
            McpCommands::Stdio => {
                mcp_stdio().await?;
            }
            McpCommands::Http { address, port } => {
                mcp_http(address, port).await?;
            }
            McpCommands::Info { detailed, category } => {
                mcp_info(detailed, category).await?;
            }
        },

        Commands::Interactive { mode } => {
            interactive_mode(mode).await?;
        }

        Commands::InternalServerRun { host, port, workers } => {
            server_run_blocking(host, port, workers).await?;
        }
    }

    Ok(())
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: bool) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = if verbose {
        EnvFilter::new("axon=debug,info")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("axon=info,warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}