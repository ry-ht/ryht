# ry.ht - Quick Start Guide

## 🚀 What is ry.ht?

**ry.ht** (rhythm + thought) is a neural-inspired platform consisting of two main systems:

- **⚡ Axon** - Multi-agent orchestration with GUI dashboard
- **🧠 Cortex** - Cognitive memory system with semantic search

## 📦 Prerequisites

### For Axon (GUI app):
- **Node.js** 18+ or **Bun** 1.0+
- **Rust** 1.75+ ([rustup.rs](https://rustup.rs))
- **Tauri dependencies**:
  - macOS: Xcode Command Line Tools
  - Linux: `webkit2gtk`, `libgtk-3-dev`, etc.
  - Windows: Visual Studio Build Tools

### For Cortex (Memory system):
- **Rust** 1.75+

## ⚡ Getting Started with Axon

### Development Mode

```bash
cd axon
npm install  # or: bun install
npm run tauri dev
```

The GUI will launch automatically with hot reload enabled.

### Production Build

```bash
cd axon
npm run build
npm run tauri build
```

Executables will be in `axon/src-tauri/target/release/`

### Web Server Mode

Run Axon as a headless web server:

```bash
cd axon/src-tauri
cargo run --bin axon-web -- --port 8080
```

Access at: http://localhost:8080

## 🧠 Getting Started with Cortex

### CLI Mode

```bash
cd cortex
cargo run -- --db-path ./cortex.db --index-path ./cortex_index
```

### Server Mode

```bash
cd cortex
cargo run -- --db-path ./cortex.db --server --port 8081
```

Access API at: http://localhost:8081

## 🔧 Development

### Build Entire Workspace

```bash
cargo build --workspace
```

### Run Tests

```bash
cargo test --workspace
```

### Check Code Quality

```bash
cargo clippy --workspace
cargo fmt --check
```

## 📁 Project Structure

```
ryht/
├── axon/              # Multi-agent GUI app (Tauri + React)
├── cortex/            # Cognitive memory system (Rust)
├── crates/            # Shared libraries
│   ├── common/        # Common utilities
│   └── types/         # Shared types
├── docs/              # Documentation
└── experiments/       # Experimental projects (gitignored)
```

## 🎯 Common Commands

### Axon
```bash
# Development
npm run dev

# Build GUI
npm run tauri build

# Run web server
cargo run --bin axon-web
```

### Cortex
```bash
# CLI mode
cargo run

# Server mode
cargo run -- --server

# With custom paths
cargo run -- --db-path /path/to/db --index-path /path/to/index
```

## 🔗 Integration

To connect Axon with Cortex, configure Axon to use Cortex's API endpoint:

```bash
# Start Cortex server
cd cortex
cargo run -- --server --port 8081

# In another terminal, start Axon
cd axon
npm run tauri dev
```

Configure in Axon settings:
- Memory API URL: `http://localhost:8081`

## 📚 Next Steps

- Read [ARCHITECTURE.md](./ARCHITECTURE.md) for system design
- Check [README.md](./README.md) for detailed features
- Explore individual project READMEs:
  - [axon/README.md](./axon/README.md)
  - cortex/README.md (TODO)

## 🐛 Troubleshooting

### Axon won't start
- Ensure Node.js/Bun and Rust are installed
- Install Tauri dependencies for your OS
- Check `npm install` completed successfully

### Cortex database errors
- Ensure the database path directory exists
- Check file permissions
- Delete corrupted database files and restart

### Build errors
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`
- Check Node modules: `rm -rf node_modules && npm install`

## 🌐 Links

- **Domain:** [ry.ht](https://ry.ht)
- **Repository:** GitHub (TBD)
- **Issues:** GitHub Issues (TBD)

---

**Built with Rust 🦀 | Powered by Tauri ⚡ | Inspired by neuroscience 🧠**
