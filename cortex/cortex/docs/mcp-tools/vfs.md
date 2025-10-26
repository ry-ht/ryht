# Virtual Filesystem (VFS) MCP Tools

## Overview

The Virtual Filesystem (VFS) tools provide a comprehensive, path-agnostic abstraction layer for file and directory operations within the Cortex cognitive system. These tools enable AI agents to interact with code and documents in a workspace-isolated, version-aware manner with content deduplication and lazy materialization.

**Total Tools:** 17
**Module:** `cortex::mcp::tools::vfs`
**Status:** Production-ready (15/17 fully functional, 2/17 simplified)

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Concepts](#core-concepts)
3. [Tool Reference](#tool-reference)
4. [Integration with Cognitive System](#integration-with-cognitive-system)
5. [Usage Patterns](#usage-patterns)
6. [Performance Considerations](#performance-considerations)
7. [Limitations & Future Enhancements](#limitations--future-enhancements)

---

## Architecture Overview

### System Context

```
┌─────────────────────────────────────────────────────────────┐
│                     Cortex Cognitive System                 │
│                                                             │
│  ┌────────────────┐      ┌──────────────┐                   │
│  │   MCP Server   │─────▶│  VFS Tools   │                   │
│  │   (AI Agent    │      │  (17 tools)  │                   │
│  │   Interface)   │      └──────┬───────┘                   │
│  └────────────────┘             │                           │
│                                 ▼                           │
│                      ┌──────────────────────┐               │
│                      │    VFS Service       │               │
│                      │  (Business Logic)    │               │
│                      └──────────┬───────────┘               │
│                                 │                           │
│                                 ▼                           │
│                      ┌──────────────────────┐               │
│                      │ VirtualFileSystem    │               │
│                      │  (Core Operations)   │               │
│                      └──────────┬───────────┘               │
│                                 │                           │
│              ┌──────────────────┼───────────────────┐       │
│              ▼                  ▼                   ▼       │
│         ┌─────────┐       ┌──────────┐       ┌──────────┐   │
│         │  Cache  │       │ SurrealDB│       │Filesystem│   │
│         │  (LRU)  │       │(Metadata)│       │ (Disk)   │   │
│         └─────────┘       └──────────┘       └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Component Stack

1. **MCP Tools Layer** (`vfs.rs`)
   - Exposes 17 filesystem operations as MCP tools
   - Handles input validation and output formatting
   - Provides workspace isolation per request

2. **VFS Service Layer** (`services/vfs.rs`)
   - Business logic for common operations
   - Atomic multi-step operations
   - Error handling and logging

3. **Virtual Filesystem Core** (`cortex-vfs` crate)
   - Path-agnostic storage (VirtualPath)
   - Content deduplication (blake3 hashing)
   - Multi-workspace support
   - Change tracking and versioning

4. **Storage Layer** (SurrealDB)
   - Metadata persistence (VNode)
   - Content storage (FileContent)
   - Version history (future)

---

## Core Concepts

### 1. Virtual Paths

Virtual paths are workspace-relative, normalized paths that abstract away physical filesystem locations:

```rust
VirtualPath::new("src/main.rs")  // ✅ Valid
VirtualPath::new("/src/main.rs") // ✅ Also valid (normalized to "src/main.rs")
```

**Key Features:**
- Always relative to workspace root
- Platform-independent (no Windows vs Unix path issues)
- Efficient comparison and hashing

### 2. Workspaces

Workspaces provide complete isolation between different projects or contexts:

- Each workspace has a unique UUID
- Files in one workspace are invisible to others
- Enables parallel work on multiple codebases
- Supports external project imports

**Types:**
- `Main` - User's primary working project
- `External` - Imported reference project
- `Fork` - Copy of another workspace for experimentation

### 3. Content Deduplication

Files with identical content share a single storage entry:

```
File A: src/utils.ts → hash:abc123 ─┐
File B: lib/utils.ts → hash:abc123 ─┼─▶ Content Storage: "abc123"
File C: main.ts      → hash:def456 ─┘
```

**Benefits:**
- Reduced storage usage
- Faster copy operations (reference counting)
- Efficient versioning (only store diffs)

### 4. Lazy Materialization

Files exist in the VFS database until explicitly flushed to disk:

```
Write to VFS → Modified in memory/DB → Flush → Written to physical disk
```

**Advantages:**
- Atomic multi-file operations
- Rollback support
- Conflict detection before disk writes
- Performance optimization (batch writes)

### 5. Node Types

The VFS supports four node types:

- **File** - Regular code/text files
- **Directory** - Container for files
- **SymLink** - Symbolic link to another node
- **Document** - Special type for ingested external documents

### 6. Synchronization Status

Each VNode tracks its sync status:

- `Synced` - In sync with physical filesystem
- `Modified` - Changed in VFS, not yet flushed
- `Created` - New in VFS, not yet materialized
- `Deleted` - Marked for deletion, not yet removed
- `Conflict` - Conflict detected during sync

---

## Tool Reference

### Basic Operations

#### 1. `cortex.vfs.get_node`

Retrieves a virtual node (file or directory) by path.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/main.rs",
  "include_content": true,
  "include_metadata": false
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "node_type": "file",
  "name": "main.rs",
  "path": "src/main.rs",
  "content": "fn main() { ... }",
  "size_bytes": 1024,
  "permissions": "644",
  "version": 1
}
```

**Use Cases:**
- Read file content for analysis
- Check file existence and type
- Get file metadata for decision-making

---

#### 2. `cortex.vfs.get_node_by_id` ⭐ NEW

Retrieves a virtual node by its unique ID (faster than path-based lookup).

**Input:**
```json
{
  "node_id": "uuid",
  "include_content": true,
  "include_metadata": true
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "node_type": "file",
  "name": "utils.ts",
  "path": "src/utils.ts",
  "content": "export const ...",
  "size_bytes": 2048,
  "permissions": "644",
  "metadata": {
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-20T14:22:00Z",
    "language": "typescript"
  },
  "version": 3
}
```

**Use Cases:**
- Quick node retrieval when ID is known
- Following references between files
- Implementing caching strategies

**Performance:** O(1) lookup vs O(log n) for path-based

---

#### 3. `cortex.vfs.list_directory`

Lists contents of a virtual directory with filtering.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src",
  "recursive": false,
  "include_hidden": false,
  "filter": {
    "node_type": "file",
    "language": "rust"
  }
}
```

**Output:**
```json
{
  "entries": [
    {
      "name": "main.rs",
      "path": "src/main.rs",
      "node_type": "file",
      "size_bytes": 1024
    }
  ],
  "total": 1
}
```

**Filter Options:**
- `node_type` - "file" or "directory"
- `language` - Filter by programming language
- `pattern` - Glob pattern matching

**Use Cases:**
- Explore directory structure
- Find all files of a specific type
- Generate project overviews

---

#### 4. `cortex.vfs.exists` ⭐ NEW

Checks if a path exists without retrieving full metadata.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/config.json"
}
```

**Output:**
```json
{
  "path": "src/config.json",
  "exists": true,
  "node_type": "file"
}
```

**Use Cases:**
- Pre-flight validation before operations
- Conditional file creation
- Avoiding exceptions in control flow

**Performance:** Lightweight operation, minimal data transfer

---

### File Operations

#### 5. `cortex.vfs.create_file`

Creates a new file in the virtual filesystem.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/new_module.rs",
  "content": "pub mod utils;",
  "encoding": "utf-8",
  "permissions": "644",
  "parse": true
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "path": "src/new_module.rs",
  "size_bytes": 15,
  "version": 1
}
```

**Features:**
- Automatic parent directory creation
- Content hashing for deduplication
- Optional syntax parsing
- Language detection from extension

**Use Cases:**
- Generate new code files
- Create configuration files
- Scaffold project structure

---

#### 6. `cortex.vfs.update_file`

Updates file content with optimistic locking.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/main.rs",
  "content": "fn main() { println!(\"Updated\"); }",
  "expected_version": 1,
  "encoding": "utf-8",
  "reparse": true
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "path": "src/main.rs",
  "version": 2,
  "size_bytes": 42
}
```

**Version Control:**
- `expected_version` prevents race conditions
- Returns error if version mismatch detected
- Automatically increments version on success

**Use Cases:**
- Safe concurrent editing
- Implement edit workflows
- Track file modifications

---

#### 7. `cortex.vfs.batch_create_files` ⭐ NEW

Creates multiple files atomically in a single operation.

**Input:**
```json
{
  "workspace_id": "uuid",
  "files": [
    {
      "path": "src/module1.rs",
      "content": "pub fn foo() {}"
    },
    {
      "path": "src/module2.rs",
      "content": "pub fn bar() {}"
    },
    {
      "path": "tests/test_module1.rs",
      "content": "#[test] fn test_foo() {}"
    }
  ]
}
```

**Output:**
```json
{
  "created": [
    {
      "path": "src/module1.rs",
      "success": true,
      "node_id": "uuid1"
    },
    {
      "path": "src/module2.rs",
      "success": true,
      "node_id": "uuid2"
    },
    {
      "path": "tests/test_module1.rs",
      "success": true,
      "node_id": "uuid3"
    }
  ],
  "total_created": 3,
  "total_failed": 0
}
```

**Features:**
- Continues on individual failures (non-atomic)
- Reports success/failure for each file
- Significantly faster than individual creates
- Preserves directory structure

**Use Cases:**
- Scaffold entire project structures
- Generate multiple related files
- Bulk import operations
- Code generation workflows

**Performance:** ~5-10x faster than sequential creates

---

#### 8. `cortex.vfs.delete_node`

Deletes a file or directory.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/old_module.rs",
  "recursive": false,
  "expected_version": 2
}
```

**Output:**
```json
{
  "path": "src/old_module.rs",
  "deleted": true
}
```

**Safety Features:**
- `recursive` required for non-empty directories
- `expected_version` for safe deletion
- Content hash preserved (for potential undo)

**Use Cases:**
- Clean up generated files
- Remove deprecated code
- Implement file management workflows

---

### Directory Operations

#### 9. `cortex.vfs.create_directory`

Creates a new directory.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/modules/core",
  "permissions": "755",
  "create_parents": true
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "path": "src/modules/core"
}
```

**Features:**
- Automatic parent creation with `create_parents`
- Unix-style permissions
- Idempotent (succeeds if already exists)

---

#### 10. `cortex.vfs.get_tree`

Gets directory tree structure with depth control.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "/",
  "max_depth": 3,
  "include_files": true
}
```

**Output:**
```json
{
  "root": {
    "name": "/",
    "path": "/",
    "node_type": "directory",
    "children": [
      {
        "name": "src",
        "path": "src",
        "node_type": "directory",
        "children": [
          {
            "name": "main.rs",
            "path": "src/main.rs",
            "node_type": "file",
            "size_bytes": 1024
          }
        ]
      }
    ]
  },
  "total_nodes": 3
}
```

**Performance Control:**
- `max_depth` limits tree depth
- `include_files` can exclude files (directories only)
- Recursive tree building with pruning

**Use Cases:**
- Generate project visualizations
- Directory structure analysis
- Navigation assistance for AI agents

---

### Advanced Operations

#### 11. `cortex.vfs.move_node`

Moves or renames a node.

**Input:**
```json
{
  "workspace_id": "uuid",
  "source_path": "src/old_name.rs",
  "target_path": "src/new_name.rs",
  "overwrite": false
}
```

**Output:**
```json
{
  "source_path": "src/old_name.rs",
  "target_path": "src/new_name.rs",
  "moved": true
}
```

**Implementation:** Currently implemented as copy + delete. Future versions will support atomic move.

**Use Cases:**
- File renaming
- Reorganize project structure
- Refactoring support

---

#### 12. `cortex.vfs.copy_node`

Copies a node to a new location.

**Input:**
```json
{
  "workspace_id": "uuid",
  "source_path": "src/template.rs",
  "target_path": "src/new_module.rs",
  "recursive": true,
  "overwrite": false
}
```

**Output:**
```json
{
  "source_path": "src/template.rs",
  "target_path": "src/new_module.rs",
  "copied": true
}
```

**Features:**
- Recursive directory copying
- Content deduplication (references same hash)
- Overwrite protection
- Preserves metadata

**Use Cases:**
- Template instantiation
- Duplicate file/directory structures
- Backup operations

---

#### 13. `cortex.vfs.create_symlink` ⭐ NEW

Creates a symbolic link in the virtual filesystem.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/alias.rs",
  "target": "src/original/module.rs"
}
```

**Output:**
```json
{
  "node_id": "uuid",
  "path": "src/alias.rs",
  "target": "src/original/module.rs"
}
```

**Features:**
- Virtual symlink support
- Target validation optional
- Preserves target path metadata

**Use Cases:**
- Create file aliases
- Maintain backward compatibility
- Implement virtual module systems

---

### Search & Discovery

#### 14. `cortex.vfs.search_files`

Searches for files by pattern or content.

**Input:**
```json
{
  "workspace_id": "uuid",
  "pattern": "*.rs",
  "base_path": "/",
  "search_content": false,
  "case_sensitive": false,
  "max_results": 100
}
```

**Output:**
```json
{
  "matches": [
    {
      "path": "src/main.rs",
      "node_type": "file",
      "size_bytes": 1024,
      "match_type": "filename"
    }
  ],
  "total": 1,
  "truncated": false
}
```

**Search Modes:**
- **Filename:** Glob pattern matching (`*.rs`, `test_*.py`)
- **Content:** Full-text search in file contents
- **Combined:** Search both filename and content

**Pattern Syntax:**
- `*` - Match any characters
- `?` - Match single character
- Plain text - Contains match

**Use Cases:**
- Find files by name pattern
- Content-based discovery
- Code archaeology

**Performance:** Content search can be slow for large workspaces

---

#### 15. `cortex.vfs.get_workspace_stats` ⭐ NEW

Gets comprehensive statistics about workspace structure and usage.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "/"
}
```

**Output:**
```json
{
  "workspace_id": "uuid",
  "path": "/",
  "total_files": 127,
  "total_directories": 23,
  "total_size_bytes": 524288,
  "file_types": {
    "rs": 45,
    "ts": 38,
    "md": 12,
    "json": 8
  },
  "language_distribution": {
    "rust": 45,
    "typescript": 38,
    "unknown": 19
  }
}
```

**Metrics Provided:**
- File and directory counts
- Total storage usage
- File type distribution (by extension)
- Programming language distribution
- Can be scoped to subdirectory

**Use Cases:**
- Workspace health monitoring
- Project composition analysis
- Storage optimization planning
- AI agent context building

**Performance:** O(n) where n = number of nodes in scope

---

### Version History (Simplified)

#### 16. `cortex.vfs.get_file_history` ⚠️ SIMPLIFIED

Retrieves version history of a file.

**Status:** Currently returns only the current version. Full version history requires database schema enhancement.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/main.rs",
  "max_versions": 10,
  "include_content": false
}
```

**Current Output:**
```json
{
  "path": "src/main.rs",
  "current_version": 5,
  "versions": [
    {
      "version": 5,
      "content_hash": "abc123...",
      "size_bytes": 1024,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-20T14:22:00Z"
    }
  ],
  "total_versions": 1
}
```

**Future Enhancement:**
- Separate `version_history` table in database
- Store historical content hashes
- Support time-based and version-based queries

---

#### 17. `cortex.vfs.restore_file_version` ⚠️ SIMPLIFIED

Restores a file to a previous version.

**Status:** Currently only supports restoring to the current version. Full restore requires version history table.

**Input:**
```json
{
  "workspace_id": "uuid",
  "path": "src/main.rs",
  "version": 3,
  "create_backup": true
}
```

**Current Behavior:**
- If `version == current_version`: Success
- If `version < current_version`: Error (not yet implemented)
- If `create_backup`: Creates backup copy

**Future Enhancement:**
- Implement version history storage
- Support rollback to any version
- Automatic conflict resolution

---

## Integration with Cognitive System

### 1. Relationship with Other Subsystems

#### Workspace Management
```
cortex.workspace.* ─┐
                    ├─▶ Manages workspace lifecycle
                    │   VFS tools operate within workspaces
cortex.vfs.*       ─┘
```

- Workspaces create the isolation boundary
- VFS tools require `workspace_id` for all operations
- Active workspace concept provides default context

#### Code Navigation
```
cortex.code.* ─────▶ Reads parsed AST data ─────▶ VFS stores source
                     (symbols, references)
```

- Code navigation tools read from parsed units
- Parsed units reference VFS nodes by ID
- Changes to VFS trigger re-parsing

#### Code Manipulation
```
cortex.code.create_unit ─┐
cortex.code.update_unit ─┼─▶ Modifies VFS ─▶ Triggers parsing
cortex.code.delete_unit ─┘
```

- High-level code operations use VFS as storage backend
- Manipulations trigger automatic re-parsing
- Maintains bi-directional sync (code ↔ files)

#### Semantic Search
```
cortex.search.* ────▶ Queries embeddings ────▶ References VFS nodes
```

- Search results include VFS node IDs
- Content changes trigger embedding updates
- VFS content used for semantic indexing

#### Materialization
```
cortex.materialization.flush ────▶ Writes VFS to disk
cortex.materialization.sync  ────▶ Reads disk into VFS
```

- Materialization tools bridge VFS ↔ Physical FS
- Flush writes pending changes to disk
- Sync imports external changes

#### Fork Management
```
cortex.fork.create ────▶ Duplicates VFS workspace
cortex.fork.merge  ────▶ Merges changes between workspaces
```

- Forks operate at VFS level
- Content deduplication makes forks efficient
- Merge tools resolve conflicts

#### Cognitive Memory
```
cortex.memory.* ────▶ Stores context about files
                      (learnings, patterns, decisions)
```

- Memory system references VFS nodes
- Maintains associations between files and insights
- Provides "why" context for "what" in VFS

---

### 2. Event Flow Example

**Scenario:** AI agent creates a new Rust module

```
1. Agent decides to create src/utils.rs
   ↓
2. cortex.vfs.exists
   → path: "src/utils.rs"
   ← exists: false
   ↓
3. cortex.vfs.create_file
   → path: "src/utils.rs"
   → content: "pub fn helper() { ... }"
   ← node_id: "uuid-123"
   ← version: 1
   ↓
4. [Automatic] Code parsing triggered
   - TreeSitter parses content
   - Creates CodeUnit in database
   - Links to VFS node ID "uuid-123"
   ↓
5. [Automatic] Semantic indexing
   - Content embedded
   - Added to vector database
   - Searchable via cortex.search.*
   ↓
6. cortex.code.get_symbols
   → unit_id: "uuid-123"
   ← symbols: [{ name: "helper", kind: "function", ... }]
   ↓
7. cortex.materialization.flush
   → scope: "modified"
   ← flushed: ["src/utils.rs"]
   [Physical file created on disk]
```

---

### 3. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Tool Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ VFS Tools│  │Code Tools│  │Search    │  │Materialization│  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───────┬───────┘  │
└───────┼─────────────┼─────────────┼─────────────────┼───────────┘
        │             │             │                 │
        ▼             ▼             ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Service Layer                               │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐  ┌──────────────┐ │
│  │VfsService│  │CodeService│ │SearchService│ │MaterialEngine│ │
│  └────┬─────┘  └────┬─────┘  └─────┬──────┘  └──────┬───────┘ │
└───────┼─────────────┼───────────────┼─────────────────┼─────────┘
        │             │               │                 │
        ▼             │               │                 ▼
┌──────────────────┐  │               │        ┌──────────────────┐
│VirtualFileSystem │  │               │        │  Physical FS     │
│   (cortex-vfs)   │◀─┘               │        │   (Disk I/O)     │
└────────┬─────────┘                  │        └──────────────────┘
         │                            │
         ▼                            ▼
┌────────────────────────────────────────────┐
│           SurrealDB Storage                │
│  ┌────────┐  ┌──────────┐  ┌───────────┐ │
│  │ VNodes │  │CodeUnits │  │Embeddings │ │
│  └────────┘  └──────────┘  └───────────┘ │
│  ┌────────────┐  ┌─────────────────────┐ │
│  │FileContent │  │   Metadata Tables   │ │
│  └────────────┘  └─────────────────────┘ │
└────────────────────────────────────────────┘
```

---

## Usage Patterns

### Pattern 1: Safe File Update

**Problem:** Update a file without race conditions

```javascript
// 1. Get current state
const node = await mcp.call("cortex.vfs.get_node", {
  workspace_id: "uuid",
  path: "src/config.json",
  include_content: true
});

// 2. Modify content
const config = JSON.parse(node.content);
config.feature_flags.new_feature = true;
const newContent = JSON.stringify(config, null, 2);

// 3. Update with version check
await mcp.call("cortex.vfs.update_file", {
  workspace_id: "uuid",
  path: "src/config.json",
  content: newContent,
  expected_version: node.version // Optimistic locking
});
```

---

### Pattern 2: Bulk File Creation

**Problem:** Create multiple related files efficiently

```javascript
// Instead of:
for (const file of files) {
  await mcp.call("cortex.vfs.create_file", {
    workspace_id: "uuid",
    path: file.path,
    content: file.content
  });
}

// Use:
const result = await mcp.call("cortex.vfs.batch_create_files", {
  workspace_id: "uuid",
  files: files.map(f => ({ path: f.path, content: f.content }))
});

// Handle partial failures
const failed = result.created.filter(f => !f.success);
if (failed.length > 0) {
  console.log("Failed files:", failed);
}
```

**Performance:** ~5-10x faster for 10+ files

---

### Pattern 3: Workspace Analysis

**Problem:** Understand workspace composition

```javascript
// Get high-level stats
const stats = await mcp.call("cortex.vfs.get_workspace_stats", {
  workspace_id: "uuid",
  path: "/"
});

console.log(`Total files: ${stats.total_files}`);
console.log(`Total size: ${(stats.total_size_bytes / 1024 / 1024).toFixed(2)} MB`);
console.log(`Language breakdown:`, stats.language_distribution);

// Find largest directory
const dirs = ["src", "tests", "docs"];
const dirStats = await Promise.all(
  dirs.map(dir => mcp.call("cortex.vfs.get_workspace_stats", {
    workspace_id: "uuid",
    path: dir
  }))
);

const largestDir = dirStats.reduce((max, curr) =>
  curr.total_size_bytes > max.total_size_bytes ? curr : max
);
console.log(`Largest directory: ${largestDir.path}`);
```

---

### Pattern 4: Search and Process

**Problem:** Find and process matching files

```javascript
// Find all test files
const searchResult = await mcp.call("cortex.vfs.search_files", {
  workspace_id: "uuid",
  pattern: "test_*.py",
  base_path: "tests",
  max_results: 1000
});

// Process each test file
const results = [];
for (const match of searchResult.matches) {
  const node = await mcp.call("cortex.vfs.get_node", {
    workspace_id: "uuid",
    path: match.path,
    include_content: true
  });

  // Analyze test coverage
  const testCount = (node.content.match(/@test/g) || []).length;
  results.push({ path: match.path, tests: testCount });
}

console.log(`Total tests: ${results.reduce((sum, r) => sum + r.tests, 0)}`);
```

---

### Pattern 5: Safe Deletion

**Problem:** Delete with validation

```javascript
// Check if file exists first
const existsResult = await mcp.call("cortex.vfs.exists", {
  workspace_id: "uuid",
  path: "src/deprecated.rs"
});

if (!existsResult.exists) {
  console.log("File already deleted");
  return;
}

// Get current version for safe deletion
const node = await mcp.call("cortex.vfs.get_node", {
  workspace_id: "uuid",
  path: "src/deprecated.rs"
});

// Delete with version check
await mcp.call("cortex.vfs.delete_node", {
  workspace_id: "uuid",
  path: "src/deprecated.rs",
  expected_version: node.version,
  recursive: false
});
```

---

### Pattern 6: Directory Tree Exploration

**Problem:** Build navigation context for AI

```javascript
// Get high-level structure
const tree = await mcp.call("cortex.vfs.get_tree", {
  workspace_id: "uuid",
  path: "/",
  max_depth: 2,
  include_files: false // Directories only
});

// Find interesting directories
const srcDirs = findNodesByType(tree.root, "directory", "src");
console.log(`Source directories: ${srcDirs.map(d => d.path).join(", ")}`);

// Deep dive into specific directory
const srcTree = await mcp.call("cortex.vfs.get_tree", {
  workspace_id: "uuid",
  path: "src",
  max_depth: 5,
  include_files: true
});

// Generate directory listing
console.log(formatTree(srcTree.root));
```

---

## Performance Considerations

### 1. Operation Complexity

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| `get_node` | O(log n) | O(1) | B-tree path lookup |
| `get_node_by_id` | O(1) | O(1) | Direct ID lookup |
| `list_directory` | O(k) | O(k) | k = children count |
| `create_file` | O(log n) | O(m) | m = content size |
| `update_file` | O(log n) | O(m) | With version check |
| `delete_node` | O(log n + k) | O(1) | k = if recursive |
| `move_node` | O(log n + m) | O(m) | Copy + delete |
| `copy_node` | O(log n + k*m) | O(1) | Deduplication |
| `get_tree` | O(k^d) | O(k^d) | k = fanout, d = depth |
| `search_files` | O(n) | O(m) | m = matches |
| `get_workspace_stats` | O(n) | O(1) | Full scan |
| `batch_create_files` | O(k log n) | O(k*m) | k = file count |

### 2. Optimization Strategies

#### Use ID-based lookups when possible
```javascript
// Slower
const node = await get_node({ path: "src/utils.rs" });

// Faster
const node = await get_node_by_id({ node_id: knownId });
```

#### Limit tree depth
```javascript
// Expensive for large projects
const tree = await get_tree({ max_depth: 10 }); // ❌

// Efficient
const tree = await get_tree({ max_depth: 3 }); // ✅
```

#### Use batch operations
```javascript
// Slow: N round trips
for (const file of files) {
  await create_file(file);
}

// Fast: 1 round trip
await batch_create_files({ files });
```

#### Cache frequently accessed data
```javascript
// Cache workspace stats (rarely changes)
const stats = cache.get("workspace_stats") ||
  await get_workspace_stats({ workspace_id: "uuid" });
cache.set("workspace_stats", stats, 3600); // 1 hour TTL
```

#### Use exists() instead of get_node() for checks
```javascript
// Wasteful
try {
  await get_node({ path: "config.json" });
  // File exists
} catch (e) {
  // File doesn't exist
}

// Efficient
const exists = await exists({ path: "config.json" });
if (exists.exists) { ... }
```

---

### 3. Content Deduplication Benefits

Content deduplication provides significant performance benefits:

**Storage:**
- Identical files share storage: O(1) space per unique content
- Forks/copies nearly free for unchanged files
- Example: 10 forks of 1GB codebase = ~1GB storage (not 10GB)

**Copy Operations:**
- Reference counting instead of content copying
- Copy = increment counter + create metadata
- ~100x faster for large files

**Network Transfer:**
- Only unique content needs transmission
- Sync operations transfer diffs only
- Reduces bandwidth for distributed systems

---

### 4. Lazy Materialization Impact

**Advantages:**
- Multiple edits don't hit disk repeatedly
- Atomic multi-file operations
- Faster read-write cycles (memory/DB vs disk)

**Considerations:**
- Must call `materialization.flush` to persist
- External tools see stale files until flush
- Memory usage grows with unflushed changes

**Best Practice:**
```javascript
// 1. Make changes in VFS
await batch_create_files({ files: [...] });
await update_file({ path: "src/main.rs", ... });

// 2. Validate changes
const errors = await validate();
if (errors.length > 0) {
  // Changes stay in VFS, not on disk
  return;
}

// 3. Flush to disk
await materialization.flush({ scope: "modified" });
```

---

## Limitations & Future Enhancements

### Current Limitations

#### 1. Version History (Simplified)
**Status:** Only current version tracked

**Impact:**
- Cannot restore to previous versions
- No version history queries
- Limited rollback capability

**Workaround:**
- Use version control tools (`cortex.vcs.*`)
- External git integration
- Manual backup with copy operations

**Future Enhancement:**
```sql
-- Planned database schema
CREATE TABLE version_history (
  id UUID,
  vnode_id UUID,
  version INT,
  content_hash STRING,
  created_at DATETIME,
  author STRING,
  message STRING
);
```

---

#### 2. Move Operation (Non-Atomic)
**Status:** Implemented as copy + delete

**Impact:**
- Not atomic (can fail midway)
- Slower than native move
- Temporary space usage

**Workaround:**
- Use copy + delete pattern explicitly
- Check disk space before large moves

**Future Enhancement:**
- Native atomic move in VirtualFileSystem
- Database-level transaction support

---

#### 3. Symlink Resolution
**Status:** Symlinks created but not automatically resolved

**Impact:**
- `get_node` on symlink returns symlink, not target
- Manual target resolution required
- No cycle detection

**Workaround:**
```javascript
const node = await get_node({ path: "alias.rs" });
if (node.node_type === "symlink") {
  const target = node.metadata.target;
  const actual = await get_node({ path: target });
  // Use actual
}
```

**Future Enhancement:**
- Auto-resolution option in get_node
- Cycle detection and prevention
- Symlink path normalization

---

#### 4. Large File Handling
**Status:** All content loaded into memory

**Impact:**
- Large files (>100MB) can cause memory pressure
- Binary files inefficiently stored as base64
- No streaming support

**Workaround:**
- Avoid VFS for large binary assets
- Use external file storage for media
- Compress large text files

**Future Enhancement:**
- Streaming read/write API
- Chunk-based storage for large files
- Native binary content type

---

#### 5. Concurrent Modification
**Status:** Optimistic locking only

**Impact:**
- Last writer wins if versions not checked
- No pessimistic locking
- Manual conflict resolution

**Workaround:**
- Always use `expected_version`
- Implement retry logic
- Use workspace forks for parallel work

**Future Enhancement:**
- Pessimistic locking option
- Automatic conflict resolution strategies
- Lock management API

---

#### 6. Search Performance
**Status:** Content search is O(n)

**Impact:**
- Slow for large workspaces (1000+ files)
- No index on content
- Regex patterns not supported

**Workaround:**
- Use semantic search for content queries
- Filter by language/path first
- Set `max_results` to limit work

**Future Enhancement:**
- Full-text search index (Tantivy/Elasticsearch)
- Regex pattern support
- Incremental index updates

---

### Planned Enhancements

#### Short Term (Next 3-6 months)

1. **Full Version History**
   - Separate version_history table
   - Time-travel queries
   - Diff generation between versions

2. **Enhanced Search**
   - Full-text index (Tantivy)
   - Regex pattern support
   - Faster content search (100x improvement)

3. **Symlink Auto-Resolution**
   - Optional auto-follow in get_node
   - Cycle detection
   - Relative path handling

4. **Batch Operations**
   - `batch_update_files`
   - `batch_delete_nodes`
   - Transaction support for rollback

5. **Metadata Operations**
   - `update_metadata` - Modify timestamps, permissions
   - `set_read_only` - Lock files
   - `set_language` - Override language detection

#### Medium Term (6-12 months)

1. **Streaming API**
   - Read/write large files in chunks
   - Progress callbacks
   - Memory-efficient processing

2. **Advanced Conflict Resolution**
   - Three-way merge support
   - Automatic resolution strategies
   - Interactive conflict UI

3. **File Watching**
   - Real-time change notifications
   - Watch-based sync
   - Event streaming to MCP clients

4. **Compression**
   - Automatic compression for large files
   - Multiple compression algorithms
   - Transparent decompression

5. **Access Control**
   - File-level permissions
   - Role-based access control
   - Audit logging

#### Long Term (12+ months)

1. **Distributed VFS**
   - Multi-node synchronization
   - Conflict-free replicated data types (CRDTs)
   - Peer-to-peer file sharing

2. **Advanced Deduplication**
   - Block-level deduplication
   - Delta compression between versions
   - Similarity-based storage

3. **AI-Assisted Features**
   - Semantic file organization
   - Intelligent merging
   - Predictive pre-fetching

4. **Performance Optimizations**
   - In-memory caching layer
   - Query optimization
   - Parallel operations

---

## Conclusion

The VFS MCP tools provide a robust, feature-rich abstraction layer for filesystem operations in the Cortex cognitive system. With 17 tools (15 fully functional, 2 simplified), the system supports:

✅ **Complete file/directory CRUD operations**
✅ **Workspace isolation and multi-tenancy**
✅ **Content deduplication and efficient storage**
✅ **Lazy materialization with atomic operations**
✅ **Advanced search and discovery**
✅ **Comprehensive workspace analytics**
✅ **Batch operations for performance**
✅ **Optimistic locking for safe concurrent access**

The VFS serves as the foundational storage layer for all code-related operations in Cortex, enabling AI agents to reason about and manipulate code at both the file and semantic levels.

---

## See Also

- **Workspace Tools** (`docs/mcp-tools/workspace.md`) - Workspace lifecycle management
- **Code Navigation Tools** (`docs/mcp-tools/code-navigation.md`) - AST-level operations
- **Materialization Tools** (`docs/mcp-tools/materialization.md`) - VFS ↔ Disk synchronization
- **Semantic Search Tools** (`docs/mcp-tools/semantic-search.md`) - Content-based discovery
- **Fork Management** (`docs/mcp-tools/fork-management.md`) - Workspace versioning

---

**Document Version:** 1.0
**Last Updated:** 2025-01-20
**Status:** Complete
**Maintainer:** Cortex Core Team
