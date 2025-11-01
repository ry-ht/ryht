# Materialization/Flush Mechanism - Complete Documentation Index

This analysis provides comprehensive documentation of the VFS materialization and flush system in Cortex.

## Documents Overview

### 1. **MATERIALIZATION_SUMMARY.md** (Quick Reference)
**Best for:** Quick lookup, implementation details at a glance
- Key metrics and statistics
- 8 materialization tools overview
- Three-phase flush mechanism
- Auto-sync mechanism
- VNode states
- Performance defaults
- File location quick map
- Use case guide
- **1,130 lines total** across all docs

### 2. **MATERIALIZATION_ANALYSIS.md** (Detailed Technical Report)
**Best for:** Deep understanding, architecture analysis, line-by-line references
- Executive summary
- 3-layer architecture overview
- Complete component list with file paths
- How VFS materialization works (3-phase process)
- VNode status tracking
- Change collection by scope
- Content materialization process
- What triggers materialization (explicit, automatic, API)
- Use cases and design patterns
- Automatic triggers with file watching
- Detailed file locations and line numbers
- Performance characteristics
- Conflict resolution strategies
- Summary table and architecture diagram

### 3. **MATERIALIZATION_VISUAL_GUIDE.md** (Flow Diagrams & State Machines)
**Best for:** Understanding flow, state transitions, visual learning
- Complete flush operation flow
- VNode materialization process
- Bidirectional sync status flows
- File watcher auto-sync flow
- Scope-based change collection
- Conflict detection decision tree
- Parallel flush execution diagram
- File watcher debounce timeline
- SyncStatus state machine
- Conflict resolution outcomes
- Type hierarchy

## Key Sections by Topic

### Understanding Materialization
1. Start with: **MATERIALIZATION_SUMMARY.md** - TL;DR section
2. Read: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 1 (Flush flow)
3. Deep dive: **MATERIALIZATION_ANALYSIS.md** - Section 2 (Architecture)

### File Locations (Code References)
1. Quick map: **MATERIALIZATION_SUMMARY.md** - "File Locations Quick Map"
2. Detailed: **MATERIALIZATION_ANALYSIS.md** - Section 6 (All line numbers)

### Tools and Commands
1. Overview: **MATERIALIZATION_SUMMARY.md** - "The 8 Materialization Tools"
2. Details: **MATERIALIZATION_ANALYSIS.md** - Section 6.2 (MCP Tools table)
3. Flows: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 4 (Watcher flow)

### Conflict Resolution
1. Strategies: **MATERIALIZATION_SUMMARY.md** - "Conflict Resolution Strategies"
2. Detection: **MATERIALIZATION_ANALYSIS.md** - Section 8
3. Decision tree: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 6

### Performance
1. Defaults: **MATERIALIZATION_SUMMARY.md** - Performance Defaults table
2. Analysis: **MATERIALIZATION_ANALYSIS.md** - Section 7
3. Visualization: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 7

### State Management
1. States: **MATERIALIZATION_SUMMARY.md** - "VNode States"
2. Lifecycle: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 3 & 8 (State machines)

### Auto-Sync & Watching
1. Overview: **MATERIALIZATION_SUMMARY.md** - Auto-Sync Mechanism
2. Complete: **MATERIALIZATION_ANALYSIS.md** - Section 5
3. Flow: **MATERIALIZATION_VISUAL_GUIDE.md** - Section 4

## Core File Paths Reference

### Main Implementation Files
```
cortex-vfs/src/materialization.rs      (1,283 lines) - MaterializationEngine
cortex-vfs/src/types.rs                (800+ lines) - Type definitions
cortex-vfs/src/watcher.rs              (300+ lines) - FileWatcher
cortex-vfs/src/virtual_filesystem.rs   - VFS operations
```

### User-Facing Tools
```
cortex/src/mcp/tools/materialization.rs (1,039 lines) - 8 MCP tools
cortex/src/api/routes/export.rs         (400 lines) - REST endpoints
```

### Key Functions

**Flush Operations:**
- `MaterializationEngine::flush()` - Main entry point
- `MaterializationEngine::flush_atomic()` - All-or-nothing mode
- `MaterializationEngine::flush_parallel()` - Concurrent writes
- `MaterializationEngine::materialize_vnode()` - Single file write

**Sync Operations:**
- `MaterializationEngine::sync_from_filesystem()` - Disk → VFS
- `MaterializationEngine::sync_existing_file()` - Conflict detection

**Watching:**
- `FileWatcher::new()` - Create filesystem monitor
- `FileWatcher::coalesce_events()` - Event debounce/batch

## Quick Navigation Guide

### "How do I...?"

**Save my VFS changes to disk?**
→ Use `cortex.flush.execute` (MATERIALIZATION_SUMMARY.md line 33)
→ See flush flow: MATERIALIZATION_VISUAL_GUIDE.md Section 1

**Preview what will be flushed?**
→ Use `cortex.flush.preview` (MATERIALIZATION_SUMMARY.md line 32)
→ See in tools list: MATERIALIZATION_ANALYSIS.md Section 6.2

**Handle a sync conflict?**
→ Use `cortex.sync.resolve_conflict` (MATERIALIZATION_SUMMARY.md line 38)
→ See strategies: MATERIALIZATION_VISUAL_GUIDE.md Section 9

**Watch filesystem for changes?**
→ Use `cortex.watch.start` with `auto_sync=true` (MATERIALIZATION_SUMMARY.md line 39)
→ See flow: MATERIALIZATION_VISUAL_GUIDE.md Section 4

**Understand VNode states?**
→ Read: MATERIALIZATION_SUMMARY.md - VNode States section
→ See state machine: MATERIALIZATION_VISUAL_GUIDE.md Section 8

**Find the flush code?**
→ `cortex-vfs/src/materialization.rs` line 36-199
→ Details: MATERIALIZATION_ANALYSIS.md Section 6.1

**Check file watcher implementation?**
→ `cortex-vfs/src/watcher.rs` (complete file)
→ Details: MATERIALIZATION_ANALYSIS.md Section 5

## Key Concepts Defined

| Concept | Definition | Location |
|---------|-----------|----------|
| Materialization | VFS → Disk write operation | SUMMARY TL;DR |
| Flush | Core materialization method | ANALYSIS Section 2.1 |
| VNode | Virtual node (file/dir/symlink) | ANALYSIS Section 2.2 |
| SyncStatus | State of VNode vs filesystem | SUMMARY VNode States |
| FlushScope | What to flush (All/Path/Specific/Workspace) | ANALYSIS Section 2.3 |
| Auto-Sync | Background filesystem → VFS | ANALYSIS Section 5 |
| Conflict | Both VFS and disk modified | ANALYSIS Section 8 |
| Atomic | All-or-nothing operation | VISUAL Section 1 |
| Parallel | Concurrent flush with workers | VISUAL Section 7 |
| Debounce | Wait for event stabilization | VISUAL Section 7 |

## Statistics

- **Total Documentation:** 1,130 lines across 3 files
- **Core Engine:** 1,283 lines (materialization.rs)
- **MCP Tools:** 1,039 lines (8 tools)
- **Type Definitions:** 800+ lines
- **Tests:** Comprehensive (980-1282 lines in materialization.rs)
- **Key Functions:** 14 documented with line references

## Architecture Summary

```
User Layer (MCP Tools)
    ↓
MaterializationEngine
    ├─ flush()              (VFS → Disk)
    ├─ sync_from_filesystem() (Disk → VFS)
    └─ conflict_detection()
    ↓
VirtualFileSystem + Storage
    ├─ VNodes with SyncStatus
    └─ Content + Metadata
```

## Design Principles (From Analysis)

1. **Lazy Materialization** - Files stay in VFS until explicitly flushed
2. **User Control** - All flushes are intentional
3. **Safety First** - Atomic operations with backup/rollback
4. **Conflict Aware** - Detects simultaneous edits
5. **Efficient** - Parallel processing, content dedup
6. **Flexible** - Scoped flushes support all use cases

## Implementation Status

### Fully Implemented
- Flush to disk (atomic & sequential modes)
- Parallel materialization with semaphores
- Conflict detection and resolution
- File watching with debounce/coalesce
- Bidirectional sync (VFS ↔ Disk)
- 8 complete MCP tools

### Not Implemented
- Time-based auto-flush
- Threshold-based auto-flush
- Session-end auto-flush
- Commit-based triggers

## Testing

Comprehensive test suite at materialization.rs lines 980-1282:
- Sync operations
- Conflict detection
- Auto-resolution
- Directory structures
- Pattern exclusions
- Bidirectional roundtrips

---

**Last Updated:** 2025-11-01
**Repository:** /Users/taaliman/projects/luxquant/ry-ht/ryht
**Codebase:** Cortex (cortex-vfs + cortex packages)
