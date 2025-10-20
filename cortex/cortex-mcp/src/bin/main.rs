//! Cortex MCP Server CLI
//!
//! This binary provides a command-line interface for running the Cortex MCP server.

use anyhow::Result;
use clap::{Parser, Subcommand};
use cortex_mcp::CortexMcpServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "cortex-mcp")]
#[command(about = "Cortex Model Context Protocol Server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server with stdio transport
    Stdio,

    /// Start the MCP server with HTTP transport
    Http {
        /// Bind address for HTTP server
        #[arg(long, default_value = "127.0.0.1:3000")]
        bind: String,
    },

    /// Show server information
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Stdio => {
            tracing::info!("Starting Cortex MCP Server in stdio mode");
            let server = CortexMcpServer::new().await?;
            server.serve_stdio().await?;
        }
        Commands::Http { bind } => {
            tracing::info!("Starting Cortex MCP Server in HTTP mode on {}", bind);
            let server = CortexMcpServer::new().await?;
            server.serve_http(&bind).await?;
        }
        Commands::Info => {
            println!("Cortex MCP Server");
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            println!("\nRegistered Tools:");
            println!("  Workspace Management: 8 tools");
            println!("    - cortex.workspace.create");
            println!("    - cortex.workspace.get");
            println!("    - cortex.workspace.list");
            println!("    - cortex.workspace.activate");
            println!("    - cortex.workspace.sync_from_disk");
            println!("    - cortex.workspace.export");
            println!("    - cortex.workspace.archive");
            println!("    - cortex.workspace.delete");
            println!("\n  Virtual Filesystem: 12 tools");
            println!("    - cortex.vfs.get_node");
            println!("    - cortex.vfs.list_directory");
            println!("    - cortex.vfs.create_file");
            println!("    - cortex.vfs.update_file");
            println!("    - cortex.vfs.delete_node");
            println!("    - cortex.vfs.move_node");
            println!("    - cortex.vfs.copy_node");
            println!("    - cortex.vfs.create_directory");
            println!("    - cortex.vfs.get_tree");
            println!("    - cortex.vfs.search_files");
            println!("    - cortex.vfs.get_file_history");
            println!("    - cortex.vfs.restore_file_version");
            println!("\n  Code Navigation: 10 tools");
            println!("    - cortex.code.get_unit");
            println!("    - cortex.code.list_units");
            println!("    - cortex.code.get_symbols");
            println!("    - cortex.code.find_definition");
            println!("    - cortex.code.find_references");
            println!("    - cortex.code.get_signature");
            println!("    - cortex.code.get_call_hierarchy");
            println!("    - cortex.code.get_type_hierarchy");
            println!("    - cortex.code.get_imports");
            println!("    - cortex.code.get_exports");
            println!("\nTotal: 30 tools");
        }
    }

    Ok(())
}
