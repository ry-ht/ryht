# Cortex Quick Start Guide

## Prerequisites

- Rust 1.75 or later
- Git (for version control)

## Installation

```bash
# Navigate to the project
cd /Users/taaliman/projects/luxquant/ry-ht/ryht/cortex

# Build the project
cargo build --release

# Run tests
cargo test --workspace

# Install the CLI
cargo install --path cortex-cli
```

## Using Make

```bash
# Check all commands
make help

# Quick development cycle
make check      # Verify compilation
make test       # Run tests
make clippy     # Lint
make fmt        # Format code

# Build and run
make build      # Debug build
make release    # Optimized build
make run        # Run CLI
```

## Basic Usage

### Initialize a Project

```bash
cortex init my-project --path /path/to/code
```

### Ingest Documents

```bash
cortex ingest my-project /path/to/code
```

### Search

```bash
cortex search "function definition" --project my-project
```

### Start MCP Server

```bash
cortex serve --addr 127.0.0.1:3000
```

### View Statistics

```bash
cortex stats
```

## Configuration

Create a configuration file at `~/.cortex/config.toml`:

```toml
[database]
connection_string = "rocksdb://~/.cortex/db"
namespace = "cortex"
database = "main"
pool_size = 10

[storage]
data_dir = "~/.cortex/data"
cache_size_mb = 1024

[mcp]
enabled = true
address = "127.0.0.1"
port = 3000
```

## Development

### Project Structure

```
cortex/
├── cortex-core/         # Core types and traits
├── cortex-storage/      # SurrealDB storage
├── cortex-vfs/          # Virtual filesystem
├── cortex-ingestion/    # Document processing
├── cortex-memory/       # Memory systems
├── cortex-mcp/          # MCP server
└── cortex-cli/          # CLI interface
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p cortex-core
cargo test -p cortex-storage

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name
```

### Generating Documentation

```bash
# Generate and open docs
cargo doc --workspace --no-deps --open

# Or use make
make doc
```

### Code Quality

```bash
# Format code
cargo fmt --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Or use make
make fmt
make clippy
```

## Architecture Overview

### Data Flow

```
Files → Ingestion → Storage (SurrealDB)
                         ↓
                    VFS + Cache
                         ↓
                    Memory Systems
                         ↓
                    MCP Server → LLMs
```

### Core Components

1. **cortex-core**: Shared types, errors, traits
2. **cortex-storage**: Database layer with pooling
3. **cortex-vfs**: Virtual filesystem with dedup
4. **cortex-ingestion**: File processing and chunking
5. **cortex-memory**: Episodic, semantic, working memory
6. **cortex-mcp**: HTTP server for LLM tools
7. **cortex-cli**: Command-line interface

### Key Features

- **Multi-level memory**: Episodic, semantic, working
- **Content deduplication**: Save space with hashing
- **Connection pooling**: Efficient database access
- **File watching**: Real-time change detection
- **Semantic chunking**: Intelligent text splitting
- **MCP protocol**: LLM integration ready

## Troubleshooting

### Build Errors

If you encounter build errors:

```bash
# Clean and rebuild
cargo clean
cargo build --release

# Update dependencies
cargo update
```

### Database Issues

For database connection problems:

```bash
# Use in-memory database for testing
export CORTEX_DB_MODE=memory

# Or specify RocksDB path
export CORTEX_DB_PATH=~/.cortex/db
```

### Logging

Enable verbose logging:

```bash
# CLI
cortex --verbose <command>

# Environment variable
export RUST_LOG=cortex=debug,info
```

## Next Steps

1. Read the [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for detailed architecture
2. Review [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for current state
3. Check the [README.md](README.md) for project overview
4. Explore the source code starting with `cortex-core`
5. Run the test suite to understand behavior
6. Add your own functionality

## Resources

- Documentation: `cargo doc --open`
- Tests: `cargo test --workspace`
- Examples: (coming soon)
- API docs: (generate with cargo doc)

## Getting Help

1. Check the documentation
2. Read the source code (well-commented)
3. Run tests to see examples
4. Review the architecture diagrams

Happy coding! 🦀
