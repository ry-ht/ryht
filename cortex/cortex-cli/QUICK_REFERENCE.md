# Cortex CLI Quick Reference

Quick command reference for daily use.

## Daily Commands

### Workspace Management
```bash
cortex init my-project              # Initialize new workspace
cortex workspace list               # List all workspaces
cortex workspace switch my-ws       # Switch active workspace
```

### Database
```bash
cortex db start                     # Start database
cortex db status                    # Check status
cortex db stop                      # Stop database
```

### Ingestion
```bash
cortex ingest ./src                 # Ingest directory
cortex ingest ./src --recursive     # Ingest recursively
```

### Search
```bash
cortex search "query"               # Search memory
cortex search "query" --limit 20    # Limit results
```

### Health & Diagnostics
```bash
cortex doctor check                 # Run diagnostics
cortex doctor check --fix           # Auto-fix issues
cortex doctor health                # Quick health check
```

### Testing
```bash
cortex test all                     # Run all tests
cortex test benchmark               # Run benchmarks
```

### Export
```bash
cortex export workspace my-ws -o out.json          # Export workspace (JSON)
cortex export workspace my-ws -o out.csv -f csv    # Export workspace (CSV)
cortex export episodes -o episodes.yaml -f yaml    # Export episodes (YAML)
cortex export stats -o stats.json                  # Export stats
```

### Configuration
```bash
cortex config list                            # Show all config
cortex config get database.namespace          # Get value
cortex config set storage.cache_size_mb 2048  # Set value
```

### Interactive
```bash
cortex interactive                   # Main menu
cortex interactive --mode wizard     # Workspace wizard
cortex interactive --mode search     # Interactive search
```

## Global Flags

```bash
--verbose              # Enable verbose logging
--config <path>        # Use custom config file
--format json          # JSON output
--format plain         # Plain text output
```

## Common Workflows

### Initial Setup
```bash
cortex db install
cortex db start
cortex init my-project
cortex workspace switch my-project
cortex ingest ./src
```

### Daily Development
```bash
cortex db status
cortex ingest ./new-code
cortex search "new feature"
cortex stats
```

### Maintenance
```bash
cortex doctor check
cortex memory consolidate
cortex export stats -o backup.json
```

### Troubleshooting
```bash
cortex --verbose doctor check
cortex db restart
cortex test all
```

## Exit Codes

- `0` - Success
- `1` - Error or failure

## Environment Variables

```bash
export CORTEX_DB_URL="ws://localhost:8000"
export CORTEX_DB_NAMESPACE="myapp"
export CORTEX_DATA_DIR="/var/lib/cortex"
export CORTEX_CACHE_SIZE_MB=2048
export CORTEX_MCP_PORT=3001
```

## Quick Tips

1. Use `--help` on any command for details
2. JSON output perfect for scripts: `--format json`
3. Doctor can auto-fix: `cortex doctor check --fix`
4. Interactive mode for complex tasks
5. Export before major changes

## Examples

### Automation Script
```bash
#!/bin/bash
cortex db start
cortex doctor health || exit 1
cortex ingest ./src
cortex export stats -o stats-$(date +%Y%m%d).json
```

### CI/CD Pipeline
```bash
cortex db start
cortex test all || exit 1
cortex doctor check
```

### Backup Script
```bash
for ws in $(cortex workspace list --format json | jq -r '.[].name'); do
  cortex export workspace "$ws" -o "backup/${ws}.json"
done
```

## Getting Help

```bash
cortex --help                        # General help
cortex workspace --help              # Command help
cortex workspace create --help       # Subcommand help
```

For detailed documentation, see [USER_GUIDE.md](USER_GUIDE.md).
