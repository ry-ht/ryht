# SurrealDB Manager Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Cortex Application                      │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  Cortex CLI                          │  │
│  │                                                      │  │
│  │  cortex db install | start | stop | status         │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                       │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │              CLI Commands Module                     │  │
│  │                                                      │  │
│  │  db_install()  db_start()  db_stop()               │  │
│  │  db_status()   db_restart()                        │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                       │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │           SurrealDBManager (Core)                    │  │
│  │                                                      │  │
│  │  • Installation Management                          │  │
│  │  • Process Lifecycle                                │  │
│  │  • Health Monitoring                                │  │
│  │  • Configuration                                    │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                       │
└─────────────────────┼───────────────────────────────────────┘
                      │
    ┌─────────────────┼─────────────────┐
    │                 │                 │
    ▼                 ▼                 ▼
┌─────────┐    ┌──────────┐    ┌──────────────┐
│  Shell  │    │   HTTP   │    │  Filesystem  │
│Commands │    │ Requests │    │   (PID/Logs) │
└─────────┘    └──────────┘    └──────────────┘
    │                 │                 │
    ▼                 ▼                 ▼
┌──────────────────────────────────────────────┐
│         SurrealDB Server Process             │
│                                              │
│  • HTTP API (127.0.0.1:8000)                │
│  • RocksDB Storage                           │
│  • Authentication                            │
└──────────────────────────────────────────────┘
```

## Component Architecture

### 1. Manager Module (`surrealdb_manager.rs`)

```rust
┌─────────────────────────────────────────────┐
│          SurrealDBManager                   │
├─────────────────────────────────────────────┤
│ Fields:                                     │
│  • config: SurrealDBConfig                  │
│  • process: Option<Child>                   │
│  • status: ServerStatus                     │
├─────────────────────────────────────────────┤
│ Responsibilities:                           │
│  1. Process Management                      │
│     - Spawn server process                  │
│     - Track via Child handle                │
│     - Monitor with PID files                │
│                                             │
│  2. Installation Management                 │
│     - Find binary in PATH                   │
│     - Download if missing                   │
│     - Verify installation                   │
│                                             │
│  3. Health Monitoring                       │
│     - HTTP health checks                    │
│     - Readiness detection                   │
│     - Retry logic                           │
│                                             │
│  4. Lifecycle Control                       │
│     - Start with config                     │
│     - Graceful shutdown                     │
│     - Restart handling                      │
└─────────────────────────────────────────────┘
```

### 2. Configuration System

```rust
┌─────────────────────────────────────────────┐
│          SurrealDBConfig                    │
├─────────────────────────────────────────────┤
│ Network:                                    │
│  • bind_address: String                     │
│                                             │
│ Storage:                                    │
│  • data_dir: PathBuf                        │
│  • storage_engine: String                   │
│                                             │
│ Logging:                                    │
│  • log_file: PathBuf                        │
│  • pid_file: PathBuf                        │
│                                             │
│ Security:                                   │
│  • username: String                         │
│  • password: String                         │
│  • allow_guests: bool                       │
│                                             │
│ Reliability:                                │
│  • max_retries: u32                         │
│  • startup_timeout_secs: u64                │
├─────────────────────────────────────────────┤
│ Builder Methods:                            │
│  • with_auth()                              │
│  • with_storage_engine()                    │
│  • with_allow_guests()                      │
│  • ensure_directories()                     │
│  • validate()                               │
└─────────────────────────────────────────────┘
```

## Data Flow

### Starting the Server

```
User Command
    │
    ▼
cortex db start
    │
    ▼
db_start() [CLI]
    │
    ▼
SurrealDBManager::new(config)
    │
    ├─► validate config
    ├─► create directories
    └─► return manager
    │
    ▼
manager.start()
    │
    ├─► ensure_installed()
    │   ├─► find_surreal_binary()
    │   │   ├─► check PATH
    │   │   └─► check common locations
    │   └─► install_surrealdb() [if needed]
    │
    ├─► build command
    │   ├─► set bind address
    │   ├─► set storage path
    │   ├─► set auth credentials
    │   └─► configure logging
    │
    ├─► spawn process
    │   ├─► redirect stdout/stderr to log
    │   ├─► get process handle
    │   └─► write PID file
    │
    └─► wait_for_ready()
        ├─► poll health endpoint
        ├─► retry on failure
        └─► timeout after N seconds
```

### Health Checking

```
manager.health_check()
    │
    ▼
Build HTTP request
    │
    ├─► URL: http://{bind_address}/health
    ├─► Timeout: 5 seconds
    └─► Client: reqwest
    │
    ▼
Send request
    │
    ├─► Success (200 OK) ──► return Ok(())
    │
    └─► Error ──► return Err(CortexError)
```

### Stopping the Server

```
manager.stop()
    │
    ▼
Check if running
    │
    ├─► Not running ──► return Ok(())
    │
    └─► Running
        │
        ▼
    Get process handle
        │
        ▼
    Send SIGTERM (Unix) / Terminate (Windows)
        │
        ▼
    Wait for exit (10s timeout)
        │
        ├─► Exited ──► cleanup PID file
        │               return Ok(())
        │
        └─► Still running
            │
            ▼
        Send SIGKILL
            │
            ▼
        Force wait
            │
            ▼
        Cleanup PID file
        return Ok(())
```

## State Machine

```
┌─────────┐     new()      ┌─────────┐
│  None   │───────────────▶│ Stopped │
└─────────┘                └────┬────┘
                                │
                         start()│
                                ▼
                          ┌──────────┐
                          │ Starting │
                          └─────┬────┘
                                │
                       success  │  failure
                      ┌─────────┼─────────┐
                      ▼                   ▼
                 ┌─────────┐        ┌─────────┐
                 │ Running │        │ Stopped │
                 └────┬────┘        └─────────┘
                      │
              stop()  │  restart()
             ┌────────┼────────┐
             ▼                 ▼
        ┌──────────┐      ┌──────────┐
        │ Stopping │      │ Starting │
        └─────┬────┘      └──────────┘
              │
              ▼
         ┌─────────┐
         │ Stopped │
         └─────────┘
```

## File System Layout

```
~/.ryht/cortex/
│
├── data/
│   └── surrealdb/          # Database storage
│       ├── db_store/       # RocksDB files
│       ├── LOG             # RocksDB logs
│       └── MANIFEST-*      # RocksDB manifest
│
├── logs/
│   └── surrealdb.log       # Server output
│       │
│       ├─► Startup messages
│       ├─► Query logs
│       ├─► Error messages
│       └─► Shutdown messages
│
└── run/
    └── surrealdb.pid       # Process ID
        │
        └─► Format: "<PID>\n"
            Example: "12345\n"
```

## Process Lifecycle

```
┌─────────────────────────────────────────────────────┐
│                    Parent Process                    │
│                  (Cortex Manager)                    │
│                                                      │
│  1. Command::new("surreal")                         │
│     .arg("start")                                   │
│     .arg("--bind").arg("127.0.0.1:8000")           │
│     .arg("--user").arg("cortex")                    │
│     .arg("--pass").arg("cortex")                    │
│     .arg("rocksdb://~/.ryht/cortex/data/surrealdb")│
│     .stdout(log_file)                               │
│     .stderr(log_file)                               │
│     .spawn()                                        │
│                                                      │
│  2. Store process handle                            │
│  3. Write PID to file                               │
│  4. Poll health endpoint                            │
└──────────────────┬──────────────────────────────────┘
                   │ spawns
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│                   Child Process                      │
│                (SurrealDB Server)                    │
│                                                      │
│  • Binds to 127.0.0.1:8000                          │
│  • Opens RocksDB database                           │
│  • Starts HTTP server                               │
│  • Handles authentication                           │
│  • Processes queries                                │
│  • Writes to log file                               │
│                                                      │
│  Signals:                                           │
│   SIGTERM → Graceful shutdown                       │
│   SIGKILL → Immediate termination                   │
└─────────────────────────────────────────────────────┘
```

## Error Handling Flow

```
Operation Attempt
    │
    ▼
Try execute
    │
    ├─► Success ──────────────────────► Return Ok(result)
    │
    └─► Failure
        │
        ▼
    Check error type
        │
        ├─► Transient error (network, timeout)
        │   │
        │   ▼
        │   Retry counter < max_retries?
        │   │
        │   ├─► Yes ──► Sleep 2s ──► Retry
        │   │
        │   └─► No ──► Return Err(...)
        │
        └─► Fatal error (config, permission)
            │
            ▼
            Log error details
            │
            ▼
            Return Err(CortexError::...)
```

## Concurrency Model

```
┌────────────────────────────────────────┐
│         Async Runtime (Tokio)          │
└───┬────────────────────────────────┬───┘
    │                                │
    ▼                                ▼
┌─────────────┐              ┌─────────────┐
│ Manager API │              │   Health    │
│   Calls     │              │   Checks    │
│             │              │             │
│ • start()   │              │ • Polling   │
│ • stop()    │              │ • Retry     │
│ • restart() │              │ • Timeout   │
└──────┬──────┘              └──────┬──────┘
       │                            │
       └────────────┬───────────────┘
                    │
                    ▼
         ┌──────────────────┐
         │   Process State  │
         │                  │
         │ • process handle │
         │ • status enum    │
         │ • config         │
         └──────────────────┘

Note: No Arc<Mutex<...>> needed because:
- Manager owns the process handle
- Operations are sequential via &mut self
- Status changes are controlled
```

## Testing Architecture

```
┌─────────────────────────────────────────┐
│          Test Categories                │
├─────────────────────────────────────────┤
│                                         │
│ 1. Unit Tests (in module)              │
│    • Config validation                  │
│    • Builder pattern                    │
│    • Default values                     │
│    • Path construction                  │
│                                         │
│ 2. Integration Tests (separate file)   │
│    • Server lifecycle                   │
│    • Health checks                      │
│    • PID management                     │
│    • Concurrent operations              │
│                                         │
│ 3. Test Environment                     │
│    • TempDir for data                   │
│    • Custom port (19000)                │
│    • Memory storage                     │
│    • Isolated from production           │
└─────────────────────────────────────────┘
```

## Security Architecture

```
┌─────────────────────────────────────────┐
│         Security Layers                 │
├─────────────────────────────────────────┤
│                                         │
│ 1. Network Security                     │
│    • Localhost binding only             │
│    • No external access                 │
│    • Configurable firewall rules        │
│                                         │
│ 2. Authentication                       │
│    • Required by default                │
│    • Username/password                  │
│    • No guest access                    │
│                                         │
│ 3. File System                          │
│    • Restricted directories             │
│    • PID file permissions               │
│    • Log file permissions               │
│                                         │
│ 4. Process Isolation                    │
│    • Runs as user                       │
│    • No privilege escalation            │
│    • Clean shutdown                     │
└─────────────────────────────────────────┘
```

## Performance Characteristics

```
Operation           Latency         Notes
────────────────────────────────────────────────
find_binary()       < 100ms         Filesystem search
install()           2-10s           Network dependent
start()             1-5s            Backend dependent
health_check()      10-100ms        HTTP roundtrip
is_running()        < 10ms          PID check
stop()              100ms-2s        Graceful shutdown
restart()           2-7s            stop + start
wait_for_ready()    1-30s           Configurable timeout

Memory Usage:
- Manager struct: ~1KB
- Process handle: ~100 bytes
- Total: < 10KB (excluding server process)

Server Process:
- Memory: 50-200 MB
- Startup: 1-5 seconds
- Shutdown: < 2 seconds
```

## Extension Points

```
┌─────────────────────────────────────────┐
│      Future Extension Areas             │
├─────────────────────────────────────────┤
│                                         │
│ 1. Additional Storage Backends          │
│    • TiKV support                       │
│    • IndxDB support                     │
│    • Custom backends                    │
│                                         │
│ 2. Clustering                           │
│    • Multi-node coordination            │
│    • Raft consensus                     │
│    • Distributed configuration          │
│                                         │
│ 3. Monitoring                           │
│    • Prometheus metrics                 │
│    • Performance tracking               │
│    • Resource monitoring                │
│                                         │
│ 4. Service Management                   │
│    • Systemd integration                │
│    • Windows service                    │
│    • Docker containerization            │
└─────────────────────────────────────────┘
```

This architecture provides a robust, maintainable, and extensible foundation for SurrealDB management within the Cortex ecosystem.
