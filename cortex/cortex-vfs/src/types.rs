//! Core types for the Virtual Filesystem.

use crate::path::VirtualPath;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// A virtual node representing a file, directory, or symlink in the VFS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VNode {
    /// Unique identifier
    pub id: Uuid,

    /// Workspace this vnode belongs to
    pub workspace_id: Uuid,

    /// Virtual path (always relative to repo root)
    pub path: VirtualPath,

    /// Type of node
    pub node_type: NodeType,

    /// Content hash (for files)
    pub content_hash: Option<String>,

    /// Size in bytes
    pub size_bytes: usize,

    /// Whether this node is read-only
    pub read_only: bool,

    /// Original physical path (for external files)
    pub source_path: Option<PathBuf>,

    /// Language detection (for code files)
    pub language: Option<Language>,

    /// Permissions (Unix-style)
    pub permissions: Option<u32>,

    /// Synchronization status
    pub status: SyncStatus,

    /// Version number
    pub version: u32,

    /// Extended metadata
    pub metadata: HashMap<String, Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,

    /// Last access timestamp
    pub accessed_at: DateTime<Utc>,
}

impl VNode {
    /// Create a new file vnode.
    pub fn new_file(
        workspace_id: Uuid,
        path: VirtualPath,
        content_hash: String,
        size_bytes: usize,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            path,
            node_type: NodeType::File,
            content_hash: Some(content_hash),
            size_bytes,
            read_only: false,
            source_path: None,
            language: None,
            permissions: Some(0o644),
            status: SyncStatus::Modified,
            version: 1,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            accessed_at: now,
        }
    }

    /// Create a new directory vnode.
    pub fn new_directory(workspace_id: Uuid, path: VirtualPath) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            path,
            node_type: NodeType::Directory,
            content_hash: None,
            size_bytes: 0,
            read_only: false,
            source_path: None,
            language: None,
            permissions: Some(0o755),
            status: SyncStatus::Modified,
            version: 1,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            accessed_at: now,
        }
    }

    /// Create a new symlink vnode.
    pub fn new_symlink(workspace_id: Uuid, path: VirtualPath, target: String) -> Self {
        let now = Utc::now();
        let mut metadata = HashMap::new();
        metadata.insert("target".to_string(), Value::String(target));

        Self {
            id: Uuid::new_v4(),
            workspace_id,
            path,
            node_type: NodeType::SymLink,
            content_hash: None,
            size_bytes: 0,
            read_only: false,
            source_path: None,
            language: None,
            permissions: Some(0o777),
            status: SyncStatus::Modified,
            version: 1,
            metadata,
            created_at: now,
            updated_at: now,
            accessed_at: now,
        }
    }

    /// Check if this is a file.
    pub fn is_file(&self) -> bool {
        matches!(self.node_type, NodeType::File)
    }

    /// Check if this is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self.node_type, NodeType::Directory)
    }

    /// Check if this is a symlink.
    pub fn is_symlink(&self) -> bool {
        matches!(self.node_type, NodeType::SymLink)
    }

    /// Mark as synchronized with physical filesystem.
    pub fn mark_synchronized(&mut self) {
        self.status = SyncStatus::Synchronized;
        self.updated_at = Utc::now();
    }

    /// Mark as modified.
    pub fn mark_modified(&mut self) {
        self.status = SyncStatus::Modified;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

/// Type of virtual node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    File,
    Directory,
    SymLink,
    Document, // Special type for ingested documents
}

/// Synchronization status of a vnode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// In sync with physical filesystem
    Synchronized,
    /// Modified in VFS, not yet flushed
    Modified,
    /// Created in VFS, not yet materialized
    Created,
    /// Deleted in VFS, not yet removed from disk
    Deleted,
    /// Conflict detected during sync
    Conflict,
}

/// Detected programming language for code files.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Elixir,
    Clojure,
    Zig,
    Unknown,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Language::Cpp,
            "c" | "h" => Language::C,
            "cs" => Language::CSharp,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            "scala" | "sc" => Language::Scala,
            "hs" | "lhs" => Language::Haskell,
            "ex" | "exs" => Language::Elixir,
            "clj" | "cljs" | "cljc" | "edn" => Language::Clojure,
            "zig" => Language::Zig,
            _ => Language::Unknown,
        }
    }
}

/// File content stored with deduplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    /// Content hash (blake3)
    pub content_hash: String,

    /// UTF-8 text content
    pub content: Option<String>,

    /// Binary content
    pub content_binary: Option<Vec<u8>>,

    /// Whether content is compressed
    pub is_compressed: bool,

    /// Compression algorithm used
    pub compression_type: Option<CompressionType>,

    /// Size in bytes (uncompressed)
    pub size_bytes: usize,

    /// Number of lines (for text files)
    pub line_count: Option<usize>,

    /// Reference count
    pub reference_count: usize,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Compression algorithm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Workspace representing a project or external content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: Uuid,

    /// Human-readable name
    pub name: String,

    /// Type of workspace
    pub workspace_type: WorkspaceType,

    /// Source type (local, external, fork)
    pub source_type: SourceType,

    /// Database namespace for isolation
    pub namespace: String,

    /// Original physical path (if applicable)
    pub source_path: Option<PathBuf>,

    /// Whether workspace is read-only
    pub read_only: bool,

    /// Parent workspace (for forks)
    pub parent_workspace: Option<Uuid>,

    /// Fork metadata (if this is a fork)
    pub fork_metadata: Option<ForkMetadata>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Type of workspace.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceType {
    /// Source code project
    Code,
    /// Documentation project
    Documentation,
    /// Mixed content
    Mixed,
    /// External library/dependency
    External,
}

/// Source type for workspace.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Local development workspace
    Local,
    /// External read-only content
    ExternalReadOnly,
    /// Fork of another workspace
    Fork,
}

/// Metadata for forked workspaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkMetadata {
    /// Original workspace ID
    pub source_id: Uuid,

    /// Original workspace name
    pub source_name: String,

    /// When the fork was created
    pub fork_point: DateTime<Utc>,

    /// Commit/version at fork point
    pub fork_commit: Option<String>,
}

/// Scope for flush operations.
#[derive(Debug, Clone)]
pub enum FlushScope {
    /// Flush all modified vnodes
    All,

    /// Flush vnodes under a specific path
    Path(VirtualPath),

    /// Flush specific vnodes by ID
    Specific(Vec<Uuid>),

    /// Flush a specific workspace
    Workspace(Uuid),
}

/// Report from a flush operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlushReport {
    /// Number of files written
    pub files_written: usize,

    /// Number of directories created
    pub directories_created: usize,

    /// Number of symlinks created
    pub symlinks_created: usize,

    /// Number of files deleted
    pub files_deleted: usize,

    /// Total bytes written
    pub bytes_written: usize,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Options for flush operations.
#[derive(Debug, Clone)]
pub struct FlushOptions {
    /// Preserve file permissions
    pub preserve_permissions: bool,

    /// Preserve timestamps
    pub preserve_timestamps: bool,

    /// Create backup before flush
    pub create_backup: bool,

    /// Atomic operation (all or nothing)
    pub atomic: bool,

    /// Parallel materialization
    pub parallel: bool,

    /// Maximum parallel workers
    pub max_workers: usize,
}

impl Default for FlushOptions {
    fn default() -> Self {
        Self {
            preserve_permissions: true,
            preserve_timestamps: true,
            create_backup: false,
            atomic: true,
            parallel: true,
            max_workers: num_cpus::get(),
        }
    }
}

/// Options for importing external projects.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Import as read-only
    pub read_only: bool,

    /// Create editable fork
    pub create_fork: bool,

    /// Namespace for isolation
    pub namespace: String,

    /// File patterns to include
    pub include_patterns: Vec<String>,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Maximum directory depth
    pub max_depth: Option<usize>,

    /// Parse and analyze code
    pub process_code: bool,

    /// Generate semantic embeddings
    pub generate_embeddings: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            read_only: true,
            create_fork: false,
            namespace: format!("external_{}", Uuid::new_v4()),
            include_patterns: vec!["**/*".to_string()],
            exclude_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
            max_depth: None,
            process_code: true,
            generate_embeddings: false,
        }
    }
}

/// Report from an import operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportReport {
    /// Workspace ID created
    pub workspace_id: Uuid,

    /// Number of files imported
    pub files_imported: usize,

    /// Number of directories imported
    pub directories_imported: usize,

    /// Number of code units extracted
    pub units_extracted: usize,

    /// Total bytes imported
    pub bytes_imported: usize,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Merge strategy for fork merging.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Prompt for manual conflict resolution
    Manual,

    /// Attempt automatic three-way merge
    AutoMerge,

    /// Prefer fork version on conflict
    PreferFork,

    /// Prefer target version on conflict
    PreferTarget,
}

/// Report from a merge operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeReport {
    /// Number of changes applied
    pub changes_applied: usize,

    /// Number of conflicts encountered
    pub conflicts_count: usize,

    /// Conflicts that need resolution
    pub conflicts: Vec<Conflict>,

    /// Number of conflicts auto-resolved
    pub auto_resolved: usize,

    /// Errors encountered
    pub errors: Vec<String>,
}

/// A merge conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Path where conflict occurred
    pub path: VirtualPath,

    /// Content from fork
    pub fork_content: String,

    /// Content from target
    pub target_content: String,

    /// Resolved content (if resolved)
    pub resolution: Option<String>,
}

/// Change record for tracking modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Change ID
    pub id: Uuid,

    /// Vnode ID
    pub vnode_id: Uuid,

    /// Virtual path
    pub path: VirtualPath,

    /// Type of change
    pub change_type: ChangeType,

    /// New content hash (if applicable)
    pub new_content_hash: Option<String>,

    /// Agent/session that made the change
    pub changed_by: Option<String>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Type of change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}
