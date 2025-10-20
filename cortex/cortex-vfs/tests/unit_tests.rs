//! Comprehensive unit tests for cortex-vfs

use cortex_vfs::types::*;
use cortex_vfs::path::VirtualPath;
use std::collections::HashMap;

// ============================================================================
// VirtualPath Tests
// ============================================================================

#[test]
fn test_virtual_path_creation() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    assert_eq!(path.as_str(), "/workspace/src/main.rs");
}

#[test]
fn test_virtual_path_root() {
    let path = VirtualPath::new("/");
    assert!(path.is_root());
    assert_eq!(path.as_str(), "/");
}

#[test]
fn test_virtual_path_parent() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    let parent = path.parent();
    assert!(parent.is_some());
    assert_eq!(parent.unwrap().as_str(), "/workspace/src");
}

#[test]
fn test_virtual_path_root_parent() {
    let path = VirtualPath::new("/");
    assert!(path.parent().is_none());
}

#[test]
fn test_virtual_path_file_name() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    assert_eq!(path.file_name(), Some("main.rs"));
}

#[test]
fn test_virtual_path_extension() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    assert_eq!(path.extension(), Some("rs"));

    let no_ext = VirtualPath::new("/workspace/Makefile");
    assert_eq!(no_ext.extension(), None);
}

#[test]
fn test_virtual_path_join() {
    let base = VirtualPath::new("/workspace");
    let joined = base.join("src/main.rs");
    assert_eq!(joined.as_str(), "/workspace/src/main.rs");
}

#[test]
fn test_virtual_path_normalization() {
    let path = VirtualPath::new("/workspace/../src/./main.rs");
    let normalized = path.normalize();
    assert_eq!(normalized.as_str(), "/src/main.rs");
}

#[test]
fn test_virtual_path_is_ancestor_of() {
    let parent = VirtualPath::new("/workspace/src");
    let child = VirtualPath::new("/workspace/src/main.rs");

    assert!(parent.is_ancestor_of(&child));
    assert!(!child.is_ancestor_of(&parent));
}

#[test]
fn test_virtual_path_components() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    let components: Vec<&str> = path.components().collect();

    assert_eq!(components, vec!["workspace", "src", "main.rs"]);
}

#[test]
fn test_virtual_path_display() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    assert_eq!(format!("{}", path), "/workspace/src/main.rs");
}

#[test]
fn test_virtual_path_equality() {
    let path1 = VirtualPath::new("/workspace/src/main.rs");
    let path2 = VirtualPath::new("/workspace/src/main.rs");
    let path3 = VirtualPath::new("/workspace/src/lib.rs");

    assert_eq!(path1, path2);
    assert_ne!(path1, path3);
}

// ============================================================================
// VirtualNode Tests
// ============================================================================

#[test]
fn test_virtual_node_file() {
    let node = VirtualNode::File(FileNode {
        path: "/test.txt".to_string(),
        content_hash: "abc123".to_string(),
        size: 1024,
        metadata: HashMap::new(),
        cached: false,
    });

    assert!(matches!(node, VirtualNode::File(_)));
    assert!(!node.is_directory());
}

#[test]
fn test_virtual_node_directory() {
    let node = VirtualNode::Directory(DirectoryNode {
        path: "/workspace".to_string(),
        children: HashMap::new(),
        metadata: HashMap::new(),
    });

    assert!(matches!(node, VirtualNode::Directory(_)));
    assert!(node.is_directory());
}

#[test]
fn test_file_node_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("language".to_string(), "rust".to_string());

    let node = FileNode {
        path: "/main.rs".to_string(),
        content_hash: "hash123".to_string(),
        size: 2048,
        metadata: metadata.clone(),
        cached: true,
    };

    assert_eq!(node.size, 2048);
    assert!(node.cached);
    assert_eq!(node.metadata.get("language"), Some(&"rust".to_string()));
}

#[test]
fn test_directory_node_add_child() {
    let mut dir = DirectoryNode {
        path: "/workspace".to_string(),
        children: HashMap::new(),
        metadata: HashMap::new(),
    };

    let file = VirtualNode::File(FileNode {
        path: "/workspace/file.txt".to_string(),
        content_hash: "hash".to_string(),
        size: 100,
        metadata: HashMap::new(),
        cached: false,
    });

    dir.children.insert("file.txt".to_string(), file);
    assert_eq!(dir.children.len(), 1);
}

// ============================================================================
// ContentHash Tests
// ============================================================================

#[test]
fn test_content_hash_sha256() {
    let content = b"Hello, world!";
    let hash1 = cortex_vfs::dedup::calculate_hash(content);
    let hash2 = cortex_vfs::dedup::calculate_hash(content);

    // Same content should produce same hash
    assert_eq!(hash1, hash2);

    // Different content should produce different hash
    let hash3 = cortex_vfs::dedup::calculate_hash(b"Different content");
    assert_ne!(hash1, hash3);
}

#[test]
fn test_content_hash_empty() {
    let empty_hash = cortex_vfs::dedup::calculate_hash(b"");
    assert!(!empty_hash.is_empty());
    assert_eq!(empty_hash.len(), 64); // SHA256 hex string length
}

#[test]
fn test_content_hash_large_content() {
    let large_content = vec![0u8; 1024 * 1024]; // 1MB
    let hash = cortex_vfs::dedup::calculate_hash(&large_content);
    assert_eq!(hash.len(), 64);
}

// ============================================================================
// Cache Entry Tests
// ============================================================================

#[test]
fn test_cache_entry_creation() {
    use cortex_vfs::cache::CacheEntry;
    use chrono::Utc;

    let entry = CacheEntry {
        path: "/test.txt".to_string(),
        content: vec![1, 2, 3, 4],
        hash: "test_hash".to_string(),
        size: 4,
        last_accessed: Utc::now(),
        access_count: 0,
    };

    assert_eq!(entry.content.len(), 4);
    assert_eq!(entry.access_count, 0);
}

#[test]
fn test_cache_entry_access_tracking() {
    use cortex_vfs::cache::CacheEntry;
    use chrono::Utc;

    let mut entry = CacheEntry {
        path: "/test.txt".to_string(),
        content: vec![],
        hash: "hash".to_string(),
        size: 0,
        last_accessed: Utc::now(),
        access_count: 0,
    };

    entry.access_count += 1;
    entry.last_accessed = Utc::now();

    assert_eq!(entry.access_count, 1);
}

// ============================================================================
// Fork Tests
// ============================================================================

#[test]
fn test_fork_metadata() {
    use cortex_vfs::fork_manager::ForkMetadata;
    use chrono::Utc;

    let fork = ForkMetadata {
        id: "fork-1".to_string(),
        parent_id: None,
        created_at: Utc::now(),
        description: "Test fork".to_string(),
        modifications: 0,
    };

    assert_eq!(fork.id, "fork-1");
    assert!(fork.parent_id.is_none());
    assert_eq!(fork.modifications, 0);
}

#[test]
fn test_fork_with_parent() {
    use cortex_vfs::fork_manager::ForkMetadata;
    use chrono::Utc;

    let fork = ForkMetadata {
        id: "fork-2".to_string(),
        parent_id: Some("fork-1".to_string()),
        created_at: Utc::now(),
        description: "Child fork".to_string(),
        modifications: 5,
    };

    assert_eq!(fork.parent_id, Some("fork-1".to_string()));
    assert_eq!(fork.modifications, 5);
}

// ============================================================================
// Materialization Tests
// ============================================================================

#[test]
fn test_materialization_request() {
    use cortex_vfs::materialization::MaterializationRequest;

    let request = MaterializationRequest {
        fork_id: "fork-1".to_string(),
        target_path: "/output".to_string(),
        files: vec!["/file1.txt".to_string(), "/file2.txt".to_string()],
        options: MaterializationOptions::default(),
    };

    assert_eq!(request.fork_id, "fork-1");
    assert_eq!(request.files.len(), 2);
}

#[test]
fn test_materialization_options() {
    use cortex_vfs::materialization::MaterializationOptions;

    let options = MaterializationOptions {
        preserve_timestamps: true,
        preserve_permissions: false,
        overwrite_existing: true,
        create_directories: true,
    };

    assert!(options.preserve_timestamps);
    assert!(!options.preserve_permissions);
}

// ============================================================================
// Path Utilities Tests
// ============================================================================

#[test]
fn test_path_is_absolute() {
    let abs_path = VirtualPath::new("/absolute/path");
    assert!(abs_path.is_absolute());

    let rel_path = VirtualPath::new("relative/path");
    assert!(!rel_path.is_absolute());
}

#[test]
fn test_path_depth() {
    let path1 = VirtualPath::new("/");
    assert_eq!(path1.depth(), 0);

    let path2 = VirtualPath::new("/workspace");
    assert_eq!(path2.depth(), 1);

    let path3 = VirtualPath::new("/workspace/src/main.rs");
    assert_eq!(path3.depth(), 3);
}

#[test]
fn test_path_starts_with() {
    let path = VirtualPath::new("/workspace/src/main.rs");
    let prefix = VirtualPath::new("/workspace");

    assert!(path.starts_with(&prefix));

    let other_prefix = VirtualPath::new("/other");
    assert!(!path.starts_with(&other_prefix));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_virtual_node_serialization() {
    let node = VirtualNode::File(FileNode {
        path: "/test.txt".to_string(),
        content_hash: "abc123".to_string(),
        size: 1024,
        metadata: HashMap::new(),
        cached: false,
    });

    let json = serde_json::to_string(&node).unwrap();
    let deserialized: VirtualNode = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, VirtualNode::File(_)));
}

#[test]
fn test_fork_metadata_serialization() {
    use cortex_vfs::fork_manager::ForkMetadata;
    use chrono::Utc;

    let fork = ForkMetadata {
        id: "fork-1".to_string(),
        parent_id: Some("parent".to_string()),
        created_at: Utc::now(),
        description: "Test".to_string(),
        modifications: 10,
    };

    let json = serde_json::to_string(&fork).unwrap();
    let deserialized: ForkMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "fork-1");
    assert_eq!(deserialized.modifications, 10);
}
