# Materialization Mechanism - Visual Reference Guide

## 1. Complete Flush Operation Flow

```
User Calls: cortex.flush.execute()
            ↓
    [Collect Scope]
    ├─ Scope::All → Query all modified/created/deleted vnodes
    ├─ Scope::Path → Query vnodes under path prefix
    ├─ Scope::Specific → Query by vnode IDs
    └─ Scope::Workspace → Query by workspace_id
            ↓
    [Optional: Create Backup]
    └─ Copy target_path to target_path.backup
            ↓
    ╔═══════════════════════════════════════════════════════╗
    ║ Execute (Atomic or Sequential Mode)                   ║
    ╚═══════════════════════════════════════════════════════╝
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Process Deletions (always sequential)                │
    │ For each vnode with status=Deleted:                  │
    │   ├─ Convert vnode.path to physical_path             │
    │   └─ fs::remove_file() or fs::remove_dir_all()       │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Process Creates/Updates                              │
    │                                                      │
    │ If parallel && len > 1:                              │
    │   ├─ Create JoinSet (tokio task pool)                │
    │   ├─ Create Semaphore (max_workers)                  │
    │   └─ For each vnode:                                 │
    │       ├─ Spawn task                                  │
    │       ├─ Acquire semaphore permit                    │
    │       └─ Call materialize_vnode_static()             │
    │                                                      │
    │ Else (sequential):                                   │
    │   └─ For each vnode:                                 │
    │       └─ Call materialize_vnode()                    │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Mark VNodes as Synchronized                          │
    │ For each processed vnode:                            │
    │   └─ vnode.status = SyncStatus::Synced               │
    │   └─ vfs.save_vnode(&vnode)                          │
    └─────────────────────────────────────────────────────┘
            ↓
    [Success Path]          [Error Path]
    ├─ Delete backup        ├─ Restore from backup
    └─ Return FlushReport   └─ Return Error

RETURN: FlushReport {
    files_written: usize,
    bytes_written: usize,
    files_deleted: usize,
    duration_ms: u64,
    errors: Vec<String>,
}
```

---

## 2. VNode Materialization (Single File)

```
materialize_vnode(&vnode, &physical_path)
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Match on vnode.node_type                             │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ NodeType::Directory                                  │
    │ └─ fs::create_dir_all(physical_path)                 │
    │    → Returns: 0 bytes (no content to write)          │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ NodeType::File | Document                            │
    │                                                      │
    │ 1. Ensure parent exists                              │
    │    └─ fs::create_dir_all(parent_path)                │
    │                                                      │
    │ 2. Read content from VFS                             │
    │    └─ vfs.read_file(&workspace_id, &vnode.path)     │
    │                                                      │
    │ 3. Write to disk                                     │
    │    └─ fs::write(physical_path, content)              │
    │                                                      │
    │ 4. Set permissions (if requested)                    │
    │    └─ fs::set_permissions(physical_path, mode)       │
    │                                                      │
    │ 5. Set timestamps (optional, platform-specific)      │
    │                                                      │
    │ Return: content.len() (bytes written)                │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ NodeType::SymLink                                    │
    │ └─ Extract target from vnode.metadata["target"]      │
    │    └─ std::os::unix::fs::symlink(target, path)       │
    │       Return: 0 bytes                                │
    └─────────────────────────────────────────────────────┘
```

---

## 3. Bidirectional Sync Status Flows

```
STATE DIAGRAM: VNode Lifecycle

                    ┌─────────────┐
                    │   SYNCED    │ ← Initial state (file on disk)
                    └─────────────┘
                      ↑         ↓
              User edits   Read from disk
              in VFS       (no VFS changes)
                          ↓
                    [External edit]
                          ↓
                    ┌──────────────────────┐
                    │    NO CONFLICT       │
                    │ VFS Modified = false │
                    └──────────────────────┘
                          ↓
    ┌───────────────────────────────────────────────────────┐
    │           User makes VFS change → MODIFIED            │
    │                                                        │
    │ ┌─────────────────────────────────────────────────┐  │
    │ │ Scenario A: No external edit                    │  │
    │ │ └─ User flushes                                 │  │
    │ │    └─ materialize_vnode() → SYNCED             │  │
    │ └─────────────────────────────────────────────────┘  │
    │                                                        │
    │ ┌─────────────────────────────────────────────────┐  │
    │ │ Scenario B: External edit while VFS modified   │  │
    │ │ └─ sync_from_filesystem() detects:             │  │
    │ │    ├─ vnode.status = Modified                  │  │
    │ │    ├─ disk content changed                      │  │
    │ │    └─ → CONFLICT                               │  │
    │ │                                                 │  │
    │ │    └─ Mark as SyncStatus::Conflict             │  │
    │ │    └─ Store both versions in metadata          │  │
    │ │    └─ Requires user resolution                 │  │
    │ └─────────────────────────────────────────────────┘  │
    └───────────────────────────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────────────────────────┐
    │           Conflict Resolution                         │
    │                                                        │
    │ "memory"/"vfs"       → Keep VFS version → SYNCED     │
    │ "disk"/"filesystem"  → Load disk version → SYNCED    │
    │ "manual"/"merge"     → Use merged content → SYNCED   │
    └───────────────────────────────────────────────────────┘
```

---

## 4. File Watcher Auto-Sync Flow

```
WatchStartTool(paths, auto_sync=true)
            ↓
┌─────────────────────────────────────────────────────────┐
│ Initialize FileWatcher                                   │
│ ├─ Create notify::RecommendedWatcher                    │
│ ├─ Watch paths recursively                              │
│ └─ Setup event channels                                 │
└─────────────────────────────────────────────────────────┘
            ↓
┌─────────────────────────────────────────────────────────┐
│ Start auto_sync Background Task (if enabled)            │
│ └─ Runs continuously in tokio::spawn()                  │
└─────────────────────────────────────────────────────────┘
            ↓
            ╔══════════════════════════════════════════════════════════╗
            ║          Filesystem Event Loop (Background)              ║
            ╚══════════════════════════════════════════════════════════╝
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Raw Filesystem Event                                │
    │ └─ notify crate detects file change                │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Debounce + Coalesce                                 │
    │ ├─ Wait 100ms (debounce_duration)                   │
    │ ├─ Batch events at 500ms intervals                  │
    │ ├─ Merge multiple edits → single Modified event     │
    │ └─ Force emit if batch > 100 items                  │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Match Event Type                                     │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌────────────────────────────────────────────────────────────┐
    │ FileEvent::Created(path)                                   │
    │ ├─ fs::read(path) → content                               │
    │ ├─ Convert path → virtual_path (relative)                 │
    │ └─ vfs.write_file(&workspace_id, &vpath, &content)        │
    │    → VNode.status = Modified/Created                       │
    └────────────────────────────────────────────────────────────┘
            ↓
    ┌────────────────────────────────────────────────────────────┐
    │ FileEvent::Modified(path)                                  │
    │ └─ [Same as Created: update VFS]                           │
    └────────────────────────────────────────────────────────────┘
            ↓
    ┌────────────────────────────────────────────────────────────┐
    │ FileEvent::Deleted(path)                                   │
    │ ├─ Convert path → virtual_path                             │
    │ └─ vfs.get_vnode() → vnode                                 │
    │    ├─ vnode.status = Deleted                               │
    │    └─ vfs.save_vnode(&vnode)                               │
    └────────────────────────────────────────────────────────────┘
            ↓
    ┌────────────────────────────────────────────────────────────┐
    │ FileEvent::Renamed { from, to }                            │
    │ ├─ Mark from as Deleted                                    │
    │ └─ Treat to as Created (sync new content)                  │
    └────────────────────────────────────────────────────────────┘
            ↓
    [Loop continues - watcher stays active until WatchStopTool called]
```

---

## 5. Scope-Based Change Collection

```
collect_changes(scope: FlushScope)
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Determine Change Statuses to query                  │
    │ └─ SyncStatus::Modified                             │
    │ └─ SyncStatus::Created                              │
    │ └─ SyncStatus::Deleted                              │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Match Scope                                          │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ All                                                  │
    │ └─ SELECT * FROM vnode WHERE status IN              │
    │    ['modified', 'created', 'deleted']               │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Path(prefix)                                         │
    │ └─ SELECT * FROM vnode WHERE                        │
    │    status IN [...] AND path LIKE 'prefix%'          │
    │    └─ Example: scope=Path("src/")                   │
    │       → All changes under src/ directory            │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Specific(ids)                                        │
    │ └─ SELECT * FROM vnode WHERE id IN (id1, id2, ...)  │
    │    └─ Direct lookup by UUID list                    │
    └─────────────────────────────────────────────────────┘
            ↓
    ┌─────────────────────────────────────────────────────┐
    │ Workspace(workspace_id)                              │
    │ └─ SELECT * FROM vnode WHERE                        │
    │    status IN [...] AND workspace_id = ?             │
    │    └─ Scope to single workspace                     │
    └─────────────────────────────────────────────────────┘
            ↓
    RETURN: Vec<VNode> (matching vnodes)
```

---

## 6. Conflict Detection Decision Tree

```
sync_existing_file(vnode_on_disk, vnode_in_vfs)
            ↓
    ┌──────────────────────────────────────────────────────┐
    │ Get content hashes                                    │
    │ ├─ old_hash = vnode_in_vfs.content_hash              │
    │ ├─ new_hash = Blake3::hash(disk_content)             │
    │ └─ Compare                                            │
    └──────────────────────────────────────────────────────┘
            ↓
    ╔════════════════════════════════════════════════════════╗
    ║ new_hash == old_hash?                                  ║
    ╚════════════════════════════════════════════════════════╝
            ↓
    ┌─────────────────┬────────────────────────────────────┐
    │ YES             │ NO (Disk changed)                  │
    │                 │                                    │
    │ No changes      │ ┌──────────────────────────────┐   │
    │ → Return Ok()   │ │ Check VFS status            │   │
    │                 │ └──────────────────────────────┘   │
    │                 │          ↓                         │
    │                 │ ╔═══════════════════════════════╗  │
    │                 │ ║ Modified/Created in VFS?      ║  │
    │                 │ ╚═════════════════════════════╳╝  │
    │                 │    ↓YES            ↓NO            │
    │                 │ ┌──────┐      ┌──────────────┐   │
    │                 │ │CONFLICT    │No conflict    │   │
    │                 │ │Status=    │Just update    │   │
    │                 │ │Conflict   │from disk      │   │
    │                 │ │           │Status=Modified│   │
    │                 │ │Both vers  │               │   │
    │                 │ │stored in  │→ Return Ok()  │   │
    │                 │ │metadata   │               │   │
    │                 │ │→Needs     │               │   │
    │                 │ │resolution │               │   │
    │                 │ └──────┘    └──────────────┘   │
    └─────────────────┴────────────────────────────────────┘
```

---

## 7. Performance Characteristics

```
PARALLEL FLUSH EXECUTION

Threads:     CPU-1     CPU-2     CPU-3     CPU-4
             │         │         │         │
Semaphore:   ╔═════════════════════════════════════╗
             ║ Max Workers = num_cpus::get()       ║
             ╚═════════════════════════════════════╝
             │         │         │         │
Files:       [File-1] [File-2] [File-3] [File-4]
             └─────────────────────────────────────┘
                      Write to Disk (concurrent)
                      ↓ ↓ ↓ ↓
                    [Report merged]

Debounce Timeline (File Watcher):

Time:   0ms       100ms      200ms      300ms      500ms
Event:  Write┐    ┌─Read     Modified  ┌─Emit Batch
        ─────┼────┤   Write─┐
             └────┤         └────────┐
                  │                  └─→ Send [Modified]
        
        Coalesce behavior:
        ├─ Detect 3 writes on same file
        ├─ Accumulate for 100ms
        ├─ Merge to single Modified event
        └─ Batch emit at 500ms interval

```

---

## 8. SyncStatus State Machine

```
                        ┌─────────────┐
                        │   CREATED   │ ← VNode newly created in VFS
                        └─────────────┘
                              │
                              │ (flush)
                              ↓
                        ┌─────────────┐
       ┌────────────────│   SYNCED    │◄───────────────┐
       │                └─────────────┘                │
       │                      △                        │
       │                      │                        │
       │        (no external   │ (ext. edit,           │
       │         changes)      │  vfs unchanged)       │
       │                  ┌────┴──────┐                │
       │                  │           ↓                │
       │            ┌──────────────────────┐           │
       │            │    MODIFIED/         │           │
       │            │    CREATED           │           │
       │            └──────────────────────┘           │
       │                  │         △                  │
       │                  │         │                  │
       │ ┌─── conflict?───┴─────────┤                  │
       │ │                          │                  │
       │ │  (both changed)          │ (no conflict)    │
       │ │         ↓                │ (flush resolves)│
       │ │    ┌─────────────┐       └──────────────── │
       │ │    │  CONFLICT   │                         │
       │ │    └─────────────┘                         │
       │ │     (needs manual or auto resolution)      │
       │ │                                             │
       │ └─────────────────────────────────────────── │
       │                                               │
       └───────────────────────────────────────────────┘

       DELETED: Special state
       └─ Marked for removal
       └─ Cleaned up on flush
       └─ Then → SYNCED
```

---

## 9. Conflict Resolution Outcomes

```
Conflict Detected
│ Status = Conflict
│ fs_content_hash stored in metadata
│
├─ Resolution Strategy: "memory" / "vfs"
│  └─ Keep VFS version (user's edits)
│  └─ Mark as SYNCED
│  └─ Disk version discarded (lost!)
│
├─ Resolution Strategy: "disk" / "filesystem"
│  └─ Load disk version into VFS
│  └─ Overwrite VFS changes
│  └─ Mark as SYNCED
│  └─ User's edits lost!
│
├─ Resolution Strategy: "manual" / "merge"
│  └─ Accept provided merge_content parameter
│  └─ Write merged content to VFS
│  └─ Mark as SYNCED
│  └─ Both versions reconciled
│
└─ No Resolution
   └─ Remains in Conflict state
   └─ Blocks flush/sync
   └─ User must resolve manually
```

---

## 10. Type Hierarchy

```
FlushScope
├─ All
├─ Path(VirtualPath)
├─ Specific(Vec<Uuid>)
└─ Workspace(Uuid)

FlushOptions {
    preserve_permissions: bool,    // Unix mode bits
    preserve_timestamps: bool,     // Atime/Mtime
    create_backup: bool,           // Pre-flush backup
    atomic: bool,                  // All-or-nothing
    parallel: bool,                // Concurrent writes
    max_workers: usize,            // CPU-count default
}

SyncStatus
├─ Synced       // In sync with disk
├─ Modified     // VFS changed, not flushed
├─ Created      // New in VFS, not materialized
├─ Deleted      // Marked for deletion
└─ Conflict     // Both VFS and disk modified

VNode {
    path: VirtualPath,             // Virtual path
    node_type: NodeType,           // File/Directory/Symlink
    content_hash: Option<String>,  // Blake3 hash
    status: SyncStatus,            // ← Current state
    version: u32,                  // Incremented on changes
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    accessed_at: DateTime<Utc>,
}
```

