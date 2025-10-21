//! Example demonstrating the binary discovery system.
//!
//! This example shows how to:
//! - Find the Claude binary
//! - Discover all installations
//! - Work with versions
//! - Use the discovery builder
//!
//! Run with: cargo run --example binary_discovery

use cc_sdk::binary::{
    compare_versions, create_command_with_env, discover_installations, find_claude_binary,
    get_claude_version, DiscoveryBuilder, Version,
};
use std::cmp::Ordering;

fn main() {
    // Enable logging to see discovery process
    tracing_subscriber::fmt::init();

    println!("=== Claude Binary Discovery Example ===\n");

    // Example 1: Find the best Claude binary (cached)
    println!("1. Finding Claude binary (best installation)...");
    match find_claude_binary() {
        Ok(path) => {
            println!("   ✓ Found Claude at: {}", path);

            // Get version info
            if let Ok(Some(version)) = get_claude_version(&path) {
                println!("   ✓ Version: {}", version);
            }
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
        }
    }

    println!();

    // Example 2: Discover all installations
    println!("2. Discovering all Claude installations...");
    let installations = discover_installations();

    if installations.is_empty() {
        println!("   No installations found.");
    } else {
        println!("   Found {} installation(s):", installations.len());
        for (i, install) in installations.iter().enumerate() {
            println!("   {}. {}", i + 1, install.path);
            println!("      Version: {:?}", install.version);
            println!("      Source: {}", install.source);
            println!("      Type: {:?}", install.installation_type);
        }
    }

    println!();

    // Example 3: Custom discovery with builder
    println!("3. Custom discovery (skipping NVM)...");
    let custom_installations = DiscoveryBuilder::new()
        .skip_nvm(true)
        .discover();

    println!("   Found {} installation(s) (without NVM)", custom_installations.len());

    println!();

    // Example 4: Version parsing and comparison
    println!("4. Working with versions...");

    let versions = vec!["1.0.41", "1.0.40", "2.0.0", "1.0.0-beta.1"];

    for v in &versions {
        if let Some(parsed) = Version::parse(v) {
            println!("   {} -> Major: {}, Minor: {}, Patch: {}",
                v, parsed.major, parsed.minor, parsed.patch);
            if parsed.is_prerelease() {
                println!("      (prerelease: {:?})", parsed.prerelease);
            }
        }
    }

    println!();

    // Example 5: Version comparison
    println!("5. Comparing versions...");
    let v1 = "1.0.41";
    let v2 = "1.0.40";
    let result = compare_versions(v1, v2);

    match result {
        Ordering::Greater => println!("   {} > {}", v1, v2),
        Ordering::Less => println!("   {} < {}", v1, v2),
        Ordering::Equal => println!("   {} == {}", v1, v2),
    }

    let v3 = "2.0.0";
    let v4 = "1.0.0-beta.1";
    match compare_versions(v3, v4) {
        Ordering::Greater => println!("   {} > {} (stable > prerelease)", v3, v4),
        _ => {}
    }

    println!();

    // Example 6: Creating a command with proper environment
    println!("6. Creating command with environment setup...");
    if let Ok(claude_path) = find_claude_binary() {
        let mut cmd = create_command_with_env(&claude_path);
        cmd.arg("--version");

        match cmd.output() {
            Ok(output) => {
                let version_output = String::from_utf8_lossy(&output.stdout);
                println!("   Command output: {}", version_output.trim());
            }
            Err(e) => {
                println!("   Error executing command: {}", e);
            }
        }
    }

    println!();

    // Example 7: Environment variable override
    println!("7. Using CLAUDE_BINARY_PATH environment variable...");
    if let Ok(custom_path) = std::env::var("CLAUDE_BINARY_PATH") {
        println!("   Custom path set: {}", custom_path);

        let installations = discover_installations();
        if let Some(custom) = installations.iter().find(|i| i.source == "custom") {
            println!("   ✓ Custom installation found: {}", custom.path);
        }
    } else {
        println!("   No custom path set. Try:");
        println!("   export CLAUDE_BINARY_PATH=/path/to/claude");
    }

    println!();

    // Example 8: Discovery builder with custom paths
    println!("8. Discovery with custom paths...");
    let custom_discovery = DiscoveryBuilder::new()
        .custom_path("/usr/local/bin/claude")
        .custom_path("/opt/homebrew/bin/claude")
        .skip_system(true) // Skip default system paths
        .discover();

    println!("   Found {} installation(s) from custom paths", custom_discovery.len());
    for install in custom_discovery {
        println!("   - {}", install.path);
    }

    println!("\n=== Example Complete ===");
}
