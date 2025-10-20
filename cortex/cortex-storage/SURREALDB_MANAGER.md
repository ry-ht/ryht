# SurrealDB Manager for Cortex

A comprehensive, production-ready manager for controlling local SurrealDB server lifecycle within the Cortex cognitive memory system.

## Overview

The SurrealDB Manager provides a robust interface for:
- Detecting and installing SurrealDB
- Starting and stopping local SurrealDB servers
- Health monitoring and readiness checks
- Process lifecycle management
- Graceful shutdown handling

## Features

### Core Capabilities

- **Automatic Installation**: Detects if SurrealDB is installed and can automatically download and install it if needed
- **Process Management**: Full control over server lifecycle with proper PID file management
- **Health Monitoring**: HTTP-based health checks with configurable timeouts
- **Graceful Shutdown**: SIGTERM-based graceful shutdown with fallback to SIGKILL
- **Configuration Management**: Flexible configuration with sensible defaults
- **Cross-platform**: Works on macOS, Linux, and Windows
- **Async/Await**: Built on tokio for efficient async operations
- **Production Ready**: Comprehensive error handling, logging, and retry logic

### Configuration

The manager uses a flexible configuration system:

```rust
pub struct SurrealDBConfig {
    pub bind_address: String,           // e.g., "127.0.0.1:8000"
    pub data_dir: PathBuf,               // Data storage location
    pub log_file: PathBuf,               // Server log file
    pub pid_file: PathBuf,               // Process ID file
    pub username: String,                // Authentication username
    pub password: String,                // Authentication password
    pub storage_engine: String,          // "rocksdb" or "memory"
    pub allow_guests: bool,              // Allow unauthenticated access
    pub max_retries: u32,                // Startup retry attempts
    pub startup_timeout_secs: u64,       // Startup timeout
}
```

Default configuration:
- **Bind Address**: `127.0.0.1:8000`
- **Data Directory**: `~/.ryht/cortex/data/surrealdb/`
- **Log File**: `~/.ryht/cortex/logs/surrealdb.log`
- **PID File**: `~/.ryht/cortex/run/surrealdb.pid`
- **Storage Engine**: RocksDB
- **Authentication**: Required (username: "cortex", password: "cortex")

## Usage

### Basic Usage

```rust
use cortex_storage::{SurrealDBConfig, SurrealDBManager};

#[tokio::main]
async fn main() -> Result<()> {
    // Create manager with default config
    let config = SurrealDBConfig::default();
    let mut manager = SurrealDBManager::new(config).await?;

    // Start the server
    manager.start().await?;

    // Check if running
    if manager.is_running().await {
        println!("Server is running at: {}", manager.connection_url());
    }

    // Perform health check
    manager.health_check().await?;

    // Stop the server
    manager.stop().await?;

    Ok(())
}
```

### Custom Configuration

```rust
use cortex_storage::SurrealDBConfig;
use std::path::PathBuf;

let config = SurrealDBConfig::new(
    "127.0.0.1:9000".to_string(),
    PathBuf::from("/custom/data/path")
)
.with_auth("myuser".to_string(), "mypassword".to_string())
.with_storage_engine("memory".to_string())
.with_allow_guests(false);

let mut manager = SurrealDBManager::new(config).await?;
```

### Installation Management

```rust
// Check if SurrealDB is installed
match SurrealDBManager::find_surreal_binary().await {
    Ok(path) => println!("Found at: {:?}", path),
    Err(_) => println!("Not installed"),
}

// Ensure it's installed (install if necessary)
let binary_path = SurrealDBManager::ensure_installed().await?;

// Manually trigger installation
SurrealDBManager::install_surrealdb().await?;
```

### Advanced Operations

```rust
// Wait for server to be ready
manager.wait_for_ready(Duration::from_secs(30)).await?;

// Restart the server
manager.restart().await?;

// Get server status
let status = manager.status(); // Returns ServerStatus enum

// Get connection URL for clients
let url = manager.connection_url(); // e.g., "http://127.0.0.1:8000"
```

## CLI Commands

The implementation includes CLI commands for managing the server:

### Start Server

```bash
cortex db start
cortex db start --bind 127.0.0.1:9000
cortex db start --data-dir /custom/path
```

### Stop Server

```bash
cortex db stop
```

### Restart Server

```bash
cortex db restart
```

### Check Status

```bash
cortex db status
```

Output example:
```
SurrealDB Server Status
======================
  URL: http://127.0.0.1:8000
  Data: /Users/user/.ryht/cortex/data/surrealdb
  Logs: /Users/user/.ryht/cortex/logs/surrealdb.log
  PID file: /Users/user/.ryht/cortex/run/surrealdb.pid

  Status: ✓ Running
  Health: ✓ Healthy
```

### Install SurrealDB

```bash
cortex db install
```

This will:
1. Check if SurrealDB is already installed
2. If not, download and install it using the official installer
3. Verify the installation

## Architecture

### Process Management

The manager handles process lifecycle through several mechanisms:

1. **Process Spawning**: Uses `std::process::Command` to spawn the SurrealDB process
2. **PID Tracking**: Writes process ID to a file for cross-session tracking
3. **Health Monitoring**: HTTP health checks via `/health` endpoint
4. **Graceful Shutdown**: SIGTERM signal with timeout before SIGKILL

### Directory Structure

```
~/.ryht/cortex/
├── data/
│   └── surrealdb/          # RocksDB data files
├── logs/
│   └── surrealdb.log       # Server logs
└── run/
    └── surrealdb.pid       # Process ID file
```

### Error Handling

The manager includes comprehensive error handling:

- **Installation Errors**: Clear messages with fallback to manual installation
- **Startup Failures**: Retry logic with configurable attempts
- **Health Check Failures**: Timeout handling and detailed error messages
- **Process Termination**: Graceful shutdown with forced kill fallback

### Logging

Uses `tracing` for structured logging:

- **Debug**: Detailed operation information
- **Info**: Major lifecycle events (start, stop, health checks)
- **Warn**: Non-fatal issues (retries, forced kills)
- **Error**: Fatal errors requiring user intervention

## Testing

### Unit Tests

Run unit tests (included in the module):

```bash
cargo test --package cortex-storage --lib surrealdb_manager
```

Tests include:
- Configuration validation
- Directory creation
- Status tracking
- Connection URL generation

### Integration Tests

Run integration tests (requires SurrealDB installed):

```bash
# Run all tests including ignored ones
cargo test --package cortex-storage --test surrealdb_manager_tests -- --ignored

# Or run specific tests
cargo test --package cortex-storage --test surrealdb_manager_tests test_start_stop_server -- --ignored
```

Integration tests cover:
- Binary detection
- Server start/stop lifecycle
- Health checks
- PID file management
- Concurrent operations
- Restart functionality

### Test Configuration

Integration tests use a separate configuration to avoid conflicts:
- Port: 19000 (instead of 8000)
- Storage: Memory backend (faster)
- Temporary directories for data

## Performance Considerations

### Startup Time

- **Memory backend**: ~1-2 seconds
- **RocksDB backend**: ~2-5 seconds (first start, database creation)
- **RocksDB backend**: ~1-2 seconds (subsequent starts)

### Resource Usage

- **Memory**: ~50-100 MB for memory backend, ~100-200 MB for RocksDB
- **Disk**: Varies based on data size (RocksDB uses compression)
- **CPU**: Minimal when idle, scales with query load

### Optimization Tips

1. Use memory backend for testing and development
2. Use RocksDB for production with persistent data
3. Configure appropriate timeouts based on your environment
4. Use connection pooling for client connections

## Security Considerations

### Authentication

- Default credentials should be changed in production
- Credentials are passed via command-line arguments (secure on most systems)
- Consider using environment variables or config files for sensitive data

### Network Binding

- Default binds to localhost (127.0.0.1) only
- Not accessible from network by default
- For remote access, use proper firewall rules and TLS

### File Permissions

- Data directory should have restricted permissions (700)
- Log files may contain sensitive information
- PID files are world-readable by default

## Troubleshooting

### Server Won't Start

1. Check if port is already in use: `lsof -i :8000`
2. Check log file for errors: `tail -f ~/.ryht/cortex/logs/surrealdb.log`
3. Verify SurrealDB is installed: `cortex db install`
4. Check directory permissions

### Health Checks Failing

1. Verify server is actually running: `cortex db status`
2. Check if port is accessible: `curl http://127.0.0.1:8000/health`
3. Review logs for startup errors
4. Increase startup timeout in config

### Zombie Processes

1. Check for stale PID file: `cat ~/.ryht/cortex/run/surrealdb.pid`
2. Verify process: `ps aux | grep surreal`
3. Manually kill if needed: `kill -9 <pid>`
4. Remove stale PID file: `rm ~/.ryht/cortex/run/surrealdb.pid`

### Permission Errors

1. Ensure directories exist: `mkdir -p ~/.ryht/cortex/{data,logs,run}`
2. Check directory permissions: `ls -la ~/.ryht/cortex/`
3. Fix permissions: `chmod 755 ~/.ryht/cortex/`

## Dependencies

- `tokio`: Async runtime
- `tracing`: Structured logging
- `reqwest`: HTTP client for health checks
- `dirs`: Cross-platform directory paths
- `serde`: Serialization
- `cortex-core`: Error types
- `libc`: Unix process signals (Unix only)

## Future Enhancements

Potential improvements:
- [ ] TLS/SSL support
- [ ] Custom authentication methods
- [ ] Cluster mode support
- [ ] Backup/restore integration
- [ ] Prometheus metrics endpoint
- [ ] Docker container support
- [ ] Systemd service file generation
- [ ] Windows service support
- [ ] Configuration hot-reload

## Contributing

When contributing to the SurrealDB manager:

1. Follow Rust best practices and idioms
2. Add tests for new functionality
3. Update documentation
4. Ensure backward compatibility
5. Add tracing for debugging

## License

This implementation is part of the Cortex project and follows the same license (MIT).

## References

- [SurrealDB Documentation](https://surrealdb.com/docs)
- [SurrealDB Installation](https://surrealdb.com/install)
- [Tokio Documentation](https://tokio.rs/)
- [Cortex Project](https://github.com/ry-ht/ryht)
