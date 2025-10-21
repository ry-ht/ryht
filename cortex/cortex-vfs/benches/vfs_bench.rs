//! Benchmarks for Virtual Filesystem operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use cortex_vfs::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

// Note: These benchmarks are designed to measure core VFS operations
// without requiring a database connection. They focus on algorithmic
// complexity and caching behavior.

fn bench_virtual_path_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("VirtualPath");

    group.bench_function("new", |b| {
        b.iter(|| {
            let path = VirtualPath::new(black_box("src/lib/very/deep/nested/path/to/file.rs"));
            black_box(path)
        })
    });

    group.bench_function("join", |b| {
        let base = VirtualPath::new("src").unwrap();
        b.iter(|| {
            let joined = base.join(black_box("lib/mod.rs"));
            black_box(joined)
        })
    });

    group.bench_function("parent", |b| {
        let path = VirtualPath::new("src/lib/deep/nested/file.rs").unwrap();
        b.iter(|| {
            let parent = path.parent();
            black_box(parent)
        })
    });

    group.bench_function("normalize", |b| {
        let path = VirtualPath::new("src/../lib/./deep/../../main.rs").unwrap();
        b.iter(|| {
            let normalized = path.clone().normalize();
            black_box(normalized)
        })
    });

    group.finish();
}

fn bench_content_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("ContentCache");

    // Benchmark cache hits
    group.bench_function("get_hit", |b| {
        let cache = ContentCache::new(1024 * 1024); // 1MB
        let content = vec![1u8; 1024]; // 1KB content
        cache.put("test_hash".to_string(), content);

        b.iter(|| {
            let result = cache.get(black_box("test_hash"));
            black_box(result)
        })
    });

    // Benchmark cache misses
    group.bench_function("get_miss", |b| {
        let cache = ContentCache::new(1024 * 1024);

        b.iter(|| {
            let result = cache.get(black_box("nonexistent"));
            black_box(result)
        })
    });

    // Benchmark puts
    for size in [1024, 10 * 1024, 100 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("put", size), size, |b, &size| {
            let cache = ContentCache::new(10 * 1024 * 1024); // 10MB cache
            let content = vec![1u8; size];
            let mut counter = 0u64;

            b.iter(|| {
                let hash = format!("hash_{}", counter);
                counter += 1;
                cache.put(black_box(hash), black_box(content.clone()));
            })
        });
    }

    // Benchmark eviction behavior
    group.bench_function("eviction", |b| {
        let mut counter = 0u64;

        b.iter(|| {
            let cache = ContentCache::new(1024); // Very small cache
            let content = vec![1u8; 100]; // 100 bytes

            // Fill cache beyond capacity to trigger evictions
            for _ in 0..20 {
                let hash = format!("hash_{}", counter);
                counter += 1;
                cache.put(hash, content.clone());
            }

            black_box(cache)
        })
    });

    group.finish();
}

fn bench_vnode_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("VNode");

    group.bench_function("new_file", |b| {
        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("test.txt").unwrap();

        b.iter(|| {
            let vnode = VNode::new_file(
                black_box(workspace_id),
                black_box(path.clone()),
                black_box("hash123".to_string()),
                black_box(1024),
            );
            black_box(vnode)
        })
    });

    group.bench_function("new_directory", |b| {
        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("src").unwrap();

        b.iter(|| {
            let vnode = VNode::new_directory(
                black_box(workspace_id),
                black_box(path.clone()),
            );
            black_box(vnode)
        })
    });

    group.bench_function("mark_modified", |b| {
        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("test.txt").unwrap();

        b.iter(|| {
            let mut vnode = VNode::new_file(workspace_id, path.clone(), "hash".to_string(), 100);
            vnode.mark_modified();
            black_box(vnode.is_modified)
        })
    });

    group.finish();
}

fn bench_language_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Language");

    let extensions = vec![
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "java",
        "cpp", "c", "cs", "rb", "php", "swift", "kt", "scala",
    ];

    for ext in &extensions {
        group.bench_with_input(BenchmarkId::new("detect", ext), ext, |b, &ext| {
            b.iter(|| {
                let lang = Language::from_extension(black_box(ext));
                black_box(lang)
            })
        });
    }

    group.finish();
}

fn bench_path_cache_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("PathCache");

    // Sequential access pattern
    group.bench_function("sequential_access", |b| {
        let cache = ContentCache::new(10 * 1024 * 1024);

        // Prepopulate with 100 items
        for i in 0..100 {
            let hash = format!("hash_{}", i);
            let content = vec![i as u8; 100];
            cache.put(hash, content);
        }

        let mut counter = 0;
        b.iter(|| {
            let hash = format!("hash_{}", counter % 100);
            counter += 1;
            let result = cache.get(black_box(&hash));
            black_box(result)
        })
    });

    // Random access pattern
    group.bench_function("random_access", |b| {
        let cache = ContentCache::new(10 * 1024 * 1024);

        // Prepopulate
        for i in 0..100 {
            let hash = format!("hash_{}", i);
            let content = vec![i as u8; 100];
            cache.put(hash, content);
        }

        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};
        let hasher = RandomState::new();

        let mut counter = 0u64;
        b.iter(|| {
            let mut h = hasher.build_hasher();
            counter.hash(&mut h);
            let idx = (h.finish() % 100) as usize;

            let hash = format!("hash_{}", idx);
            let result = cache.get(black_box(&hash));
            counter += 1;
            black_box(result)
        })
    });

    group.finish();
}

fn bench_content_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("Deduplication");

    group.bench_function("hash_content_small", |b| {
        let content = vec![1u8; 1024]; // 1KB

        b.iter(|| {
            let hash = blake3::hash(black_box(&content));
            black_box(hash.to_hex().to_string())
        })
    });

    group.bench_function("hash_content_medium", |b| {
        let content = vec![1u8; 100 * 1024]; // 100KB

        b.iter(|| {
            let hash = blake3::hash(black_box(&content));
            black_box(hash.to_hex().to_string())
        })
    });

    group.bench_function("hash_content_large", |b| {
        let content = vec![1u8; 1024 * 1024]; // 1MB

        b.iter(|| {
            let hash = blake3::hash(black_box(&content));
            black_box(hash.to_hex().to_string())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_virtual_path_operations,
    bench_content_cache,
    bench_vnode_operations,
    bench_language_detection,
    bench_path_cache_access_patterns,
    bench_content_deduplication,
);

criterion_main!(benches);
