# Cortex CLI Usage Guide

Comprehensive guide to using the Cortex CLI for cognitive memory management.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Core Concepts](#core-concepts)
3. [Command Reference](#command-reference)
4. [Common Workflows](#common-workflows)
5. [Advanced Usage](#advanced-usage)
6. [Troubleshooting](#troubleshooting)

## Getting Started

### Installation

```bash
# Install SurrealDB
cortex db install

# Verify installation
cortex db status
```

### First-Time Setup

```bash
# Start the database
cortex db start

# Initialize your first workspace
cortex init my-first-project

# Check the configuration
cortex config list
```

### Your First Ingestion

```bash
# Ingest your current project
cortex ingest .

# Search for something
cortex search "main function"

# View statistics
cortex stats
```

## Core Concepts

### Workspaces

Workspaces are isolated containers for organizing your projects and data. They come in three types:

- **Project**: For individual codebases or projects
- **Agent**: For agent-specific memory and context
- **Shared**: For data shared across multiple projects or agents

```bash
# Create different workspace types
cortex workspace create backend --type project
cortex workspace create my-agent --type agent
cortex workspace create common-libs --type shared
```

### Configuration Hierarchy

Cortex uses a layered configuration system:

```
CLI Flags (highest)
    ↓
Environment Variables
    ↓
Project Config (.cortex/config.toml)
    ↓
System Config (~/.config/cortex/config.toml)
    ↓
Defaults (lowest)
```

### Memory Types

Cortex implements a cognitive memory architecture:

- **Working Memory**: Short-term, active context
- **Episodic Memory**: Event-based, temporal memories
- **Semantic Memory**: Fact-based, conceptual knowledge
- **Procedural Memory**: Learned patterns and procedures

## Command Reference

### Database Commands

#### Install SurrealDB

```bash
cortex db install
```

Downloads and installs SurrealDB to your system.

#### Start Database

```bash
# Start with defaults
cortex db start

# Custom bind address
cortex db start --bind 127.0.0.1:9000

# Custom data directory
cortex db start --data-dir /custom/path

# Both custom settings
cortex db start --bind 127.0.0.1:9000 --data-dir /custom/path
```

#### Stop Database

```bash
cortex db stop
```

#### Restart Database

```bash
cortex db restart
```

#### Check Database Status

```bash
cortex db status
```

Output example:
```
SurrealDB Server Status
=======================
  URL: http://127.0.0.1:8000
  Data: /home/user/.local/share/cortex/data
  Logs: /home/user/.local/share/cortex/surrealdb.log
  PID file: /home/user/.local/share/cortex/surrealdb.pid

  Status: ✓ Running
  Health: ✓ Healthy
```

### Workspace Commands

#### Initialize New Workspace

```bash
# Initialize in current directory
cortex init my-project

# Initialize in specific directory
cortex init my-project --path /path/to/project

# Initialize with workspace type
cortex init my-project --workspace-type agent
```

#### Create Workspace

```bash
# Create with default type (project)
cortex workspace create backend

# Create with specific type
cortex workspace create my-agent --type agent
cortex workspace create shared-code --type shared
```

#### List Workspaces

```bash
# Human-readable list
cortex workspace list

# JSON output
cortex workspace list --format json
```

#### Switch Workspace

```bash
cortex workspace switch backend
```

This sets the active workspace for subsequent commands.

#### Delete Workspace

```bash
# With confirmation prompt
cortex workspace delete old-project

# Force delete without confirmation
cortex workspace delete old-project --force
```

### Ingestion Commands

#### Ingest Files

```bash
# Ingest current directory
cortex ingest .

# Ingest specific directory
cortex ingest /path/to/code

# Ingest to specific workspace
cortex ingest ./src --workspace backend

# Non-recursive ingestion
cortex ingest ./src --recursive false
```

What gets ingested:
- Source code files (all languages)
- Documentation (Markdown, text)
- Configuration files (JSON, YAML, TOML)
- Data files (CSV, structured data)

What gets ignored:
- Binary files
- Large files (>10MB by default)
- Common build artifacts (`node_modules`, `target`, etc.)
- Hidden files and directories
- `.git` directories

### Search Commands

#### Basic Search

```bash
# Search everywhere
cortex search "authentication"

# Limit results
cortex search "error handling" --limit 5

# Search specific workspace
cortex search "api" --workspace backend

# JSON output for scripting
cortex search "function" --format json
```

#### Search Output

Human-readable format shows:
- File path
- Line number
- Context snippet
- Relevance score

JSON format provides:
- Full metadata
- Timestamps
- Embeddings (if available)
- Related nodes

### List Commands

#### List Projects

```bash
# All projects
cortex list projects

# Projects in workspace
cortex list projects --workspace backend

# JSON output
cortex list projects --format json
```

#### List Documents

```bash
# All documents
cortex list documents

# In specific workspace
cortex list documents --workspace backend
```

#### List Episodes

```bash
# Recent 20 episodes
cortex list episodes

# Limit to 50
cortex list episodes --limit 50

# In specific workspace
cortex list episodes --workspace backend
```

### MCP Server Commands

#### Start Server

```bash
# Start with defaults (127.0.0.1:3000)
cortex serve

# Custom address and port
cortex serve --address 0.0.0.0 --port 3001
```

The server runs in the foreground. Press `Ctrl+C` to stop.

#### Using MCP with Claude

Once running, configure your Claude Desktop config:

```json
{
  "mcpServers": {
    "cortex": {
      "command": "cortex",
      "args": ["serve"]
    }
  }
}
```

### VFS Commands

#### Flush to Disk

```bash
# Flush workspace to directory
cortex flush my-workspace /output/path

# Flush with specific scope
cortex flush my-workspace ./output --scope workspace
cortex flush my-workspace ./output --scope project
cortex flush my-workspace ./output --scope all
```

The flush operation:
- Creates directories as needed
- Preserves file timestamps
- Handles conflicts
- Reports statistics

### Statistics Commands

#### Show Stats

```bash
# Human-readable statistics
cortex stats

# JSON output
cortex stats --format json
```

Output includes:
- Number of workspaces
- Total files ingested
- Total storage used
- Memory statistics
- Database metrics

### Configuration Commands

#### List Configuration

```bash
cortex config list
```

Shows all configuration values from all sources.

#### Get Value

```bash
cortex config get database.namespace
cortex config get mcp.port
cortex config get storage.cache_size_mb
```

#### Set Value

```bash
# Set project-level (current directory)
cortex config set database.namespace my-project

# Set system-level (global)
cortex config set database.namespace my-namespace --global
```

Available keys:
- `database.connection_string`
- `database.namespace`
- `database.database`
- `database.pool_size`
- `storage.data_dir`
- `storage.cache_size_mb`
- `storage.compression_enabled`
- `mcp.enabled`
- `mcp.address`
- `mcp.port`
- `active_workspace`

### Agent Commands

#### Create Agent Session

```bash
# Create with default type
cortex agent create my-agent

# Create with specific type
cortex agent create coding-assistant --type coding
cortex agent create research-bot --type research
```

#### List Agent Sessions

```bash
# Human-readable list
cortex agent list

# JSON output
cortex agent list --format json
```

#### Delete Agent Session

```bash
cortex agent delete <session-id>
```

Prompts for confirmation before deleting.

### Memory Commands

#### Consolidate Memory

```bash
# Consolidate all workspaces
cortex memory consolidate

# Consolidate specific workspace
cortex memory consolidate --workspace backend
```

This moves memories from working memory to episodic and semantic memory.

#### Forget Old Memories

```bash
# Delete memories before date
cortex memory forget 2024-01-01

# In specific workspace
cortex memory forget 2024-01-01 --workspace old-project
```

Prompts for confirmation before deleting.

## Common Workflows

### Solo Developer Workflow

```bash
# Setup
cortex db start
cortex init my-app

# Daily work
cortex ingest .
cortex search "bug in user service"
cortex search "TODO: implement"

# Weekly maintenance
cortex stats
cortex memory consolidate
```

### Multi-Project Workflow

```bash
# Create project workspaces
cortex workspace create frontend --type project
cortex workspace create backend --type project
cortex workspace create shared --type shared

# Ingest each project
cortex workspace switch frontend
cortex ingest ./frontend

cortex workspace switch backend
cortex ingest ./backend

cortex workspace switch shared
cortex ingest ./shared-libs

# Search across specific project
cortex workspace switch backend
cortex search "database migration"

# Or search specific workspace without switching
cortex search "api endpoint" --workspace backend
```

### Agent Integration Workflow

```bash
# Create agent workspace
cortex workspace create coding-agent --type agent

# Start MCP server
cortex serve

# Agent interacts via MCP
# Memories are stored in agent workspace

# Review agent activity
cortex workspace switch coding-agent
cortex list episodes
cortex search "refactoring"
```

### Team Workflow

```bash
# Shared configuration
export CORTEX_DB_URL="postgres://team-db:5432/cortex"
export CORTEX_DB_USER="team"
export CORTEX_DB_PASSWORD="secret"

# Each developer
cortex init $PROJECT_NAME --workspace-type project
cortex ingest .

# Shared knowledge base
cortex workspace create team-knowledge --type shared
cortex ingest ./docs --workspace team-knowledge
cortex ingest ./wiki --workspace team-knowledge

# Anyone can search shared knowledge
cortex search "deployment process" --workspace team-knowledge
```

## Advanced Usage

### Custom Configuration Files

Create `.cortex/config.toml` in your project:

```toml
active_workspace = "my-project"

[database]
namespace = "my-project"
pool_size = 20

[storage]
cache_size_mb = 2048
compression_enabled = true

[mcp]
enabled = true
port = 3000
```

### Environment-Based Configuration

```bash
# Development
export CORTEX_DB_URL="mem://"
export CORTEX_CACHE_SIZE_MB=512

# Production
export CORTEX_DB_URL="postgres://prod-db/cortex"
export CORTEX_CACHE_SIZE_MB=4096
export CORTEX_DB_POOL_SIZE=50
```

### Scripting with JSON Output

```bash
#!/bin/bash

# Get all workspaces
workspaces=$(cortex workspace list --format json)

# Process each workspace
echo "$workspaces" | jq -r '.[] | .name' | while read workspace; do
    echo "Processing $workspace..."
    cortex ingest . --workspace "$workspace"
done

# Search and save results
cortex search "TODO" --format json > todos.json
cortex search "FIXME" --format json > fixmes.json

# Generate report
jq -s '.[0] + .[1]' todos.json fixmes.json > issues.json
```

### Monitoring and Metrics

```bash
#!/bin/bash

# Monitor stats over time
while true; do
    cortex stats --format json | jq '{
        timestamp: now,
        workspaces: .workspaces,
        files: .files,
        size: .total_size_bytes
    }' >> metrics.jsonl
    sleep 3600  # Every hour
done
```

### Batch Operations

```bash
# Ingest multiple directories
for dir in api web mobile; do
    cortex workspace create "$dir" --type project
    cortex ingest "./$dir" --workspace "$dir"
done

# Consolidate all workspaces
cortex workspace list --format json | \
    jq -r '.[] | .name' | \
    xargs -I {} cortex memory consolidate --workspace {}
```

### Integration with CI/CD

```yaml
# .github/workflows/cortex-ingest.yml
name: Ingest to Cortex
on: [push]
jobs:
  ingest:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install Cortex
        run: cargo install cortex-cli

      - name: Configure Cortex
        env:
          CORTEX_DB_URL: ${{ secrets.CORTEX_DB_URL }}
          CORTEX_DB_USER: ${{ secrets.CORTEX_DB_USER }}
          CORTEX_DB_PASSWORD: ${{ secrets.CORTEX_DB_PASSWORD }}
        run: |
          cortex workspace create $GITHUB_REPOSITORY
          cortex ingest . --workspace $GITHUB_REPOSITORY
```

## Troubleshooting

### Database Won't Start

```bash
# Check if already running
cortex db status

# Check logs
tail -f ~/.local/share/cortex/surrealdb.log

# Kill any stuck processes
pkill -f surreal

# Remove PID file if stale
rm ~/.local/share/cortex/surrealdb.pid

# Try again
cortex db start
```

### Configuration Issues

```bash
# Check all configuration sources
cortex config list --verbose

# Verify environment
env | grep CORTEX_

# Test with minimal config
cortex --config /dev/null stats
```

### Slow Performance

```bash
# Increase cache
cortex config set storage.cache_size_mb 4096 --global

# Increase pool size
cortex config set database.pool_size 30 --global

# Check statistics
cortex stats

# Consolidate memories
cortex memory consolidate
```

### Search Returns No Results

```bash
# Verify workspace
cortex workspace list

# Check active workspace
cortex config get active_workspace

# Re-ingest
cortex ingest . --recursive true

# Verify ingestion
cortex list documents
cortex stats
```

### Permission Errors

```bash
# Fix data directory permissions
chmod -R u+rw ~/.local/share/cortex

# Fix config directory permissions
chmod -R u+rw ~/.config/cortex

# Check database data dir
cortex db status
```

### Out of Disk Space

```bash
# Check usage
cortex stats

# Clear old memories
cortex memory forget 2023-01-01

# Remove old workspaces
cortex workspace list
cortex workspace delete old-project --force
```

## Tips and Best Practices

1. **Use Workspaces Effectively**: Separate workspaces for different projects helps with organization and search precision.

2. **Regular Consolidation**: Run `cortex memory consolidate` weekly to optimize memory storage.

3. **Descriptive Names**: Use clear workspace and agent names for better organization.

4. **Backup Configuration**: Keep your configuration in version control (without secrets).

5. **Monitor Growth**: Check `cortex stats` regularly to monitor storage usage.

6. **Use JSON for Scripts**: Always use `--format json` when scripting for reliable parsing.

7. **Set Active Workspace**: Use `cortex workspace switch` to avoid specifying `--workspace` repeatedly.

8. **Environment Variables**: Use environment variables for sensitive data like passwords.

9. **Log Verbosity**: Use `--verbose` flag when troubleshooting issues.

10. **Test Changes**: Test configuration changes with `cortex config list` before committing.
