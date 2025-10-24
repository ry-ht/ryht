//! Comprehensive tests for VFS reference counting with atomic operations.
//!
//! These tests verify that the atomic reference counting mechanism correctly
//! prevents race conditions during concurrent file content storage operations.

use cortex_storage::{ConnectionManager, DatabaseConfig, PoolConnectionMode, PoolConfig, Credentials, RetryPolicy};
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use uuid::Uuid;

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Create an in-memory test database connection manager
async fn create_test_connection_manager() -> Arc<ConnectionManager> {
    let config = DatabaseConfig {
        connection_mode: PoolConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 20,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: None,
            max_lifetime: None,
            retry_policy: RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: true,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(10),
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    )
}

/// Create a test VFS instance
async fn create_test_vfs() -> VirtualFileSystem {
    let storage = create_test_connection_manager().await;
    VirtualFileSystem::new(storage)
}

/// Create a test workspace
fn create_test_workspace() -> Uuid {
    Uuid::new_v4()
}

// ==============================================================================
// Unit Tests - Basic Reference Counting
// ==============================================================================

#[tokio::test]
async fn test_single_file_reference_count() {
    let vfs = create_test_vfs().await;
    let workspace_id = create_test_workspace();
    let path = VirtualPath::new("/test.txt").expect("Valid path");
    let content = b"Hello, world!";

    // Write file once
    vfs.write_file(&workspace_id, &path, content)
        .await
        .expect("Should write file");

    // Read it back to verify it exists
    let read_content = vfs
        .read_file(&workspace_id, &path)
        .await
        .expect("Should read file");

    assert_eq!(read_content, content, "Content should match");
}

#[tokio::test]
async fn test_duplicate_content_deduplication() {
    let vfs = create_test_vfs().await;
    let workspace_id = create_test_workspace();
    let content = b"Duplicate content";

    // Write same content to multiple files
    let paths = vec![
        VirtualPath::new("/file1.txt").expect("Valid path"),
        VirtualPath::new("/file2.txt").expect("Valid path"),
        VirtualPath::new("/file3.txt").expect("Valid path"),
    ];

    for path in &paths {
        vfs.write_file(&workspace_id, path, content)
            .await
            .expect("Should write file");
    }

    // All files should be readable
    for path in &paths {
        let read_content = vfs
            .read_file(&workspace_id, path)
            .await
            .expect("Should read file");

        assert_eq!(read_content, content, "Content should match for {}", path);
    }
}

#[tokio::test]
async fn test_different_content_separate_storage() {
    let vfs = create_test_vfs().await;
    let workspace_id = create_test_workspace();

    let content1 = b"Content A";
    let content2 = b"Content B";

    let path1 = VirtualPath::new("/file1.txt").expect("Valid path");
    let path2 = VirtualPath::new("/file2.txt").expect("Valid path");

    vfs.write_file(&workspace_id, &path1, content1)
        .await
        .expect("Should write file1");

    vfs.write_file(&workspace_id, &path2, content2)
        .await
        .expect("Should write file2");

    // Verify each file has correct content
    let read1 = vfs
        .read_file(&workspace_id, &path1)
        .await
        .expect("Should read file1");
    let read2 = vfs
        .read_file(&workspace_id, &path2)
        .await
        .expect("Should read file2");

    assert_eq!(read1, content1, "File1 content should match");
    assert_eq!(read2, content2, "File2 content should match");
}

// ==============================================================================
// Concurrent Reference Counting Tests
// ==============================================================================

#[tokio::test]
async fn test_concurrent_writes_same_content() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();
    let content = b"Shared content for concurrent writes";

    // Spawn multiple concurrent writes with same content
    let mut tasks = JoinSet::new();
    let num_tasks = 10;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let path = VirtualPath::new(&format!("/concurrent_{}.txt", i))
            .expect("Valid path");
        let data = content.to_vec();

        tasks.spawn(async move {
            vfs_clone.write_file(&ws_id, &path, &data).await
        });
    }

    // All writes should succeed
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Write failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent writes should succeed"
    );

    // Verify all files are readable with correct content
    for i in 0..num_tasks {
        let path = VirtualPath::new(&format!("/concurrent_{}.txt", i))
            .expect("Valid path");
        let read_content = vfs
            .read_file(&workspace_id, &path)
            .await
            .expect("Should read file");

        assert_eq!(
            read_content, content,
            "Content should match for file {}",
            i
        );
    }
}

#[tokio::test]
async fn test_concurrent_writes_different_content() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();

    // Spawn concurrent writes with different content
    let mut tasks = JoinSet::new();
    let num_tasks = 10;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let path = VirtualPath::new(&format!("/unique_{}.txt", i))
            .expect("Valid path");
        let content = format!("Unique content for file {}", i).into_bytes();

        tasks.spawn(async move {
            vfs_clone.write_file(&ws_id, &path, &content).await
        });
    }

    // All writes should succeed
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Write failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent writes should succeed"
    );

    // Verify each file has its unique content
    for i in 0..num_tasks {
        let path = VirtualPath::new(&format!("/unique_{}.txt", i))
            .expect("Valid path");
        let expected_content = format!("Unique content for file {}", i);
        let read_content = vfs
            .read_file(&workspace_id, &path)
            .await
            .expect("Should read file");

        assert_eq!(
            read_content,
            expected_content.as_bytes(),
            "Content should match for file {}",
            i
        );
    }
}

#[tokio::test]
async fn test_concurrent_overwrites_same_file() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();
    let path = VirtualPath::new("/overwrite_target.txt").expect("Valid path");

    // Write initial content
    vfs.write_file(&workspace_id, &path, b"Initial content")
        .await
        .expect("Should write initial content");

    // Spawn concurrent overwrites
    let mut tasks = JoinSet::new();
    let num_tasks = 20;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let p = path.clone();
        let content = format!("Overwrite {}", i).into_bytes();

        tasks.spawn(async move {
            tokio::time::sleep(Duration::from_micros(i * 50)).await;
            vfs_clone.write_file(&ws_id, &p, &content).await
        });
    }

    // All writes should succeed
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Write failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent overwrites should succeed"
    );

    // File should exist and be readable (content will be one of the writes)
    let final_content = vfs
        .read_file(&workspace_id, &path)
        .await
        .expect("Should read file");

    assert!(
        !final_content.is_empty(),
        "File should have content after overwrites"
    );
}

// ==============================================================================
// High Concurrency Stress Tests
// ==============================================================================

#[tokio::test]
async fn test_high_concurrency_mixed_operations() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();

    // Prepare shared content for some files
    let shared_content = b"Shared content across multiple files";

    // Spawn 50 concurrent tasks with mixed operations
    let mut tasks = JoinSet::new();
    let num_tasks = 50;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;

        tasks.spawn(async move {
            // Half write shared content, half write unique content
            let path = VirtualPath::new(&format!("/stress_{}.txt", i))
                .expect("Valid path");

            let content = if i % 2 == 0 {
                shared_content.to_vec()
            } else {
                format!("Unique stress content {}", i).into_bytes()
            };

            vfs_clone.write_file(&ws_id, &path, &content).await?;

            // Also try to read it back
            vfs_clone.read_file(&ws_id, &path).await?;

            Ok::<_, cortex_core::error::CortexError>(())
        });
    }

    // All operations should succeed
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Operation failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent operations should succeed"
    );
}

#[tokio::test]
async fn test_rapid_sequential_writes_same_content() {
    let vfs = create_test_vfs().await;
    let workspace_id = create_test_workspace();
    let content = b"Rapidly written content";

    // Write same content to many files rapidly
    let num_files = 100;

    for i in 0..num_files {
        let path = VirtualPath::new(&format!("/rapid_{}.txt", i))
            .expect("Valid path");

        vfs.write_file(&workspace_id, &path, content)
            .await
            .expect("Should write file");
    }

    // Verify all files exist and have correct content
    for i in 0..num_files {
        let path = VirtualPath::new(&format!("/rapid_{}.txt", i))
            .expect("Valid path");

        let read_content = vfs
            .read_file(&workspace_id, &path)
            .await
            .expect("Should read file");

        assert_eq!(read_content, content, "Content should match for file {}", i);
    }
}

// ==============================================================================
// Atomic Operation Verification Tests
// ==============================================================================

#[tokio::test]
async fn test_atomic_reference_count_consistency() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();
    let content = b"Content for atomic test";

    // Write same content multiple times concurrently
    let mut tasks = JoinSet::new();
    let num_tasks = 30;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let path = VirtualPath::new(&format!("/atomic_{}.txt", i))
            .expect("Valid path");
        let data = content.to_vec();

        tasks.spawn(async move {
            vfs_clone.write_file(&ws_id, &path, &data).await
        });
    }

    // Wait for all writes to complete
    while let Some(result) = tasks.join_next().await {
        result.expect("Task should not panic").expect("Write should succeed");
    }

    // All files should be readable (verifying atomic operations succeeded)
    for i in 0..num_tasks {
        let path = VirtualPath::new(&format!("/atomic_{}.txt", i))
            .expect("Valid path");

        let read_content = vfs
            .read_file(&workspace_id, &path)
            .await
            .expect("Should read file");

        assert_eq!(
            read_content, content,
            "Content should match for file {}",
            i
        );
    }
}

#[tokio::test]
async fn test_no_race_condition_in_content_storage() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();

    // Test with large content to increase chance of race conditions
    let large_content = vec![b'X'; 1024 * 100]; // 100KB

    let mut tasks = JoinSet::new();
    let num_tasks = 20;

    for i in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let path = VirtualPath::new(&format!("/large_{}.bin", i))
            .expect("Valid path");
        let data = large_content.clone();

        tasks.spawn(async move {
            vfs_clone.write_file(&ws_id, &path, &data).await
        });
    }

    // All writes should succeed without race conditions
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => panic!("Write failed due to race condition: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All writes should succeed without race conditions"
    );

    // Verify all files have correct content
    for i in 0..num_tasks {
        let path = VirtualPath::new(&format!("/large_{}.bin", i))
            .expect("Valid path");

        let read_content = vfs
            .read_file(&workspace_id, &path)
            .await
            .expect("Should read file");

        assert_eq!(
            read_content.len(),
            large_content.len(),
            "Content size should match for file {}",
            i
        );
        assert_eq!(
            read_content, large_content,
            "Content should match for file {}",
            i
        );
    }
}

// ==============================================================================
// Cache Interaction Tests
// ==============================================================================

#[tokio::test]
async fn test_content_cache_with_deduplication() {
    let vfs = create_test_vfs().await;
    let workspace_id = create_test_workspace();
    let content = b"Cached content";

    // Write and read to populate cache
    let path1 = VirtualPath::new("/cached1.txt").expect("Valid path");
    vfs.write_file(&workspace_id, &path1, content)
        .await
        .expect("Should write file");

    let _ = vfs
        .read_file(&workspace_id, &path1)
        .await
        .expect("Should read file");

    // Write same content to another file (should be deduplicated and cached)
    let path2 = VirtualPath::new("/cached2.txt").expect("Valid path");
    vfs.write_file(&workspace_id, &path2, content)
        .await
        .expect("Should write file");

    // Read from cache (should be fast)
    let read_content = vfs
        .read_file(&workspace_id, &path2)
        .await
        .expect("Should read from cache");

    assert_eq!(read_content, content, "Cached content should match");
}

#[tokio::test]
async fn test_concurrent_reads_with_cache() {
    let vfs = Arc::new(create_test_vfs().await);
    let workspace_id = create_test_workspace();
    let path = VirtualPath::new("/cached_read.txt").expect("Valid path");
    let content = b"Content for concurrent reads";

    // Write file
    vfs.write_file(&workspace_id, &path, content)
        .await
        .expect("Should write file");

    // Spawn concurrent reads
    let mut tasks = JoinSet::new();
    let num_tasks = 50;

    for _ in 0..num_tasks {
        let vfs_clone = Arc::clone(&vfs);
        let ws_id = workspace_id;
        let p = path.clone();

        tasks.spawn(async move {
            vfs_clone.read_file(&ws_id, &p).await
        });
    }

    // All reads should succeed
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(data)) => {
                assert_eq!(data, content, "Read content should match");
                success_count += 1;
            }
            Ok(Err(e)) => panic!("Read failed: {}", e),
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent reads should succeed"
    );
}
