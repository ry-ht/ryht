# Axon CLI Architecture

## Overview

The Axon CLI provides a comprehensive command-line interface for managing multi-agent systems, following the architectural patterns established in Cortex CLI. It includes agent lifecycle management, workflow orchestration, REST API server, and monitoring capabilities.

## Architecture Components

### 1. Entry Point (`src/main.rs`)

**Purpose**: CLI parsing and command routing

**Key Features**:
- Clap-based CLI with comprehensive command structure
- Output format support (human, json, plain)
- Global flags (verbose, config, format)
- Tokio async runtime
- Logging initialization

**Command Structure**:
```
axon
├── init                 # Initialize workspace
├── agent                # Agent management
│   ├── start           # Start an agent
│   ├── stop            # Stop an agent
│   ├── list            # List agents
│   ├── info            # Get agent info
│   ├── pause           # Pause agent
│   ├── resume          # Resume agent
│   └── logs            # View agent logs
├── workflow            # Workflow orchestration
│   ├── run             # Run workflow
│   ├── list            # List workflows
│   ├── status          # Get workflow status
│   ├── cancel          # Cancel workflow
│   └── validate        # Validate workflow
├── server              # REST API server
│   ├── start           # Start server
│   ├── stop            # Stop server
│   └── status          # Check server status
├── status              # System status
├── config              # Configuration management
│   ├── get             # Get config value
│   ├── set             # Set config value
│   └── list            # List all config
├── monitor             # Monitoring
│   ├── dashboard       # Show dashboard
│   ├── metrics         # Show metrics
│   └── telemetry       # Show telemetry
├── doctor              # Diagnostics
│   ├── check           # Run diagnostics
│   └── health          # Quick health check
├── export              # Export data
│   ├── metrics         # Export metrics
│   └── workflows       # Export workflows
└── interactive         # Interactive mode
```

### 2. Command Handlers (`src/commands.rs`)

**Purpose**: Implementation of all CLI commands

**Key Modules**:
- `config` - Configuration management
- `output` - Output formatting utilities
- `runtime_manager` - Agent runtime management
- `server_manager` - REST API server lifecycle
- `api` - REST API server implementation

**Design Pattern**: Each command is implemented as an async function that:
1. Shows progress with spinners/progress bars
2. Loads configuration
3. Executes the operation via appropriate manager
4. Provides formatted output
5. Handles errors gracefully

### 3. Configuration (`src/commands/config.rs`)

**Purpose**: Configuration loading, saving, and management

**Configuration Structure**:
```toml
[workspace]
workspace_name = "my-agents"
workspace_path = "."

[server]
host = "127.0.0.1"
port = 9090
workers = 4

[runtime]
max_agents = 10
agent_timeout_seconds = 300
task_queue_size = 100
enable_auto_recovery = true

[cortex]
enabled = true
mcp_server_url = "stdio://cortex"
workspace = "default"
```

**Configuration Locations**:
1. Local: `.axon/config/workspace.toml` (workspace-specific)
2. Global: `~/.ryht/axon/config.toml` (user-wide defaults)

### 4. Output Formatting (`src/commands/output.rs`)

**Purpose**: Consistent, beautiful CLI output

**Features**:
- Progress spinners with indicatif
- Color-coded messages (success, error, warning, info)
- Formatted tables with comfy-table
- Multiple output formats (human, json, plain)
- Unicode emojis for better UX

**Components**:
- `spinner()` - Progress indicators
- `success()`, `error()`, `warn()`, `info()` - Status messages
- `print_agent_table()` - Agent listing
- `print_workflow_table()` - Workflow listing
- `print_metrics_table()` - Metrics display

### 5. Runtime Manager (`src/commands/runtime_manager.rs`)

**Purpose**: Agent lifecycle and execution management

**Responsibilities**:
- Start/stop agents
- Pause/resume agents
- List and query agents
- Execute workflows
- Collect metrics and telemetry
- Export data

**Directory Structure**:
```
~/.ryht/axon/
├── agents/          # Agent state
├── workflows/       # Workflow definitions
└── logs/            # Agent logs

.axon/
├── config/          # Workspace config
├── agents/          # Local agent configs
├── workflows/       # Workflow files
└── logs/            # Agent logs
```

**Agent Management**:
- Agents stored in HashMap with AgentInfo
- Async operations for all agent management
- Integration with agent types from `axon::agents`

### 6. Server Manager (`src/commands/server_manager.rs`)

**Purpose**: REST API server process lifecycle

**Design Pattern**: Similar to Cortex's `ServerManager`

**Features**:
- Background process spawning
- PID file management
- Health check polling
- Graceful shutdown (SIGTERM on Unix, taskkill on Windows)
- Process monitoring
- Automatic startup timeout

**Process Management**:
1. Spawn server as detached process
2. Write PID to `~/.ryht/axon/api-server/api-server.pid`
3. Redirect logs to `~/.ryht/axon/api-server/logs/api-server.log`
4. Poll health endpoint until ready
5. Support stop/status operations

### 7. REST API Server (`src/commands/api/`)

**Purpose**: HTTP API for dashboard and inter-agent communication

**Architecture**:

```
api/
├── mod.rs          # Module exports
├── server.rs       # Server initialization
├── routes.rs       # Route handlers
├── middleware.rs   # Request logging
└── error.rs        # Error types
```

**Endpoints**:

```
GET  /health                    # Health check
GET  /agents                    # List agents
POST /agents                    # Create agent
GET  /agents/:id                # Get agent
DELETE /agents/:id              # Stop agent
POST /agents/:id/pause          # Pause agent
POST /agents/:id/resume         # Resume agent
GET  /workflows                 # List workflows
POST /workflows                 # Run workflow
GET  /workflows/:id             # Get workflow status
POST /workflows/:id/cancel      # Cancel workflow
GET  /metrics                   # Get metrics
GET  /telemetry                 # Get telemetry
GET  /status                    # System status
```

**Technology Stack**:
- **Axum**: Web framework
- **Tower**: Middleware
- **Tower-HTTP**: CORS and tracing
- **Tokio**: Async runtime

**Features**:
- CORS enabled for browser access
- Request/response logging
- JSON API with proper error handling
- Shared state via Arc<RwLock<RuntimeManager>>

## Integration with Axon

### Agent Types

The CLI integrates with existing agent types:
- Orchestrator
- Developer
- Reviewer
- Tester
- Documenter
- Architect
- Researcher
- Optimizer

### Capabilities

Agents have capabilities defined in `axon::agents::Capability`:
- CodeGeneration, CodeReview, Testing, Documentation
- ArchitectureDesign, Research, Optimization
- And many more specialized capabilities

### Cortex Integration

Via `axon::cortex_bridge`:
- MCP communication with Cortex
- Memory and context management
- Tool execution via Cortex stdio

## Production Features

### Error Handling

- `anyhow::Result` for command functions
- Proper error context with `.context()`
- User-friendly error messages
- JSON error responses in API

### Logging

- `tracing` for structured logging
- File-based logging for server
- Configurable log levels
- Request/response logging in API

### Configuration Management

- TOML-based configuration
- Multiple config sources (local, global)
- Environment variable support
- Validation and defaults

### Process Management

- PID file tracking
- Process health monitoring
- Graceful shutdown
- Platform-specific implementations (Unix/Windows)

### Security

- No hardcoded credentials
- Local-only server by default (127.0.0.1)
- Proper file permissions
- Input validation

## Usage Examples

### Initialize Workspace

```bash
axon init my-agents
cd my-agents
```

### Start Agents

```bash
# Start an orchestrator
axon agent start orchestrator --name main --max-tasks 5

# Start workers with specific capabilities
axon agent start developer --name dev-1 --capabilities coding,testing
axon agent start reviewer --name reviewer-1 --capabilities review
```

### List Agents

```bash
# Human-readable output
axon agent list

# JSON output
axon agent list --format json

# Detailed view
axon agent list --detailed
```

### Run Workflows

```bash
# Run a workflow
axon workflow run workflow.yaml --input '{"repo": "..."}'

# Validate without running
axon workflow validate workflow.yaml

# Check status
axon workflow status <workflow-id>
```

### Start REST API Server

```bash
# Start server
axon server start --port 9090

# Check status
axon server status

# Stop server
axon server stop
```

### Monitor System

```bash
# Show system status
axon status --detailed

# Show metrics
axon monitor metrics

# Export metrics
axon export metrics --output metrics.json
```

### Configuration

```bash
# Get configuration
axon config get server.port

# Set configuration
axon config set server.port 8080

# List all configuration
axon config list
```

### Diagnostics

```bash
# Run health check
axon doctor health

# Full diagnostics
axon doctor check --fix
```

## Development

### Building

```bash
cargo build --bin axon
```

### Running

```bash
cargo run --bin axon -- [command]
```

### Testing

```bash
# Check compilation
cargo check --bin axon

# Run tests
cargo test

# Run specific binary
./target/debug/axon --help
```

## Dependencies

### CLI Dependencies
- `clap` - Command-line argument parsing
- `console` - Terminal colors and formatting
- `indicatif` - Progress bars and spinners
- `dialoguer` - Interactive prompts
- `comfy-table` - Table formatting

### API Dependencies
- `axum` - Web framework
- `tower` - Middleware
- `tower-http` - HTTP middleware (CORS, tracing)
- `tokio` - Async runtime

### Configuration
- `toml` - TOML parsing
- `serde` - Serialization

### Utilities
- `anyhow` - Error handling
- `tracing` - Logging
- `dirs` - Directory paths
- `uuid` - Unique identifiers
- `chrono` - Date/time handling

### Platform-Specific
- `nix` (Unix) - Process signals

## Future Enhancements

### Planned Features

1. **WebSocket Support**: Real-time agent updates
2. **Dashboard UI**: Web-based monitoring
3. **Workflow DSL**: Enhanced workflow definitions
4. **Agent Pools**: Managed agent pools with auto-scaling
5. **Distributed Mode**: Multi-node agent orchestration
6. **Metrics Export**: Prometheus/Grafana integration
7. **Authentication**: API key/JWT support
8. **Rate Limiting**: Request throttling
9. **TUI**: Terminal UI for monitoring
10. **Plugin System**: Extensible agent types

### Architecture Improvements

1. **Service Layer**: Shared business logic between CLI and API
2. **Event System**: Event-driven agent communication
3. **State Machine**: Formal agent state transitions
4. **Resource Pool**: Shared resource management
5. **Circuit Breaker**: Fault tolerance patterns

## Comparison with Cortex

### Similarities
- CLI structure and command organization
- Server manager pattern for REST API
- Configuration management approach
- Output formatting utilities
- Error handling patterns

### Differences
- **Focus**: Agents vs Memory
- **Runtime**: Agent execution vs Data storage
- **API**: Agent management vs Memory queries
- **Integration**: Cortex as MCP provider for agents

## Architecture Principles

1. **Separation of Concerns**: CLI, commands, runtime, API
2. **Async First**: All I/O operations are async
3. **Error Propagation**: Use Result types consistently
4. **Configuration**: Multiple sources with proper precedence
5. **Logging**: Structured logging for debugging
6. **User Experience**: Progress indicators and formatted output
7. **Production Ready**: Proper error handling and process management
8. **Extensibility**: Easy to add new commands and features

## File Structure

```
axon/
├── src/
│   ├── main.rs                    # CLI entry point
│   ├── commands.rs                # Command implementations
│   ├── commands/
│   │   ├── mod.rs                 # Module exports
│   │   ├── config.rs              # Configuration management
│   │   ├── output.rs              # Output formatting
│   │   ├── runtime_manager.rs     # Agent runtime
│   │   ├── server_manager.rs      # Server lifecycle
│   │   └── api/
│   │       ├── mod.rs             # API module
│   │       ├── server.rs          # Server initialization
│   │       ├── routes.rs          # Route handlers
│   │       ├── middleware.rs      # Request logging
│   │       └── error.rs           # Error types
│   ├── agents/                    # Agent implementations
│   ├── orchestration/             # Workflow orchestration
│   ├── coordination/              # Agent coordination
│   └── lib.rs                     # Library exports
├── Cargo.toml                     # Dependencies and binary config
└── CLI_ARCHITECTURE.md            # This document
```

## Conclusion

The Axon CLI provides a production-ready interface for managing multi-agent systems, with comprehensive features for agent lifecycle management, workflow orchestration, monitoring, and configuration. The architecture follows best practices from Cortex while being tailored for agent-specific operations.
