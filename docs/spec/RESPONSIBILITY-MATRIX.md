# Responsibility Separation Matrix: Axon vs Cortex

## Summary

Clear separation of responsibilities between systems to create a coherent architecture without functional duplication.

## Core Principles

### Cortex (Cognitive Memory System)
**Role:** Knowledge, memory and data storage system
**Focus:** Persistence, indexing, search, learning

### Axon (Multi-Agent Orchestration)
**Role:** Agent orchestration and coordination system
**Focus:** Execution, coordination, consensus, UI

## Detailed Responsibility Separation

### Cortex is responsible for:

#### 1. Data Storage and Management
- ✅ Virtual File System (VFS)
- ✅ Code and metadata storage
- ✅ Versioning and change history
- ✅ Indexing and semantic graph
- ✅ Caching and access optimization

#### 2. Knowledge Management
- ✅ Episodic memory (Episodes)
- ✅ Patterns and learning from experience
- ✅ Semantic code search
- ✅ Knowledge Graph and entity relationships
- ✅ Contextual memory for agents

#### 3. Sessions and Isolation (Data Layer)
- ✅ Creating isolated namespaces for agents
- ✅ Copy-on-write semantics for isolation
- ✅ Version management within sessions
- ✅ Merge changes at data level
- ✅ Conflict resolution during merging

#### 4. Task Management (Task Storage)
- ✅ Task definition storage
- ✅ Task status and progress tracking
- ✅ Task execution history
- ✅ Task metrics and analytics
- ✅ Task-episode relationships

#### 5. API and Interfaces
- ✅ REST API for all operations
- ✅ WebSocket for real-time updates
- ✅ MCP Tools for Claude agents
- ✅ SDK for various languages
- ✅ GraphQL API (optional)

### Axon is responsible for:

#### 1. Agent Orchestration
- ✅ Agent lifecycle creation and management
- ✅ Task distribution between agents
- ✅ Load balancing
- ✅ Agent state monitoring
- ✅ Agent pool management

#### 2. Workflow Engine
- ✅ DAG-based workflow execution
- ✅ Parallel and sequential execution
- ✅ Task dependency management
- ✅ Retry and error handling logic
- ✅ Workflow templates and DSL

#### 3. Coordination and Communication
- ✅ Message bus between agents
- ✅ Pub/Sub for agent events
- ✅ Agent interaction protocols
- ✅ Broadcasting and multicast messages
- ✅ Request-response patterns

#### 4. Consensus and Decision Making
- ✅ Sangha consensus mechanism
- ✅ Voting systems for agents
- ✅ Decision-level conflict resolution
- ✅ Priority-based scheduling
- ✅ Democratic decision making

#### 5. UI and Visualization
- ✅ Tauri desktop application
- ✅ Dashboard for agent monitoring
- ✅ Workflow and DAG visualization
- ✅ Real-time agent status
- ✅ Agent configuration management

#### 6. Cortex Integration
- ✅ Using Cortex REST API
- ✅ Caching data from Cortex
- ✅ Session management via Cortex API
- ✅ Sending episodes to Cortex
- ✅ Requesting context and memory

## System Interaction

### Interaction Architecture
```
┌──────────────────────────────────────┐
│          Axon Desktop App             │
│         (Tauri + React UI)            │
├──────────────────────────────────────┤
│       Agent Orchestration Engine      │
│  ┌──────────┐  ┌──────────────────┐  │
│  │  Agents  │  │ Workflow Engine  │  │
│  └──────────┘  └──────────────────┘  │
├──────────────────────────────────────┤
│         Cortex Client Library         │
│      (REST API + WebSocket Client)    │
└──────────────────────────────────────┘
                    ↓ HTTP/WS
┌──────────────────────────────────────┐
│          Cortex REST API              │
│         (Port 8081 by default)        │
├──────────────────────────────────────┤
│      Cognitive Memory System          │
│  ┌──────────┐  ┌──────────────────┐  │
│  │   VFS    │  │  Knowledge Graph │  │
│  └──────────┘  └──────────────────┘  │
│  ┌──────────┐  ┌──────────────────┐  │
│  │ Sessions │  │    Episodes      │  │
│  └──────────┘  └──────────────────┘  │
└──────────────────────────────────────┘
```

### Typical Task Execution Flow

1. **Task Acquisition (Axon)**
   - User creates task through UI
   - Axon determines required agents

2. **Session Creation (Cortex)**
   - Axon calls `POST /sessions`
   - Cortex creates isolated session

3. **Context Retrieval (Cortex)**
   - Axon requests relevant episodes
   - Cortex returns similar solutions

4. **Execution (Axon)**
   - Agents execute the task
   - Coordination through message bus

5. **Change Persistence (Cortex)**
   - Axon sends changes to Cortex
   - Cortex performs merge to main branch

6. **Episode Storage (Cortex)**
   - Axon sends result as episode
   - Cortex stores for future learning

## Migration of Existing Functionality

### What needs to be moved from Cortex to Axon:
1. **Workflow Engine** - currently partially in Cortex 06-multi-agent.md
2. **Agent Communication** - message bus and protocols
3. **Task Assignment** - task distribution logic
4. **Coordination Protocols** - coordination protocols

### What needs to be removed/refactored in Cortex:
1. "Workflow Orchestration" section in 06-multi-agent.md
2. "Agent Communication" section in 06-multi-agent.md
3. "Task Assignment" section in 06-multi-agent.md

### What remains in Cortex:
1. Session Management (data isolation)
2. Lock Management (data-level locks)
3. Conflict Resolution (merge conflicts)
4. Episode Storage (experience storage)

## API Integration Points

### Cortex API endpoints used by Axon:

#### Sessions
- `POST /sessions` - create session for agent
- `GET /sessions/{id}` - session status
- `DELETE /sessions/{id}` - close session
- `POST /sessions/{id}/merge` - merge changes
- `GET /sessions/{id}/files/{path}` - read file from session
- `PUT /sessions/{id}/files/{path}` - write file to session
- `GET /sessions/{id}/files` - list session files

#### Locks (Multi-Agent Coordination)
- `POST /locks` - acquire lock
- `DELETE /locks/{id}` - release lock

#### Tasks
- `GET /tasks` - get tasks
- `POST /tasks` - create task
- `PUT /tasks/{id}` - update status

#### Episodes (Episodic Memory)
- `POST /memory/episodes` - save episode
- `POST /memory/search` - search similar episodes
- `GET /memory/patterns` - get patterns

#### Knowledge Graph
- `GET /knowledge/entities` - get entities
- `GET /knowledge/dependencies` - dependency graph
- `POST /knowledge/patterns/search` - search patterns

#### Semantic Search
- `POST /search/semantic` - semantic code search
- `GET /workspaces/{id}/units` - workspace code units

#### Analysis (Code Intelligence)
- `POST /analysis/impact` - impact analysis
- `GET /analysis/cycles` - detect circular dependencies

#### Context Optimization
- `POST /context/optimize` - context optimization (Agentwise 3.0)

#### Workspace Operations
- `GET /workspaces/{id}/files` - read workspace files
- `PUT /files/{id}` - update files

#### Real-time Events
- `WebSocket /ws` - subscribe to real-time updates
  - session.created, session.merged
  - lock.acquired, lock.released
  - conflict.detected, file.changed
  - pattern.detected

## Separation Benefits

### For Architecture:
- ✅ Clear responsibility boundaries
- ✅ No duplication
- ✅ Modularity and extensibility
- ✅ Independent scaling

### For Development:
- ✅ Parallel team development
- ✅ Independent testing
- ✅ Clear API contracts
- ✅ Easy maintenance

### For Performance:
- ✅ Task-specific optimization
- ✅ Efficient caching
- ✅ Distributed execution
- ✅ Horizontal scaling

## Next Steps

1. **Update Cortex specification**
   - Remove duplicate orchestration functionality
   - Focus on data storage and management

2. **Create complete Axon specification**
   - Detail all orchestration components
   - Define Cortex integration points

3. **Agree on API contracts**
   - Finalize REST API endpoints
   - Define data formats

4. **Create integration tests**
   - System interaction tests
   - Performance benchmarks

---

**Status:** Approved for implementation
**Date:** 2025-10-20
**Version:** 1.0