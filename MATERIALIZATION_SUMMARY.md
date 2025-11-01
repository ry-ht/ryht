# Materialization/Flush Mechanism - Quick Reference

## TL;DR

**Materialization** is the process of writing Virtual Filesystem (VFS) content to physical disk. The system is fully manual (user-triggered) with optional automatic filesystem watching.

---

## Key Metrics

| Metric | Value |
|--------|-------|
| **Core Engine File** | `cortex-vfs/src/materialization.rs` |
| **Engine Size** | 1,283 lines |
| **MCP Tools** | 8 tools in `cortex/src/mcp/tools/materialization.rs` |
| **Auto-Sync Supported** | Yes (file watcher with auto-sync option) |
| **Atomic Flush** | Yes (with backup/rollback) |
| **Parallel Processing** | Yes (JoinSet + Semaphore) |
| **Conflict Detection** | Yes (both VFS and disk modified) |

---

## The 8 Materialization Tools

```
1. cortex.flush.preview          → Preview changes before flush
2. cortex.flush.execute          → Execute flush (main tool)
3. cortex.flush.selective        → Flush specific entities by ID
4. cortex.sync.from_disk         → Sync filesystem changes INTO VFS
5. cortex.sync.status            → Check sync status (pending changes)
6. cortex.sync.resolve_conflict  → Resolve sync conflicts
7. cortex.watch.start            → Start filesystem watcher (with auto-sync option)
8. cortex.watch.stop             → Stop filesystem watcher
```

---

## Flush Mechanism (VFS → Disk)

### Three Phases

```
1. PREPARATION
   └─ Backup target directory (optional)
   └─ Collect changes (by scope: All/Path/Specific/Workspace)
   
2. EXECUTION
   ├─ Option A: Atomic
   │  └─ Write to temp dir, then move (all-or-nothing)
   ├─ Option B: Sequential
   │  └─ Write directly, continue on error
   └─ Process: Deletes → Creates/Updates (parallel or sequential)
   
3. COMPLETION
   ├─ On Success: Clean up backup
   └─ On Error: Restore from backup
```

### Change Collection by Scope

- **All**: All modified/created/deleted vnodes
- **Path**: Vnodes under specific path prefix
- **Specific**: Direct vnode ID lookup
- **Workspace**: All vnodes in workspace

---

## Auto-Sync Mechanism (Disk → VFS)

### File Watcher Flow

```
Filesystem Monitor
   ↓
Event: Created/Modified/Deleted/Renamed
   ↓
Debounce (100ms) + Coalesce Events
   ↓
[If auto_sync enabled] Background Task
   ↓
Update VFS (write_file / mark_deleted)
```

**NOT materialization** - flows in opposite direction (Disk → VFS)

---

## VNode States (SyncStatus)

```
Synced       ← In sync with disk (no pending changes)
Modified     ← Changed in VFS, not yet flushed
Created      ← New in VFS, not yet materialized
Deleted      ← Marked deleted in VFS, not yet removed from disk
Conflict     ← Both VFS and disk modified (needs resolution)
```

---

## Conflict Resolution Strategies

When both VFS and disk modified:

```
"memory" / "vfs"     → Keep VFS version
"disk" / "filesystem" → Load disk version into VFS
"manual" / "merge"   → Use provided merge content
```

---

## Performance Defaults

| Setting | Default | Source |
|---------|---------|--------|
| **Parallel Workers** | num_cpus::get() | CPU count |
| **Debounce (watcher)** | 100ms | watcher.rs:27 |
| **Batch Interval** | 500ms | watcher.rs:28 |
| **Max Batch Size** | 100 events | watcher.rs:44 |
| **Atomic Mode** | Enabled | FlushOptions::default() |
| **Preserve Perms** | Yes | FlushOptions::default() |
| **Preserve Timestamps** | Yes | FlushOptions::default() |

---

## File Locations Quick Map

### Core Implementation
```
cortex-vfs/src/materialization.rs     (1,283 lines)
  ├─ MaterializationEngine::flush()    (main entry point)
  ├─ flush_atomic()                    (all-or-nothing mode)
  ├─ flush_sequential()                (best-effort mode)
  ├─ flush_parallel()                  (concurrent writes)
  ├─ materialize_vnode()               (single file write)
  ├─ sync_from_filesystem()            (disk → VFS)
  └─ sync_existing_file()              (conflict detection)

cortex-vfs/src/watcher.rs             (300+ lines)
  └─ FileWatcher                       (filesystem monitor)

cortex-vfs/src/types.rs               (800+ lines)
  ├─ FlushScope                        (scope enum)
  ├─ FlushOptions                      (options struct)
  ├─ SyncStatus                        (vnode state enum)
  └─ VNode                             (virtual node type)
```

### User-Facing Tools
```
cortex/src/mcp/tools/materialization.rs (1,039 lines)
  ├─ FlushPreviewTool
  ├─ FlushExecuteTool
  ├─ FlushSelectiveTool
  ├─ SyncFromDiskTool
  ├─ SyncStatusTool
  ├─ SyncResolveConflictTool
  ├─ WatchStartTool
  └─ WatchStopTool

cortex/src/api/routes/export.rs        (400 lines)
  └─ Export/import REST endpoints
```

---

## Use Case Quick Guide

| Need | Tool | Example |
|------|------|---------|
| Save edited files | `cortex.flush.execute` | flush all changes to disk |
| Check what changed | `cortex.flush.preview` | preview pending changes |
| Save specific files | `cortex.flush.selective` | flush only modified.rs |
| Read disk changes | `cortex.sync.from_disk` | import external edits |
| Watch for changes | `cortex.watch.start` | auto-sync filesystem changes |
| Check status | `cortex.sync.status` | pending writes/conflicts? |
| Handle conflicts | `cortex.sync.resolve_conflict` | resolve sync conflicts |
| Export workspace | Export REST API | create tar.gz snapshot |

---

## Design Principles

1. **Lazy Materialization** - Files stay in VFS until explicitly flushed
2. **User Control** - All flushes are intentional (no surprise disk writes)
3. **Safety First** - Atomic operations with backup/rollback
4. **Conflict Aware** - Detects when both VFS and disk change
5. **Efficient** - Parallel processing, content deduplication
6. **Flexible** - Scoped flushes (all, path, specific IDs, workspace)

---

## Not Currently Implemented

- Time-based auto-flush (e.g., flush every 5 minutes)
- Threshold-based auto-flush (e.g., flush when 100 changes accumulated)
- Session-end auto-flush
- Commit-based triggers (e.g., flush on git commit)

These would require adding explicit trigger points in the code.

---

## Testing

Comprehensive tests exist in `cortex-vfs/src/materialization.rs` (lines 980-1282):

- `test_sync_new_file_from_filesystem()`
- `test_sync_modified_file_no_conflict()`
- `test_sync_conflict_detection()`
- `test_sync_auto_resolve_conflict()`
- `test_sync_directory_structure()`
- `test_sync_exclusion_patterns()`
- `test_sync_skip_hidden_files()`
- `test_bidirectional_sync_roundtrip()`

