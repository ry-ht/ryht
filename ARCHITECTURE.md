# ry.ht - Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                          ry.ht                              │
│                 Cognitive Infrastructure                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              │
                ┌─────────────┴─────────────┐
                │                           │
                ▼                           ▼
    ┌───────────────────┐       ┌───────────────────┐
    │   ⚡ AXON        │       │   🧠 CORTEX      │
    │   Multi-Agent     │◀─────▶│   Cognitive      │
    │   Orchestration   │       │   Memory         │
    └───────────────────┘       └───────────────────┘
            │                           │
            │                           │
            ▼                           ▼
    ┌───────────────────┐       ┌───────────────────┐
    │   Dashboard       │       │   Storage         │
    │   - Tauri GUI     │       │   - SQLite DB     │
    │   - React UI      │       │   - Search Index  │
    │   - WebSocket     │       │   - Associations  │
    └───────────────────┘       └───────────────────┘
            │                           │
            │                           │
            └────────────┬──────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   Shared Crates     │
              │                     │
              │  • ryht-common      │
              │  • ryht-types       │
              └─────────────────────┘
```

## System Components

### ⚡ Axon - Multi-Agent System

**Purpose:** Neural-inspired agent coordination platform with GUI dashboard

**Core Components:**
- **Agent Runtime:** Dynamic agent lifecycle management
- **Orchestrator:** Central coordination hub
- **Dashboard:** Tauri-based desktop GUI with React
- **API Layer:** RESTful endpoints + WebSocket support
- **Web Server Mode:** Optional headless mode for server deployment

**Tech Stack:**
- Tauri 2 (desktop framework)
- React + TypeScript (frontend)
- Tokio async runtime (backend)
- Axum web framework
- SQLite for state persistence

**Deployment:**
- Desktop app (macOS, Linux, Windows)
- Web server mode (`axon-web` binary)

### 🧠 Cortex - Cognitive Memory

**Purpose:** Neural-inspired memory architecture with semantic search

**Core Components:**
- **Memory Store:** Multi-type memory system
  - Short-term (working memory)
  - Episodic (events, experiences)
  - Semantic (facts, concepts)
  - Procedural (skills, processes)
- **Search Engine:** Full-text + semantic indexing
- **Association Graph:** Memory relationship network
- **Retrieval System:** Context-aware query strategies

**Tech Stack:**
- SQLite for persistence
- Tantivy for full-text search
- Custom semantic indexing
- Tokio async runtime

**Port:** 8081 (server mode)

### 📦 Shared Libraries

#### ryht-common
Common utilities, error handling, configuration management

#### ryht-types
Shared type definitions, serialization formats, data structures

## Communication Patterns

```
┌─────────┐         ┌─────────┐         ┌─────────┐
│  Agent  │────────▶│  Axon   │────────▶│ Cortex  │
│         │         │  (Hub)  │         │(Memory) │
└─────────┘         └─────────┘         └─────────┘
     │                   │                    │
     │                   │                    │
     └───────────────────┴────────────────────┘
              Shared Event Bus
```

## Data Flow

1. **Agent Registration:** Agent → Axon orchestrator
2. **Task Execution:** Axon → Agent → Execution
3. **Memory Storage:** Agent → Cortex (via Axon)
4. **Memory Retrieval:** Agent ← Cortex (context)
5. **Monitoring:** User ← Dashboard (real-time)

## Design Principles

1. **Neural Architecture:** Axon (signal transmission) + Cortex (memory/processing)
2. **Modularity:** Independent projects with clear boundaries
3. **Async-First:** Non-blocking I/O throughout
4. **Type Safety:** Strong typing with shared definitions
5. **Observability:** Built-in tracing and metrics
6. **Scalability:** Designed for distributed deployment

## Project Structure

```
ryht/
├── axon/                   # Multi-agent orchestration
│   ├── src/                # React frontend
│   │   ├── components/
│   │   ├── stores/
│   │   └── App.tsx
│   └── src-tauri/          # Rust backend
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs     # GUI mode
│           └── web_main.rs # Server mode
│
├── cortex/                 # Cognitive memory system
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── memory.rs
│       ├── storage.rs
│       ├── indexing.rs
│       └── retrieval.rs
│
└── crates/                 # Shared libraries
    ├── common/
    │   └── src/
    │       ├── config.rs
    │       ├── error.rs
    │       └── utils.rs
    └── types/
        └── src/
            └── lib.rs
```

## Future Integration

```
┌──────────────────────────────────────────────┐
│              ry.ht Platform                  │
├──────────────────────────────────────────────┤
│                                              │
│  Axon (Agents) ◀──▶ Cortex (Memory)        │
│        ▲                    ▲                │
│        │                    │                │
│        └────────┬───────────┘                │
│                 │                            │
│         IPC / WebSocket                      │
│         Shared State                         │
│         Authentication                       │
│         Event Streaming                      │
│                                              │
└──────────────────────────────────────────────┘
```

## Naming Concept

**ry.ht** = rhythm + thought

- **Axon** = Neural pathways for signal transmission (agent coordination)
- **Cortex** = Brain's cognitive center (memory and processing)

Together they form a complete neural architecture for intelligent systems.

---

**Domain:** [ry.ht](https://ry.ht)
**Version:** 0.1.0
**Licenses:**
- Axon: AGPL-3.0
- Cortex: MIT OR Apache-2.0
