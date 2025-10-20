# Cortex Configuration System

Comprehensive guide to the Cortex global configuration system.

## Overview

The Cortex configuration system provides a centralized way to manage all system settings through a TOML configuration file located at `~/.ryht/cortex/config.toml`. The system supports:

- **Automatic directory creation** - All required directories are created on first use
- **Environment variable overrides** - Override any config value via environment variables
- **Atomic updates** - Configuration changes are saved atomically to prevent corruption
- **Validation** - All configuration values are validated before saving
- **Type-safe access** - Strongly-typed configuration sections with accessor methods

## Directory Structure

The configuration system manages the following directory structure:

```
~/.ryht/cortex/
├── config.toml          # Main configuration file
├── data/
│   └── surrealdb/      # SurrealDB database files
├── logs/               # Application log files
├── run/                # PID files for running processes
├── cache/              # Cache directory
└── workspaces/         # Workspace metadata
```

All directories are created automatically on first use.

## Configuration File

### Location

Default: `~/.ryht/cortex/config.toml`

Override with: `CORTEX_CONFIG_PATH` environment variable

### Format

The configuration file uses TOML format with the following sections:

#### General Configuration

```toml
[general]
version = "0.1.0"      # Config version for migration
log_level = "info"     # Log level: trace, debug, info, warn, error
```

#### Database Configuration

```toml
[database]
mode = "local"                    # Mode: local, remote, hybrid
local_bind = "127.0.0.1:8000"    # Local database binding address
remote_urls = []                  # Remote database URLs (for remote/hybrid)
username = "root"                 # Database username
password = "root"                 # Database password
namespace = "cortex"              # SurrealDB namespace
database = "knowledge"            # SurrealDB database name
```

**Database Modes:**
- `local` - Embedded SurrealDB instance (best for single-user, development)
- `remote` - Connect to remote SurrealDB instance(s) (best for production, multi-user)
- `hybrid` - Local cache with remote sync (best for distributed systems)

#### Connection Pool Configuration

```toml
[pool]
min_connections = 2              # Minimum connections in pool
max_connections = 10             # Maximum connections in pool
connection_timeout_ms = 5000     # Connection timeout (milliseconds)
idle_timeout_ms = 300000         # Idle timeout (5 minutes)
```

#### Cache Configuration

```toml
[cache]
memory_size_mb = 512             # In-memory cache size (MB)
ttl_seconds = 300                # Default TTL for cache entries (seconds)
redis_url = ""                   # Optional Redis URL for distributed caching
```

#### Virtual Filesystem Configuration

```toml
[vfs]
max_file_size_mb = 100           # Maximum file size (MB)
auto_flush = false               # Enable automatic flushing
flush_interval_seconds = 60      # Flush interval (seconds)
```

#### Ingestion Pipeline Configuration

```toml
[ingestion]
parallel_workers = 4                          # Number of parallel workers
chunk_size = 1000                             # Batch processing chunk size
generate_embeddings = true                    # Enable embedding generation
embedding_model = "text-embedding-3-small"    # Embedding model
```

**Supported Embedding Models:**
- `text-embedding-3-small` (default)
- `text-embedding-3-large`
- `text-embedding-ada-002`

#### MCP Server Configuration

```toml
[mcp]
server_bind = "127.0.0.1:3000"   # Server binding address
cors_enabled = true               # Enable CORS
max_request_size_mb = 10         # Maximum request size (MB)
```

## Usage

### Loading Configuration

```rust
use cortex_core::config::GlobalConfig;

// Load existing config (fails if doesn't exist)
let config = GlobalConfig::load().await?;

// Load or create default config (recommended)
let config = GlobalConfig::load_or_create_default().await?;

// Load from custom path
let config = GlobalConfig::load_from_path("/path/to/config.toml").await?;
```

### Accessing Configuration

```rust
// Access sections (immutable)
let log_level = config.general().log_level;
let db_mode = config.database().mode;
let max_connections = config.pool().max_connections;
let cache_size = config.cache().memory_size_mb;
let max_file_size = config.vfs().max_file_size_mb;
let workers = config.ingestion().parallel_workers;
let server_bind = config.mcp().server_bind;
```

### Modifying Configuration

```rust
// Modify sections (mutable)
config.general_mut().log_level = "debug".to_string();
config.database_mut().namespace = "production".to_string();
config.pool_mut().max_connections = 20;
config.cache_mut().memory_size_mb = 1024;
```

### Saving Configuration

```rust
// Save to default location (atomic)
config.save().await?;

// Save to custom path (atomic)
config.save_to_path("/path/to/config.toml").await?;
```

### Validation

```rust
// Validate configuration
match config.validate() {
    Ok(_) => println!("Configuration is valid"),
    Err(e) => println!("Configuration error: {}", e),
}

// Note: Validation is automatically performed on load and save
```

### Directory Paths

```rust
// Get directory paths
let base_dir = GlobalConfig::base_dir()?;
let data_dir = GlobalConfig::data_dir()?;
let logs_dir = GlobalConfig::logs_dir()?;
let cache_dir = GlobalConfig::cache_dir()?;
let surrealdb_dir = GlobalConfig::surrealdb_dir()?;
let workspaces_dir = GlobalConfig::workspaces_dir()?;

// Ensure all directories exist
GlobalConfig::ensure_directories().await?;
```

## Environment Variable Overrides

All configuration values can be overridden using environment variables with the `CORTEX_` prefix:

| Environment Variable | Configuration Path | Example |
|---------------------|-------------------|---------|
| `CORTEX_CONFIG_PATH` | Config file location | `/custom/path/config.toml` |
| `CORTEX_LOG_LEVEL` | `general.log_level` | `debug` |
| `CORTEX_DB_MODE` | `database.mode` | `remote` |
| `CORTEX_DB_URL` | `database.remote_urls` | `ws://localhost:8001` |
| `CORTEX_DB_LOCAL_BIND` | `database.local_bind` | `127.0.0.1:9000` |
| `CORTEX_DB_USERNAME` | `database.username` | `admin` |
| `CORTEX_DB_PASSWORD` | `database.password` | `secret` |
| `CORTEX_DB_NAMESPACE` | `database.namespace` | `prod` |
| `CORTEX_DB_DATABASE` | `database.database` | `main` |
| `CORTEX_MCP_SERVER_BIND` | `mcp.server_bind` | `0.0.0.0:3000` |
| `CORTEX_CACHE_SIZE_MB` | `cache.memory_size_mb` | `2048` |
| `CORTEX_CACHE_REDIS_URL` | `cache.redis_url` | `redis://localhost:6379` |

### Example

```bash
# Override configuration via environment variables
export CORTEX_LOG_LEVEL=debug
export CORTEX_DB_MODE=remote
export CORTEX_DB_URL=ws://production.example.com:8000
export CORTEX_CACHE_SIZE_MB=2048

# Run your application - environment variables will override file config
./cortex-server
```

## Configuration Presets

### Development Configuration

Optimized for local development:

```toml
[general]
log_level = "debug"

[database]
mode = "local"

[pool]
max_connections = 5

[cache]
memory_size_mb = 256

[ingestion]
parallel_workers = 2
```

### Production Configuration

Optimized for production deployment:

```toml
[general]
log_level = "warn"

[database]
mode = "hybrid"
remote_urls = ["ws://primary:8000", "ws://backup:8000"]

[pool]
min_connections = 5
max_connections = 50

[cache]
memory_size_mb = 4096
redis_url = "redis://cache:6379"

[ingestion]
parallel_workers = 16

[mcp]
max_request_size_mb = 50
```

### Testing Configuration

Optimized for testing:

```toml
[general]
log_level = "trace"

[database]
mode = "local"

[pool]
min_connections = 1
max_connections = 2

[cache]
memory_size_mb = 128

[ingestion]
parallel_workers = 1
generate_embeddings = false
```

## Validation Rules

The configuration system enforces the following validation rules:

### General
- `log_level` must be one of: `trace`, `debug`, `info`, `warn`, `error`

### Database
- `mode` must be one of: `local`, `remote`, `hybrid`
- For `remote` or `hybrid` modes, `remote_urls` must not be empty

### Pool
- `min_connections` must be ≤ `max_connections`
- `max_connections` must be > 0

### Cache
- `memory_size_mb` can be 0 (disables caching)

### VFS
- `max_file_size_mb` must be > 0

### Ingestion
- `parallel_workers` must be > 0
- `chunk_size` must be > 0

### MCP
- `max_request_size_mb` must be > 0

## Atomic Updates

All configuration saves use atomic writes to prevent corruption:

1. Configuration is serialized to TOML
2. Content is written to a temporary file (`config.toml.tmp`)
3. Temporary file is renamed to `config.toml` (atomic operation)
4. If any step fails, the original configuration remains unchanged

This ensures that the configuration file is always in a valid state, even if the process is interrupted.

## Migration Support

The configuration system includes version tracking for future migration support:

- Current version is stored in `general.version`
- Version is checked on load
- Future versions can include migration logic to upgrade old configurations

## Error Handling

The configuration system provides detailed error messages for common issues:

- **File not found** - Configuration file doesn't exist (use `load_or_create_default`)
- **Invalid TOML** - Syntax error in configuration file
- **Validation error** - Configuration values don't meet validation rules
- **IO error** - Permission or disk space issues
- **Parse error** - Environment variable contains invalid value

## Best Practices

1. **Use `load_or_create_default()`** - This ensures configuration always exists
2. **Validate before use** - Call `validate()` after programmatic changes
3. **Use environment variables for secrets** - Don't commit passwords to config files
4. **Use atomic saves** - Always use the built-in `save()` methods
5. **Handle errors gracefully** - Configuration errors should not crash the application
6. **Document custom values** - Add comments in config file for non-default settings
7. **Version control** - Keep example configs in version control, not actual configs
8. **Test configurations** - Validate config in CI/CD before deployment

## Examples

See the following examples for more details:

- **Basic Usage**: `examples/config_usage.rs`
- **Example Config**: `examples/config_example.toml`

Run the example:

```bash
cd cortex-core
cargo run --example config_usage
```

## Testing

The configuration system includes comprehensive tests:

```bash
# Run unit tests
cargo test --lib config

# Run integration tests
cargo test --test config_integration

# Run all tests
cargo test
```

Note: Integration tests use `--test-threads=1` to avoid environment variable conflicts.

## Troubleshooting

### Configuration file not found

**Error**: `Failed to read config file: No such file or directory`

**Solution**: Use `GlobalConfig::load_or_create_default()` instead of `GlobalConfig::load()`

### Invalid log level

**Error**: `Invalid log level 'xxx'. Must be one of: trace, debug, info, warn, error`

**Solution**: Use a valid log level or check environment variable `CORTEX_LOG_LEVEL`

### Remote mode without URLs

**Error**: `Remote database URLs must be provided for remote/hybrid mode`

**Solution**: Add at least one URL to `database.remote_urls` or set `CORTEX_DB_URL`

### Permission denied

**Error**: `Failed to create directory: Permission denied`

**Solution**: Ensure write permissions for `~/.ryht/cortex/` or set `CORTEX_CONFIG_PATH` to a writable location

### Pool configuration error

**Error**: `min_connections cannot be greater than max_connections`

**Solution**: Ensure `pool.min_connections` ≤ `pool.max_connections`

## Contributing

When adding new configuration options:

1. Add the field to the appropriate struct in `config.rs`
2. Add a default value in the `Default` implementation
3. Add validation logic in the `validate()` method
4. Add environment variable support in `merge_env_vars()`
5. Update this documentation
6. Add tests for the new configuration option
7. Update `examples/config_example.toml`
