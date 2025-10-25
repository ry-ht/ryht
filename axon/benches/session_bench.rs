//! Benchmarks for session management operations.
//!
//! This benchmark suite measures the performance of session operations:
//! - Session discovery (projects and sessions)
//! - Session caching with different TTLs
//! - Session filtering and searching
//! - Cache hit vs miss performance

use cc_sdk::session::{
    cache::{CacheConfig, SessionCache},
    filter::{SessionFilter, SortBy},
    types::{Project, Session},
};
use cc_sdk::core::SessionId;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::PathBuf;
use std::time::Duration;

/// Create a realistic set of test projects.
fn create_test_projects(count: usize) -> Vec<Project> {
    (0..count)
        .map(|i| {
            Project::new(
                format!("project-{}", i),
                PathBuf::from(format!("/test/project-{}", i)),
                vec![],
            )
        })
        .collect()
}

/// Create a realistic set of test sessions.
fn create_test_sessions(count: usize) -> Vec<Session> {
    (0..count)
        .map(|i| {
            Session::new(
                SessionId::new(format!("session-{}", i)),
                PathBuf::from(format!("/test/session-{}", i)),
                Utc::now(),
                Some(format!("Test message for session {}", i)),
            )
        })
        .collect()
}

/// Benchmark cache operations with different data sizes.
fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    for size in [10, 50, 100, 500].iter() {
        let projects = create_test_projects(*size);
        let sessions = create_test_sessions(*size);

        group.bench_with_input(BenchmarkId::new("set_projects", size), size, |b, _| {
            let cache = SessionCache::default();
            b.iter(|| {
                cache.set_projects(black_box(projects.clone()));
            });
        });

        group.bench_with_input(BenchmarkId::new("get_projects", size), size, |b, _| {
            let cache = SessionCache::default();
            cache.set_projects(projects.clone());
            b.iter(|| {
                let result = black_box(cache.get_projects());
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("set_sessions", size), size, |b, _| {
            let cache = SessionCache::default();
            b.iter(|| {
                cache.set_sessions(black_box("project-1".to_string()), black_box(sessions.clone()));
            });
        });

        group.bench_with_input(BenchmarkId::new("get_sessions", size), size, |b, _| {
            let cache = SessionCache::default();
            cache.set_sessions("project-1".to_string(), sessions.clone());
            b.iter(|| {
                let result = black_box(cache.get_sessions("project-1"));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark cache hit vs miss performance.
fn bench_cache_hit_miss(c: &mut Criterion) {
    let projects = create_test_projects(100);
    let sessions = create_test_sessions(100);

    c.bench_function("cache_hit_projects", |b| {
        let cache = SessionCache::default();
        cache.set_projects(projects.clone());
        b.iter(|| {
            let result = black_box(cache.get_projects());
            black_box(result)
        });
    });

    c.bench_function("cache_miss_projects", |b| {
        let cache = SessionCache::default();
        b.iter(|| {
            let result = black_box(cache.get_projects());
            black_box(result)
        });
    });

    c.bench_function("cache_hit_sessions", |b| {
        let cache = SessionCache::default();
        cache.set_sessions("project-1".to_string(), sessions.clone());
        b.iter(|| {
            let result = black_box(cache.get_sessions("project-1"));
            black_box(result)
        });
    });

    c.bench_function("cache_miss_sessions", |b| {
        let cache = SessionCache::default();
        b.iter(|| {
            let result = black_box(cache.get_sessions("nonexistent"));
            black_box(result)
        });
    });
}

/// Benchmark cache with different TTL configurations.
fn bench_cache_ttl(c: &mut Criterion) {
    let projects = create_test_projects(100);

    c.bench_function("cache_with_short_ttl", |b| {
        let config = CacheConfig {
            ttl: Duration::from_millis(100),
            enabled: true,
        };
        let cache = SessionCache::new(config);
        cache.set_projects(projects.clone());
        b.iter(|| {
            let result = black_box(cache.get_projects());
            black_box(result)
        });
    });

    c.bench_function("cache_with_long_ttl", |b| {
        let config = CacheConfig {
            ttl: Duration::from_secs(3600),
            enabled: true,
        };
        let cache = SessionCache::new(config);
        cache.set_projects(projects.clone());
        b.iter(|| {
            let result = black_box(cache.get_projects());
            black_box(result)
        });
    });

    c.bench_function("cache_disabled", |b| {
        let config = CacheConfig {
            ttl: Duration::from_secs(300),
            enabled: false,
        };
        let cache = SessionCache::new(config);
        cache.set_projects(projects.clone());
        b.iter(|| {
            let result = black_box(cache.get_projects());
            black_box(result)
        });
    });
}

/// Benchmark cache cleanup operations.
fn bench_cache_cleanup(c: &mut Criterion) {
    c.bench_function("cache_cleanup_empty", |b| {
        let cache = SessionCache::default();
        b.iter(|| {
            let removed = black_box(cache.cleanup());
            black_box(removed)
        });
    });

    c.bench_function("cache_cleanup_valid", |b| {
        let cache = SessionCache::default();
        cache.set_projects(create_test_projects(100));
        for i in 0..10 {
            cache.set_sessions(format!("project-{}", i), create_test_sessions(50));
        }
        b.iter(|| {
            let removed = black_box(cache.cleanup());
            black_box(removed)
        });
    });

    c.bench_function("cache_cleanup_expired", |b| {
        let config = CacheConfig {
            ttl: Duration::from_millis(1),
            enabled: true,
        };
        let cache = SessionCache::new(config);
        cache.set_projects(create_test_projects(100));
        for i in 0..10 {
            cache.set_sessions(format!("project-{}", i), create_test_sessions(50));
        }
        std::thread::sleep(Duration::from_millis(5));
        b.iter(|| {
            let removed = black_box(cache.cleanup());
            black_box(removed)
        });
    });
}

/// Benchmark cache clear operations.
fn bench_cache_clear(c: &mut Criterion) {
    c.bench_function("cache_clear_all", |b| {
        let cache = SessionCache::default();
        cache.set_projects(create_test_projects(100));
        for i in 0..10 {
            cache.set_sessions(format!("project-{}", i), create_test_sessions(50));
        }
        b.iter(|| {
            cache.clear();
        });
    });

    c.bench_function("cache_clear_projects", |b| {
        let cache = SessionCache::default();
        cache.set_projects(create_test_projects(100));
        b.iter(|| {
            cache.clear_projects();
        });
    });

    c.bench_function("cache_clear_sessions", |b| {
        let cache = SessionCache::default();
        for i in 0..10 {
            cache.set_sessions(format!("project-{}", i), create_test_sessions(50));
        }
        b.iter(|| {
            cache.clear_sessions("project-5");
        });
    });
}

/// Benchmark session filter construction.
fn bench_filter_construction(c: &mut Criterion) {
    c.bench_function("filter_new", |b| {
        b.iter(|| {
            let filter = black_box(SessionFilter::new());
            black_box(filter)
        });
    });

    c.bench_function("filter_simple", |b| {
        b.iter(|| {
            let filter = black_box(
                SessionFilter::new()
                    .with_project_id("test-project")
                    .with_limit(10),
            );
            black_box(filter)
        });
    });

    c.bench_function("filter_complex", |b| {
        b.iter(|| {
            let filter = black_box(
                SessionFilter::new()
                    .with_project_id("test-project")
                    .with_date_range(Some(Utc::now()), Some(Utc::now()))
                    .with_content_search("error")
                    .with_regex(true)
                    .with_case_sensitive(true)
                    .with_min_messages(10)
                    .with_max_messages(100)
                    .with_sort_by(SortBy::CreatedDesc)
                    .with_limit(50)
                    .with_offset(10),
            );
            black_box(filter)
        });
    });
}

/// Benchmark cache len and is_empty operations.
fn bench_cache_info(c: &mut Criterion) {
    let cache = SessionCache::default();
    cache.set_projects(create_test_projects(100));
    for i in 0..10 {
        cache.set_sessions(format!("project-{}", i), create_test_sessions(50));
    }

    c.bench_function("cache_len", |b| {
        b.iter(|| {
            let result = black_box(cache.len());
            black_box(result)
        });
    });

    c.bench_function("cache_is_empty", |b| {
        b.iter(|| {
            let result = black_box(cache.is_empty());
            black_box(result)
        });
    });
}

/// Benchmark concurrent cache access (thread-safety overhead).
fn bench_cache_concurrent(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    c.bench_function("cache_concurrent_reads", |b| {
        let cache = Arc::new(SessionCache::default());
        cache.set_projects(create_test_projects(100));

        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let cache = Arc::clone(&cache);
                    thread::spawn(move || {
                        let result = cache.get_projects();
                        black_box(result)
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        });
    });

    c.bench_function("cache_concurrent_writes", |b| {
        let cache = Arc::new(SessionCache::default());
        let projects = create_test_projects(100);

        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|i| {
                    let cache = Arc::clone(&cache);
                    let projects = projects.clone();
                    thread::spawn(move || {
                        cache.set_sessions(format!("project-{}", i), projects.clone());
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_cache_hit_miss,
    bench_cache_ttl,
    bench_cache_cleanup,
    bench_cache_clear,
    bench_filter_construction,
    bench_cache_info,
    bench_cache_concurrent,
);

criterion_main!(benches);
