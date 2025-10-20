//! Project identity system
//!
//! Provides identity-based project IDs that are stable across path changes.
//! Uses content hashing to ensure the same project gets the same ID regardless of location.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Type of project based on its manifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// npm/pnpm/yarn package (package.json)
    Npm,
    /// Cargo crate (Cargo.toml)
    Cargo,
    /// Generic project (no standard manifest)
    Generic,
}

/// Project identity that is stable across path changes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIdentity {
    /// Package name/ID (e.g., "@omnitron-dev/titan" or "meridian-core")
    pub id: String,

    /// Semantic version
    pub version: String,

    /// Full ID with version (e.g., "@omnitron-dev/titan@1.0.0")
    pub full_id: String,

    /// SHA256 hash of manifest content for verification
    pub content_hash: String,

    /// Project type
    pub project_type: ProjectType,
}

impl ProjectIdentity {
    /// Create identity from npm package.json
    pub fn from_npm(path: &Path) -> Result<Self> {
        let package_json = path.join("package.json");
        if !package_json.exists() {
            anyhow::bail!("package.json not found at {:?}", package_json);
        }

        let content = std::fs::read_to_string(&package_json)
            .with_context(|| format!("Failed to read package.json at {:?}", package_json))?;

        let value: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| "Failed to parse package.json")?;

        let id = value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("package.json missing 'name' field"))?
            .to_string();

        let version = value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let full_id = format!("{}@{}", id, version);
        let content_hash = Self::compute_hash(&content);

        Ok(Self {
            id,
            version,
            full_id,
            content_hash,
            project_type: ProjectType::Npm,
        })
    }

    /// Create identity from Cargo.toml
    pub fn from_cargo(path: &Path) -> Result<Self> {
        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.exists() {
            anyhow::bail!("Cargo.toml not found at {:?}", cargo_toml);
        }

        let content = std::fs::read_to_string(&cargo_toml)
            .with_context(|| format!("Failed to read Cargo.toml at {:?}", cargo_toml))?;

        let value: toml::Value = toml::from_str(&content)
            .with_context(|| "Failed to parse Cargo.toml")?;

        let package = value
            .get("package")
            .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package] section"))?;

        let id = package
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing package.name"))?
            .to_string();

        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let full_id = format!("{}@{}", id, version);
        let content_hash = Self::compute_hash(&content);

        Ok(Self {
            id,
            version,
            full_id,
            content_hash,
            project_type: ProjectType::Cargo,
        })
    }

    /// Create identity for generic project (no standard manifest)
    pub fn from_generic(path: &Path) -> Result<Self> {
        // Create a stable hash from the project structure
        let path_str = path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
            .to_string();

        let hash = Self::compute_hash(&path_str);
        let short_hash = &hash[..12];

        let id = format!("generic-{}", short_hash);
        let version = "0.0.1".to_string();
        let full_id = format!("{}@{}", id, version);

        Ok(Self {
            id,
            version,
            full_id,
            content_hash: hash,
            project_type: ProjectType::Generic,
        })
    }

    /// Auto-detect project type and create identity
    pub fn from_path(path: &Path) -> Result<Self> {
        // Try npm first
        if path.join("package.json").exists() {
            return Self::from_npm(path);
        }

        // Try cargo
        if path.join("Cargo.toml").exists() {
            return Self::from_cargo(path);
        }

        // Fallback to generic
        Self::from_generic(path)
    }

    /// Compute SHA256 hash of content
    fn compute_hash(content: &str) -> String {
        let hash = blake3::hash(content.as_bytes());
        hash.to_hex().to_string()
    }

    /// Verify that content matches the stored hash
    pub fn verify_content(&self, content: &str) -> bool {
        Self::compute_hash(content) == self.content_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_npm_identity() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"{
                "name": "@test/package",
                "version": "1.2.3"
            }"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(temp_dir.path()).unwrap();

        assert_eq!(identity.id, "@test/package");
        assert_eq!(identity.version, "1.2.3");
        assert_eq!(identity.full_id, "@test/package@1.2.3");
        assert_eq!(identity.project_type, ProjectType::Npm);
        assert!(!identity.content_hash.is_empty());
    }

    #[test]
    fn test_cargo_identity() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-crate"
version = "2.0.0"
"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_cargo(temp_dir.path()).unwrap();

        assert_eq!(identity.id, "test-crate");
        assert_eq!(identity.version, "2.0.0");
        assert_eq!(identity.full_id, "test-crate@2.0.0");
        assert_eq!(identity.project_type, ProjectType::Cargo);
        assert!(!identity.content_hash.is_empty());
    }

    #[test]
    fn test_generic_identity() {
        let temp_dir = TempDir::new().unwrap();

        let identity = ProjectIdentity::from_generic(temp_dir.path()).unwrap();

        assert!(identity.id.starts_with("generic-"));
        assert_eq!(identity.version, "0.0.1");
        assert_eq!(identity.project_type, ProjectType::Generic);
    }

    #[test]
    fn test_auto_detect_npm() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_path(temp_dir.path()).unwrap();
        assert_eq!(identity.project_type, ProjectType::Npm);
    }

    #[test]
    fn test_auto_detect_cargo() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        let identity = ProjectIdentity::from_path(temp_dir.path()).unwrap();
        assert_eq!(identity.project_type, ProjectType::Cargo);
    }

    #[test]
    fn test_content_hash_stability() {
        let content = r#"{"name": "test", "version": "1.0.0"}"#;
        let hash1 = ProjectIdentity::compute_hash(content);
        let hash2 = ProjectIdentity::compute_hash(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_content() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"{"name": "test", "version": "1.0.0"}"#;
        fs::write(temp_dir.path().join("package.json"), content).unwrap();

        let identity = ProjectIdentity::from_npm(temp_dir.path()).unwrap();
        assert!(identity.verify_content(content));
        assert!(!identity.verify_content("different content"));
    }

    // Edge case tests for npm package.json parsing
    #[test]
    fn test_npm_missing_version() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test-no-version"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(temp_dir.path()).unwrap();
        assert_eq!(identity.version, "0.0.0"); // Default version
    }

    #[test]
    fn test_npm_missing_name() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"version": "1.0.0"}"#,
        )
        .unwrap();

        let result = ProjectIdentity::from_npm(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing 'name'"));
    }

    #[test]
    fn test_npm_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "broken", invalid json}"#,
        )
        .unwrap();

        let result = ProjectIdentity::from_npm(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_npm_scoped_package() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "@scope/package", "version": "1.2.3"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(temp_dir.path()).unwrap();
        assert_eq!(identity.id, "@scope/package");
        assert_eq!(identity.full_id, "@scope/package@1.2.3");
    }

    #[test]
    fn test_npm_no_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let result = ProjectIdentity::from_npm(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // Edge case tests for Cargo.toml parsing
    #[test]
    fn test_cargo_missing_version() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-no-version"
"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_cargo(temp_dir.path()).unwrap();
        assert_eq!(identity.version, "0.0.0"); // Default version
    }

    #[test]
    fn test_cargo_missing_package_section() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let result = ProjectIdentity::from_cargo(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing [package]"));
    }

    #[test]
    fn test_cargo_missing_name() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
version = "1.0.0"
"#,
        )
        .unwrap();

        let result = ProjectIdentity::from_cargo(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing package.name"));
    }

    #[test]
    fn test_cargo_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package
name = "broken"
"#,
        )
        .unwrap();

        let result = ProjectIdentity::from_cargo(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_cargo_no_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let result = ProjectIdentity::from_cargo(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // Content hash stability tests
    #[test]
    fn test_content_hash_stable_across_file_moves() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let content = r#"{"name": "stable-hash-test", "version": "1.0.0"}"#;

        fs::write(temp_dir1.path().join("package.json"), content).unwrap();
        fs::write(temp_dir2.path().join("package.json"), content).unwrap();

        let identity1 = ProjectIdentity::from_npm(temp_dir1.path()).unwrap();
        let identity2 = ProjectIdentity::from_npm(temp_dir2.path()).unwrap();

        // Content hash should be identical even in different directories
        assert_eq!(identity1.content_hash, identity2.content_hash);
        assert_eq!(identity1.id, identity2.id);
        assert_eq!(identity1.version, identity2.version);
    }

    #[test]
    fn test_content_hash_changes_with_content() {
        let temp_dir = TempDir::new().unwrap();

        let content1 = r#"{"name": "test", "version": "1.0.0"}"#;
        let content2 = r#"{"name": "test", "version": "2.0.0"}"#;

        fs::write(temp_dir.path().join("package.json"), content1).unwrap();
        let identity1 = ProjectIdentity::from_npm(temp_dir.path()).unwrap();

        fs::write(temp_dir.path().join("package.json"), content2).unwrap();
        let identity2 = ProjectIdentity::from_npm(temp_dir.path()).unwrap();

        // Content hash should differ when content changes
        assert_ne!(identity1.content_hash, identity2.content_hash);
    }

    #[test]
    fn test_content_hash_whitespace_sensitive() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let content1 = r#"{"name":"test","version":"1.0.0"}"#;
        let content2 = r#"{"name": "test", "version": "1.0.0"}"#;

        fs::write(temp_dir1.path().join("package.json"), content1).unwrap();
        fs::write(temp_dir2.path().join("package.json"), content2).unwrap();

        let identity1 = ProjectIdentity::from_npm(temp_dir1.path()).unwrap();
        let identity2 = ProjectIdentity::from_npm(temp_dir2.path()).unwrap();

        // Hash should differ due to whitespace differences
        assert_ne!(identity1.content_hash, identity2.content_hash);
    }

    // ID uniqueness and collision detection
    #[test]
    fn test_id_uniqueness_same_name_different_version() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        fs::write(
            temp_dir2.path().join("package.json"),
            r#"{"name": "test", "version": "2.0.0"}"#,
        )
        .unwrap();

        let identity1 = ProjectIdentity::from_npm(temp_dir1.path()).unwrap();
        let identity2 = ProjectIdentity::from_npm(temp_dir2.path()).unwrap();

        // Same name but different versions = different full_id
        assert_eq!(identity1.id, identity2.id);
        assert_ne!(identity1.full_id, identity2.full_id);
        assert_eq!(identity1.full_id, "test@1.0.0");
        assert_eq!(identity2.full_id, "test@2.0.0");
    }

    #[test]
    fn test_path_changes_dont_affect_id() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let content = r#"{"name": "path-independent", "version": "1.0.0"}"#;

        fs::write(temp_dir1.path().join("package.json"), content).unwrap();
        fs::write(temp_dir2.path().join("package.json"), content).unwrap();

        let identity1 = ProjectIdentity::from_npm(temp_dir1.path()).unwrap();
        let identity2 = ProjectIdentity::from_npm(temp_dir2.path()).unwrap();

        // Moving files shouldn't change the identity
        assert_eq!(identity1.id, identity2.id);
        assert_eq!(identity1.full_id, identity2.full_id);
        assert_eq!(identity1.content_hash, identity2.content_hash);
    }

    #[test]
    fn test_generic_identity_hash_based() {
        let temp_dir = TempDir::new().unwrap();
        let identity = ProjectIdentity::from_generic(temp_dir.path()).unwrap();

        assert!(identity.id.starts_with("generic-"));
        assert_eq!(identity.version, "0.0.1");
        assert_eq!(identity.project_type, ProjectType::Generic);
        assert!(!identity.content_hash.is_empty());
    }

    #[test]
    fn test_auto_detect_priority() {
        let temp_dir = TempDir::new().unwrap();

        // Create both package.json and Cargo.toml
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "npm-project", "version": "1.0.0"}"#,
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "cargo-project"
version = "2.0.0"
"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_path(temp_dir.path()).unwrap();

        // npm should take priority over cargo
        assert_eq!(identity.project_type, ProjectType::Npm);
        assert_eq!(identity.id, "npm-project");
    }

    #[test]
    fn test_auto_detect_fallback_to_generic() {
        let temp_dir = TempDir::new().unwrap();
        // No package.json or Cargo.toml

        let identity = ProjectIdentity::from_path(temp_dir.path()).unwrap();
        assert_eq!(identity.project_type, ProjectType::Generic);
    }

    #[test]
    fn test_hash_computation_deterministic() {
        let content = "test content for hashing";
        let hash1 = ProjectIdentity::compute_hash(content);
        let hash2 = ProjectIdentity::compute_hash(content);
        let hash3 = ProjectIdentity::compute_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
        assert_eq!(hash1.len(), 64); // blake3 produces 256-bit (64 hex chars) hash
    }

    #[test]
    fn test_verify_content_with_modified_file() {
        let temp_dir = TempDir::new().unwrap();
        let original_content = r#"{"name": "test", "version": "1.0.0"}"#;
        let modified_content = r#"{"name": "test", "version": "2.0.0"}"#;

        fs::write(temp_dir.path().join("package.json"), original_content).unwrap();
        let identity = ProjectIdentity::from_npm(temp_dir.path()).unwrap();

        // Original content should verify
        assert!(identity.verify_content(original_content));

        // Modified content should not verify
        assert!(!identity.verify_content(modified_content));
    }
}
