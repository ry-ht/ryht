//! Example demonstrating Phase 2 binary discovery enhancements.
//!
//! This example shows the new Phase 2 features:
//! - Discovery caching with configurable TTL
//! - Performance metrics
//! - Modern error types integration
//! - Environment setup validation
//!
//! Run with: cargo run --example binary_discovery_phase2

use cc_sdk::binary::{
    cache::{CacheConfig, DiscoveryCache},
    create_command_with_env, discover_installations, find_claude_binary,
    get_claude_version, DiscoveryBuilder, Version,
};
use std::time::{Duration, Instant};

fn main() {
    // Enable logging to see discovery process
    tracing_subscriber::fmt::init();

    println!("=== Phase 2: Binary Discovery Enhancements ===\n");

    // Example 1: Caching demonstration
    println!("1. Discovery with caching...");
    demo_caching();
    println!();

    // Example 2: Performance comparison
    println!("2. Performance comparison (cached vs uncached)...");
    demo_performance();
    println!();

    // Example 3: Custom cache configuration
    println!("3. Custom cache configuration...");
    demo_custom_cache();
    println!();

    // Example 4: Environment validation
    println!("4. Environment setup validation...");
    demo_environment_setup();
    println!();

    // Example 5: Version requirements
    println!("5. Version requirements checking...");
    demo_version_requirements();
    println!();

    println!("=== Example Complete ===");
}

fn demo_caching() {
    // First discovery - will hit the filesystem
    println!("   First discovery (uncached)...");
    let start = Instant::now();
    let installations = discover_installations();
    let uncached_time = start.elapsed();
    println!("   ✓ Found {} installations in {:?}", installations.len(), uncached_time);

    // Second discovery - should use cache
    println!("   Second discovery (cached)...");
    let start = Instant::now();
    let cached_installations = discover_installations();
    let cached_time = start.elapsed();
    println!("   ✓ Found {} installations in {:?}", cached_installations.len(), cached_time);

    println!("   ℹ Cache speedup: {:.2}x faster",
        uncached_time.as_secs_f64() / cached_time.as_secs_f64().max(0.001));
}

fn demo_performance() {
    // Clear cache for fair comparison
    cc_sdk::binary::cache::clear_cache();

    println!("   Running 5 uncached discoveries...");
    let mut uncached_times = Vec::new();
    for i in 0..5 {
        cc_sdk::binary::cache::clear_cache();
        let start = Instant::now();
        let _ = discover_installations();
        let elapsed = start.elapsed();
        uncached_times.push(elapsed);
        println!("      Run {}: {:?}", i + 1, elapsed);
    }

    println!("   Running 5 cached discoveries...");
    let mut cached_times = Vec::new();
    for i in 0..5 {
        let start = Instant::now();
        let _ = discover_installations();
        let elapsed = start.elapsed();
        cached_times.push(elapsed);
        println!("      Run {}: {:?}", i + 1, elapsed);
    }

    let avg_uncached: Duration = uncached_times.iter().sum::<Duration>() / uncached_times.len() as u32;
    let avg_cached: Duration = cached_times.iter().sum::<Duration>() / cached_times.len() as u32;

    println!("   Average uncached: {:?}", avg_uncached);
    println!("   Average cached: {:?}", avg_cached);
    println!("   ℹ Average speedup: {:.2}x",
        avg_uncached.as_secs_f64() / avg_cached.as_secs_f64().max(0.001));
}

fn demo_custom_cache() {
    // Create a custom cache with short TTL
    let config = CacheConfig {
        ttl: Duration::from_secs(2),
        enabled: true,
    };
    let mut cache = DiscoveryCache::new(config);

    println!("   Creating custom cache with 2s TTL...");

    // Cache some results
    let installations = discover_installations();
    cache.set_default(installations.clone());
    println!("   ✓ Cached {} installations", installations.len());

    // Retrieve from cache
    if let Some(cached) = cache.get_default() {
        println!("   ✓ Retrieved {} installations from cache", cached.len());
    }

    // Wait for expiration
    println!("   Waiting 3 seconds for cache to expire...");
    std::thread::sleep(Duration::from_secs(3));

    // Try to retrieve (should be None)
    if cache.get_default().is_none() {
        println!("   ✓ Cache expired as expected");
    }

    // Cleanup expired entries
    let removed = cache.cleanup();
    println!("   ✓ Cleaned up {} expired entries", removed);
}

fn demo_environment_setup() {
    if let Ok(claude_path) = find_claude_binary() {
        println!("   Using Claude at: {}", claude_path);

        // Create command with proper environment
        let mut cmd = create_command_with_env(&claude_path);
        cmd.arg("--version");

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                println!("   ✓ Command executed successfully");
                println!("   Output: {}", stdout.trim());

                if !stderr.is_empty() {
                    println!("   Stderr: {}", stderr.trim());
                }

                // Validate version
                if let Some(version) = cc_sdk::binary::extract_version_from_output(&output.stdout) {
                    println!("   ✓ Detected version: {}", version);
                }
            }
            Err(e) => {
                println!("   ✗ Command failed: {}", e);
            }
        }
    } else {
        println!("   ✗ Claude binary not found");
    }
}

fn demo_version_requirements() {
    let installations = discover_installations();

    println!("   Checking version requirements...");

    for install in installations.iter().take(3) {
        println!("   Installation: {}", install.path);
        println!("      Source: {}", install.source);

        if let Some(version_str) = &install.version {
            println!("      Version: {}", version_str);

            if let Some(version) = Version::parse(version_str) {
                // Check various requirements
                let requirements = vec![
                    ">=1.0.0",
                    ">=2.0.0",
                    ">1.0.0",
                    "=2.0.24",
                ];

                for req in requirements {
                    let satisfies = match req.strip_prefix(">=") {
                        Some(min) => {
                            Version::parse(min.trim())
                                .map(|min_ver| version >= min_ver)
                                .unwrap_or(false)
                        }
                        None => match req.strip_prefix('>') {
                            Some(min) => {
                                Version::parse(min.trim())
                                    .map(|min_ver| version > min_ver)
                                    .unwrap_or(false)
                            }
                            None => match req.strip_prefix('=') {
                                Some(exact) => {
                                    Version::parse(exact.trim())
                                        .map(|exact_ver| version == exact_ver)
                                        .unwrap_or(false)
                                }
                                None => {
                                    Version::parse(req)
                                        .map(|exact_ver| version == exact_ver)
                                        .unwrap_or(false)
                                }
                            }
                        }
                    };

                    let check = if satisfies { "✓" } else { "✗" };
                    println!("      {} Requirement '{}': {}", check, req, satisfies);
                }
            }
        } else {
            println!("      Version: unknown");
        }
        println!();
    }
}
