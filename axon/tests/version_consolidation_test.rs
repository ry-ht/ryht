//! Test to verify Version consolidation works correctly
//!
//! This test verifies that core::Version properly re-exports binary::Version
//! and that the field name is `prerelease` instead of `pre`.

use cc_sdk::core::Version;
use cc_sdk::binary::Version as BinaryVersion;

#[test]
fn test_version_is_from_binary_module() {
    // Both should be the same type
    let v1: Version = Version::parse("1.0.0").unwrap();
    let v2: BinaryVersion = BinaryVersion::parse("1.0.0").unwrap();

    assert_eq!(v1, v2);
}

#[test]
fn test_version_uses_prerelease_field() {
    let v = Version::parse("1.0.0-beta").unwrap();

    // The field should be named `prerelease`, not `pre`
    assert_eq!(v.prerelease, Some("beta".to_string()));
    assert!(v.is_prerelease());
}

#[test]
fn test_version_has_build_metadata() {
    let v = Version::parse("1.0.0+build123").unwrap();

    // Should support build metadata
    assert_eq!(v.build, Some("build123".to_string()));
    assert!(!v.is_prerelease()); // build metadata doesn't make it a prerelease
}

#[test]
fn test_version_full_semver() {
    let v = Version::parse("1.2.3-beta.1+build.456").unwrap();

    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.prerelease, Some("beta.1".to_string()));
    assert_eq!(v.build, Some("build.456".to_string()));
}

#[test]
fn test_version_comparison() {
    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("1.0.1").unwrap();
    let v3 = Version::parse("2.0.0").unwrap();

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1 < v3);
}

#[test]
fn test_version_prerelease_comparison() {
    let stable = Version::parse("1.0.0").unwrap();
    let prerelease = Version::parse("1.0.0-beta").unwrap();

    // According to semver, prerelease versions have lower precedence
    assert!(prerelease < stable);
}

#[test]
fn test_version_display() {
    let v1 = Version::parse("1.2.3").unwrap();
    assert_eq!(v1.to_string(), "1.2.3");

    let v2 = Version::parse("1.0.0-beta").unwrap();
    assert_eq!(v2.to_string(), "1.0.0-beta");

    let v3 = Version::parse("1.0.0+build").unwrap();
    assert_eq!(v3.to_string(), "1.0.0+build");

    let v4 = Version::parse("1.0.0-beta+build").unwrap();
    assert_eq!(v4.to_string(), "1.0.0-beta+build");
}

#[test]
fn test_version_serialization() {
    let v = Version::parse("1.2.3-beta.1+build.456").unwrap();

    // Test serialization
    let json = serde_json::to_string(&v).unwrap();
    let deserialized: Version = serde_json::from_str(&json).unwrap();

    assert_eq!(v, deserialized);
}

#[test]
fn test_version_core_version() {
    let v = Version::parse("1.2.3-beta+build").unwrap();

    // Should have core_version method
    assert_eq!(v.core_version(), "1.2.3");
}
