//! Benchmarks specifically for version parsing and comparison operations.
//!
//! This benchmark suite provides detailed performance analysis of version-related
//! operations that are critical for binary discovery:
//! - Parsing different version formats
//! - Comparing versions (major, minor, patch, prerelease)
//! - Version string operations
//! - Batch version processing

use cc_sdk::binary::{compare_versions, Version};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::cmp::Ordering;

/// Test version strings covering various formats.
const VERSION_SAMPLES: &[&str] = &[
    "0.0.1",
    "0.1.0",
    "1.0.0",
    "1.0.41",
    "2.0.0",
    "10.20.30",
    "1.0.0-alpha",
    "1.0.0-alpha.1",
    "1.0.0-beta",
    "1.0.0-beta.2",
    "1.0.0-rc.1",
    "1.0.0-0.3.7",
    "1.0.0-x.7.z.92",
    "1.0.0+20130313144700",
    "1.0.0-beta+exp.sha.5114f85",
    "1.0.0+21AF26D3-117B344092BD",
];

/// Benchmark parsing versions of different complexity.
fn bench_version_parsing_by_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_parse_complexity");

    // Simple version (major.minor.patch)
    group.bench_function("simple", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.0.41"));
            black_box(v)
        });
    });

    // With prerelease
    group.bench_function("with_prerelease", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.0.0-beta.1"));
            black_box(v)
        });
    });

    // With build metadata
    group.bench_function("with_build", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.0.0+20130313144700"));
            black_box(v)
        });
    });

    // Full complexity (prerelease + build)
    group.bench_function("full", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("1.0.0-beta.1+build.456"));
            black_box(v)
        });
    });

    // Invalid version
    group.bench_function("invalid", |b| {
        b.iter(|| {
            let v = black_box(Version::parse("invalid"));
            black_box(v)
        });
    });

    group.finish();
}

/// Benchmark parsing a batch of version strings.
fn bench_version_parsing_batch(c: &mut Criterion) {
    c.bench_function("parse_batch_16", |b| {
        b.iter(|| {
            for version_str in VERSION_SAMPLES {
                let v = black_box(Version::parse(version_str));
                black_box(v);
            }
        });
    });

    let large_batch: Vec<String> = (0..100)
        .map(|i| format!("{}.{}.{}", i / 100, (i / 10) % 10, i % 10))
        .collect();

    c.bench_function("parse_batch_100", |b| {
        b.iter(|| {
            for version_str in &large_batch {
                let v = black_box(Version::parse(version_str));
                black_box(v);
            }
        });
    });
}

/// Benchmark version comparisons with different version differences.
fn bench_version_comparison_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_compare_diff");

    // Same version
    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    group.bench_function("equal", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    // Patch difference
    let v1 = Version::parse("1.0.1").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    group.bench_function("patch_diff", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    // Minor difference
    let v1 = Version::parse("1.1.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    group.bench_function("minor_diff", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    // Major difference
    let v1 = Version::parse("2.0.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    group.bench_function("major_diff", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    // Prerelease comparison
    let v1 = Version::parse("1.0.0-beta.2").unwrap();
    let v2 = Version::parse("1.0.0-beta.1").unwrap();
    group.bench_function("prerelease_diff", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    // Stable vs prerelease
    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("1.0.0-beta.1").unwrap();
    group.bench_function("stable_vs_prerelease", |b| {
        b.iter(|| {
            let result = black_box(v1.cmp(&v2));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark string-based version comparison.
fn bench_version_string_comparison(c: &mut Criterion) {
    c.bench_function("compare_versions_simple", |b| {
        b.iter(|| {
            let result = black_box(compare_versions("1.0.41", "1.0.40"));
            black_box(result)
        });
    });

    c.bench_function("compare_versions_prerelease", |b| {
        b.iter(|| {
            let result = black_box(compare_versions("1.0.0-beta.2", "1.0.0-beta.1"));
            black_box(result)
        });
    });

    c.bench_function("compare_versions_invalid", |b| {
        b.iter(|| {
            let result = black_box(compare_versions("invalid1", "invalid2"));
            black_box(result)
        });
    });

    c.bench_function("compare_versions_one_invalid", |b| {
        b.iter(|| {
            let result = black_box(compare_versions("1.0.0", "invalid"));
            black_box(result)
        });
    });
}

/// Benchmark version sorting operations.
fn bench_version_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_sort");

    for size in [10, 50, 100].iter() {
        let versions: Vec<Version> = (0..*size)
            .map(|i| {
                Version::parse(&format!("{}.{}.{}", i / 100, (i / 10) % 10, i % 10)).unwrap()
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut v = black_box(versions.clone());
                v.sort();
                black_box(v)
            });
        });
    }

    group.finish();
}

/// Benchmark version sorting with mixed formats.
fn bench_version_sorting_mixed(c: &mut Criterion) {
    let versions: Vec<Version> = VERSION_SAMPLES
        .iter()
        .filter_map(|s| Version::parse(s))
        .collect();

    c.bench_function("sort_mixed_versions", |b| {
        b.iter(|| {
            let mut v = black_box(versions.clone());
            v.sort();
            black_box(v)
        });
    });

    c.bench_function("sort_mixed_versions_reverse", |b| {
        b.iter(|| {
            let mut v = black_box(versions.clone());
            v.sort_by(|a, b| b.cmp(a));
            black_box(v)
        });
    });
}

/// Benchmark version struct operations.
fn bench_version_struct_operations(c: &mut Criterion) {
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

    c.bench_function("version_eq", |b| {
        let other = Version::parse("1.2.3-beta.1+build.456").unwrap();
        b.iter(|| {
            let result = black_box(version == other);
            black_box(result)
        });
    });
}

/// Benchmark finding the maximum version from a list.
fn bench_version_max_finding(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_find_max");

    for size in [10, 50, 100].iter() {
        let versions: Vec<Version> = (0..*size)
            .map(|i| {
                Version::parse(&format!("{}.{}.{}", i / 100, (i / 10) % 10, i % 10)).unwrap()
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let max = black_box(versions.iter().max());
                black_box(max)
            });
        });
    }

    group.finish();
}

/// Benchmark version filtering operations.
fn bench_version_filtering(c: &mut Criterion) {
    let versions: Vec<Version> = VERSION_SAMPLES
        .iter()
        .filter_map(|s| Version::parse(s))
        .collect();

    c.bench_function("filter_stable_only", |b| {
        b.iter(|| {
            let stable: Vec<_> = black_box(
                versions
                    .iter()
                    .filter(|v| !v.is_prerelease())
                    .collect::<Vec<_>>(),
            );
            black_box(stable)
        });
    });

    c.bench_function("filter_prerelease_only", |b| {
        b.iter(|| {
            let prerelease: Vec<_> = black_box(
                versions
                    .iter()
                    .filter(|v| v.is_prerelease())
                    .collect::<Vec<_>>(),
            );
            black_box(prerelease)
        });
    });

    c.bench_function("filter_major_version", |b| {
        b.iter(|| {
            let v1: Vec<_> = black_box(
                versions
                    .iter()
                    .filter(|v| v.major == 1)
                    .collect::<Vec<_>>(),
            );
            black_box(v1)
        });
    });
}

/// Benchmark version parsing error handling.
fn bench_version_error_handling(c: &mut Criterion) {
    let invalid_versions = vec![
        "invalid",
        "1",
        "1.2",
        "1.2.3.4",
        "a.b.c",
        "1.2.x",
        "",
        "v1.2.3",
    ];

    c.bench_function("parse_invalid_batch", |b| {
        b.iter(|| {
            for version_str in &invalid_versions {
                let v = black_box(Version::parse(version_str));
                black_box(v);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_version_parsing_by_complexity,
    bench_version_parsing_batch,
    bench_version_comparison_types,
    bench_version_string_comparison,
    bench_version_sorting,
    bench_version_sorting_mixed,
    bench_version_struct_operations,
    bench_version_max_finding,
    bench_version_filtering,
    bench_version_error_handling,
);

criterion_main!(benches);
