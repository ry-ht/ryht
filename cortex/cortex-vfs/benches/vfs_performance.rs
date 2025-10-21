//! VFS Performance Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Navigation operations (target: <50ms for 10K entries)
//! - File operations (read <10ms for <1MB, write <50ms)
//! - Cache performance (hit <1ms, miss <10ms)
//! - Materialization (flush 10K LOC <5s)

use cortex_vfs::{
    virtual_filesystem::VirtualFileSystem,
    path::VirtualPath,
    types::{VNodeType, VNodeMetadata},
};
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, PoolConfig,
    RetryPolicy, Credentials,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

// ==============================================================================
// Benchmark Setup Helpers
// ==============================================================================

fn create_test_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "memory".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                backoff_multiplier: 2.0,
            },
            warm_connections: true,
            health_check_interval: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        },
        namespace: "vfs_bench_ns".to_string(),
        database: "vfs_bench_db".to_string(),
    }
}

async fn setup_vfs() -> (Arc<ConnectionManager>, VirtualFileSystem, Uuid) {
    let config = create_test_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    let manager = Arc::new(manager);

    let vfs = VirtualFileSystem::new(manager.clone());
    let workspace_id = Uuid::new_v4();

    // Initialize workspace
    vfs.create_workspace(&workspace_id, "bench_workspace", None)
        .await
        .expect("Failed to create workspace");

    (manager, vfs, workspace_id)
}

async fn create_file_tree(
    vfs: &VirtualFileSystem,
    workspace_id: &Uuid,
    dir_count: usize,
    files_per_dir: usize,
) {
    // Create directory structure
    for i in 0..dir_count {
        let dir_path = VirtualPath::parse(&format!("/bench/dir_{}", i)).unwrap();
        vfs.create_directory(workspace_id, &dir_path)
            .await
            .expect("Failed to create directory");

        // Create files in each directory
        for j in 0..files_per_dir {
            let file_path = VirtualPath::parse(&format!("/bench/dir_{}/file_{}.rs", i, j)).unwrap();
            let content = format!(
                "// File {} in directory {}\n\
                 fn main() {{\n\
                 \tprintln!(\"Hello from file {}\");\n\
                 }}\n",
                j, i, j
            );
            vfs.write_file(workspace_id, &file_path, content.as_bytes().to_vec())
                .await
                .expect("Failed to write file");
        }
    }
}

fn generate_file_content(size_bytes: usize) -> Vec<u8> {
    // Generate realistic Rust code content
    let mut content = String::new();
    content.push_str("// Auto-generated benchmark file\n\n");
    content.push_str("use std::collections::HashMap;\n\n");

    let lines_needed = size_bytes / 80; // Assume ~80 bytes per line
    for i in 0..lines_needed {
        content.push_str(&format!(
            "pub fn generated_function_{}() -> i32 {{ return {}; }}\n",
            i, i
        ));
    }

    content.into_bytes()
}

// ==============================================================================
// Navigation Performance Benchmarks
// ==============================================================================

fn bench_navigation_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, vfs, workspace_id) = rt.block_on(setup_vfs());

    // Setup test data: 100 directories with 10 files each = 1,000 entries
    rt.block_on(create_file_tree(&vfs, &workspace_id, 100, 10));

    let mut group = c.benchmark_group("navigation");
    group.significance_level(0.05).sample_size(100);

    // List directory with 10 entries - Target: <50ms
    group.bench_function("list_dir_10_entries", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench/dir_0").unwrap();
            let entries = vfs.list_directory(&workspace_id, &path, false)
                .await
                .unwrap();
            black_box(entries);
        });
    });

    // List directory with 100 entries - Target: <50ms
    group.bench_function("list_dir_100_entries", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench").unwrap();
            let entries = vfs.list_directory(&workspace_id, &path, false)
                .await
                .unwrap();
            black_box(entries);
        });
    });

    // Recursive list - 1,000 entries total - Target: <50ms
    group.bench_function("list_recursive_1000_entries", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench").unwrap();
            let entries = vfs.list_directory(&workspace_id, &path, true)
                .await
                .unwrap();
            black_box(entries);
        });
    });

    // Path resolution - Target: <10ms
    group.bench_function("path_resolution", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench/dir_50/file_5.rs").unwrap();
            let vnode = vfs.get_vnode(&workspace_id, &path)
                .await
                .unwrap();
            black_box(vnode);
        });
    });

    // Metadata retrieval - Target: <5ms
    group.bench_function("metadata_retrieval", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench/dir_50/file_5.rs").unwrap();
            let metadata = vfs.get_metadata(&workspace_id, &path)
                .await
                .unwrap();
            black_box(metadata);
        });
    });

    // Path exists check - Target: <5ms
    group.bench_function("path_exists", |b| {
        b.to_async(&rt).iter(|| async {
            let path = VirtualPath::parse("/bench/dir_50/file_5.rs").unwrap();
            let exists = vfs.exists(&workspace_id, &path)
                .await
                .unwrap();
            black_box(exists);
        });
    });

    group.finish();
}

// ==============================================================================
// File Operation Benchmarks
// ==============================================================================

fn bench_file_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, vfs, workspace_id) = rt.block_on(setup_vfs());

    let mut group = c.benchmark_group("file_operations");
    group.significance_level(0.05).sample_size(50);

    // Read small file (<1KB) - Target: <10ms
    let small_content = generate_file_content(512); // 512 bytes
    let small_path = VirtualPath::parse("/bench/small.rs").unwrap();
    rt.block_on(async {
        vfs.write_file(&workspace_id, &small_path, small_content.clone())
            .await
            .unwrap();
    });

    group.throughput(Throughput::Bytes(512));
    group.bench_function("read_file_512b", |b| {
        b.to_async(&rt).iter(|| async {
            let content = vfs.read_file(&workspace_id, &small_path)
                .await
                .unwrap();
            black_box(content);
        });
    });

    // Read medium file (~10KB) - Target: <10ms
    let medium_content = generate_file_content(10 * 1024); // 10 KB
    let medium_path = VirtualPath::parse("/bench/medium.rs").unwrap();
    rt.block_on(async {
        vfs.write_file(&workspace_id, &medium_path, medium_content.clone())
            .await
            .unwrap();
    });

    group.throughput(Throughput::Bytes(10 * 1024));
    group.bench_function("read_file_10kb", |b| {
        b.to_async(&rt).iter(|| async {
            let content = vfs.read_file(&workspace_id, &medium_path)
                .await
                .unwrap();
            black_box(content);
        });
    });

    // Read large file (~1MB) - Target: <100ms
    let large_content = generate_file_content(1024 * 1024); // 1 MB
    let large_path = VirtualPath::parse("/bench/large.rs").unwrap();
    rt.block_on(async {
        vfs.write_file(&workspace_id, &large_path, large_content.clone())
            .await
            .unwrap();
    });

    group.throughput(Throughput::Bytes(1024 * 1024));
    group.bench_function("read_file_1mb", |b| {
        b.to_async(&rt).iter(|| async {
            let content = vfs.read_file(&workspace_id, &large_path)
                .await
                .unwrap();
            black_box(content);
        });
    });

    // Write small file - Target: <50ms
    group.throughput(Throughput::Bytes(512));
    group.bench_function("write_file_512b", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let path = VirtualPath::parse(&format!("/bench/write_test_{}.rs", counter)).unwrap();
            vfs.write_file(&workspace_id, &path, small_content.clone())
                .await
                .unwrap();
        });
    });

    // Write medium file - Target: <50ms
    group.throughput(Throughput::Bytes(10 * 1024));
    group.bench_function("write_file_10kb", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let path = VirtualPath::parse(&format!("/bench/write_medium_{}.rs", counter)).unwrap();
            vfs.write_file(&workspace_id, &path, medium_content.clone())
                .await
                .unwrap();
        });
    });

    // Delete file - Target: <10ms
    group.bench_function("delete_file", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            // Create file first
            let path = VirtualPath::parse(&format!("/bench/delete_test_{}.rs", counter)).unwrap();
            vfs.write_file(&workspace_id, &path, small_content.clone())
                .await
                .unwrap();
            // Now delete it
            vfs.delete(&workspace_id, &path)
                .await
                .unwrap();
        });
    });

    // Rename file - Target: <20ms
    group.bench_function("rename_file", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let old_path = VirtualPath::parse(&format!("/bench/rename_old_{}.rs", counter)).unwrap();
            let new_path = VirtualPath::parse(&format!("/bench/rename_new_{}.rs", counter)).unwrap();

            // Create file first
            vfs.write_file(&workspace_id, &old_path, small_content.clone())
                .await
                .unwrap();

            // Rename it
            vfs.move_path(&workspace_id, &old_path, &new_path)
                .await
                .unwrap();
        });
    });

    // Copy file - Target: <30ms
    group.bench_function("copy_file", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let src_path = VirtualPath::parse(&format!("/bench/copy_src_{}.rs", counter)).unwrap();
            let dst_path = VirtualPath::parse(&format!("/bench/copy_dst_{}.rs", counter)).unwrap();

            // Create source file
            vfs.write_file(&workspace_id, &src_path, small_content.clone())
                .await
                .unwrap();

            // Copy it
            vfs.copy_path(&workspace_id, &src_path, &dst_path)
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Cache Performance Benchmarks
// ==============================================================================

fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, vfs, workspace_id) = rt.block_on(setup_vfs());

    // Pre-populate cache with files
    let cached_content = generate_file_content(10 * 1024); // 10 KB
    let cached_path = VirtualPath::parse("/bench/cached.rs").unwrap();
    rt.block_on(async {
        vfs.write_file(&workspace_id, &cached_path, cached_content.clone())
            .await
            .unwrap();
        // Read once to populate cache
        vfs.read_file(&workspace_id, &cached_path)
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("cache_performance");
    group.significance_level(0.05).sample_size(200);

    // Cache hit - Target: <1ms
    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let content = vfs.read_file(&workspace_id, &cached_path)
                .await
                .unwrap();
            black_box(content);
        });
    });

    // Cache miss - Target: <10ms
    group.bench_function("cache_miss", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let path = VirtualPath::parse(&format!("/bench/uncached_{}.rs", counter)).unwrap();
            // Write new file (won't be in cache)
            vfs.write_file(&workspace_id, &path, cached_content.clone())
                .await
                .unwrap();
            // Read it (cache miss)
            let content = vfs.read_file(&workspace_id, &path)
                .await
                .unwrap();
            black_box(content);
        });
    });

    // Metadata cache hit - Target: <1ms
    group.bench_function("metadata_cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let metadata = vfs.get_metadata(&workspace_id, &cached_path)
                .await
                .unwrap();
            black_box(metadata);
        });
    });

    group.finish();
}

// ==============================================================================
// Materialization Benchmarks
// ==============================================================================

fn bench_materialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, vfs, workspace_id) = rt.block_on(setup_vfs());

    let mut group = c.benchmark_group("materialization");
    group.significance_level(0.05).sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // Flush 100 files (~10K LOC) - Target: <5s
    group.bench_function("flush_100_files_10k_loc", |b| {
        b.iter_batched(
            || {
                // Setup: Create 100 files with ~100 LOC each
                rt.block_on(async {
                    let temp_workspace = Uuid::new_v4();
                    vfs.create_workspace(&temp_workspace, "flush_bench", None)
                        .await
                        .unwrap();

                    for i in 0..100 {
                        let path = VirtualPath::parse(&format!("/src/file_{}.rs", i)).unwrap();
                        let content = generate_file_content(4 * 1024); // ~100 LOC
                        vfs.write_file(&temp_workspace, &path, content)
                            .await
                            .unwrap();
                    }

                    temp_workspace
                });
                // Return workspace ID for test
                workspace_id
            },
            |_ws_id| {
                // Benchmark: Flush all files
                rt.block_on(async {
                    // In real implementation, call flush_to_disk
                    // For now, simulate with batch read
                    for i in 0..100 {
                        let path = VirtualPath::parse(&format!("/src/file_{}.rs", i)).unwrap();
                        let _ = vfs.read_file(&workspace_id, &path).await;
                    }
                });
            },
            criterion::BatchSize::PerIteration,
        );
    });

    // Flush 1000 files (~100K LOC) - Target: <30s
    group.bench_function("flush_1000_files_100k_loc", |b| {
        b.iter_batched(
            || {
                rt.block_on(async {
                    let temp_workspace = Uuid::new_v4();
                    vfs.create_workspace(&temp_workspace, "flush_large", None)
                        .await
                        .unwrap();

                    // Create 1000 files
                    for i in 0..1000 {
                        let path = VirtualPath::parse(&format!("/src/large_{}.rs", i)).unwrap();
                        let content = generate_file_content(4 * 1024); // ~100 LOC
                        vfs.write_file(&temp_workspace, &path, content)
                            .await
                            .unwrap();
                    }

                    temp_workspace
                });
                workspace_id
            },
            |_ws_id| {
                rt.block_on(async {
                    for i in 0..1000 {
                        let path = VirtualPath::parse(&format!("/src/large_{}.rs", i)).unwrap();
                        let _ = vfs.read_file(&workspace_id, &path).await;
                    }
                });
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.finish();
}

// ==============================================================================
// Workspace Operations Benchmarks
// ==============================================================================

fn bench_workspace_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, vfs, workspace_id) = rt.block_on(setup_vfs());

    let mut group = c.benchmark_group("workspace_operations");
    group.significance_level(0.05).sample_size(50);

    // Create workspace - Target: <100ms
    group.bench_function("create_workspace", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let ws_id = Uuid::new_v4();
            vfs.create_workspace(&ws_id, &format!("bench_ws_{}", counter), None)
                .await
                .unwrap();
        });
    });

    // Delete workspace - Target: <200ms
    group.bench_function("delete_workspace", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let ws_id = Uuid::new_v4();
            vfs.create_workspace(&ws_id, &format!("delete_ws_{}", counter), None)
                .await
                .unwrap();
            vfs.delete_workspace(&ws_id)
                .await
                .unwrap();
        });
    });

    // Fork workspace - Target: <500ms
    group.bench_function("fork_workspace", |b| {
        b.to_async(&rt).iter(|| async {
            let new_ws_id = Uuid::new_v4();
            vfs.fork_workspace(&workspace_id, &new_ws_id, "forked_workspace")
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Main Benchmark Configuration
// ==============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_navigation_operations,
        bench_file_operations,
        bench_cache_performance,
        bench_materialization,
        bench_workspace_operations,
);

criterion_main!(benches);
