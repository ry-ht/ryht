//! Dependency parsing for package.json and Cargo.toml
//!
//! This module provides parsers for extracting dependency information
//! from various manifest files to build cross-monorepo dependency graphs.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Type of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    /// Runtime dependency
    Runtime,
    /// Development dependency
    Dev,
    /// Peer dependency
    Peer,
    /// Optional dependency
    Optional,
}

/// A parsed dependency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version specifier
    pub version: String,
    /// Type of dependency
    pub dep_type: DependencyType,
}

/// Result of parsing a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestDependencies {
    /// Project name
    pub project_name: String,
    /// Project version
    pub project_version: String,
    /// All dependencies
    pub dependencies: Vec<Dependency>,
}

/// Package.json structure
#[derive(Debug, Deserialize)]
struct PackageJson {
    name: String,
    version: String,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
    #[serde(default, rename = "optionalDependencies")]
    optional_dependencies: HashMap<String, String>,
}

/// Cargo.toml structure
#[derive(Debug, Deserialize)]
struct CargoToml {
    package: CargoPackage,
    #[serde(default)]
    dependencies: HashMap<String, CargoDepValue>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: HashMap<String, CargoDepValue>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CargoDepValue {
    Simple(String),
    Detailed(CargoDepDetailed),
}

#[derive(Debug, Deserialize)]
struct CargoDepDetailed {
    version: Option<String>,
    path: Option<String>,
    #[serde(default)]
    optional: bool,
}

/// Dependency parser for various manifest formats
pub struct DependencyParser;

impl DependencyParser {
    /// Parse package.json
    pub fn parse_package_json(path: &Path) -> Result<ManifestDependencies> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read package.json from {:?}", path))?;

        let pkg: PackageJson = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;

        let mut dependencies = Vec::new();

        // Runtime dependencies
        for (name, version) in pkg.dependencies {
            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Runtime,
            });
        }

        // Dev dependencies
        for (name, version) in pkg.dev_dependencies {
            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Dev,
            });
        }

        // Peer dependencies
        for (name, version) in pkg.peer_dependencies {
            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Peer,
            });
        }

        // Optional dependencies
        for (name, version) in pkg.optional_dependencies {
            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Optional,
            });
        }

        Ok(ManifestDependencies {
            project_name: pkg.name,
            project_version: pkg.version,
            dependencies,
        })
    }

    /// Parse Cargo.toml
    pub fn parse_cargo_toml(path: &Path) -> Result<ManifestDependencies> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Cargo.toml from {:?}", path))?;

        let cargo: CargoToml = toml::from_str(&content)
            .context("Failed to parse Cargo.toml")?;

        let mut dependencies = Vec::new();

        // Runtime dependencies
        for (name, value) in cargo.dependencies {
            let (version, dep_type) = match value {
                CargoDepValue::Simple(v) => (v, DependencyType::Runtime),
                CargoDepValue::Detailed(d) => {
                    let version = d.version.or(d.path).unwrap_or_else(|| "*".to_string());
                    let dep_type = if d.optional {
                        DependencyType::Optional
                    } else {
                        DependencyType::Runtime
                    };
                    (version, dep_type)
                }
            };

            dependencies.push(Dependency {
                name,
                version,
                dep_type,
            });
        }

        // Dev dependencies
        for (name, value) in cargo.dev_dependencies {
            let version = match value {
                CargoDepValue::Simple(v) => v,
                CargoDepValue::Detailed(d) => {
                    d.version.or(d.path).unwrap_or_else(|| "*".to_string())
                }
            };

            dependencies.push(Dependency {
                name,
                version,
                dep_type: DependencyType::Dev,
            });
        }

        Ok(ManifestDependencies {
            project_name: cargo.package.name,
            project_version: cargo.package.version,
            dependencies,
        })
    }

    /// Auto-detect and parse manifest file
    pub fn parse_manifest(project_path: &Path) -> Result<ManifestDependencies> {
        let package_json = project_path.join("package.json");
        let cargo_toml = project_path.join("Cargo.toml");

        if package_json.exists() {
            Self::parse_package_json(&package_json)
        } else if cargo_toml.exists() {
            Self::parse_cargo_toml(&cargo_toml)
        } else {
            anyhow::bail!("No supported manifest file found in {:?}", project_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        std::fs::write(
            &package_json,
            r#"{
                "name": "test-package",
                "version": "1.0.0",
                "dependencies": {
                    "react": "^18.0.0",
                    "lodash": "^4.17.21"
                },
                "devDependencies": {
                    "typescript": "^5.0.0"
                }
            }"#,
        )
        .unwrap();

        let result = DependencyParser::parse_package_json(&package_json).unwrap();

        assert_eq!(result.project_name, "test-package");
        assert_eq!(result.project_version, "1.0.0");
        assert_eq!(result.dependencies.len(), 3);

        let runtime_deps: Vec<_> = result
            .dependencies
            .iter()
            .filter(|d| d.dep_type == DependencyType::Runtime)
            .collect();
        assert_eq!(runtime_deps.len(), 2);

        let dev_deps: Vec<_> = result
            .dependencies
            .iter()
            .filter(|d| d.dep_type == DependencyType::Dev)
            .collect();
        assert_eq!(dev_deps.len(), 1);
    }

    #[test]
    fn test_parse_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
            "#,
        )
        .unwrap();

        let result = DependencyParser::parse_cargo_toml(&cargo_toml).unwrap();

        assert_eq!(result.project_name, "test-crate");
        assert_eq!(result.project_version, "0.1.0");
        assert!(result.dependencies.len() >= 2);

        let runtime_deps: Vec<_> = result
            .dependencies
            .iter()
            .filter(|d| d.dep_type == DependencyType::Runtime)
            .collect();
        assert!(runtime_deps.len() >= 2);
    }

    #[test]
    fn test_parse_manifest_auto_detect() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        std::fs::write(
            &package_json,
            r#"{"name": "auto-detect", "version": "1.0.0"}"#,
        )
        .unwrap();

        let result = DependencyParser::parse_manifest(temp_dir.path()).unwrap();
        assert_eq!(result.project_name, "auto-detect");
    }
}
