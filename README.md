# ry.ht

> A cyberpunk cognitive infrastructure: Multi-agent orchestration meets neural memory architecture

## 🔷 Overview

**ry.ht** is a dual-system platform combining intelligent agent coordination with cognitive memory storage. Built in Rust for performance and reliability.

### Projects

#### ⚡ **Axon** - Multi-Agent Orchestration System
A neural-inspired agent coordination platform with real-time dashboard. Built on Tauri with React frontend and Rust backend for maximum performance.

**Features:**
- Dynamic agent registration and lifecycle management
- Modern GUI dashboard for agent monitoring
- Async task coordination and event propagation
- RESTful API and WebSocket support
- Cross-platform desktop application (macOS, Linux, Windows)

**Location:** `axon/`

#### 🧠 **Cortex** - Cognitive Memory System
A neural-inspired memory architecture with semantic understanding, temporal context, and associative retrieval.

**Features:**
- Multiple memory types (short-term, episodic, semantic, procedural)
- Full-text search with semantic indexing
- Memory salience and retention modeling
- Associative memory networks

**Location:** `cortex/`

### Shared Libraries

- **`ryht-common`**: Shared utilities, error types, and configuration
- **`ryht-types`**: Common type definitions and data structures

**Location:** `crates/`

## 🚀 Quick Start

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs/))

### Build All Projects
```bash
cargo build --release
```

### Run Axon (Multi-Agent Dashboard)
```bash
cd axon
npm install  # or bun install
npm run tauri dev
```

GUI will launch automatically

### Run Cortex (Memory System)
```bash
cd cortex
cargo run -- --db-path ./cortex.db --server --port 8081
```

API: http://localhost:8081

## 📁 Project Structure

```
ryht/
├── Cargo.toml              # Workspace configuration
├── .gitignore
├── README.md
│
├── axon/                   # Multi-agent system (Tauri app)
│   ├── package.json
│   ├── src/                # React frontend
│   └── src-tauri/          # Rust backend
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── web_main.rs # Web server mode
│
├── cortex/                 # Cognitive memory
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── memory.rs       # Memory structures
│       ├── storage.rs      # Persistence layer
│       ├── indexing.rs     # Search indexing
│       └── retrieval.rs    # Query strategies
│
└── crates/                 # Shared libraries
    ├── common/             # Common utilities
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── config.rs
    │       ├── error.rs
    │       └── utils.rs
    │
    └── types/              # Shared types
        ├── Cargo.toml
        └── src/
            └── lib.rs
```

## 🛠️ Development

### Run Tests
```bash
cargo test --workspace
```

### Check Code
```bash
cargo clippy --workspace
cargo fmt --check
```

### Build Documentation
```bash
cargo doc --workspace --open
```

## 🎯 Roadmap

### Axon
- [ ] Agent communication protocols
- [ ] Distributed agent deployment
- [ ] Advanced scheduling algorithms
- [ ] Real-time metrics and telemetry
- [ ] Enhanced WebSocket support for live updates
- [ ] Plugin system for custom agents

### Cortex
- [ ] Vector embeddings for semantic search
- [ ] Memory consolidation algorithms
- [ ] GraphQL API
- [ ] Memory decay and forgetting curves
- [ ] Multi-modal memory (text, images, audio)

### Integration
- [ ] Axon ↔ Cortex bridge
- [ ] Agents with persistent memory via Cortex
- [ ] Shared authentication layer
- [ ] Unified monitoring dashboard
- [ ] Inter-process communication optimizations

## 📜 License

MIT OR Apache-2.0

## 🌐 Domain

**ry.ht** - Where rhythm meets thought

---

*Built with Rust 🦀 | Powered by async/await | Inspired by neuroscience*
