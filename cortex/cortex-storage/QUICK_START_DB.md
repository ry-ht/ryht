# SurrealDB Manager - Quick Start Guide

## Installation

### 1. Install SurrealDB (if not already installed)

```bash
cortex db install
```

Or manually:
```bash
# macOS/Linux
curl -sSf https://install.surrealdb.com | sh

# Windows
iwr https://install.surrealdb.com -useb | iex
```

### 2. Verify Installation

```bash
cortex db status
```

## Basic Usage

### Start Database Server

```bash
# Start with default settings (127.0.0.1:8000)
cortex db start

# Start on custom port
cortex db start --bind 127.0.0.1:9000

# Start with custom data directory
cortex db start --data-dir /path/to/data
```

### Check Server Status

```bash
cortex db status
```

### Stop Database Server

```bash
cortex db stop
```

### Restart Server

```bash
cortex db restart
```

## Programmatic Usage

### Basic Example

```rust
use cortex_storage::{SurrealDBConfig, SurrealDBManager};

#[tokio::main]
async fn main() -> Result<()> {
    // Create and start server
    let config = SurrealDBConfig::default();
    let mut manager = SurrealDBManager::new(config).await?;

    manager.start().await?;
    println!("Server started at: {}", manager.connection_url());

    // Use the database...

    // Stop when done
    manager.stop().await?;

    Ok(())
}
```

### Custom Configuration

```rust
use cortex_storage::SurrealDBConfig;
use std::path::PathBuf;

let config = SurrealDBConfig::default()
    .with_auth("admin".into(), "secure_password".into())
    .with_storage_engine("rocksdb".into());

let mut manager = SurrealDBManager::new(config).await?;
manager.start().await?;
```

### With Health Checks

```rust
use std::time::Duration;

// Start server
manager.start().await?;

// Wait for server to be ready
manager.wait_for_ready(Duration::from_secs(30)).await?;

// Perform health check
if manager.health_check().await.is_ok() {
    println!("Server is healthy!");
}
```

## Configuration

### Default Locations

- **Data**: `~/.ryht/cortex/data/surrealdb/`
- **Logs**: `~/.ryht/cortex/logs/surrealdb.log`
- **PID**: `~/.ryht/cortex/run/surrealdb.pid`

### Default Settings

- **Port**: 8000
- **Host**: 127.0.0.1 (localhost only)
- **Storage**: RocksDB
- **Auth**: Required (user: cortex, pass: cortex)

## Common Operations

### Check if Server is Running

```rust
if manager.is_running().await {
    println!("Server is running");
}
```

### Get Connection URL

```rust
let url = manager.connection_url();
// Returns: "http://127.0.0.1:8000"
```

### Get Server Status

```rust
use cortex_storage::ServerStatus;

match manager.status() {
    ServerStatus::Running => println!("Running"),
    ServerStatus::Stopped => println!("Stopped"),
    ServerStatus::Starting => println!("Starting..."),
    ServerStatus::Stopping => println!("Stopping..."),
    ServerStatus::Unknown => println!("Unknown"),
}
```

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the port
lsof -i :8000

# Use a different port
cortex db start --bind 127.0.0.1:9000
```

### Check Logs

```bash
tail -f ~/.ryht/cortex/logs/surrealdb.log
```

### Clean Start

```bash
# Stop server
cortex db stop

# Remove old data (WARNING: deletes all data)
rm -rf ~/.ryht/cortex/data/surrealdb/

# Start fresh
cortex db start
```

### Manual Cleanup

```bash
# Remove PID file if stuck
rm ~/.ryht/cortex/run/surrealdb.pid

# Kill process manually
ps aux | grep surreal
kill -9 <PID>
```

## Testing

### Run Unit Tests

```bash
cargo test --package cortex-storage --lib surrealdb_manager
```

### Run Integration Tests

```bash
# Install SurrealDB first if not installed
cortex db install

# Run integration tests
cargo test --package cortex-storage --test surrealdb_manager_tests -- --ignored
```

### Run Specific Test

```bash
cargo test --package cortex-storage test_start_stop_server -- --ignored
```

## Environment-Specific Usage

### Development

```rust
let config = SurrealDBConfig::default()
    .with_storage_engine("memory".into())  // Faster for testing
    .with_allow_guests(true);               // No auth required
```

### Production

```rust
let config = SurrealDBConfig::default()
    .with_storage_engine("rocksdb".into())
    .with_auth("admin".into(), env::var("DB_PASSWORD")?);
```

### Testing

```rust
use tempfile::TempDir;

let temp_dir = TempDir::new()?;
let config = SurrealDBConfig::new(
    "127.0.0.1:19000".into(),
    temp_dir.path().to_path_buf()
)
.with_storage_engine("memory".into());
```

## Best Practices

1. **Always use authentication in production**
2. **Use RocksDB for persistent data**
3. **Monitor logs for errors**
4. **Implement proper shutdown in applications**
5. **Use health checks before queries**
6. **Keep backups of RocksDB data directory**

## Next Steps

- Read the full documentation: `SURREALDB_MANAGER.md`
- Explore SurrealDB: https://surrealdb.com/docs
- Configure connection pooling in your application
- Set up automated backups

## Support

For issues or questions:
- Check logs: `~/.ryht/cortex/logs/surrealdb.log`
- Run status check: `cortex db status`
- Review documentation: `SURREALDB_MANAGER.md`
