//! Phase 1 Modernization Demo
//!
//! This example demonstrates the new error hierarchy, type-state markers,
//! and strong typing introduced in Phase 1 of the cc-sdk modernization.
//!
//! Run with: cargo run --example phase1_demo

use cc_sdk::prelude::*;
use cc_sdk::core::models;
use std::path::PathBuf;

fn main() {
    println!("=== Phase 1 cc-sdk Modernization Demo ===\n");

    // Demonstrate newtypes
    demo_newtypes();
    println!();

    // Demonstrate version handling
    demo_version();
    println!();

    // Demonstrate error handling
    demo_errors();
    println!();

    println!("=== Demo Complete ===");
}

fn demo_newtypes() {
    println!("1. Newtypes for Type Safety");
    println!("----------------------------");

    // SessionId - prevents mixing with other string types
    let session_id = SessionId::generate();
    println!("Generated Session ID: {}", session_id);

    let custom_session = SessionId::new("my-session-123");
    println!("Custom Session ID: {}", custom_session);

    // BinaryPath - type-safe path handling
    let binary_path = BinaryPath::new("/usr/local/bin/claude");
    println!("Binary Path: {}", binary_path);
    println!("  Exists: {}", binary_path.exists());

    // ModelId - prevents mixing model names
    let model = models::sonnet_4_5();
    println!("Model: {}", model);
    println!("  Is Sonnet: {}", model.is_sonnet());
    println!("  Is Opus: {}", model.is_opus());

    let custom_model = ModelId::new("claude-3-opus-20240229");
    println!("Custom Model: {}", custom_model);
    println!("  Is Opus: {}", custom_model.is_opus());
}

fn demo_version() {
    println!("2. Version Handling");
    println!("-------------------");

    // Parse versions
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("v2.0.0-beta").unwrap();
    let v3 = Version::parse("1.2.4").unwrap();

    println!("v1: {}", v1);
    println!("v2: {}", v2);
    println!("v3: {}", v3);

    // Compare versions
    println!("\nComparisons:");
    println!("  v1 < v3: {}", v1 < v3);
    println!("  v1 == Version::new(1, 2, 3): {}", v1 == Version::new(1, 2, 3));

    // Check requirements
    println!("\nRequirement Checking:");
    println!("  v1.satisfies(\">=1.0.0\"): {}", v1.satisfies(">=1.0.0"));
    println!("  v1.satisfies(\">=2.0.0\"): {}", v1.satisfies(">=2.0.0"));
    println!("  v2.satisfies(\">1.5.0\"): {}", v2.satisfies(">1.5.0"));
}

fn demo_errors() {
    println!("3. Modern Error Hierarchy");
    println!("-------------------------");

    // Binary errors
    let binary_error = BinaryError::NotFound {
        searched_paths: vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ],
    };
    println!("Binary Error:\n  {}", binary_error);

    let version_error = BinaryError::IncompatibleVersion {
        found: "0.1.0".to_string(),
        required: ">=0.2.0".to_string(),
    };
    println!("\nVersion Error:\n  {}", version_error);

    // Transport errors
    let timeout_error = TransportError::Timeout {
        duration: std::time::Duration::from_secs(30),
    };
    println!("\nTimeout Error:\n  {}", timeout_error);

    let invalid_msg = TransportError::InvalidMessage {
        reason: "malformed JSON".to_string(),
        raw: "{invalid}".to_string(),
    };
    println!("\nInvalid Message Error:\n  {}", invalid_msg);

    // Session errors
    let session_error = SessionError::NotFound {
        session_id: cc_sdk::core::SessionId::new("abc123"),
    };
    println!("\nSession Error:\n  {}", session_error);

    // Client errors
    let perm_error = ClientError::PermissionDenied {
        tool_name: "Bash".to_string(),
        reason: "user rejected".to_string(),
    };
    println!("\nPermission Error:\n  {}", perm_error);

    // Top-level error with helpers
    let error = Error::Transport(timeout_error);
    println!("\nError Helpers:");
    println!("  is_recoverable: {}", error.is_recoverable());
    println!("  is_config_error: {}", error.is_config_error());
    println!("  is_connection_error: {}", error.is_connection_error());

    // Error conversion demonstration
    demo_error_conversion();
}

fn demo_error_conversion() {
    println!("\n4. Error Conversion Examples");
    println!("----------------------------");

    // Function that returns Result with automatic error conversion
    fn example_operation() -> Result<String> {
        // This would normally come from actual operations
        Err(BinaryError::NotFound {
            searched_paths: vec![PathBuf::from("/usr/bin")],
        }.into())
    }

    match example_operation() {
        Ok(result) => println!("Success: {}", result),
        Err(Error::Binary(BinaryError::NotFound { searched_paths })) => {
            println!("Binary not found in:");
            for path in searched_paths {
                println!("  - {}", path.display());
            }
        }
        Err(e) => println!("Other error: {}", e),
    }

    // Legacy compatibility
    #[allow(deprecated)]
    {
        println!("\nLegacy Compatibility:");
        let legacy_error = Error::timeout(30);
        println!("  Legacy timeout constructor: {}", legacy_error);

        let parse_err = Error::parse_error("bad JSON", "{invalid}");
        println!("  Legacy parse error constructor: {}", parse_err);
    }
}
