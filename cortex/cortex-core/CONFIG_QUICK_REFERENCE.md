# Cortex Configuration Quick Reference

## Table of Contents
- [Quick Start](#quick-start)
- [Environment Variables](#environment-variables)
- [Configuration Profiles](#configuration-profiles)
- [Common Operations](#common-operations)
- [Directory Structure](#directory-structure)
- [API Cheat Sheet](#api-cheat-sheet)

## Quick Start

### Load Configuration
```rust
use cortex_core::config::GlobalConfig;

// Simple load or create
let config = GlobalConfig::load_or_create_default().await?;
```

### Use ConfigManager (Recommended)
```rust
use cortex_core::config::ConfigManager;

// Get global singleton
let manager = ConfigManager::global().await?;

// Read config
let config = manager.read().await;
println!("Log level: {}", config.general().log_level);
```

### Create with Profile
```rust
use cortex_core::config::{GlobalConfig, ConfigProfile};

let config = GlobalConfig::with_profile(ConfigProfile::Prod);
config.save().await?;
```

## Environment Variables

All environment variables use the `CORTEX_` prefix:

| Variable | Description | Example |
|----------|-------------|---------|
| `CORTEX_CONFIG_PATH` | Override config file location | `/custom/path/config.toml` |
| `CORTEX_CONFIG_PROFILE` | Set profile (dev/prod/test) | `prod` |
| `CORTEX_LOG_LEVEL` | Override log level | `debug` |
| `CORTEX_DB_MODE` | Database mode | `remote` |
| `CORTEX_DB_URL` | Database URL | `ws://db.example.com:8000` |
| `CORTEX_DB_LOCAL_BIND` | Local bind address | `127.0.0.1:8000` |
| `CORTEX_DB_USERNAME` | Database username | `admin` |
| `CORTEX_DB_PASSWORD` | Database password | `secret` |
| `CORTEX_DB_NAMESPACE` | SurrealDB namespace | `production` |
| `CORTEX_DB_DATABASE` | SurrealDB database | `knowledge` |
| `CORTEX_MCP_SERVER_BIND` | MCP server address | `0.0.0.0:3000` |
| `CORTEX_CACHE_SIZE_MB` | Cache size in MB | `2048` |
| `CORTEX_CACHE_REDIS_URL` | Redis URL | `redis://cache:6379` |

### Usage
```bash
# Set for development
export CORTEX_CONFIG_PROFILE=dev
export CORTEX_LOG_LEVEL=debug

# Set for production
export CORTEX_CONFIG_PROFILE=prod
export CORTEX_DB_MODE=remote
export CORTEX_DB_URL=ws://production-db:8000
```

## Configuration Profiles

### Dev Profile
```rust
let config = GlobalConfig::with_profile(ConfigProfile::Dev);
// - Log level: debug
// - Hot reload: enabled
// - Max connections: 5
// - Cache: 256 MB
```

### Prod Profile
```rust
let config = GlobalConfig::with_profile(ConfigProfile::Prod);
// - Log level: info
// - Hot reload: disabled
// - Max connections: 20
// - Cache: 2048 MB
```

### Test Profile
```rust
let config = GlobalConfig::with_profile(ConfigProfile::Test);
// - Log level: warn
// - Hot reload: disabled
// - Max connections: 2
// - Cache: 128 MB
// - Database namespace: cortex_test
```

## Common Operations

### Read Configuration
```rust
let manager = ConfigManager::global().await?;
let config = manager.read().await;
println!("Database mode: {}", config.database().mode);
```

### Update Configuration
```rust
let manager = ConfigManager::global().await?;
manager.update(|cfg| {
    cfg.general_mut().log_level = "debug".to_string();
    cfg.pool_mut().max_connections = 20;
    Ok(())
}).await?;
```

### Save Configuration
```rust
let manager = ConfigManager::global().await?;
manager.save().await?;
```

### Reload Configuration
```rust
let manager = ConfigManager::global().await?;
manager.reload().await?;
```

### Validate Configuration
```rust
let config = GlobalConfig::load().await?;
config.validate()?;
```

### Export Configuration
```rust
let config = GlobalConfig::load().await?;

// Export to TOML
let toml = config.export_toml()?;
std::fs::write("backup.toml", toml)?;

// Export to JSON
let json = config.export_json()?;
std::fs::write("backup.json", json)?;
```

### Import Configuration
```rust
// Import from TOML
let toml = std::fs::read_to_string("config.toml")?;
let config = GlobalConfig::import_toml(&toml)?;

// Import from JSON
let json = std::fs::read_to_string("config.json")?;
let config = GlobalConfig::import_json(&json)?;
```

### Enable Hot-Reload
```rust
use std::sync::Arc;

let manager = Arc::new(ConfigManager::new(config, config_path));
manager.clone().start_hot_reload().await?;
// Config will now automatically reload when file changes
```

## Directory Structure

All directories are created automatically under `~/.ryht/cortex/`:

```
~/.ryht/cortex/
├── config.toml          # Main configuration file
├── surrealdb/          # SurrealDB data and logs
├── cache/              # Content cache
├── sessions/           # Agent sessions
├── temp/               # Temporary files
├── data/               # Additional data files
├── logs/               # Application logs
├── run/                # PID files
└── workspaces/         # Workspace metadata
```

### Get Directory Paths
```rust
let base = GlobalConfig::base_dir()?;
let surrealdb = GlobalConfig::surrealdb_dir()?;
let cache = GlobalConfig::cache_dir()?;
let sessions = GlobalConfig::sessions_dir()?;
let temp = GlobalConfig::temp_dir()?;
let logs = GlobalConfig::logs_dir()?;
```

## API Cheat Sheet

### GlobalConfig Methods

| Method | Description |
|--------|-------------|
| `load()` | Load from default location |
| `load_from_path(path)` | Load from specific path |
| `load_or_create_default()` | Load or create with defaults |
| `save()` | Save to default location |
| `save_to_path(path)` | Save to specific path |
| `validate()` | Validate configuration |
| `with_profile(profile)` | Create with profile |
| `export_json()` | Export to JSON |
| `import_json(json)` | Import from JSON |
| `export_toml()` | Export to TOML |
| `import_toml(toml)` | Import from TOML |
| `migrate()` | Apply migrations |
| `metadata()` | Get metadata |

### ConfigManager Methods

| Method | Description |
|--------|-------------|
| `global()` | Get singleton |
| `new(config, path)` | Create new manager |
| `read()` | Get read lock |
| `write()` | Get write lock |
| `save()` | Save to disk |
| `reload()` | Reload from disk |
| `start_hot_reload()` | Start monitoring |
| `update(closure)` | Update with closure |
| `clone_config()` | Clone configuration |

### Accessor Methods

| Accessor | Mutable | Section |
|----------|---------|---------|
| `general()` | `general_mut()` | General settings |
| `database()` | `database_mut()` | Database config |
| `pool()` | `pool_mut()` | Connection pool |
| `cache()` | `cache_mut()` | Cache settings |
| `vfs()` | `vfs_mut()` | VFS settings |
| `ingestion()` | `ingestion_mut()` | Ingestion config |
| `mcp()` | `mcp_mut()` | MCP server |
| `profile()` | `set_profile()` | Config profile |

## Configuration File Format (TOML)

```toml
[general]
version = "0.1.0"
log_level = "info"
hot_reload = true
hot_reload_interval_secs = 5

[database]
mode = "local"
local_bind = "127.0.0.1:8000"
remote_urls = []
username = "root"
password = "root"
namespace = "cortex"
database = "knowledge"

[pool]
min_connections = 2
max_connections = 10
connection_timeout_ms = 5000
idle_timeout_ms = 300000

[cache]
memory_size_mb = 512
ttl_seconds = 300
redis_url = ""

[vfs]
max_file_size_mb = 100
auto_flush = false
flush_interval_seconds = 60

[ingestion]
parallel_workers = 4
chunk_size = 1000
generate_embeddings = true
embedding_model = "text-embedding-3-small"

[mcp]
server_bind = "127.0.0.1:3000"
cors_enabled = true
max_request_size_mb = 10

profile = "dev"
```

## Error Handling

All configuration operations return `Result<T, CortexError>`:

```rust
use cortex_core::Result;

async fn load_config() -> Result<()> {
    let config = GlobalConfig::load().await?;
    config.validate()?;
    config.save().await?;
    Ok(())
}
```

Common errors:
- `CortexError::Config` - Configuration-related errors
- `CortexError::Io` - File I/O errors
- `CortexError::Serialization` - Parsing errors

## Best Practices

1. ✅ Use `ConfigManager::global()` for application-wide access
2. ✅ Validate configuration after modifications
3. ✅ Use profiles instead of manual configuration
4. ✅ Override with environment variables in production
5. ✅ Enable hot-reload in development
6. ✅ Disable hot-reload in production
7. ✅ Use `update()` closure for complex changes
8. ✅ Export configuration for backups

## Examples

See `/cortex/cortex-core/examples/config_usage.rs` for comprehensive examples demonstrating all features.

Run the example:
```bash
cargo run --example config_usage
```
