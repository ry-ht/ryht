//! Benchmarks for binary discovery operations.
//!
//! This benchmark suite measures the performance of Claude binary discovery,
//! focusing on:
//! - Uncached discovery (full filesystem scan)
//! - Cached discovery (OnceLock lookup)
//! - Discovery with different builder configurations
//! - Version extraction and parsing

use cc_sdk::binary::{
    compare_versions, discover_installations, extract_version_from_output, find_claude_binary,
    DiscoveryBuilder, Version,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark uncached binary discovery.
///
/// This tests the full discovery process including:
/// - Checking which/where commands
/// - Scanning NVM directories
/// - Scanning standard installation paths
/// - Path deduplication
/// - Version extraction
fn bench_discover_installations(c: &mut Criterion) {
    c.bench_function("discover_installations_full", |b| {
        b.iter(|| {
            // Note: This will use cached results after first call
            // To truly test uncached, we'd need to clear OnceLock which isn't possible
            let installations = black_box(discover_installations());
            black_box(installations)
        });
    });
}

/// Benchmark cached binary discovery.
///
/// After the first call, find_claude_binary uses OnceLock for instant retrieval.
/// This benchmarks the cached path which should be extremely fast.
fn bench_find_claude_binary_cached(c: &mut Criterion) {
    // Warm up the cache
    let _ = find_claude_binary();

    c.bench_function("find_claude_binary_cached", |b| {
        b.iter(|| {
            let result = black_box(find_claude_binary());
            black_box(result)
        });
    });
}

/// Benchmark discovery with custom builder configurations.
fn bench_discovery_builder(c: &mut Criterion) {
    c.bench_function("discovery_builder_skip_nvm", |b| {
        b.iter(|| {
            let builder = black_box(DiscoveryBuilder::new().skip_nvm(true).use_cache(false));
            let installations = black_box(builder.discover());
            black_box(installations)
        });
    });

    c.bench_function("discovery_builder_skip_system", |b| {
        b.iter(|| {
            let builder = black_box(DiscoveryBuilder::new().skip_system(true).use_cache(false));
            let installations = black_box(builder.discover());
            black_box(installations)
        });
    });

    c.bench_function("discovery_builder_cached", |b| {
        b.iter(|| {
            let builder = black_box(DiscoveryBuilder::new().use_cache(true));
            let installations = black_box(builder.discover());
            black_box(installations)
        });
    });
}

/// Benchmark version parsing operations.
fn bench_version_parsing(c: &mut Criterion) {
    let version_strings = vec![
        "1.0.41",
        "2.0.0-beta.1",
        "1.2.3-rc.2+build.456",
        "0.1.0",
        "10.20.30",
    ];

    c.bench_function("version_parse_simple", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.0.41"));
            black_box(v)
        });
    });

    c.bench_function("version_parse_prerelease", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("2.0.0-beta.1"));
            black_box(v)
        });
    });

    c.bench_function("version_parse_full", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.2.3-rc.2+build.456"));
            black_box(v)
        });
    });

    c.bench_function("version_parse_batch", |b| {
        b.iter(|| {
            for version_str in &version_strings {
                let v = black_box(Version::parse(version_str));
                black_box(v);
            }
        });
    });
}

/// Benchmark version comparison operations.
fn bench_version_comparison(c: &mut Criterion) {
    let v1 = Version::parse("1.0.41").unwrap();
    let v2 = Version::parse("1.0.40").unwrap();
    let v3 = Version::parse("2.0.0").unwrap();

    c.bench_function("version_compare_direct", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    c.bench_function("version_compare_strings", |b| {
        b.iter(|| {
            let result = black_box(compare_versions("1.0.41", "1.0.40"));
            black_box(result)
        });
    });

    c.bench_function("version_compare_major_diff", |b| {
        b.iter(|| {
            let result = black_box(v3.cmp(&v1));
            black_box(result)
        });
    });
}

/// Benchmark version extraction from command output.
fn bench_version_extraction(c: &mut Criterion) {
    let outputs = vec![
        b"claude version 1.0.41\n".to_vec(),
        b"Claude Code CLI v2.0.0-beta.1\n".to_vec(),
        b"version: 1.2.3-rc.2+build.456\n".to_vec(),
        b"10.20.30\n".to_vec(),
    ];

    c.bench_function("extract_version_simple", |b| {
        let output = b"claude version 1.0.41\n";
        b.iter(|| {
            let version = black_box(extract_version_from_output(output));
            black_box(version)
        });
    });

    c.bench_function("extract_version_batch", |b| {
        b.iter(|| {
            for output in &outputs {
                let version = black_box(extract_version_from_output(output));
                black_box(version);
            }
        });
    });

    c.bench_function("extract_version_complex", |b| {
        let output = b"Claude Code CLI v2.0.0-beta.1+build.123\nCopyright 2024\n";
        b.iter(|| {
            let version = black_box(extract_version_from_output(output));
            black_box(version)
        });
    });
}

/// Benchmark version struct operations.
fn bench_version_operations(c: &mut Criterion) {
    let version = Version::parse("1.2.3-beta.1+build.456").unwrap();

    c.bench_function("version_is_prerelease", |b| {
        b.iter(|| {
            let result = black_box(version.is_prerelease());
            black_box(result)
        });
    });

    c.bench_function("version_core_version", |b| {
        b.iter(|| {
            let result = black_box(version.core_version());
            black_box(result)
        });
    });

    c.bench_function("version_to_string", |b| {
        b.iter(|| {
            let result = black_box(version.to_string());
            black_box(result)
        });
    });

    c.bench_function("version_clone", |b| {
        b.iter(|| {
            let cloned = black_box(version.clone());
            black_box(cloned)
        });
    });
}

criterion_group!(
    benches,
    bench_discover_installations,
    bench_find_claude_binary_cached,
    bench_discovery_builder,
    bench_version_parsing,
    bench_version_comparison,
    bench_version_extraction,
    bench_version_operations,
);

criterion_main!(benches);
