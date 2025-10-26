# Cortex CLI

Production-ready command-line interface for the Cortex cognitive memory system.

## Features

- **Workspace Management**: Create, list, delete, and switch between workspaces
- **File Ingestion**: Import codebases and documents into Cortex memory
- **Semantic Search**: Query across all ingested content
- **MCP Server**: Start the Model Context Protocol server for LLM integration
- **VFS Operations**: Materialize virtual filesystem to disk
- **Memory Management**: Consolidate and manage cognitive memory
- **Database Control**: Start, stop, and manage SurrealDB instances
- **Configuration**: Flexible multi-level configuration system
- **Beautiful Output**: Colored terminal output, progress bars, and tables
- **JSON Support**: Machine-readable output for scripting

## Installation

```bash
# Build from source
cd cortex/cortex
cargo build --release

# The binary will be at: target/release/cortex
# Optionally, install to cargo bin:
cargo install --path .
```

## Quick Start

```bash
# 1. Install SurrealDB (if not already installed)
cortex db install

# 2. Start the database
cortex db start

# 3. Initialize a workspace
cortex init my-project

# 4. Ingest some code
cortex ingest ./src

# 5. Search the memory
cortex search "authentication logic"

# 6. Start the MCP server
cortex serve
```

## Configuration

Cortex uses a multi-level configuration system with the following priority order:

1. Command-line flags (highest priority)
2. Environment variables (`CORTEX_*`)
3. Project-specific config (`.cortex/config.toml`)
4. System-wide config (`~/.config/cortex/config.toml`)
5. Default values (lowest priority)

### Environment Variables

```bash
# Database
export CORTEX_DB_URL="file:///path/to/db"
export CORTEX_DB_NAMESPACE="cortex"
export CORTEX_DB_NAME="main"
export CORTEX_DB_POOL_SIZE=10
export CORTEX_DB_USER="root"
export CORTEX_DB_PASSWORD="root"

# Storage
export CORTEX_DATA_DIR="~/.local/share/cortex"
export CORTEX_CACHE_SIZE_MB=1024
export CORTEX_COMPRESSION=true

# MCP Server
export CORTEX_MCP_ENABLED=true
export CORTEX_MCP_ADDRESS="127.0.0.1"
export CORTEX_MCP_PORT=3000

# Active workspace
export CORTEX_WORKSPACE="my-workspace"
```

### Configuration Files

Create a global configuration at `~/.config/cortex/config.toml`:

```toml
[database]
connection_string = "file:///home/user/.local/share/cortex/db"
namespace = "cortex"
database = "main"
pool_size = 10

[storage]
data_dir = "/home/user/.local/share/cortex"
cache_size_mb = 1024
compression_enabled = true

[mcp]
enabled = true
address = "127.0.0.1"
port = 3000
```

Or create a project-specific configuration at `.cortex/config.toml`:

```toml
active_workspace = "my-project"

[database]
namespace = "my-project"
```

### Configuration Commands

```bash
# List all configuration values
cortex config list

# Get a specific value
cortex config get database.namespace

# Set a value (project-level)
cortex config set database.namespace my-namespace

# Set a value (global/system-level)
cortex config set database.namespace my-namespace --global
```

## Commands

### Initialization

```bash
# Initialize a new workspace in current directory
cortex init my-project

# Initialize with custom path
cortex init my-project --path /path/to/workspace

# Initialize with specific type
cortex init my-project --workspace-type agent
cortex init my-project --workspace-type project
cortex init my-project --workspace-type shared
```

### Workspace Management

```bash
# Create a new workspace
cortex workspace create my-workspace

# Create with specific type
cortex workspace create my-agent --type agent

# List all workspaces
cortex workspace list
cortex workspace list --format json

# Switch active workspace
cortex workspace switch my-workspace

# Delete a workspace
cortex workspace delete my-workspace
cortex workspace delete my-workspace --force  # Skip confirmation
```

### Ingestion

```bash
# Ingest current directory
cortex ingest .

# Ingest specific path
cortex ingest /path/to/project

# Ingest into specific workspace
cortex ingest ./src --workspace my-workspace

# Non-recursive ingestion
cortex ingest ./src --recursive false
```

### Search

```bash
# Search across all memory
cortex search "authentication logic"

# Search with limit
cortex search "error handling" --limit 20

# Search in specific workspace
cortex search "database query" --workspace my-project

# JSON output for scripting
cortex search "api endpoint" --format json
```

### Listing

```bash
# List projects
cortex list projects
cortex list projects --workspace my-workspace
cortex list projects --format json

# List documents
cortex list documents
cortex list documents --workspace my-workspace

# List memory episodes
cortex list episodes
cortex list episodes --limit 50
cortex list episodes --workspace my-workspace
```

### MCP Server

```bash
# Start MCP server with defaults
cortex serve

# Start on specific address/port
cortex serve --address 0.0.0.0 --port 3001

# The server runs in the foreground. Press Ctrl+C to stop.
```

### VFS Flush

```bash
# Flush workspace to disk
cortex flush my-workspace /path/to/output

# Flush with specific scope
cortex flush my-workspace ./output --scope workspace
cortex flush my-workspace ./output --scope project
cortex flush my-workspace ./output --scope all
```

### Statistics

```bash
# Show system statistics
cortex stats

# JSON output
cortex stats --format json
```

### Agent Sessions

```bash
# Create an agent session
cortex agent create my-agent

# Create with specific type
cortex agent create my-agent --type coding

# List agent sessions
cortex agent list
cortex agent list --format json

# Delete an agent session
cortex agent delete <session-id>
```

### Memory Operations

```bash
# Consolidate memory
cortex memory consolidate
cortex memory consolidate --workspace my-workspace

# Forget old memory
cortex memory forget 2024-01-01
cortex memory forget 2024-01-01 --workspace my-workspace
```

### Database Management

```bash
# Install SurrealDB
cortex db install

# Start the database
cortex db start

# Start with custom settings
cortex db start --bind 127.0.0.1:9000 --data-dir /custom/path

# Stop the database
cortex db stop

# Restart the database
cortex db restart

# Check database status
cortex db status
```

## Output Formats

Cortex CLI supports three output formats:

### Human (Default)

Colored, formatted output designed for terminal use:

```bash
cortex workspace list
```

### JSON

Machine-readable output for scripting:

```bash
cortex workspace list --format json | jq '.[] | .name'
```

### Plain

Plain text without colors (useful for piping):

```bash
cortex workspace list --format plain
```

## Global Flags

All commands support these global flags:

```bash
# Verbose logging
cortex search "query" --verbose

# Custom config file
cortex search "query" --config /path/to/config.toml

# Output format
cortex search "query" --format json
```

## Exit Codes

- `0`: Success
- `1`: Error occurred

## Examples

### Complete Workflow

```bash
# Setup
cortex db install
cortex db start
cortex init my-project

# Ingest multiple sources
cortex ingest ./backend
cortex ingest ./frontend
cortex ingest ./docs

# Search and explore
cortex search "user authentication"
cortex list projects
cortex list documents
cortex stats

# Start MCP server for LLM integration
cortex serve
```

### Scripting with JSON Output

```bash
#!/bin/bash

# Get all workspaces as JSON
workspaces=$(cortex workspace list --format json)

# Parse with jq
echo "$workspaces" | jq -r '.[] | .name'

# Search and process results
results=$(cortex search "error handling" --format json)
echo "$results" | jq -r '.[] | .file_path'
```

### Multi-Workspace Management

```bash
# Create multiple workspaces
cortex workspace create backend --type project
cortex workspace create frontend --type project
cortex workspace create shared --type shared

# Ingest into specific workspaces
cortex ingest ./backend --workspace backend
cortex ingest ./frontend --workspace frontend

# Switch between workspaces
cortex workspace switch backend
cortex search "api endpoint"  # Searches only backend

cortex workspace switch frontend
cortex search "component"  # Searches only frontend
```

### Configuration Management

```bash
# Set up project-specific configuration
cd my-project
cortex init my-project
cortex config set database.namespace my-project-db
cortex config set active_workspace my-project

# View all settings
cortex config list

# Export configuration
cortex config list --format json > config-backup.json
```

## Troubleshooting

### Database Connection Issues

```bash
# Check database status
cortex db status

# View database logs
tail -f ~/.local/share/cortex/surrealdb.log

# Restart database
cortex db restart
```

### Configuration Issues

```bash
# Verify configuration
cortex config list

# Check environment variables
env | grep CORTEX_

# Test with explicit config
cortex stats --config /path/to/config.toml --verbose
```

### Performance Issues

```bash
# Increase cache size
cortex config set storage.cache_size_mb 2048 --global

# Increase connection pool
cortex config set database.pool_size 20 --global

# View statistics
cortex stats
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run integration tests (requires database)
cargo test --test integration_tests

# Run with verbose output
cargo test -- --nocapture --test-threads=1
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# With specific features
cargo build --release --features <feature>
```

### Code Style

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check documentation
cargo doc --no-deps --open
```

## Architecture

The CLI is structured as follows:

- **main.rs**: Command-line parsing and routing
- **commands.rs**: Command implementations
- **config.rs**: Configuration management
- **output.rs**: Output formatting utilities
- **lib.rs**: Library interface

## Contributing

Contributions are welcome! Please:

1. Follow the existing code style
2. Add tests for new features
3. Update documentation
4. Ensure all tests pass

## License

See the root LICENSE file for details.

## Support

For issues, questions, or contributions, please visit the project repository.
