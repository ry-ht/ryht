# Cortex Configuration - Quick Start Guide

5-minute guide to using the Cortex configuration system.

## Installation

Already included in `cortex-core`. No additional dependencies needed.

## Basic Usage

### 1. Load Configuration

```rust
use cortex_core::config::GlobalConfig;

// Recommended: Load or create default
let config = GlobalConfig::load_or_create_default().await?;

// Or load existing (fails if doesn't exist)
let config = GlobalConfig::load().await?;
```

### 2. Read Configuration

```rust
// Access any configuration section
let log_level = config.general().log_level;
let db_mode = config.database().mode;
let max_connections = config.pool().max_connections;
let cache_size = config.cache().memory_size_mb;
let workers = config.ingestion().parallel_workers;
```

### 3. Modify Configuration

```rust
// Get mutable reference to modify
let mut config = config;

config.general_mut().log_level = "debug".to_string();
config.pool_mut().max_connections = 20;
config.cache_mut().memory_size_mb = 1024;

// Validate changes
config.validate()?;
```

### 4. Save Configuration

```rust
// Save to default location (atomic)
config.save().await?;
```

## Common Patterns

### Development Setup

```rust
let mut config = GlobalConfig::load_or_create_default().await?;
config.general_mut().log_level = "debug".to_string();
config.database_mut().mode = "local".to_string();
config.pool_mut().max_connections = 5;
config.save().await?;
```

### Production Setup

```rust
let mut config = GlobalConfig::load_or_create_default().await?;
config.general_mut().log_level = "warn".to_string();
config.database_mut().mode = "remote".to_string();
config.database_mut().remote_urls = vec!["ws://db.prod.com:8000".to_string()];
config.pool_mut().max_connections = 50;
config.cache_mut().memory_size_mb = 4096;
config.save().await?;
```

### Using Environment Variables

```bash
# Override via environment variables (no code changes needed)
export CORTEX_LOG_LEVEL=debug
export CORTEX_DB_MODE=remote
export CORTEX_DB_URL=ws://localhost:8001
export CORTEX_CACHE_SIZE_MB=2048

# Run your app - env vars automatically applied
./your-app
```

## Configuration Sections

### General
- `version` - Config version
- `log_level` - Log level (trace/debug/info/warn/error)

### Database
- `mode` - Database mode (local/remote/hybrid)
- `local_bind` - Local database address
- `remote_urls` - Remote database URLs
- `username`, `password` - Credentials
- `namespace`, `database` - SurrealDB namespace/database

### Pool
- `min_connections`, `max_connections` - Pool size
- `connection_timeout_ms` - Connection timeout
- `idle_timeout_ms` - Idle timeout

### Cache
- `memory_size_mb` - In-memory cache size
- `ttl_seconds` - Default TTL
- `redis_url` - Optional Redis URL

### VFS
- `max_file_size_mb` - Max file size
- `auto_flush` - Enable auto-flush
- `flush_interval_seconds` - Flush interval

### Ingestion
- `parallel_workers` - Worker count
- `chunk_size` - Batch size
- `generate_embeddings` - Enable embeddings
- `embedding_model` - Model name

### MCP
- `server_bind` - Server address
- `cors_enabled` - Enable CORS
- `max_request_size_mb` - Max request size

## Directory Paths

```rust
// Get any directory path
let config_path = GlobalConfig::config_path()?;
let data_dir = GlobalConfig::data_dir()?;
let logs_dir = GlobalConfig::logs_dir()?;
let cache_dir = GlobalConfig::cache_dir()?;

// Ensure all directories exist
GlobalConfig::ensure_directories().await?;
```

## Error Handling

```rust
use cortex_core::Result;

async fn load_config() -> Result<GlobalConfig> {
    match GlobalConfig::load().await {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            // Fall back to default
            GlobalConfig::load_or_create_default().await
        }
    }
}
```

## Environment Variables

Quick reference for all environment variables:

| Variable | Example | Description |
|----------|---------|-------------|
| `CORTEX_CONFIG_PATH` | `/custom/config.toml` | Config file location |
| `CORTEX_LOG_LEVEL` | `debug` | Log level |
| `CORTEX_DB_MODE` | `remote` | Database mode |
| `CORTEX_DB_URL` | `ws://db:8000` | Database URL |
| `CORTEX_DB_USERNAME` | `admin` | Database username |
| `CORTEX_DB_PASSWORD` | `secret` | Database password |
| `CORTEX_CACHE_SIZE_MB` | `2048` | Cache size (MB) |
| `CORTEX_CACHE_REDIS_URL` | `redis://cache:6379` | Redis URL |

## Tips & Best Practices

1. **Always use `load_or_create_default()`** - Ensures config always exists
2. **Validate before save** - Call `validate()` after programmatic changes
3. **Use environment variables for secrets** - Don't commit passwords
4. **Test your config** - Run validation in tests
5. **Handle errors gracefully** - Config errors shouldn't crash your app

## Example Application

```rust
use cortex_core::config::GlobalConfig;
use cortex_core::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration
    let config = GlobalConfig::load_or_create_default().await?;

    // 2. Use configuration
    println!("Starting Cortex with log level: {}", config.general().log_level);
    println!("Database mode: {}", config.database().mode);

    // 3. Your application logic here
    // ...

    Ok(())
}
```

## Next Steps

- **Full Documentation**: See `CONFIG.md` for complete guide
- **Run Example**: `cargo run --example config_usage`
- **View Tests**: See `tests/config_integration.rs` for more examples

## Troubleshooting

**Q: Config file not found?**
A: Use `load_or_create_default()` instead of `load()`

**Q: Validation error?**
A: Check `CONFIG.md` for validation rules

**Q: Permission denied?**
A: Ensure write access to `~/.ryht/cortex/`

**Q: Environment variables not working?**
A: Environment variables override on load, not save

## Getting Help

- Documentation: `CONFIG.md`
- Examples: `examples/config_usage.rs`
- Tests: `tests/config_integration.rs`
- API docs: `cargo doc --open`
