//! Virtual Filesystem implementation for Cortex.
//!
//! This crate provides a production-grade virtual filesystem layer with:
//! - Path-agnostic design (virtual paths independent of physical location)
//! - Content deduplication using blake3 hashing
//! - Lazy materialization (files exist in memory until explicitly flushed)
//! - Multi-workspace support with isolation
//! - External project import with fork capability
//! - LRU content caching with TTL support
//! - Change tracking and atomic operations
//!
//! # Architecture
//!
//! The VFS is built on SurrealDB for metadata and content storage, with:
//! - `VirtualPath`: Path-agnostic path representation
//! - `VNode`: Virtual node representing files, directories, or symlinks
//! - `VirtualFileSystem`: Core filesystem operations
//! - `MaterializationEngine`: Flush VFS to physical disk
//! - `ExternalProjectLoader`: Import external projects
//! - `ForkManager`: Create and merge forks
//! - `ContentCache`: LRU cache for frequently accessed content
//!
//! # Example
//!
//! ```no_run
//! use cortex_vfs::{VirtualFileSystem, VirtualPath, FlushScope, FlushOptions};
//! use cortex_storage::ConnectionManager;
//! use std::sync::Arc;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create VFS
//! let storage = Arc::new(ConnectionManager::default());
//! let vfs = VirtualFileSystem::new(storage);
//!
//! // Create a workspace
//! let workspace_id = uuid::Uuid::new_v4();
//!
//! // Write a file
//! let path = VirtualPath::new("src/main.rs")?;
//! vfs.write_file(&workspace_id, &path, b"fn main() {}").await?;
//!
//! // Read it back
//! let content = vfs.read_file(&workspace_id, &path).await?;
//!
//! // Flush to disk
//! let engine = cortex_vfs::MaterializationEngine::new(vfs.clone());
//! let target_path = Path::new("/home/user/project");
//! engine.flush(FlushScope::All, target_path, FlushOptions::default()).await?;
//! # Ok(())
//! # }
//! ```

pub mod path;
pub mod types;
pub mod virtual_filesystem;
pub mod content_cache;
pub mod materialization;
pub mod external_loader;
pub mod fork_manager;
pub mod watcher;
pub mod cache;
pub mod dedup;

// Re-export main types
pub use path::{VirtualPath, VirtualPathError};
pub use types::*;
pub use virtual_filesystem::VirtualFileSystem;
pub use content_cache::{ContentCache, CacheStatistics};
pub use materialization::MaterializationEngine;
pub use external_loader::ExternalProjectLoader;
pub use fork_manager::ForkManager;
pub use watcher::FileWatcher;

/// Prelude module with commonly used types.
pub mod prelude {
    pub use crate::path::{VirtualPath, VirtualPathError};
    pub use crate::types::{
        Change, ChangeType, Conflict, FileContent, FlushOptions, FlushReport, FlushScope,
        ForkMetadata, ImportOptions, ImportReport, Language, MergeReport, MergeStrategy, NodeType,
        SourceType, SyncStatus, VNode, Workspace, WorkspaceType,
    };
    pub use crate::virtual_filesystem::VirtualFileSystem;
    pub use crate::content_cache::{ContentCache, CacheStatistics};
    pub use crate::materialization::MaterializationEngine;
    pub use crate::external_loader::ExternalProjectLoader;
    pub use crate::fork_manager::ForkManager;
    pub use crate::watcher::FileWatcher;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_path_basic() {
        let path = VirtualPath::new("src/main.rs").unwrap();
        assert_eq!(path.file_name(), Some("main.rs"));
        assert_eq!(path.extension(), Some("rs"));
    }

    #[test]
    fn test_virtual_path_join() {
        let base = VirtualPath::new("src").unwrap();
        let joined = base.join("lib/mod.rs").unwrap();
        assert_eq!(joined.to_string(), "src/lib/mod.rs");
    }

    #[test]
    fn test_virtual_path_parent() {
        let path = VirtualPath::new("src/lib/mod.rs").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_string(), "src/lib");
    }

    #[test]
    fn test_virtual_path_relative() {
        // All paths are relative - leading slashes are stripped
        let path1 = VirtualPath::new("/absolute/path").unwrap();
        let path2 = VirtualPath::new("relative/path").unwrap();

        // Both should normalize to the same string format (no leading slash)
        assert_eq!(path1.to_string(), "absolute/path");
        assert_eq!(path2.to_string(), "relative/path");
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("unknown"), Language::Unknown);
    }

    #[test]
    fn test_vnode_creation() {
        use uuid::Uuid;

        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("test.txt").unwrap();

        let vnode = VNode::new_file(workspace_id, path.clone(), "hash123".to_string(), 100);

        assert_eq!(vnode.workspace_id, workspace_id);
        assert_eq!(vnode.path, path);
        assert_eq!(vnode.size_bytes, 100);
        assert!(vnode.is_file());
        assert!(!vnode.is_directory());
    }

    #[test]
    fn test_vnode_directory() {
        use uuid::Uuid;

        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("src").unwrap();

        let vnode = VNode::new_directory(workspace_id, path.clone());

        assert!(vnode.is_directory());
        assert!(!vnode.is_file());
        assert_eq!(vnode.size_bytes, 0);
    }
}
