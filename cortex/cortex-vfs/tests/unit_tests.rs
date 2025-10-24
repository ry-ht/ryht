//! Comprehensive unit tests for cortex-vfs

use cortex_vfs::types::*;
use cortex_vfs::path::VirtualPath;

// ============================================================================
// VirtualPath Tests
// ============================================================================

#[test]
fn test_virtual_path_creation() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    assert_eq!(path.to_string(), "workspace/src/main.rs");
}

#[test]
fn test_virtual_path_root() {
    let path = VirtualPath::new("/").unwrap();
    assert!(path.is_root());
    assert_eq!(path.to_string(), "/");
}

#[test]
fn test_virtual_path_parent() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    let parent = path.parent();
    assert!(parent.is_some());
    assert_eq!(parent.unwrap().to_string(), "workspace/src");
}

#[test]
fn test_virtual_path_root_parent() {
    let path = VirtualPath::new("/").unwrap();
    assert!(path.parent().is_none());
}

#[test]
fn test_virtual_path_file_name() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    assert_eq!(path.file_name(), Some("main.rs"));
}

#[test]
fn test_virtual_path_extension() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    assert_eq!(path.extension(), Some("rs"));

    let no_ext = VirtualPath::new("/workspace/Makefile").unwrap();
    assert_eq!(no_ext.extension(), None);
}

#[test]
fn test_virtual_path_join() {
    let base = VirtualPath::new("/workspace").unwrap();
    let joined = base.join("src/main.rs").unwrap();
    assert_eq!(joined.to_string(), "workspace/src/main.rs");
}

#[test]
fn test_virtual_path_normalization() {
    let path = VirtualPath::new("/workspace/../src/./main.rs").unwrap();
    let normalized = path.normalize();
    assert_eq!(normalized.to_string(), "src/main.rs");
}

#[test]
fn test_virtual_path_is_ancestor_of() {
    let parent = VirtualPath::new("/workspace/src").unwrap();
    let child = VirtualPath::new("/workspace/src/main.rs").unwrap();

    assert!(parent.starts_with(&parent));
    assert!(child.starts_with(&parent));
}

#[test]
fn test_virtual_path_components() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    let segments = path.segments();

    assert_eq!(segments, &["workspace", "src", "main.rs"]);
}

#[test]
fn test_virtual_path_display() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    assert_eq!(format!("{}", path), "workspace/src/main.rs");
}

#[test]
fn test_virtual_path_equality() {
    let path1 = VirtualPath::new("/workspace/src/main.rs").unwrap();
    let path2 = VirtualPath::new("/workspace/src/main.rs").unwrap();
    let path3 = VirtualPath::new("/workspace/src/lib.rs").unwrap();

    assert_eq!(path1, path2);
    assert_ne!(path1, path3);
}

// ============================================================================
// VNode Tests
// ============================================================================

#[test]
fn test_vnode_file() {
    use uuid::Uuid;

    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("test.txt").unwrap();
    let node = VNode::new_file(workspace_id, path.clone(), "abc123".to_string(), 1024);

    assert!(node.is_file());
    assert!(!node.is_directory());
    assert_eq!(node.size_bytes, 1024);
    assert_eq!(node.content_hash, Some("abc123".to_string()));
}

#[test]
fn test_vnode_directory() {
    use uuid::Uuid;

    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("workspace").unwrap();
    let node = VNode::new_directory(workspace_id, path.clone());

    assert!(node.is_directory());
    assert!(!node.is_file());
    assert_eq!(node.size_bytes, 0);
}

#[test]
fn test_vnode_metadata() {
    use uuid::Uuid;
    use serde_json::Value;

    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("main.rs").unwrap();
    let mut node = VNode::new_file(workspace_id, path, "hash123".to_string(), 2048);

    node.metadata.insert("language".to_string(), Value::String("rust".to_string()));

    assert_eq!(node.size_bytes, 2048);
    assert_eq!(node.metadata.get("language"), Some(&Value::String("rust".to_string())));
}

#[test]
fn test_vnode_symlink() {
    use uuid::Uuid;

    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("link.txt").unwrap();
    let node = VNode::new_symlink(workspace_id, path, "/target/file.txt".to_string());

    assert!(node.is_symlink());
    assert!(!node.is_file());
    assert!(!node.is_directory());
}

// ============================================================================
// Language Detection Tests
// ============================================================================

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("go"), Language::Go);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}

// ============================================================================
// ForkMetadata Tests
// ============================================================================

#[test]
fn test_fork_metadata() {
    use uuid::Uuid;
    use chrono::Utc;

    let fork = ForkMetadata {
        source_id: Uuid::new_v4(),
        source_name: "original-workspace".to_string(),
        fork_point: Utc::now(),
        fork_commit: Some("abc123".to_string()),
    };

    assert_eq!(fork.source_name, "original-workspace");
    assert_eq!(fork.fork_commit, Some("abc123".to_string()));
}

#[test]
fn test_fork_metadata_serialization() {
    use uuid::Uuid;
    use chrono::Utc;

    let fork = ForkMetadata {
        source_id: Uuid::new_v4(),
        source_name: "Test".to_string(),
        fork_point: Utc::now(),
        fork_commit: Some("hash123".to_string()),
    };

    let json = serde_json::to_string(&fork).unwrap();
    let deserialized: ForkMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.source_name, "Test");
    assert_eq!(deserialized.fork_commit, Some("hash123".to_string()));
}

// ============================================================================
// Path Utilities Tests
// ============================================================================

#[test]
fn test_path_depth() {
    let path1 = VirtualPath::new("/").unwrap();
    assert_eq!(path1.len(), 0);

    let path2 = VirtualPath::new("/workspace").unwrap();
    assert_eq!(path2.len(), 1);

    let path3 = VirtualPath::new("/workspace/src/main.rs").unwrap();
    assert_eq!(path3.len(), 3);
}

#[test]
fn test_path_starts_with() {
    let path = VirtualPath::new("/workspace/src/main.rs").unwrap();
    let prefix = VirtualPath::new("/workspace").unwrap();

    assert!(path.starts_with(&prefix));

    let other_prefix = VirtualPath::new("/other").unwrap();
    assert!(!path.starts_with(&other_prefix));
}

// ============================================================================
// VNode Serialization Tests
// ============================================================================

#[test]
fn test_vnode_serialization() {
    use uuid::Uuid;

    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("test.txt").unwrap();
    let node = VNode::new_file(workspace_id, path, "abc123".to_string(), 1024);

    let json = serde_json::to_string(&node).unwrap();
    let deserialized: VNode = serde_json::from_str(&json).unwrap();

    assert!(deserialized.is_file());
    assert_eq!(deserialized.size_bytes, 1024);
}

// ============================================================================
// FlushOptions Tests
// ============================================================================

#[test]
fn test_flush_options_default() {
    let options = FlushOptions::default();

    assert!(options.preserve_permissions);
    assert!(options.preserve_timestamps);
    assert!(!options.create_backup);
    assert!(options.atomic);
    assert!(options.parallel);
}

// ============================================================================
// ImportOptions Tests
// ============================================================================

#[test]
fn test_import_options_default() {
    let options = ImportOptions::default();

    assert!(options.read_only);
    assert!(!options.create_fork);
    assert!(options.process_code);
    assert!(!options.generate_embeddings);
    assert!(!options.include_patterns.is_empty());
    assert!(!options.exclude_patterns.is_empty());
}

// ============================================================================
// Workspace Tests
// ============================================================================

#[test]
fn test_workspace_type() {
    assert_eq!(WorkspaceType::Code, WorkspaceType::Code);
    assert_ne!(WorkspaceType::Code, WorkspaceType::Documentation);
}

#[test]
fn test_source_type() {
    assert_eq!(SourceType::Local, SourceType::Local);
    assert_ne!(SourceType::Local, SourceType::Fork);
}

// ============================================================================
// Change Tracking Tests
// ============================================================================

#[test]
fn test_change_type() {
    assert_eq!(ChangeType::Created, ChangeType::Created);
    assert_ne!(ChangeType::Created, ChangeType::Modified);
}

#[test]
fn test_sync_status() {
    assert_eq!(SyncStatus::Modified, SyncStatus::Modified);
    assert_ne!(SyncStatus::Modified, SyncStatus::Synchronized);
}

// ============================================================================
// NodeType Tests
// ============================================================================

#[test]
fn test_node_type() {
    assert_eq!(NodeType::File, NodeType::File);
    assert_ne!(NodeType::File, NodeType::Directory);
    assert_eq!(NodeType::SymLink, NodeType::SymLink);
}

// ============================================================================
// MergeStrategy Tests
// ============================================================================

#[test]
fn test_merge_strategy() {
    assert_eq!(MergeStrategy::Manual, MergeStrategy::Manual);
    assert_ne!(MergeStrategy::Manual, MergeStrategy::AutoMerge);
    assert_eq!(MergeStrategy::PreferFork, MergeStrategy::PreferFork);
}

// ============================================================================
// FlushReport Tests
// ============================================================================

#[test]
fn test_flush_report_default() {
    let report = FlushReport::default();

    assert_eq!(report.files_written, 0);
    assert_eq!(report.directories_created, 0);
    assert_eq!(report.symlinks_created, 0);
    assert_eq!(report.files_deleted, 0);
    assert_eq!(report.bytes_written, 0);
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.duration_ms, 0);
}

// ============================================================================
// ImportReport Tests
// ============================================================================

#[test]
fn test_import_report_default() {
    let report = ImportReport::default();

    assert_eq!(report.files_imported, 0);
    assert_eq!(report.directories_imported, 0);
    assert_eq!(report.units_extracted, 0);
    assert_eq!(report.bytes_imported, 0);
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.duration_ms, 0);
}

// ============================================================================
// MergeReport Tests
// ============================================================================

#[test]
fn test_merge_report_default() {
    let report = MergeReport::default();

    assert_eq!(report.changes_applied, 0);
    assert_eq!(report.conflicts_count, 0);
    assert_eq!(report.conflicts.len(), 0);
    assert_eq!(report.auto_resolved, 0);
    assert_eq!(report.errors.len(), 0);
}

// ============================================================================
// Conflict Tests
// ============================================================================

#[test]
fn test_conflict_creation() {
    let path = VirtualPath::new("src/main.rs").unwrap();
    let conflict = Conflict {
        path: path.clone(),
        fork_content: "fork version".to_string(),
        target_content: "target version".to_string(),
        resolution: None,
    };

    assert_eq!(conflict.path, path);
    assert_eq!(conflict.fork_content, "fork version");
    assert_eq!(conflict.target_content, "target version");
    assert!(conflict.resolution.is_none());
}

#[test]
fn test_conflict_with_resolution() {
    let path = VirtualPath::new("src/main.rs").unwrap();
    let conflict = Conflict {
        path,
        fork_content: "fork version".to_string(),
        target_content: "target version".to_string(),
        resolution: Some("merged version".to_string()),
    };

    assert!(conflict.resolution.is_some());
    assert_eq!(conflict.resolution.unwrap(), "merged version");
}
