# Cortex CLI User Guide

Complete guide to using the Cortex command-line interface.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Commands](#commands)
5. [Interactive Mode](#interactive-mode)
6. [Examples](#examples)
7. [Troubleshooting](#troubleshooting)

## Installation

### From Source

```bash
cd cortex/cortex-cli
cargo build --release
./target/release/cortex --version
```

### Adding to PATH

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$PATH:/path/to/cortex/target/release"
```

## Quick Start

### 1. Initialize a Workspace

```bash
# Create a new workspace
cortex init my-project --path ./my-project

# Or use interactive wizard
cortex interactive --mode wizard
```

### 2. Start the Database

```bash
# Install SurrealDB (if needed)
cortex db install

# Start the database
cortex db start

# Check status
cortex db status
```

### 3. Ingest Some Code

```bash
# Ingest a directory
cortex ingest ./src

# Ingest specific files
cortex ingest ./main.rs

# Ingest recursively with filters
cortex ingest ./project --recursive
```

### 4. Search and Query

```bash
# Semantic search
cortex search "authentication logic"

# Search in specific workspace
cortex search "database connection" --workspace my-project

# Limit results
cortex search "error handling" --limit 5
```

## Configuration

### Configuration Files

Cortex uses a hierarchical configuration system:

1. **System-wide**: `~/.config/cortex/config.toml`
2. **Project-specific**: `./.cortex/config.toml`
3. **Environment variables**: `CORTEX_*`
4. **Command-line flags**: Highest priority

### Example Configuration

```toml
# ~/.config/cortex/config.toml

[database]
connection_string = "file://~/.local/share/cortex/db"
namespace = "cortex"
database = "main"
pool_size = 10

[storage]
data_dir = "~/.local/share/cortex"
cache_size_mb = 1024
compression_enabled = true

[mcp]
enabled = true
address = "127.0.0.1"
port = 3000
```

### Configuration Commands

```bash
# View all configuration
cortex config list

# Get specific value
cortex config get database.namespace

# Set value (project-local)
cortex config set storage.cache_size_mb 2048

# Set value (global)
cortex config set database.pool_size 20 --global
```

### Environment Variables

```bash
# Database configuration
export CORTEX_DB_URL="ws://localhost:8000"
export CORTEX_DB_NAMESPACE="myapp"
export CORTEX_DB_NAME="production"
export CORTEX_DB_POOL_SIZE=20

# Storage configuration
export CORTEX_DATA_DIR="/var/lib/cortex"
export CORTEX_CACHE_SIZE_MB=2048

# MCP server
export CORTEX_MCP_PORT=3001
```

## Commands

### Workspace Management

```bash
# Create workspace
cortex workspace create my-workspace --type project

# List all workspaces
cortex workspace list
cortex workspace list --format json

# Switch active workspace
cortex workspace switch my-workspace

# Delete workspace
cortex workspace delete old-workspace
cortex workspace delete old-workspace --force
```

### Database Management

```bash
# Install SurrealDB
cortex db install

# Start database
cortex db start
cortex db start --bind 127.0.0.1:8000
cortex db start --data-dir /custom/path

# Stop database
cortex db stop

# Restart database
cortex db restart

# Check status
cortex db status
```

### Ingestion

```bash
# Ingest directory
cortex ingest ./src

# Ingest to specific workspace
cortex ingest ./docs --workspace my-docs

# Ingest recursively
cortex ingest ./project --recursive

# Ingest without recursion
cortex ingest ./config --recursive false
```

### Search and Query

```bash
# Basic search
cortex search "authentication"

# Search with limit
cortex search "database" --limit 20

# Search in workspace
cortex search "api endpoint" --workspace backend

# JSON output
cortex search "logging" --format json
```

### List Commands

```bash
# List projects
cortex list projects
cortex list projects --workspace my-workspace

# List documents
cortex list documents

# List memory episodes
cortex list episodes --limit 50
cortex list episodes --workspace my-workspace
```

### Memory Operations

```bash
# Consolidate memory
cortex memory consolidate
cortex memory consolidate --workspace my-workspace

# Forget old memory
cortex memory forget 2024-01-01
cortex memory forget 2024-01-01 --workspace old-project
```

### Agent Session Management

```bash
# Create agent session
cortex agent create my-agent --type general

# List agent sessions
cortex agent list
cortex agent list --format json

# Delete agent session
cortex agent delete session-id-here
```

### System Diagnostics

```bash
# Run all diagnostics
cortex doctor check

# Run diagnostics and auto-fix issues
cortex doctor check --fix

# Quick health check
cortex doctor health
```

### Testing

```bash
# Run all system tests
cortex test all

# Run performance benchmarks
cortex test benchmark

# Test specific component
cortex test component database
cortex test component storage
cortex test component vfs
```

### Export

```bash
# Export workspace (JSON)
cortex export workspace my-workspace --output workspace.json

# Export workspace (CSV)
cortex export workspace my-workspace --output workspace.csv --format csv

# Export workspace (YAML)
cortex export workspace my-workspace --output workspace.yaml --format yaml

# Export workspace (Markdown)
cortex export workspace my-workspace --output workspace.md --format markdown

# Export memory episodes
cortex export episodes --output episodes.json
cortex export episodes --workspace my-workspace --output episodes.csv --format csv --limit 100

# Export statistics
cortex export stats --output stats.json
cortex export stats --output stats.yaml --format yaml
```

### VFS Operations

```bash
# Flush VFS to disk
cortex flush my-workspace ./output --scope workspace

# Flush with different scopes
cortex flush my-workspace ./output --scope all
cortex flush my-workspace ./output --scope project
```

### System Information

```bash
# Show statistics
cortex stats
cortex stats --format json

# MCP server
cortex serve
cortex serve --address 0.0.0.0 --port 8080
```

## Interactive Mode

### Main Menu

```bash
# Launch interactive menu
cortex interactive

# Or explicitly
cortex interactive --mode menu
```

### Workspace Wizard

```bash
# Launch workspace setup wizard
cortex interactive --mode wizard
```

### Interactive Search

```bash
# Launch interactive search interface
cortex interactive --mode search
```

### Health Check

```bash
# Interactive health check
cortex interactive --mode health
```

## Examples

### Example 1: Complete Setup Workflow

```bash
# Install and setup
cortex db install
cortex db start
cortex init my-project
cortex workspace switch my-project

# Ingest code
cortex ingest ./src --recursive

# Search
cortex search "error handling"

# Export results
cortex export stats --output stats.json
```

### Example 2: Multi-Project Setup

```bash
# Create multiple workspaces
cortex workspace create frontend --type project
cortex workspace create backend --type project
cortex workspace create shared --type shared

# Ingest different projects
cortex ingest ./frontend --workspace frontend
cortex ingest ./backend --workspace backend
cortex ingest ./shared --workspace shared

# Search across specific workspace
cortex search "authentication" --workspace backend
```

### Example 3: Automation with JSON Output

```bash
# Get workspace list as JSON
workspaces=$(cortex workspace list --format json)

# Get stats as JSON for monitoring
stats=$(cortex stats --format json)

# Run health check in CI/CD
cortex doctor health || exit 1
```

### Example 4: Memory Management

```bash
# Consolidate working memory to long-term
cortex memory consolidate --workspace my-workspace

# Clean up old episodes
cortex memory forget 2023-12-31 --workspace my-workspace

# Export episodes before cleanup
cortex export episodes --workspace my-workspace --output backup.json
```

### Example 5: Development Workflow

```bash
# Start database
cortex db start

# Run diagnostics
cortex doctor check --fix

# Run tests
cortex test all

# If tests pass, ingest new code
cortex ingest ./new-feature --workspace dev

# Search for similar patterns
cortex search "similar pattern" --workspace dev
```

## Global Options

All commands support these global options:

```bash
# Verbose logging
cortex --verbose <command>

# Custom config file
cortex --config /path/to/config.toml <command>

# JSON output
cortex --format json <command>

# Plain output (no colors)
cortex --format plain <command>
```

## Output Formats

### Human Format (Default)

Colorized, formatted output with tables and progress bars.

```bash
cortex workspace list
```

### JSON Format

Machine-readable JSON output for scripting.

```bash
cortex workspace list --format json
```

### Plain Format

Plain text without colors, suitable for piping.

```bash
cortex workspace list --format plain
```

## Troubleshooting

### Database Connection Issues

```bash
# Check if database is running
cortex db status

# Try restarting
cortex db restart

# Run diagnostics
cortex doctor check

# Check logs
tail -f ~/.local/share/cortex/db/surreal.log
```

### Configuration Issues

```bash
# Validate configuration
cortex config list

# Check specific value
cortex config get database.connection_string

# Reset to defaults
rm ~/.config/cortex/config.toml
cortex config list  # Will show defaults
```

### Ingestion Issues

```bash
# Run with verbose logging
cortex --verbose ingest ./problematic-dir

# Check file permissions
ls -la ./problematic-dir

# Try with smaller batch
cortex ingest ./specific-file.rs
```

### Performance Issues

```bash
# Run benchmarks
cortex test benchmark

# Check system resources
cortex stats

# Increase cache size
cortex config set storage.cache_size_mb 2048
```

### Getting Help

```bash
# General help
cortex --help

# Command-specific help
cortex workspace --help
cortex search --help
cortex doctor --help

# Subcommand help
cortex workspace create --help
cortex export episodes --help
```

## Exit Codes

- `0`: Success
- `1`: General error
- `1`: Failed health check (doctor health)
- `1`: Failed tests (test all)
- `1`: Failed diagnostics with failures (doctor check)

## Tips and Best Practices

1. **Use workspaces** to organize different projects or contexts
2. **Run diagnostics** regularly with `cortex doctor check`
3. **Export important data** periodically with `cortex export`
4. **Use JSON format** for scripting and automation
5. **Enable verbose logging** when debugging with `--verbose`
6. **Set up environment variables** for different environments
7. **Use interactive mode** for complex workflows
8. **Regular consolidation** of memory with `cortex memory consolidate`
9. **Monitor stats** with `cortex stats --format json`
10. **Run tests** before deploying with `cortex test all`

## Advanced Usage

### Scripting with Cortex

```bash
#!/bin/bash
# Example: Automated backup script

# Ensure database is running
if ! cortex doctor health > /dev/null 2>&1; then
    echo "Database not healthy, starting..."
    cortex db start
fi

# Export all data
timestamp=$(date +%Y%m%d_%H%M%S)
backup_dir="./backups/$timestamp"
mkdir -p "$backup_dir"

# Export each workspace
for workspace in $(cortex workspace list --format json | jq -r '.[].name'); do
    echo "Backing up $workspace..."
    cortex export workspace "$workspace" --output "$backup_dir/${workspace}.json"
    cortex export episodes --workspace "$workspace" --output "$backup_dir/${workspace}_episodes.json"
done

# Export stats
cortex export stats --output "$backup_dir/stats.json"

echo "Backup completed: $backup_dir"
```

### CI/CD Integration

```yaml
# .github/workflows/cortex-check.yml
name: Cortex Health Check

on: [push, pull_request]

jobs:
  health-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install Cortex
        run: cargo install --path cortex/cortex-cli

      - name: Start Database
        run: |
          cortex db install
          cortex db start

      - name: Run Health Check
        run: cortex doctor health

      - name: Run Tests
        run: cortex test all

      - name: Generate Report
        if: always()
        run: cortex stats --format json > stats.json

      - name: Upload Report
        if: always()
        uses: actions/upload-artifact@v2
        with:
          name: cortex-report
          path: stats.json
```

## See Also

- [Configuration System Documentation](../cortex-core/CONFIG.md)
- [MCP Server Documentation](../cortex-mcp/README.md)
- [VFS Documentation](../cortex-vfs/IMPLEMENTATION.md)
- [Memory System Documentation](../cortex-memory/README.md)
