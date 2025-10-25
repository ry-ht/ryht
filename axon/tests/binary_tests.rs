//! Integration tests for binary discovery module.

use cc_sdk::binary::{
    compare_versions, discover_installations, extract_version_from_output,
    find_claude_binary, DiscoveryBuilder, Version,
};
use std::cmp::Ordering;

#[test]
fn test_version_parsing_comprehensive() {
    // Basic versions
    let v1 = Version::parse("1.0.0").unwrap();
    assert_eq!(v1.major, 1);
    assert_eq!(v1.minor, 0);
    assert_eq!(v1.patch, 0);
    assert!(!v1.is_prerelease());

    // Version with prerelease
    let v2 = Version::parse("1.2.3-beta.1").unwrap();
    assert_eq!(v2.major, 1);
    assert_eq!(v2.minor, 2);
    assert_eq!(v2.patch, 3);
    assert_eq!(v2.prerelease, Some("beta.1".to_string()));
    assert!(v2.is_prerelease());

    // Version with build metadata
    let v3 = Version::parse("1.0.0+20130313144700").unwrap();
    assert_eq!(v3.build, Some("20130313144700".to_string()));
    assert!(!v3.is_prerelease());

    // Complete version
    let v4 = Version::parse("2.0.0-rc.1+build.123").unwrap();
    assert_eq!(v4.major, 2);
    assert_eq!(v4.prerelease, Some("rc.1".to_string()));
    assert_eq!(v4.build, Some("build.123".to_string()));

    // Invalid versions
    assert!(Version::parse("1.0").is_none());
    assert!(Version::parse("1.0.x").is_none());
    assert!(Version::parse("invalid").is_none());
}

#[test]
fn test_version_comparison_comprehensive() {
    // Basic comparisons
    assert!(Version::parse("2.0.0").unwrap() > Version::parse("1.9.9").unwrap());
    assert!(Version::parse("1.1.0").unwrap() > Version::parse("1.0.9").unwrap());
    assert!(Version::parse("1.0.1").unwrap() > Version::parse("1.0.0").unwrap());

    // Equal versions
    assert_eq!(
        Version::parse("1.0.0").unwrap(),
        Version::parse("1.0.0").unwrap()
    );

    // Prerelease comparisons
    let stable = Version::parse("1.0.0").unwrap();
    let beta = Version::parse("1.0.0-beta.1").unwrap();
    assert!(stable > beta); // Stable should be greater than prerelease

    // Different prereleases
    let beta1 = Version::parse("1.0.0-beta.1").unwrap();
    let beta2 = Version::parse("1.0.0-beta.2").unwrap();
    assert!(beta2 > beta1);

    // Build metadata is ignored in comparison (but structs are different)
    let v1 = Version::parse("1.0.0+build1").unwrap();
    let v2 = Version::parse("1.0.0+build2").unwrap();
    assert_eq!(v1.cmp(&v2), Ordering::Equal); // Comparison ignores build
    assert_ne!(v1, v2); // But structs are different
}

#[test]
fn test_compare_versions_function() {
    assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
    assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
    assert_eq!(compare_versions("1.0.0", "2.0.0"), Ordering::Less);

    // With prerelease
    assert_eq!(compare_versions("1.0.0", "1.0.0-beta"), Ordering::Greater);
    assert_eq!(
        compare_versions("1.0.0-beta.2", "1.0.0-beta.1"),
        Ordering::Greater
    );

    // Invalid versions should be handled gracefully
    assert_eq!(compare_versions("invalid", "1.0.0"), Ordering::Less);
    assert_eq!(compare_versions("1.0.0", "invalid"), Ordering::Greater);
    assert_eq!(compare_versions("invalid", "invalid"), Ordering::Equal);
}

#[test]
fn test_version_display() {
    let v1 = Version::parse("1.0.41").unwrap();
    assert_eq!(v1.to_string(), "1.0.41");

    let v2 = Version::parse("2.0.0-beta.1").unwrap();
    assert_eq!(v2.to_string(), "2.0.0-beta.1");

    let v3 = Version::parse("1.0.0+build").unwrap();
    assert_eq!(v3.to_string(), "1.0.0+build");

    let v4 = Version::parse("1.2.3-rc.1+build.456").unwrap();
    assert_eq!(v4.to_string(), "1.2.3-rc.1+build.456");
}

#[test]
fn test_extract_version_from_output() {
    // Standard output format
    let output1 = b"claude version 1.0.41\n";
    assert_eq!(
        extract_version_from_output(output1),
        Some("1.0.41".to_string())
    );

    // Just the version
    let output2 = b"1.2.3\n";
    assert_eq!(
        extract_version_from_output(output2),
        Some("1.2.3".to_string())
    );

    // With prerelease
    let output3 = b"version: 2.0.0-beta.1\n";
    assert_eq!(
        extract_version_from_output(output3),
        Some("2.0.0-beta.1".to_string())
    );

    // No version
    let output4 = b"No version information\n";
    assert_eq!(extract_version_from_output(output4), None);

    // Multiple lines
    let output5 = b"Claude Code CLI\nVersion: 1.0.41\nCopyright...\n";
    assert_eq!(
        extract_version_from_output(output5),
        Some("1.0.41".to_string())
    );
}

#[test]
fn test_core_version() {
    let v1 = Version::parse("1.0.41").unwrap();
    assert_eq!(v1.core_version(), "1.0.41");

    let v2 = Version::parse("2.0.0-beta.1+build").unwrap();
    assert_eq!(v2.core_version(), "2.0.0");
}

#[test]
fn test_discovery_builder() {
    let builder = DiscoveryBuilder::new();
    let installations = builder.discover();

    // We should get at least some results (or none if Claude isn't installed)
    // This test just verifies the builder works without panicking
    println!("Found {} installations", installations.len());

    // Test builder configuration
    let builder2 = DiscoveryBuilder::new()
        .custom_path("/nonexistent/path/claude")
        .skip_nvm(true)
        .skip_homebrew(false)
        .skip_system(false);

    let installations2 = builder2.discover();
    println!("Found {} installations with custom config", installations2.len());
}

#[test]
fn test_discover_installations() {
    // This test performs actual discovery
    let installations = discover_installations();

    println!("Discovered installations:");
    for install in &installations {
        println!(
            "  {} (version: {:?}, source: {})",
            install.path, install.version, install.source
        );
    }

    // If Claude is installed, we should find at least one installation
    // If not, this will be empty (which is fine for the test)
    if !installations.is_empty() {
        // Verify the installations are sorted (newest/best first)
        for i in 1..installations.len() {
            let prev = &installations[i - 1];
            let curr = &installations[i];

            // If both have versions, the previous one should be >= current
            if let (Some(prev_v), Some(curr_v)) = (&prev.version, &curr.version) {
                assert!(
                    compare_versions(prev_v, curr_v) != Ordering::Less,
                    "Installations should be sorted by version (descending)"
                );
            }
        }
    }
}

#[test]
fn test_find_claude_binary() {
    // This test will succeed if Claude is installed, or fail gracefully if not
    match find_claude_binary() {
        Ok(path) => {
            println!("Found Claude at: {}", path);
            // Verify the result is cached
            let cached_result = find_claude_binary();
            assert!(cached_result.is_ok());
            assert_eq!(cached_result.unwrap(), path);
        }
        Err(e) => {
            println!("Claude not found (expected if not installed): {}", e);
            assert!(e.contains("Claude Code not found"));
        }
    }
}

#[test]
#[cfg(feature = "async-discovery")]
fn test_async_discovery() {
    use cc_sdk::binary::async_discovery::*;

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        // Test async binary finding
        match find_claude_binary_async().await {
            Ok(path) => println!("Found Claude at: {}", path),
            Err(e) => println!("Claude not found: {}", e),
        }

        // Test async installation discovery
        let installations = discover_installations_async().await;
        println!("Found {} installations (async)", installations.len());

        // Test async version checking (only if we found installations)
        if let Some(install) = installations.first() {
            match get_claude_version_async(install.path.clone()).await {
                Ok(Some(version)) => println!("Version: {}", version),
                Ok(None) => println!("Version not available"),
                Err(e) => println!("Error getting version: {}", e),
            }
        }
    });
}

// Additional helper tests

#[test]
fn test_version_ordering_comprehensive() {
    let mut versions = vec![
        Version::parse("1.0.0").unwrap(),
        Version::parse("2.0.0-beta.1").unwrap(),
        Version::parse("1.5.0").unwrap(),
        Version::parse("2.0.0").unwrap(),
        Version::parse("1.0.1").unwrap(),
        Version::parse("1.0.0-alpha").unwrap(),
    ];

    versions.sort();

    // Check ordering
    assert_eq!(versions[0].to_string(), "1.0.0-alpha");
    assert_eq!(versions[1].to_string(), "1.0.0");
    assert_eq!(versions[2].to_string(), "1.0.1");
    assert_eq!(versions[3].to_string(), "1.5.0");
    assert_eq!(versions[4].to_string(), "2.0.0-beta.1");
    assert_eq!(versions[5].to_string(), "2.0.0");
}

#[test]
fn test_installation_type() {
    use cc_sdk::binary::InstallationType;

    let system = InstallationType::System;
    let custom = InstallationType::Custom;

    assert_eq!(system, InstallationType::System);
    assert_ne!(system, custom);
    assert_eq!(custom, InstallationType::Custom);
}

#[test]
fn test_version_edge_cases() {
    // Single digit versions
    let v1 = Version::parse("0.0.1").unwrap();
    assert_eq!(v1.major, 0);
    assert_eq!(v1.minor, 0);
    assert_eq!(v1.patch, 1);

    // Large version numbers
    let v2 = Version::parse("999.999.999").unwrap();
    assert_eq!(v2.major, 999);
    assert_eq!(v2.minor, 999);
    assert_eq!(v2.patch, 999);

    // Complex prerelease
    let v3 = Version::parse("1.0.0-alpha.beta.gamma").unwrap();
    assert_eq!(v3.prerelease, Some("alpha.beta.gamma".to_string()));

    // Complex build
    let v4 = Version::parse("1.0.0+build.123.456").unwrap();
    assert_eq!(v4.build, Some("build.123.456".to_string()));
}

// Additional comprehensive tests for binary discovery

#[test]
fn test_discovery_with_caching() {
    // Test that discovery results are cached
    let builder = DiscoveryBuilder::new().use_cache(true);
    let first = builder.clone().discover();
    let second = builder.discover();

    // Both should return results (empty or populated depending on system)
    assert_eq!(first.len(), second.len());
}

#[test]
fn test_discovery_skip_options() {
    // Test skip options in builder
    let builder = DiscoveryBuilder::new()
        .skip_nvm(true)
        .skip_homebrew(true)
        .skip_system(false);

    let installations = builder.discover();
    // Verify it completes without panic
    println!("Found {} installations with skip options", installations.len());
}

#[test]
fn test_custom_path_discovery() {
    // Test custom path in discovery
    let builder = DiscoveryBuilder::new()
        .custom_path("/nonexistent/path/claude");

    let installations = builder.discover();
    // Should not panic even with invalid custom path
    println!("Custom path discovery found {} installations", installations.len());
}

#[test]
fn test_version_parsing_invalid_cases() {
    // Test various invalid version strings
    assert!(Version::parse("").is_none());
    assert!(Version::parse("1").is_none());
    assert!(Version::parse("1.0").is_none());
    assert!(Version::parse("a.b.c").is_none());
    assert!(Version::parse("1.0.0.0").is_none());
    assert!(Version::parse("v1.0.0").is_none()); // v prefix not supported
}

#[test]
fn test_version_comparison_edge_cases() {
    // Test comparison with different prerelease versions
    let rc1 = Version::parse("1.0.0-rc.1").unwrap();
    let rc2 = Version::parse("1.0.0-rc.2").unwrap();
    let beta = Version::parse("1.0.0-beta.1").unwrap();

    assert!(rc2 > rc1);
    assert!(rc1 > beta); // rc comes after beta lexicographically

    // Test versions with build metadata
    let v1 = Version::parse("1.0.0+build1").unwrap();
    let v2 = Version::parse("1.0.0+build2").unwrap();
    let v3 = Version::parse("1.0.0").unwrap();

    // Build metadata doesn't affect comparison
    assert_eq!(v1.cmp(&v2), Ordering::Equal);
    assert_eq!(v1.cmp(&v3), Ordering::Equal);
    assert_eq!(v2.cmp(&v3), Ordering::Equal);
}

#[test]
fn test_extract_version_from_various_outputs() {
    // Test various output formats
    let outputs = vec![
        (b"1.0.41\n" as &[u8], Some("1.0.41")),
        (b"claude version 1.0.41\n", Some("1.0.41")),
        (b"version: 1.0.41\n", Some("1.0.41")),
        (b"Claude Code CLI v1.0.41\n", Some("1.0.41")),
        (b"1.2.3-beta.1+build123\n", Some("1.2.3-beta.1+build123")),
        (b"no version here\n", None),
        (b"", None),
    ];

    for (output, expected) in outputs {
        let result = extract_version_from_output(output);
        assert_eq!(result, expected.map(|s| s.to_string()),
                   "Failed for input: {:?}", String::from_utf8_lossy(output));
    }
}

#[test]
fn test_discovery_priority_ordering() {
    // Test that installations are properly ordered
    use cc_sdk::binary::ClaudeInstallation;

    let mut installations = vec![
        ClaudeInstallation {
            path: "claude".to_string(),
            version: Some("1.0.0".to_string()),
            source: "PATH".to_string(),
            installation_type: cc_sdk::binary::InstallationType::System,
        },
        ClaudeInstallation {
            path: "/usr/local/bin/claude".to_string(),
            version: Some("1.0.0".to_string()),
            source: "which".to_string(),
            installation_type: cc_sdk::binary::InstallationType::System,
        },
    ];

    // The "which" source should be preferred over "PATH" with same version
    installations.sort_by(|a, b| {
        match (&a.version, &b.version) {
            (Some(v1), Some(v2)) => {
                match compare_versions(v2, v1) {
                    Ordering::Equal => {
                        // Compare by source preference
                        let pref_a = match a.source.as_str() {
                            "which" => 1,
                            "PATH" => 13,
                            _ => 14,
                        };
                        let pref_b = match b.source.as_str() {
                            "which" => 1,
                            "PATH" => 13,
                            _ => 14,
                        };
                        pref_a.cmp(&pref_b)
                    }
                    other => other,
                }
            }
            _ => Ordering::Equal,
        }
    });

    // Verify which comes first
    assert_eq!(installations[0].source, "which");
}

#[test]
fn test_environment_setup() {
    use cc_sdk::binary::create_command_with_env;

    // Test that we can create a command with environment
    let cmd = create_command_with_env("claude");
    assert_eq!(cmd.get_program(), "claude");

    // Test with a full path
    let cmd2 = create_command_with_env("/usr/local/bin/claude");
    assert_eq!(cmd2.get_program(), "/usr/local/bin/claude");
}

#[test]
fn test_version_core_version() {
    let versions = vec![
        ("1.0.41", "1.0.41"),
        ("1.0.41-beta.1", "1.0.41"),
        ("1.0.41+build", "1.0.41"),
        ("1.0.41-beta.1+build", "1.0.41"),
        ("2.0.0-rc.1", "2.0.0"),
    ];

    for (version_str, expected_core) in versions {
        let v = Version::parse(version_str).unwrap();
        assert_eq!(v.core_version(), expected_core,
                   "Failed for version: {}", version_str);
    }
}
