//! Integration tests for enhanced binary discovery features.
//!
//! These tests demonstrate the new capabilities added to match/exceed axon's
//! binary discovery functionality.

use cc_sdk::binary::*;

#[test]
fn test_comprehensive_discovery_workflow() {
    // 1. Discover all installations
    let installations = discover_installations();
    println!("Found {} installations", installations.len());

    for install in &installations {
        println!(
            "  - {} (version: {:?}, source: {})",
            install.path, install.version, install.source
        );
    }

    // 2. Score and rank installations
    if !installations.is_empty() {
        let ranked = scoring::rank_installations(&installations);
        println!("\nRanked by quality:");
        for (i, scored) in ranked.iter().enumerate() {
            println!(
                "  {}. {} (score: {}/100, breakdown: source={} version={} path={} type={} bonus={})",
                i + 1,
                scored.installation.path,
                scored.score,
                scored.score_breakdown.source_score,
                scored.score_breakdown.version_score,
                scored.score_breakdown.path_score,
                scored.score_breakdown.type_score,
                scored.score_breakdown.bonus_score,
            );
        }

        // 3. Get the best installation
        if let Some(best) = scoring::compare_installations(&installations) {
            println!("\nBest installation: {}", best.installation.path);
            println!("  Score: {}/100", best.score);
        }
    }
}

#[test]
fn test_validation_workflow() {
    let installations = discover_installations();

    if !installations.is_empty() {
        println!("\nValidating installations:");

        for install in &installations {
            match validation::verify_binary(&install.path) {
                Ok(health) => {
                    println!("\n  {}", install.path);
                    println!("    Exists: {}", health.exists);
                    println!("    Executable: {}", health.is_executable);
                    println!("    Version check passed: {}", health.version_check_passed);
                    println!("    Valid: {}", health.is_valid);

                    if !health.warnings.is_empty() {
                        println!("    Warnings:");
                        for warning in &health.warnings {
                            println!("      - {}", warning);
                        }
                    }

                    // Test version compatibility check
                    if health.meets_version_requirement("1.0.0") {
                        println!("    ✓ Meets minimum version requirement (1.0.0)");
                    }
                }
                Err(e) => {
                    println!("  {} - Error: {}", install.path, e);
                }
            }
        }
    }
}

#[test]
fn test_environment_setup() {
    // Test comprehensive environment setup
    let env = setup_environment();

    println!("\nEnvironment variables for Claude execution:");
    for (key, value) in env.iter() {
        if key.contains("PATH") || key.contains("NVM") || key.contains("PROXY") {
            println!("  {}={}", key, value);
        }
    }

    // Test PATH reconstruction
    let full_path = reconstruct_path();
    println!("\nReconstructed PATH:");
    #[cfg(unix)]
    for (i, p) in full_path.split(':').enumerate() {
        println!("  {}: {}", i + 1, p);
    }
    #[cfg(windows)]
    for (i, p) in full_path.split(';').enumerate() {
        println!("  {}: {}", i + 1, p);
    }
}

#[test]
fn test_preference_store() {
    use std::env;
    use preferences::*;

    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join(format!("test-prefs-integration-{}.json", std::process::id()));

    // Clean up any existing file
    let _ = std::fs::remove_file(&test_file);

    let store = FilePreferenceStore::new(test_file.clone());

    // Store preferences
    store.set_preferred_path("/usr/local/bin/claude").unwrap();
    store.set_installation_preference("homebrew").unwrap();

    // Retrieve preferences
    assert_eq!(
        store.get_preferred_path().unwrap(),
        Some("/usr/local/bin/claude".to_string())
    );
    assert_eq!(
        store.get_installation_preference().unwrap(),
        Some("homebrew".to_string())
    );

    println!("\nPreference storage test passed!");
    println!("  Preferred path: {}", store.get_preferred_path().unwrap().unwrap());
    println!("  Installation preference: {}", store.get_installation_preference().unwrap().unwrap());

    // Clean up
    store.clear().unwrap();
}

#[test]
fn test_discovery_builder() {
    // Test custom discovery configuration
    let installations = DiscoveryBuilder::new()
        .skip_nvm(false)
        .skip_homebrew(false)
        .skip_system(false)
        .use_cache(false)
        .discover();

    println!("\nDiscovery builder test:");
    println!("  Found {} installations", installations.len());
}

#[test]
fn test_version_parsing_and_comparison() {
    // Test comprehensive version handling
    let test_versions = vec![
        "1.0.0",
        "1.0.41",
        "2.0.0",
        "1.0.0-beta.1",
        "1.0.0+build123",
    ];

    println!("\nVersion parsing tests:");
    for ver_str in test_versions {
        if let Some(ver) = Version::parse(ver_str) {
            println!("  {} -> major={}, minor={}, patch={}, prerelease={:?}",
                ver_str, ver.major, ver.minor, ver.patch, ver.prerelease);
        }
    }

    // Test version comparison
    println!("\nVersion comparisons:");
    println!("  1.0.41 vs 1.0.40: {:?}", compare_versions("1.0.41", "1.0.40"));
    println!("  2.0.0 vs 1.9.9: {:?}", compare_versions("2.0.0", "1.9.9"));
    println!("  1.0.0 vs 1.0.0-beta: {:?}", compare_versions("1.0.0", "1.0.0-beta"));
}
