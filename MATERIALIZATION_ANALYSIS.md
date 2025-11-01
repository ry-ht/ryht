# Materialization/Flush Mechanism Analysis Report

## Executive Summary

The Cortex codebase implements a sophisticated **Virtual Filesystem (VFS)** with a comprehensive materialization/flush mechanism that enables bidirectional synchronization between in-memory VFS state and physical filesystem storage. The system supports atomic operations, conflict detection, parallel processing, and automatic change tracking.

---

## 1. Architecture Overview

### 1.1 Core Components

The materialization/flush system consists of three main layers:

```
┌─────────────────────────────────────────────────────┐
│         User-Facing MCP Tools Layer                  │
│  (Tools for flush, sync, export, watch operations)   │
├─────────────────────────────────────────────────────┤
│       Materialization Engine (cortex-vfs)            │
│  (Core flush/sync logic, atomic operations)          │
├─────────────────────────────────────────────────────┤
│    Virtual Filesystem + Storage Layer                │
│  (VNodes, SyncStatus tracking, SurrealDB storage)    │
└─────────────────────────────────────────────────────┘
```

### 1.2 Key Files and Components

**Core Materialization:**
- `/cortex/cortex-vfs/src/materialization.rs` - MaterializationEngine (lines 1-1283)
- `/cortex/cortex-vfs/src/types.rs` - Type definitions (lines 572-646 for FlushScope/FlushOptions)
- `/cortex/cortex-vfs/src/virtual_filesystem.rs` - VirtualFileSystem core operations

**MCP Tools Interface:**
- `/cortex/cortex/src/mcp/tools/materialization.rs` - 8 user-facing MCP tools (lines 1-1039)
  - FlushPreviewTool
  - FlushExecuteTool
  - FlushSelectiveTool
  - SyncFromDiskTool
  - SyncStatusTool
  - SyncResolveConflictTool
  - WatchStartTool
  - WatchStopTool

**API Endpoints:**
- `/cortex/cortex/src/api/routes/export.rs` - REST endpoints for export/import jobs

**File Watching:**
- `/cortex/cortex-vfs/src/watcher.rs` - FileWatcher with debouncing and event coalescing

---

## 2. Current Architecture - How VFS Materialization Works

### 2.1 Three-Phase Flush Operation

The `MaterializationEngine::flush()` method (line 36) orchestrates materialization:

```
Phase 1: Preparation
├── Create backup (if requested)
├── Collect changes based on scope
└── Report if no changes

Phase 2: Execution (Atomic or Sequential)
├── Process deletions first
├── Process creates/updates (parallel or sequential)
└── Mark vnodes as synchronized

Phase 3: Completion
├── Clean up backup on success
└── Restore backup on error
```

**Key Features:**

1. **Atomic Operations** (lines 126-170):
   - Creates temporary directory for staging
   - All-or-nothing semantics
   - Automatic rollback on error with backup restoration

2. **Parallel Materialization** (lines 234-274):
   - Uses tokio JoinSet for concurrent writes
   - Semaphore-based worker pool (configurable max_workers)
   - Each vnode written independently

3. **Sequential Fallback** (lines 173-199):
   - Best-effort approach
   - Continues despite errors
   - Useful for partial recoveries

### 2.2 VNode Synchronization Status Tracking

**SyncStatus Enum** (types.rs, line 236):
```rust
pub enum SyncStatus {
    Synced,      // In sync with physical filesystem
    Modified,    // Modified in VFS, not yet flushed
    Created,     // Created in VFS, not yet materialized
    Deleted,     // Deleted in VFS, not yet removed from disk
    Conflict,    // Conflict detected during sync
}
```

During flush, vnodes are:
1. Queried by status (line 104): "SELECT * FROM vnode WHERE status IN ['modified', 'created', 'deleted']"
2. Processed by type and operation
3. Marked as Synced after successful materialization (line 162)

### 2.3 Change Collection by Scope

**FlushScope Enum** (types.rs, line 574):
```rust
pub enum FlushScope {
    All,                      // All modified vnodes
    Path(VirtualPath),        // Vnodes under specific path
    Specific(Vec<Uuid>),      // Specific vnode IDs
    Workspace(Uuid),          // All vnodes in workspace
}
```

Collection logic (lines 102-123):
- **All**: Queries all vnodes with modified/created/deleted status
- **Path**: Filters by path prefix: "path LIKE '{}%'"
- **Specific**: Direct ID lookup
- **Workspace**: Workspace-scoped query

### 2.4 Content Materialization Process

For each vnode (lines 303-368):

**Files/Documents:**
1. Ensure parent directory exists (line 318)
2. Read content from VFS (line 323)
3. Write to physical disk (line 327)
4. Set permissions if requested (lines 331-340)
5. Return bytes written for reporting

**Directories:**
1. Create with create_dir_all (line 311)

**Symlinks:**
1. Extract target from metadata (line 352)
2. Create symlink (platform-specific) (line 356)

---

## 3. What Triggers Materialization

### 3.1 Explicit Materialization Triggers

**User-Initiated (MCP Tools):**

1. **FlushExecuteTool** (`cortex.flush.execute`, line 244):
   - User explicitly calls flush with scope and options
   - Parameters: scope_paths, format_code, create_backup, atomic
   - Executes against current working directory

2. **FlushPreviewTool** (`cortex.flush.preview`, line 156):
   - Preview changes before flushing (no actual write)
   - Shows diffs between VFS and filesystem
   - Used for validation before actual flush

3. **FlushSelectiveTool** (`cortex.flush.selective`, line 330):
   - Flush specific entity IDs
   - Useful for targeted updates

4. **WorkspaceExportTool** (in workspace.rs):
   - Export workspace to disk as tar.gz, zip, or git format
   - Creates materialized snapshot for distribution

### 3.2 Automatic/Implicit Triggers

**File Watcher with Auto-Sync** (lines 794-886 in materialization.rs):

1. **WatchStartTool** (`cortex.watch.start`):
   - Initiates filesystem monitoring on specified paths
   - Optional `auto_sync` parameter enables automatic VFS updates
   - Spawns background task (lines 870-874)

2. **Auto-Sync Task** (lines 889-930):
   - Runs continuously in background
   - Listens for FileEvent events:
     - Created/Modified → sync to VFS
     - Deleted → mark as deleted in VFS
     - Renamed → handle as delete + create
   - Updates VFS in real-time (NOT flushing to disk)

**Note**: The watcher performs **sync-to-VFS** (filesystem → VFS), not materialization (VFS → filesystem).

### 3.3 Triggered by External API Calls

**REST Endpoints** (api/routes/export.rs):
- `POST /api/v1/export` - Create export job (line 149)
  - Status: Processing → Completed (asynchronous)
- `GET /api/v1/export/{id}` - Check status
- `GET /api/v1/export/{id}/download` - Get download URL

### 3.4 Manual Sync Operations

**SyncFromDiskTool** (`cortex.sync.from_disk`, line 465):
- Syncs filesystem changes INTO VFS
- Opposite direction of flush
- Detects conflicts when both VFS and disk modified
- Can auto-resolve conflicts if configured

---

## 4. Use Cases for Materialization in Current Design

### 4.1 Primary Use Cases

| Use Case | Tool | Trigger | Direction |
|----------|------|---------|-----------|
| **Save work to disk** | FlushExecuteTool | User action | VFS → Disk |
| **Preview changes** | FlushPreviewTool | User action | VFS (read-only) |
| **Targeted save** | FlushSelectiveTool | User action | VFS → Disk |
| **Export workspace** | WorkspaceExportTool | User action | VFS → Archive |
| **Watch filesystem** | WatchStartTool | User action | Disk → VFS |
| **Sync from disk** | SyncFromDiskTool | User action | Disk → VFS |
| **Check sync state** | SyncStatusTool | Query | Status check |
| **Resolve conflicts** | SyncResolveConflictTool | User action | Conflict resolution |

### 4.2 Design Patterns

**1. Lazy Materialization**
- Files exist in VFS (in-memory/database) until explicitly flushed
- Enables safe editing without immediate disk writes
- Deferred I/O for better performance

**2. Change Tracking**
- Every vnode has SyncStatus
- Enable selective flushing (only modified files)
- Efficient incremental updates

**3. Conflict Detection**
- When syncing from disk:
  - If VFS has unsaved changes AND disk changed → conflict
  - Marked with SyncStatus::Conflict
  - Multiple resolution strategies available (memory, disk, manual, merge)

**4. Atomic Safety**
- Backup before flush (optional)
- Rollback on error
- Temp directory staging
- All-or-nothing semantics

**5. Parallel Processing**
- Multiple files written concurrently
- Semaphore-based worker pool
- Configurable parallelism (default: number of CPUs)

---

## 5. Automatic Materialization Triggers

### 5.1 File Watching (Auto-Sync FROM Disk)

**Location**: `watcher.rs` (lines 1-300+)

**Mechanism**:
1. FileWatcher monitors filesystem using `notify` crate (line 77)
2. Events debounced (default 100ms) and batched (default 500ms)
3. Events coalesced for same path (e.g., multiple writes → one Modified)
4. If auto_sync enabled, background task processes events (lines 889-930)

**Event Handling**:
```rust
FileEvent::Created(path) | FileEvent::Modified(path)
    → Call sync_file_to_vfs() → VFS::write_file()

FileEvent::Deleted(path)
    → Call mark_deleted_in_vfs() → Set status = Deleted

FileEvent::Renamed { from, to }
    → Delete old, create new
```

**Key Code**: Lines 932-971 in materialization.rs

### 5.2 Potential Auto-Flush Triggers (Not Currently Implemented)

Based on code structure, the following COULD be auto-triggered but aren't currently:

1. **Time-based**: Periodic flush based on timer (not implemented)
2. **Threshold-based**: Flush when X changes accumulated (not implemented)
3. **Session-end**: Auto-flush when session closes (not implemented)
4. **Commit-based**: Flush on "save" command from agent (not implemented)

These would require additional trigger points not present in current code.

---

## 6. Detailed File Locations and Line Numbers

### 6.1 Core Materialization Logic

| Component | File | Key Functions | Lines |
|-----------|------|----------------|-------|
| **MaterializationEngine** | `cortex-vfs/src/materialization.rs` | `flush()`, `flush_atomic()`, `flush_sequential()` | 36-199 |
| **Change Collection** | `cortex-vfs/src/materialization.rs` | `collect_changes()` | 102-123 |
| **Vnode Processing** | `cortex-vfs/src/materialization.rs` | `materialize_vnode()` | 303-368 |
| **Parallel Flush** | `cortex-vfs/src/materialization.rs` | `flush_parallel()` | 234-274 |
| **Sync from Filesystem** | `cortex-vfs/src/materialization.rs` | `sync_from_filesystem()` | 542-591 |
| **Conflict Detection** | `cortex-vfs/src/materialization.rs` | `sync_existing_file()` | 843-942 |

### 6.2 MCP Tools

| Tool | File | Lines | Description |
|------|------|-------|-------------|
| FlushPreviewTool | `cortex/src/mcp/tools/materialization.rs` | 56-211 | Preview flush changes |
| FlushExecuteTool | `cortex/src/mcp/tools/materialization.rs` | 213-302 | Execute flush |
| FlushSelectiveTool | `cortex/src/mcp/tools/materialization.rs` | 304-374 | Selective flush by ID |
| SyncFromDiskTool | `cortex/src/mcp/tools/materialization.rs` | 376-547 | Sync filesystem → VFS |
| SyncStatusTool | `cortex/src/mcp/tools/materialization.rs` | 549-650 | Check sync status |
| SyncResolveConflictTool | `cortex/src/mcp/tools/materialization.rs` | 652-768 | Resolve conflicts |
| WatchStartTool | `cortex/src/mcp/tools/materialization.rs` | 770-886 | Start filesystem watcher |
| WatchStopTool | `cortex/src/mcp/tools/materialization.rs` | 974-1031 | Stop filesystem watcher |

### 6.3 Type Definitions

| Type | File | Lines | Purpose |
|------|------|-------|---------|
| FlushScope | `cortex-vfs/src/types.rs` | 572-586 | Scope of flush operation |
| FlushOptions | `cortex-vfs/src/types.rs` | 613-646 | Options for flush (atomic, parallel, etc.) |
| FlushReport | `cortex-vfs/src/types.rs` | 588-611 | Flush operation report |
| SyncStatus | `cortex-vfs/src/types.rs` | 233-247 | VNode sync state |
| VNode | `cortex-vfs/src/types.rs` | 63-114 | Virtual node representation |

### 6.4 API Routes

| Endpoint | File | Lines | Purpose |
|----------|------|-------|---------|
| POST /api/v1/export | `cortex/src/api/routes/export.rs` | 149-218 | Create export job |
| GET /api/v1/export/{id} | `cortex/src/api/routes/export.rs` | 220-248 | Get export status |
| GET /api/v1/export/{id}/download | `cortex/src/api/routes/export.rs` | 250-292 | Download export |

---

## 7. Performance Characteristics

### 7.1 Parallelization

**Default Configuration**:
- Workers: `num_cpus::get()` (line 284 in materialization.rs)
- Max batch size: 100 files (line 844 in watcher.rs)
- Debounce: 100ms (line 27 in watcher.rs)
- Batch interval: 500ms (line 28 in watcher.rs)

**Semaphore-Based Throttling**:
- Prevents overwhelming system
- Lock acquisition + file write per worker
- Efficient for I/O bound operations

### 7.2 Change Collection

**Query Performance**:
- Status-based index on vnodes critical
- Path prefix queries (LIKE) may need optimization
- Workspace-scoped queries for isolation

---

## 8. Conflict Resolution Strategies

The `SyncResolveConflictTool` supports four strategies (lines 721-753):

```rust
"memory" | "vfs"    → Keep VFS version, mark synchronized
"disk" | "filesystem" → Load disk version into VFS
"manual" | "merge"  → Use provided merge_content
```

Conflicts occur when:
- Both VFS and filesystem modified (line 869-876 in materialization.rs)
- SyncStatus::Conflict set
- Both versions stored in metadata (fs_content_hash)

---

## 9. Summary Table: Materialization Mechanisms

| Aspect | Details |
|--------|---------|
| **Core Engine** | MaterializationEngine (materialization.rs) |
| **Scope Options** | All, Path, Specific IDs, Workspace |
| **Modes** | Atomic (rollback) or Sequential (best-effort) |
| **Parallelization** | JoinSet + Semaphore, CPU-count workers |
| **Backup Strategy** | Optional pre-flush backup, restore on error |
| **Change Tracking** | SyncStatus enum on each VNode |
| **Conflict Detection** | Both VFS and disk modified = conflict |
| **Watcher** | Filesystem monitor with debounce/coalesce |
| **Auto-Sync** | Background task for filesystem → VFS |
| **Manual Triggers** | 8 MCP tools + REST API endpoints |
| **Export** | tar.gz, zip, or git format (api/routes/export.rs) |

---

## 10. Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│              Cortex Application                          │
├─────────────────────────────────────────────────────────┤
│  MCP Server + REST API                                   │
│  ├─ 8 Materialization Tools                             │
│  └─ Export/Import REST Routes                           │
├─────────────────────────────────────────────────────────┤
│  MaterializationEngine (VFS → Disk)                     │
│  ├─ flush() - Main entry point                         │
│  ├─ flush_atomic() - All-or-nothing                     │
│  ├─ flush_parallel() - Concurrent writes                │
│  └─ materialize_vnode() - Single file write             │
├─────────────────────────────────────────────────────────┤
│  FileWatcher (Disk → VFS)                              │
│  ├─ Watch filesystem changes                            │
│  ├─ Debounce/coalesce events                           │
│  └─ Auto-sync to VFS (if enabled)                      │
├─────────────────────────────────────────────────────────┤
│  VirtualFileSystem + SurrealDB Storage                  │
│  ├─ VNodes (files/dirs/symlinks)                       │
│  ├─ SyncStatus tracking                                 │
│  └─ Content deduplication (blake3)                      │
└─────────────────────────────────────────────────────────┘
```

---

## Conclusion

The Cortex materialization/flush mechanism provides a sophisticated, production-grade system for:

1. **Lazy Materialization**: Files live in VFS until explicitly flushed
2. **Atomic Safety**: Backup/rollback prevents corruption
3. **Conflict Detection**: Warns when both VFS and disk change
4. **Parallel Processing**: Efficient multi-worker flush operations
5. **Bidirectional Sync**: Both filesystem → VFS and VFS → filesystem
6. **Change Tracking**: Fine-grained SyncStatus per vnode
7. **Automatic Watching**: Background file monitoring with auto-sync option

The design enables agents and users to safely edit virtual content with full control over when physical filesystem updates occur.
