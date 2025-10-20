# Cortex Project Structure

## Overview

Production-ready Rust workspace for the Cortex cognitive memory system with proper separation of concerns.

## Directory Structure

```
cortex/
├── Cargo.toml                 # Workspace configuration
├── rust-toolchain.toml        # Rust toolchain specification
├── README.md                  # Project documentation
├── .gitignore                # Git ignore rules
│
├── cortex-core/              # Core types and abstractions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root with prelude
│       ├── error.rs          # Error types and Result
│       ├── id.rs             # CortexId (UUID-based)
│       ├── types.rs          # Core domain types
│       ├── traits.rs         # Core traits (Storage, Memory, etc.)
│       └── metadata.rs       # Metadata utilities
│
├── cortex-storage/           # SurrealDB storage layer
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root
│       ├── connection.rs     # Connection configuration
│       ├── pool.rs           # Connection pooling
│       ├── query.rs          # Query builder
│       ├── schema.rs         # Database schema
│       └── surreal.rs        # SurrealDB implementation
│
├── cortex-vfs/               # Virtual filesystem
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root
│       ├── vfs.rs            # VFS implementation
│       ├── watcher.rs        # File watching
│       ├── cache.rs          # Caching layer
│       └── dedup.rs          # Deduplication
│
├── cortex-ingestion/         # Document ingestion
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root
│       ├── ingester.rs       # Document ingester
│       ├── chunker.rs        # Text chunking
│       ├── extractor.rs      # Metadata extraction
│       └── filters.rs        # File filtering
│
├── cortex-memory/            # Cognitive memory systems
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root
│       ├── episodic.rs       # Episodic memory
│       ├── semantic.rs       # Semantic memory
│       ├── working.rs        # Working memory
│       └── consolidation.rs # Memory consolidation
│
├── cortex-mcp/               # MCP server
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Library root
│       ├── server.rs         # HTTP server
│       ├── handlers.rs       # HTTP handlers
│       ├── tools.rs          # Tool definitions
│       └── types.rs          # MCP types
│
└── cortex-cli/               # CLI interface
    ├── Cargo.toml
    └── src/
        ├── main.rs           # CLI entry point
        ├── lib.rs            # Library root
        ├── commands.rs       # Command implementations
        └── config.rs         # Configuration management
```

## Crate Dependencies

```
cortex-cli
├── cortex-core
├── cortex-storage
├── cortex-vfs
├── cortex-ingestion
├── cortex-memory
└── cortex-mcp

cortex-mcp
├── cortex-core
├── cortex-storage
├── cortex-vfs
└── cortex-memory

cortex-memory
├── cortex-core
└── cortex-storage

cortex-ingestion
├── cortex-core
├── cortex-storage
└── cortex-vfs

cortex-vfs
├── cortex-core
└── cortex-storage

cortex-storage
└── cortex-core

cortex-core
└── (no internal dependencies)
```

## Key Features

### cortex-core
- Comprehensive error types with context
- UUID-based entity identifiers
- Domain types (Project, Document, Chunk, Episode, etc.)
- Traits for storage, memory, ingestion
- Metadata utilities

### cortex-storage
- SurrealDB connection pooling
- Configurable backends (memory, RocksDB, remote)
- Query builder utilities
- Complete schema definitions
- Full CRUD implementations

### cortex-vfs
- Virtual filesystem abstraction
- Content-based deduplication
- LRU cache with TTL
- File watching with notify
- Efficient I/O operations

### cortex-ingestion
- Multi-format document ingestion
- Semantic text chunking
- Metadata extraction
- File filtering and ignore patterns
- Parallel processing support

### cortex-memory
- Episodic memory (experiences)
- Semantic memory (facts)
- Working memory (temporary)
- Memory consolidation
- Importance-based retention

### cortex-mcp
- HTTP-based MCP server
- Tool definitions for LLMs
- Async request handling
- CORS support
- Error handling

### cortex-cli
- Comprehensive CLI with clap
- Project initialization
- Document ingestion
- Search capabilities
- Configuration management
- MCP server control

## Build Commands

```bash
# Check entire workspace
cargo check --workspace

# Build release
cargo build --release --workspace

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --workspace

# Build documentation
cargo doc --workspace --no-deps --open

# Run the CLI
cargo run --bin cortex -- --help

# Run specific tests
cargo test -p cortex-core
cargo test -p cortex-storage
```

## Production Considerations

### Error Handling
- Custom `CortexError` enum with context
- Consistent `Result<T>` type alias
- Detailed error messages
- Error categorization

### Logging
- Structured logging with tracing
- Configurable log levels
- JSON output support
- Per-module filtering

### Performance
- Connection pooling
- Content deduplication
- Caching layers
- Parallel processing
- Optimized release builds

### Testing
- Unit tests in each module
- Integration tests
- Test fixtures with tempfile
- Mock support with mockall

### Dependencies
- All workspace dependencies centralized
- Version constraints specified
- Feature flags controlled
- Minimal dependency tree

## Next Steps

1. Implement embedding generation (ort integration)
2. Add semantic search capabilities
3. Implement memory consolidation algorithms
4. Complete MCP tool implementations
5. Add CLI command handlers
6. Set up CI/CD pipeline
7. Add benchmarks
8. Write integration tests
9. Add examples
10. Performance profiling

## Configuration

Default configuration is in `cortex-cli/src/config.rs`. Override with:
- Environment variables
- TOML configuration file
- CLI arguments

## License

MIT OR Apache-2.0
