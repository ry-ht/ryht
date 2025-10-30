//! Core types for the Virtual Filesystem.

use crate::path::VirtualPath;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Custom serialization for UUID fields to avoid SurrealDB byte array issues.
/// SurrealDB SDK serializes UUIDs in nested structures as byte arrays, which causes
/// "invalid type: byte array" errors during deserialization. This module ensures
/// UUIDs are always serialized as strings.
mod uuid_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&uuid.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Custom serialization for optional UUID fields.
mod uuid_option_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S>(uuid: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match uuid {
            Some(id) => serializer.serialize_some(&id.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(s) => Ok(Some(Uuid::parse_str(&s).map_err(serde::de::Error::custom)?)),
            None => Ok(None),
        }
    }
}

/// A virtual node representing a file, directory, or symlink in the VFS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VNode {
    /// Unique identifier
    #[serde(with = "uuid_serde")]
    pub id: Uuid,

    /// Workspace this vnode belongs to
    #[serde(with = "uuid_serde")]
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
        self.status = SyncStatus::Synced;
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
    Synced,
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

/// Workspace representing a universal container for any type of content.
/// A workspace can be synchronized with multiple sources (local paths, GitHub repos, SSH remotes, etc.)
/// and can contain any type of content (code, documentation, research, data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    #[serde(with = "uuid_serde")]
    pub id: Uuid,

    /// Human-readable name
    pub name: String,

    /// Database namespace for isolation
    pub namespace: String,

    /// Multiple synchronization sources (can be empty for purely virtual workspaces)
    pub sync_sources: Vec<SyncSource>,

    /// Extended metadata (can store workspace_type here if needed for heuristics)
    pub metadata: HashMap<String, Value>,

    /// Whether workspace is read-only
    pub read_only: bool,

    /// Parent workspace (for forks)
    #[serde(with = "uuid_option_serde")]
    pub parent_workspace: Option<Uuid>,

    /// Fork metadata (if this is a fork)
    pub fork_metadata: Option<ForkMetadata>,

    /// Cross-workspace dependencies and links
    pub dependencies: Vec<WorkspaceDependency>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Source type enum (for backward compatibility with old API).
/// In the new model, workspaces have multiple SyncSources.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    Local,
    ExternalReadOnly,
    Fork,
}

impl Workspace {
    /// Get source type from sync sources (for backward compatibility).
    /// Returns "local", "github", "ssh", "s3", etc.
    pub fn source_type(&self) -> String {
        if self.sync_sources.is_empty() {
            return "local".to_string();
        }

        // Return the type of the first (primary) sync source
        match &self.sync_sources[0].source {
            SyncSourceType::LocalPath { .. } => "local".to_string(),
            SyncSourceType::GitHub { .. } => "github".to_string(),
            SyncSourceType::Git { .. } => "git".to_string(),
            SyncSourceType::SshRemote { .. } => "ssh".to_string(),
            SyncSourceType::S3 { .. } => "s3".to_string(),
            SyncSourceType::CrossWorkspace { .. } => "cross_workspace".to_string(),
            SyncSourceType::HttpUrl { .. } => "http".to_string(),
        }
    }

    /// Get source path from sync sources (for backward compatibility).
    /// Returns the path of the first local source, if any.
    pub fn source_path(&self) -> Option<String> {
        for source in &self.sync_sources {
            if let SyncSourceType::LocalPath { path, .. } = &source.source {
                return Some(path.clone());
            }
        }
        None
    }
}

/// Synchronization source for a workspace.
/// A workspace can have multiple sync sources of different types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSource {
    /// Unique identifier for this sync source
    #[serde(with = "uuid_serde")]
    pub id: Uuid,

    /// Type and configuration of the sync source
    pub source: SyncSourceType,

    /// Whether this source is read-only
    pub read_only: bool,

    /// Priority for conflict resolution (higher = preferred)
    pub priority: i32,

    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,

    /// Sync status
    pub status: SyncSourceStatus,

    /// Metadata specific to this source
    pub metadata: HashMap<String, Value>,
}

/// Types of synchronization sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncSourceType {
    /// Local filesystem path
    LocalPath {
        path: String,
        watch: bool, // Enable filesystem watching
    },

    /// GitHub repository
    GitHub {
        owner: String,
        repo: String,
        branch: Option<String>,
        path: Option<String>, // Subdirectory in the repo
        token: Option<String>, // For private repos
    },

    /// SSH remote
    SshRemote {
        host: String,
        port: Option<u16>,
        user: String,
        path: String,
        key_path: Option<String>,
    },

    /// S3 bucket
    S3 {
        bucket: String,
        region: String,
        prefix: Option<String>,
        endpoint: Option<String>, // For S3-compatible services
    },

    /// Another Cortex workspace
    CrossWorkspace {
        #[serde(with = "uuid_serde")]
        workspace_id: Uuid,
        namespace: String,
    },

    /// HTTP/HTTPS URL (for downloading archives, etc.)
    HttpUrl {
        url: String,
        auth_header: Option<String>,
    },

    /// Git repository (generic)
    Git {
        url: String,
        branch: Option<String>,
        path: Option<String>,
    },
}

/// Status of a sync source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncSourceStatus {
    /// Never synced
    Unsynced,
    /// Currently syncing
    Syncing,
    /// Synced successfully
    Synced,
    /// Sync failed
    Failed,
    /// Source is offline/unreachable
    Offline,
}

/// Cross-workspace dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDependency {
    /// ID of the dependent workspace
    #[serde(with = "uuid_serde")]
    pub workspace_id: Uuid,

    /// Type of dependency
    pub dependency_type: DependencyType,

    /// Optional version constraint
    pub version: Option<String>,

    /// Metadata about the dependency
    pub metadata: HashMap<String, Value>,
}

/// Type of dependency between workspaces.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Uses code from another workspace
    Code,
    /// References documentation from another workspace
    Documentation,
    /// Shares data with another workspace
    Data,
    /// Generic dependency
    Generic,
}

/// Metadata for forked workspaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkMetadata {
    /// Original workspace ID
    #[serde(with = "uuid_serde")]
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

    /// Maximum file size in bytes (files larger than this will be skipped)
    pub max_file_size_bytes: Option<usize>,
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
            max_file_size_bytes: None,
        }
    }
}

/// Report from an import operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportReport {
    /// Workspace ID created
    #[serde(with = "uuid_serde")]
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
    #[serde(with = "uuid_serde")]
    pub id: Uuid,

    /// Vnode ID
    #[serde(with = "uuid_serde")]
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

/// Options for bidirectional filesystem sync.
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Skip hidden files (starting with .)
    pub skip_hidden: bool,

    /// Follow symlinks
    pub follow_symlinks: bool,

    /// Maximum directory depth (None = unlimited)
    pub max_depth: Option<usize>,

    /// Auto-resolve conflicts (prefer filesystem version)
    pub auto_resolve_conflicts: bool,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            skip_hidden: true,
            follow_symlinks: false,
            max_depth: None,
            auto_resolve_conflicts: false,
            exclude_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
        }
    }
}

/// Report from a filesystem sync operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    /// Number of files synced from filesystem
    pub files_synced: usize,

    /// Number of directories synced
    pub directories_synced: usize,

    /// Total bytes synced
    pub bytes_synced: usize,

    /// Number of conflicts detected
    pub conflicts_detected: usize,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Result of syncing a single file.
#[derive(Debug, Clone)]
pub struct FileSyncResult {
    /// Size of the file in bytes
    pub size_bytes: usize,

    /// Whether a conflict was detected
    pub is_conflict: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_source_type_variants() {
        // Test local path variant
        let local = SyncSourceType::LocalPath {
            path: "/home/user/project".to_string(),
            watch: true,
        };

        match local {
            SyncSourceType::LocalPath { path, watch } => {
                assert_eq!(path, "/home/user/project");
                assert!(watch);
            }
            _ => panic!("Expected LocalPath variant"),
        }
    }

    #[test]
    fn test_workspace_with_multiple_sources() {
        let ws = Workspace {
            id: Uuid::new_v4(),
            name: "multi-source-project".to_string(),
            namespace: "ws_test".to_string(),
            sync_sources: vec![
                SyncSource {
                    id: Uuid::new_v4(),
                    source: SyncSourceType::LocalPath {
                        path: "/local/path".to_string(),
                        watch: true,
                    },
                    read_only: false,
                    priority: 10,
                    last_sync: None,
                    status: SyncSourceStatus::Unsynced,
                    metadata: HashMap::new(),
                },
                SyncSource {
                    id: Uuid::new_v4(),
                    source: SyncSourceType::GitHub {
                        owner: "user".to_string(),
                        repo: "repo".to_string(),
                        branch: Some("main".to_string()),
                        path: None,
                        token: None,
                    },
                    read_only: true,
                    priority: 5,
                    last_sync: None,
                    status: SyncSourceStatus::Unsynced,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
            read_only: false,
            parent_workspace: None,
            fork_metadata: None,
            dependencies: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(ws.sync_sources.len(), 2);
        assert_eq!(ws.name, "multi-source-project");
    }
}
