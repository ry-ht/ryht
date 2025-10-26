# Cortex - Cognitive Memory System

A production-ready cognitive memory system for code intelligence and knowledge management.

## Architecture

Cortex is organized as a Rust workspace with multiple crates:

### Core Crates

- **cortex-core**: Core types, traits, and abstractions
- **cortex-storage**: SurrealDB storage layer with connection pooling
- **cortex-vfs**: Virtual filesystem with caching and deduplication
- **cortex-ingestion**: Document ingestion and processing
- **cortex-memory**: Cognitive memory systems (episodic, semantic, working)
- **cortex-mcp**: MCP server for LLM integration
- **cortex**: Command-line interface

## Features

- Multi-level memory architecture (episodic, semantic, working)
- Virtual filesystem with content deduplication
- SurrealDB for scalable storage
- MCP protocol support for LLM integration
- Production-ready error handling and logging
- Comprehensive test coverage

## Quick Start

```bash
# Build the project
cargo build --release

# Run the CLI
cargo run --bin cortex -- --help

# Run tests
cargo test

# Start the MCP server
cargo run --bin cortex -- serve
```

## Configuration

Configuration is managed through TOML files. See `cortex/src/config.rs` for details.

## Development

```bash
# Check code
cargo check --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt --workspace
```

## License

MIT OR Apache-2.0
