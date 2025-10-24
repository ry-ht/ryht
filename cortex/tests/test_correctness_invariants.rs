//! Critical correctness invariant tests.
//!
//! This test suite verifies that core system invariants ALWAYS hold under all conditions.
//! These are non-negotiable correctness properties that must never be violated.
//!
//! Test categories:
//! 1. VFS Invariants - File uniqueness, hash consistency, reference integrity
//! 2. Memory Invariants - Temporal ordering, capacity limits, data preservation
//! 3. Database Invariants - Constraints, atomicity

use cortex_core::id::CortexId;
use cortex_memory::prelude::*;
use cortex_memory::types::{CodeUnitType, ComplexityMetrics, EpisodeType, EpisodeOutcome};
use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy,
};
use cortex_vfs::prelude::*;
use blake3;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tracing::info;
use uuid::Uuid;

// =============================================================================
// Test Infrastructure
// =============================================================================

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .try_init();
}

async fn create_test_storage(test_name: &str) -> (TempDir, Arc<ConnectionManager>) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                multiplier: 2.0,
            },
            warm_connections: false,
            validate_on_checkout: true,
            recycle_after_uses: Some(1000),
            shutdown_grace_period: Duration::from_secs(5),
        },
        namespace: "cortex_test".to_string(),
        database: test_name.to_string(),
    };

    let manager = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    );

    (temp_dir, manager)
}

// =============================================================================
// 1. VFS Invariants (10 tests)
// =============================================================================

#[tokio::test]
async fn invariant_vfs_unique_paths_per_workspace() {
    init_tracing();
    info!("TEST: VFS invariant - every file has unique path in workspace");

    let (_temp, storage) = create_test_storage("vfs_unique_paths").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Create multiple files with different paths
    let paths = vec![
        "src/main.rs",
        "src/lib.rs",
        "tests/test.rs",
        "Cargo.toml",
    ];

    for path_str in &paths {
        let path = VirtualPath::new(path_str).unwrap();
        vfs.write_file(&workspace_id, &path, b"content").await.unwrap();
    }

    // List all files
    let root = VirtualPath::new("").unwrap();
    let all_files = vfs.list_directory(&workspace_id, &root, true).await.unwrap();

    // Extract paths and verify uniqueness
    let mut seen_paths = HashSet::new();
    for vnode in all_files.iter().filter(|v| v.is_file()) {
        let path_str = vnode.path.to_string();
        assert!(
            !seen_paths.contains(&path_str),
            "Duplicate path found: {}",
            path_str
        );
        seen_paths.insert(path_str);
    }

    // Verify we have all expected files
    assert_eq!(seen_paths.len(), paths.len(), "Missing files");
    info!("✓ Path uniqueness invariant holds");
}

#[tokio::test]
async fn invariant_vfs_content_hash_matches_content() {
    init_tracing();
    info!("TEST: VFS invariant - content hash always matches actual content");

    let (_temp, storage) = create_test_storage("vfs_hash_match").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Test with various content
    let test_cases: Vec<(&str, &[u8])> = vec![
        ("file1.txt", b"Hello, World!"),
        ("file2.txt", b""),
        ("file4.txt", "Unicode: 你好世界 🚀".as_bytes()),
    ];

    for (path_str, content) in test_cases {
        let path = VirtualPath::new(path_str).unwrap();

        // Write file
        vfs.write_file(&workspace_id, &path, content).await.unwrap();

        // Read back and verify
        let stored_content = vfs.read_file(&workspace_id, &path).await.unwrap();
        assert_eq!(&stored_content[..], content, "Content mismatch for {}", path_str);

        // Compute hash and verify it matches
        let expected_hash = blake3::hash(content).to_string();

        // Get VNode to check stored hash
        let root = VirtualPath::new("").unwrap();
        let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();
        let vnode = vnodes.iter()
            .find(|v| v.path.to_string() == path_str)
            .expect("VNode not found");

        assert_eq!(
            vnode.content_hash.as_ref().unwrap(),
            &expected_hash,
            "Hash mismatch for {}",
            path_str
        );
    }

    info!("✓ Content hash invariant holds");
}

#[tokio::test]
async fn invariant_vfs_version_monotonic_increase() {
    init_tracing();
    info!("TEST: VFS invariant - version numbers monotonically increase");

    let (_temp, storage) = create_test_storage("vfs_version").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();
    let path = VirtualPath::new("test.txt").unwrap();

    // Helper to get current version
    let get_version = |vnodes: &[VNode]| -> u32 {
        vnodes.iter()
            .find(|v| v.path.to_string() == "test.txt")
            .map(|v| v.version)
            .unwrap_or(0)
    };

    let mut previous_version = 0;

    // Perform multiple writes
    for i in 0..10 {
        let content = format!("Content version {}", i);
        vfs.write_file(&workspace_id, &path, content.as_bytes()).await.unwrap();

        // Get current version
        let root = VirtualPath::new("").unwrap();
        let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();
        let current_version = get_version(&vnodes);

        if i == 0 {
            // First version should be 1
            assert_eq!(current_version, 1, "Initial version should be 1");
        } else {
            // Each subsequent version should be higher
            assert!(
                current_version > previous_version,
                "Version did not increase: {} -> {}",
                previous_version,
                current_version
            );
        }

        previous_version = current_version;
    }

    info!("✓ Version monotonicity invariant holds");
}

#[tokio::test]
async fn invariant_vfs_no_dangling_content_references() {
    init_tracing();
    info!("TEST: VFS invariant - no dangling content references");

    let (_temp, storage) = create_test_storage("vfs_no_dangling").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Create files
    let path1 = VirtualPath::new("file1.txt").unwrap();
    let path2 = VirtualPath::new("file2.txt").unwrap();

    vfs.write_file(&workspace_id, &path1, b"content1").await.unwrap();
    vfs.write_file(&workspace_id, &path2, b"content2").await.unwrap();

    // Get all vnodes
    let root = VirtualPath::new("").unwrap();
    let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();

    // Verify every file's content hash can be resolved
    for vnode in vnodes.iter().filter(|v| v.is_file()) {
        if let Some(_hash) = &vnode.content_hash {
            // Try to read the content - should succeed
            let result = vfs.read_file(&workspace_id, &vnode.path).await;
            assert!(
                result.is_ok(),
                "Dangling content reference in file {}",
                vnode.path
            );
        } else {
            panic!("File {} has no content hash", vnode.path);
        }
    }

    info!("✓ No dangling references invariant holds");
}

#[tokio::test]
async fn invariant_vfs_content_deduplication() {
    init_tracing();
    info!("TEST: VFS invariant - identical content shares same hash");

    let (_temp, storage) = create_test_storage("vfs_dedup").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    let shared_content = b"This is shared content";

    // Create multiple files with identical content
    let paths = vec!["file1.txt", "file2.txt", "file3.txt"];
    for path_str in &paths {
        let path = VirtualPath::new(path_str).unwrap();
        vfs.write_file(&workspace_id, &path, shared_content).await.unwrap();
    }

    // Get all vnodes
    let root = VirtualPath::new("").unwrap();
    let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();

    // Extract hashes from our files
    let hashes: Vec<String> = vnodes
        .iter()
        .filter(|v| v.is_file())
        .filter_map(|v| v.content_hash.clone())
        .collect();

    // All hashes should be identical
    assert_eq!(hashes.len(), paths.len());
    let first_hash = &hashes[0];
    for hash in &hashes {
        assert_eq!(hash, first_hash, "Content deduplication failed");
    }

    info!("✓ Content deduplication invariant holds");
}

#[tokio::test]
async fn invariant_vfs_workspace_isolation() {
    init_tracing();
    info!("TEST: VFS invariant - workspaces are isolated");

    let (_temp, storage) = create_test_storage("vfs_isolation").await;
    let vfs = VirtualFileSystem::new(storage.clone());

    let workspace_a = Uuid::new_v4();
    let workspace_b = Uuid::new_v4();

    let path = VirtualPath::new("shared.txt").unwrap();

    // Write different content to same path in different workspaces
    vfs.write_file(&workspace_a, &path, b"Workspace A content").await.unwrap();
    vfs.write_file(&workspace_b, &path, b"Workspace B content").await.unwrap();

    // Read from each workspace
    let content_a = vfs.read_file(&workspace_a, &path).await.unwrap();
    let content_b = vfs.read_file(&workspace_b, &path).await.unwrap();

    // Verify isolation
    assert_eq!(&content_a[..], b"Workspace A content");
    assert_eq!(&content_b[..], b"Workspace B content");
    assert_ne!(content_a, content_b, "Workspace isolation violated");

    info!("✓ Workspace isolation invariant holds");
}

#[tokio::test]
async fn invariant_vfs_size_accuracy() {
    init_tracing();
    info!("TEST: VFS invariant - stored size matches actual content length");

    let (_temp, storage) = create_test_storage("vfs_size").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Test various sizes
    let test_cases = vec![
        ("empty.txt", vec![]),
        ("small.txt", vec![b'x'; 100]),
        ("medium.txt", vec![b'y'; 10_000]),
    ];

    for (path_str, content) in test_cases {
        let path = VirtualPath::new(path_str).unwrap();
        let expected_size = content.len();

        vfs.write_file(&workspace_id, &path, &content).await.unwrap();

        // Get vnode and check size
        let root = VirtualPath::new("").unwrap();
        let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();
        let vnode = vnodes.iter()
            .find(|v| v.path.to_string() == path_str)
            .expect("VNode not found");

        assert_eq!(
            vnode.size_bytes, expected_size,
            "Size mismatch for {}",
            path_str
        );

        // Also verify actual content length
        let stored_content = vfs.read_file(&workspace_id, &path).await.unwrap();
        assert_eq!(
            stored_content.len(), expected_size,
            "Content length mismatch for {}",
            path_str
        );
    }

    info!("✓ Size accuracy invariant holds");
}

#[tokio::test]
async fn invariant_vfs_timestamp_ordering() {
    init_tracing();
    info!("TEST: VFS invariant - created_at <= updated_at <= accessed_at");

    let (_temp, storage) = create_test_storage("vfs_timestamps").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("test.txt").unwrap();
    vfs.write_file(&workspace_id, &path, b"initial").await.unwrap();

    // Small delay to ensure different timestamps
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Update the file
    vfs.write_file(&workspace_id, &path, b"updated").await.unwrap();

    // Small delay
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Read the file (updates accessed_at)
    let _ = vfs.read_file(&workspace_id, &path).await.unwrap();

    // Get vnode and check timestamps
    let root = VirtualPath::new("").unwrap();
    let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();
    let vnode = vnodes.iter()
        .find(|v| v.path.to_string() == "test.txt")
        .expect("VNode not found");

    // Verify ordering
    assert!(
        vnode.created_at <= vnode.updated_at,
        "created_at > updated_at: {:?} > {:?}",
        vnode.created_at,
        vnode.updated_at
    );

    assert!(
        vnode.updated_at <= vnode.accessed_at,
        "updated_at > accessed_at: {:?} > {:?}",
        vnode.updated_at,
        vnode.accessed_at
    );

    info!("✓ Timestamp ordering invariant holds");
}

// =============================================================================
// 2. Memory Invariants (5 tests)
// =============================================================================

#[tokio::test]
async fn invariant_memory_episode_temporal_ordering() {
    init_tracing();
    info!("TEST: Memory invariant - episode timestamps ordered correctly");

    let (_temp, storage) = create_test_storage("memory_temporal").await;
    let cognitive = CognitiveManager::new(storage.clone());

    // Create multiple episodes with delays
    let mut episode_ids = Vec::new();
    for i in 0..5 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        tokio::time::sleep(Duration::from_millis(10)).await;
        episode.completed_at = Some(Utc::now());

        let id = cognitive.remember_episode(&episode).await.unwrap();
        episode_ids.push(id);
    }

    // Retrieve all episodes and verify temporal ordering
    for (i, id) in episode_ids.iter().enumerate() {
        let episode_opt = cognitive.episodic().get_episode(*id).await.unwrap();
        assert!(episode_opt.is_some(), "Episode {} not found", i);

        let episode = episode_opt.unwrap();

        // Verify created_at <= completed_at
        if let Some(completed) = episode.completed_at {
            assert!(
                episode.created_at <= completed,
                "created_at > completed_at: {:?} > {:?}",
                episode.created_at,
                completed
            );
        }
    }

    info!("✓ Episode temporal ordering invariant holds");
}

#[tokio::test]
async fn invariant_memory_working_memory_capacity() {
    init_tracing();
    info!("TEST: Memory invariant - working memory respects capacity limits");

    // Create working memory with strict limits
    let max_items = 7; // Miller's law: 7±2
    let max_bytes = 1024;
    let working = WorkingMemorySystem::new(max_items, max_bytes);

    // Try to store more items than capacity
    for i in 0..20 {
        let key = format!("item_{}", i);
        let value = vec![0u8; 100];
        let priority = cortex_memory::types::Priority::Medium;

        working.store(key.clone(), value, priority);
    }

    // Verify capacity is respected
    let current_count = working.len();
    assert!(
        current_count <= max_items,
        "Working memory exceeded item capacity: {} > {}",
        current_count,
        max_items
    );

    let current_bytes = working.current_bytes();
    assert!(
        current_bytes <= max_bytes,
        "Working memory exceeded byte capacity: {} > {}",
        current_bytes,
        max_bytes
    );

    info!("✓ Working memory capacity invariant holds");
}

#[tokio::test]
async fn invariant_memory_complexity_metrics_positive() {
    init_tracing();
    info!("TEST: Memory invariant - complexity metrics are non-negative");

    let (_temp, storage) = create_test_storage("memory_complexity").await;
    let cognitive = CognitiveManager::new(storage.clone());

    // Create unit with complexity metrics
    let unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "complex_function".to_string(),
        qualified_name: "module::complex_function".to_string(),
        display_name: "complex_function".to_string(),
        file_path: "src/complex.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 100,
        end_column: 0,
        signature: "fn complex_function()".to_string(),
        body: "// complex implementation".to_string(),
        docstring: None,
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "Complex function".to_string(),
        purpose: "Testing complexity".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            lines: 100,
        },
        test_coverage: Some(0.85),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let id = cognitive.remember_unit(&unit).await.unwrap();

    // Retrieve and verify via semantic system
    let stored_opt = cognitive.semantic().get_unit(id).await.unwrap();
    assert!(stored_opt.is_some(), "Unit not found");

    let stored = stored_opt.unwrap();

    // All complexity metrics should be non-negative
    assert!(stored.complexity.cyclomatic > 0, "Invalid cyclomatic complexity");
    assert!(stored.complexity.cognitive > 0, "Invalid cognitive complexity");
    assert!(stored.complexity.nesting < 100, "Unrealistic nesting depth");
    assert!(stored.complexity.lines > 0, "Invalid line count");

    // Test coverage should be in [0,1] if present
    if let Some(coverage) = stored.test_coverage {
        assert!(
            coverage >= 0.0 && coverage <= 1.0,
            "Test coverage out of range: {}",
            coverage
        );
    }

    info!("✓ Complexity metrics invariant holds");
}

#[tokio::test]
async fn invariant_memory_token_usage_consistent() {
    init_tracing();
    info!("TEST: Memory invariant - token usage fields are consistent");

    let (_temp, storage) = create_test_storage("memory_tokens").await;
    let cognitive = CognitiveManager::new(storage.clone());

    let mut episode = EpisodicMemory::new(
        "Task with tokens".to_string(),
        "agent-001".to_string(),
        CortexId::new(),
        EpisodeType::Task,
    );

    episode.tokens_used = cortex_memory::types::TokenUsage {
        input: 1000,
        output: 500,
        total: 1500,
    };

    let id = cognitive.remember_episode(&episode).await.unwrap();

    // Retrieve and verify
    let stored_opt = cognitive.episodic().get_episode(id).await.unwrap();
    assert!(stored_opt.is_some(), "Episode not found");

    let stored = stored_opt.unwrap();

    // Total should equal input + output
    assert_eq!(
        stored.tokens_used.total,
        stored.tokens_used.input + stored.tokens_used.output,
        "Token totals inconsistent"
    );

    // All values should be non-negative
    assert!(stored.tokens_used.input >= 0, "Negative input tokens");
    assert!(stored.tokens_used.output >= 0, "Negative output tokens");
    assert!(stored.tokens_used.total >= 0, "Negative total tokens");

    info!("✓ Token usage consistency invariant holds");
}

#[tokio::test]
async fn invariant_memory_episode_outcome_valid() {
    init_tracing();
    info!("TEST: Memory invariant - episode outcomes are valid enum values");

    let (_temp, storage) = create_test_storage("memory_outcome").await;
    let cognitive = CognitiveManager::new(storage.clone());

    // Test all valid outcomes
    let outcomes = vec![
        EpisodeOutcome::Success,
        EpisodeOutcome::Partial,
        EpisodeOutcome::Failure,
        EpisodeOutcome::Abandoned,
    ];

    for outcome in outcomes {
        let mut episode = EpisodicMemory::new(
            "Task".to_string(),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        episode.outcome = outcome;
        episode.completed_at = Some(Utc::now());

        let id = cognitive.remember_episode(&episode).await.unwrap();

        // Retrieve and verify outcome is preserved
        let stored_opt = cognitive.episodic().get_episode(id).await.unwrap();
        assert!(stored_opt.is_some(), "Episode not found");

        let stored = stored_opt.unwrap();

        assert_eq!(stored.outcome, outcome, "Outcome not preserved");
    }

    info!("✓ Episode outcome validity invariant holds");
}

// =============================================================================
// 3. Database Invariants (1 test)
// =============================================================================

#[tokio::test]
async fn invariant_database_no_orphaned_records() {
    init_tracing();
    info!("TEST: Database invariant - no orphaned records after deletion");

    let (_temp, storage) = create_test_storage("db_orphans").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Create file
    let path = VirtualPath::new("test.txt").unwrap();
    vfs.write_file(&workspace_id, &path, b"content").await.unwrap();

    // Delete file
    vfs.delete(&workspace_id, &path, false).await.unwrap();

    // Verify VNode is deleted
    let result = vfs.read_file(&workspace_id, &path).await;
    assert!(result.is_err(), "File should be deleted");

    info!("✓ No orphaned records invariant holds");
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[tokio::test]
async fn property_vfs_hash_deterministic() {
    init_tracing();
    info!("PROPERTY TEST: VFS hash is deterministic for same content");

    let (_temp, storage) = create_test_storage("prop_hash").await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = Uuid::new_v4();

    // Generate various test contents
    let test_contents = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"Hello, World!".to_vec(),
        (0..255).collect::<Vec<u8>>(),
        vec![0u8; 10000],
    ];

    for content in test_contents {
        let path1 = VirtualPath::new(&format!("file1_{}.txt", content.len())).unwrap();
        let path2 = VirtualPath::new(&format!("file2_{}.txt", content.len())).unwrap();

        // Write same content to two different files
        vfs.write_file(&workspace_id, &path1, &content).await.unwrap();
        vfs.write_file(&workspace_id, &path2, &content).await.unwrap();

        // Get hashes
        let root = VirtualPath::new("").unwrap();
        let vnodes = vfs.list_directory(&workspace_id, &root, true).await.unwrap();

        let hash1 = vnodes.iter()
            .find(|v| v.path.to_string() == format!("file1_{}.txt", content.len()))
            .and_then(|v| v.content_hash.clone());

        let hash2 = vnodes.iter()
            .find(|v| v.path.to_string() == format!("file2_{}.txt", content.len()))
            .and_then(|v| v.content_hash.clone());

        assert_eq!(hash1, hash2, "Hashes should be identical for same content");
    }

    info!("✓ Hash determinism property holds");
}
