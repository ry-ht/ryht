//! Integration tests for Virtual Filesystem.

use cortex_vfs::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// Note: These tests require a running SurrealDB instance for full integration testing.
// For now, they serve as documentation of the expected API usage.

#[test]
fn test_virtual_path_operations() {
    // Test basic path operations
    let path = VirtualPath::new("src/main.rs").unwrap();
    assert_eq!(path.to_string(), "src/main.rs");
    assert_eq!(path.file_name(), Some("main.rs"));
    assert_eq!(path.extension(), Some("rs"));

    // Test path joining
    let base = VirtualPath::new("src").unwrap();
    let joined = base.join("lib/mod.rs").unwrap();
    assert_eq!(joined.to_string(), "src/lib/mod.rs");

    // Test parent
    let parent = joined.parent().unwrap();
    assert_eq!(parent.to_string(), "src/lib");

    // Test root
    let root = VirtualPath::root();
    assert!(root.is_root());
    assert_eq!(root.to_string(), "/");
}

#[test]
fn test_virtual_path_normalization() {
    let path = VirtualPath::new("src/../lib/./main.rs").unwrap();
    let normalized = path.normalize();
    assert_eq!(normalized.to_string(), "lib/main.rs");
}

#[test]
fn test_virtual_path_to_physical() {
    let vpath = VirtualPath::new("src/main.rs").unwrap();
    let base = PathBuf::from("/home/user/project");
    let physical = vpath.to_physical(&base);
    assert_eq!(physical, PathBuf::from("/home/user/project/src/main.rs"));
}

#[test]
fn test_virtual_path_from_physical() {
    use std::path::Path;

    let physical = Path::new("/home/user/project/src/main.rs");
    let base = Path::new("/home/user/project");
    let vpath = VirtualPath::from_physical(physical, base).unwrap();
    assert_eq!(vpath.to_string(), "src/main.rs");
}

#[test]
fn test_vnode_creation() {
    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("test.txt").unwrap();

    // Test file creation
    let file = VNode::new_file(workspace_id, path.clone(), "hash123".to_string(), 100);
    assert!(file.is_file());
    assert!(!file.is_directory());
    assert_eq!(file.size_bytes, 100);
    assert_eq!(file.content_hash, Some("hash123".to_string()));

    // Test directory creation
    let dir_path = VirtualPath::new("src").unwrap();
    let dir = VNode::new_directory(workspace_id, dir_path);
    assert!(dir.is_directory());
    assert!(!dir.is_file());
    assert_eq!(dir.size_bytes, 0);

    // Test symlink creation
    let link_path = VirtualPath::new("link").unwrap();
    let link = VNode::new_symlink(workspace_id, link_path, "/target/path".to_string());
    assert!(link.is_symlink());
    assert_eq!(link.metadata.get("target").unwrap().as_str(), Some("/target/path"));
}

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    assert_eq!(Language::from_extension("js"), Language::JavaScript);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("go"), Language::Go);
    assert_eq!(Language::from_extension("java"), Language::Java);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}

#[test]
fn test_content_cache() {
    let cache = ContentCache::new(1024);

    // Test put and get
    let content = b"hello world".to_vec();
    let hash = "test_hash".to_string();
    cache.put(hash.clone(), content.clone());

    let retrieved = cache.get(&hash).unwrap();
    assert_eq!(&**retrieved, &content);

    // Test statistics
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.puts, 1);
    assert_eq!(stats.misses, 0);

    // Test cache miss
    assert!(cache.get("nonexistent").is_none());
    let stats = cache.stats();
    assert_eq!(stats.misses, 1);
}

#[test]
fn test_content_cache_eviction() {
    let cache = ContentCache::new(20); // Very small cache

    // Fill cache
    cache.put("key1".to_string(), vec![1, 2, 3, 4, 5]);
    cache.put("key2".to_string(), vec![6, 7, 8, 9, 10]);

    // This should evict key1 (LRU)
    cache.put("key3".to_string(), vec![11, 12, 13, 14, 15]);

    // key1 should be evicted
    assert!(cache.get("key1").is_none());

    // key2 and key3 should still be there
    assert!(cache.get("key2").is_some());
    assert!(cache.get("key3").is_some());
}

#[test]
fn test_content_cache_lru_order() {
    let cache = ContentCache::new(30);

    cache.put("key1".to_string(), vec![1; 10]);
    cache.put("key2".to_string(), vec![2; 10]);

    // Access key1 to make it more recent
    cache.get("key1");

    // Add key3, should evict key2 (least recently used)
    cache.put("key3".to_string(), vec![3; 10]);

    assert!(cache.get("key1").is_some());
    assert!(cache.get("key2").is_none()); // Evicted
    assert!(cache.get("key3").is_some());
}

#[test]
fn test_flush_scope() {
    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("src/main.rs").unwrap();

    // Test different flush scopes
    let _scope_all = FlushScope::All;
    let _scope_path = FlushScope::Path(path);
    let _scope_specific = FlushScope::Specific(vec![Uuid::new_v4()]);
    let _scope_workspace = FlushScope::Workspace(workspace_id);
}

#[test]
fn test_flush_options() {
    let options = FlushOptions::default();
    assert!(options.preserve_permissions);
    assert!(options.preserve_timestamps);
    assert!(!options.create_backup);
    assert!(options.atomic);
    assert!(options.parallel);

    let custom_options = FlushOptions {
        preserve_permissions: false,
        preserve_timestamps: false,
        create_backup: true,
        atomic: false,
        parallel: false,
        max_workers: 4,
    };

    assert!(!custom_options.preserve_permissions);
    assert!(custom_options.create_backup);
}

#[test]
fn test_import_options() {
    let options = ImportOptions::default();
    assert!(options.read_only);
    assert!(!options.create_fork);
    assert!(options.process_code);
    assert!(!options.generate_embeddings);
    assert!(!options.include_patterns.is_empty());
    assert!(!options.exclude_patterns.is_empty());
}

#[test]
fn test_merge_strategies() {
    use MergeStrategy::*;

    let strategies = vec![Manual, AutoMerge, PreferFork, PreferTarget];

    for strategy in strategies {
        match strategy {
            Manual => assert_eq!(strategy, Manual),
            AutoMerge => assert_eq!(strategy, AutoMerge),
            PreferFork => assert_eq!(strategy, PreferFork),
            PreferTarget => assert_eq!(strategy, PreferTarget),
        }
    }
}

#[test]
fn test_workspace_types() {
    use WorkspaceType::*;

    let types = vec![Code, Documentation, Mixed, External];

    for wt in types {
        match wt {
            Code => assert_eq!(wt, Code),
            Documentation => assert_eq!(wt, Documentation),
            Mixed => assert_eq!(wt, Mixed),
            External => assert_eq!(wt, External),
        }
    }
}

#[test]
fn test_source_types() {
    use SourceType::*;

    let types = vec![Local, ExternalReadOnly, Fork];

    for st in types {
        match st {
            Local => assert_eq!(st, Local),
            ExternalReadOnly => assert_eq!(st, ExternalReadOnly),
            Fork => assert_eq!(st, Fork),
        }
    }
}

#[test]
fn test_sync_status() {
    use SyncStatus::*;

    let statuses = vec![Synchronized, Modified, Created, Deleted, Conflict];

    for status in statuses {
        match status {
            Synchronized => assert_eq!(status, Synchronized),
            Modified => assert_eq!(status, Modified),
            Created => assert_eq!(status, Created),
            Deleted => assert_eq!(status, Deleted),
            Conflict => assert_eq!(status, Conflict),
        }
    }
}

#[test]
fn test_vnode_status_transitions() {
    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("test.txt").unwrap();

    let mut vnode = VNode::new_file(workspace_id, path, "hash".to_string(), 100);

    // Initial status should be Modified
    assert_eq!(vnode.status, SyncStatus::Modified);
    let initial_version = vnode.version;

    // Mark as synchronized
    vnode.mark_synchronized();
    assert_eq!(vnode.status, SyncStatus::Synchronized);

    // Mark as modified (should increment version)
    vnode.mark_modified();
    assert_eq!(vnode.status, SyncStatus::Modified);
    assert_eq!(vnode.version, initial_version + 1);
}

#[test]
fn test_node_types() {
    use NodeType::*;

    let types = vec![File, Directory, SymLink, Document];

    for nt in types {
        match nt {
            File => assert_eq!(nt, File),
            Directory => assert_eq!(nt, Directory),
            SymLink => assert_eq!(nt, SymLink),
            Document => assert_eq!(nt, Document),
        }
    }
}

// Async tests would require tokio runtime and database setup
// Example structure for async integration tests:
#[cfg(test)]
mod async_tests {
    use super::*;

    // These tests would require a running SurrealDB instance
    // and proper setup/teardown

    /*
    #[tokio::test]
    async fn test_vfs_read_write() {
        let storage = setup_test_storage().await;
        let vfs = VirtualFileSystem::new(storage);
        let workspace_id = Uuid::new_v4();

        let path = VirtualPath::new("test.txt").unwrap();
        let content = b"Hello, VFS!";

        vfs.write_file(&workspace_id, &path, content).await.unwrap();
        let read_content = vfs.read_file(&workspace_id, &path).await.unwrap();

        assert_eq!(content, &read_content[..]);
    }

    #[tokio::test]
    async fn test_vfs_directory_operations() {
        let storage = setup_test_storage().await;
        let vfs = VirtualFileSystem::new(storage);
        let workspace_id = Uuid::new_v4();

        let dir_path = VirtualPath::new("src").unwrap();
        vfs.create_directory(&workspace_id, &dir_path, false).await.unwrap();

        assert!(vfs.exists(&workspace_id, &dir_path).await.unwrap());

        let file_path = dir_path.join("main.rs").unwrap();
        vfs.write_file(&workspace_id, &file_path, b"fn main() {}").await.unwrap();

        let entries = vfs.list_directory(&workspace_id, &dir_path, false).await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_materialization() {
        let storage = setup_test_storage().await;
        let vfs = VirtualFileSystem::new(storage);
        let workspace_id = Uuid::new_v4();

        // Write some files
        vfs.write_file(&workspace_id, &VirtualPath::new("a.txt").unwrap(), b"A").await.unwrap();
        vfs.write_file(&workspace_id, &VirtualPath::new("b.txt").unwrap(), b"B").await.unwrap();

        // Flush to disk
        let temp_dir = TempDir::new().unwrap();
        let engine = MaterializationEngine::new(vfs);
        let report = engine.flush(
            FlushScope::Workspace(workspace_id),
            temp_dir.path(),
            FlushOptions::default()
        ).await.unwrap();

        assert_eq!(report.files_written, 2);

        // Verify files exist on disk
        assert!(temp_dir.path().join("a.txt").exists());
        assert!(temp_dir.path().join("b.txt").exists());
    }

    #[tokio::test]
    async fn test_external_project_import() {
        let storage = setup_test_storage().await;
        let vfs = VirtualFileSystem::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), b"content").unwrap();

        let loader = ExternalProjectLoader::new(vfs);
        let report = loader.import_project(
            temp_dir.path(),
            ImportOptions::default()
        ).await.unwrap();

        assert_eq!(report.files_imported, 1);
    }

    #[tokio::test]
    async fn test_fork_creation() {
        let storage = setup_test_storage().await;
        let vfs = VirtualFileSystem::new(Arc::clone(&storage));

        // Create original workspace with files
        let original_id = Uuid::new_v4();
        vfs.write_file(&original_id, &VirtualPath::new("test.txt").unwrap(), b"original").await.unwrap();

        // Create fork
        let fork_manager = ForkManager::new(vfs, storage);
        let fork = fork_manager.create_fork(&original_id, "test-fork".to_string()).await.unwrap();

        assert!(!fork.read_only);
        assert_eq!(fork.parent_workspace, Some(original_id));
    }
    */
}
