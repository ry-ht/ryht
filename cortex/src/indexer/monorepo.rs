use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Monorepo type detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonorepoType {
    /// pnpm workspaces (pnpm-workspace.yaml)
    PnpmWorkspace,
    /// npm/yarn workspaces (package.json with "workspaces")
    NpmWorkspace,
    /// Lerna monorepo (lerna.json)
    Lerna,
    /// Turborepo (turbo.json)
    Turbo,
    /// Cargo workspace (Cargo.toml with workspace)
    CargoWorkspace,
    /// Generic monorepo (has packages/ or apps/ directories)
    Generic,
    /// Not a monorepo
    None,
}

/// Monorepo configuration detected from the project
#[derive(Debug, Clone)]
pub struct MonorepoConfig {
    pub monorepo_type: MonorepoType,
    pub root: PathBuf,
    pub workspace_patterns: Vec<String>,
    pub workspace_dirs: Vec<PathBuf>,
}

impl MonorepoConfig {
    /// Detect monorepo configuration from a directory
    pub async fn detect(path: &Path) -> Result<Self> {
        let mut config = Self {
            monorepo_type: MonorepoType::None,
            root: path.to_path_buf(),
            workspace_patterns: Vec::new(),
            workspace_dirs: Vec::new(),
        };

        // Check for pnpm workspace
        let pnpm_workspace = path.join("pnpm-workspace.yaml");
        if pnpm_workspace.exists() {
            info!("Detected pnpm workspace at {:?}", path);
            config.monorepo_type = MonorepoType::PnpmWorkspace;
            config.workspace_patterns = Self::parse_pnpm_workspace(&pnpm_workspace).await?;
            config.workspace_dirs = Self::resolve_workspace_patterns(path, &config.workspace_patterns)?;
            return Ok(config);
        }

        // Check for npm/yarn workspace
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Some(patterns) = Self::parse_npm_workspace(&package_json).await? {
                info!("Detected npm/yarn workspace at {:?}", path);
                config.monorepo_type = MonorepoType::NpmWorkspace;
                config.workspace_patterns = patterns;
                config.workspace_dirs = Self::resolve_workspace_patterns(path, &config.workspace_patterns)?;
                return Ok(config);
            }
        }

        // Check for Lerna
        let lerna_json = path.join("lerna.json");
        if lerna_json.exists() {
            info!("Detected Lerna monorepo at {:?}", path);
            config.monorepo_type = MonorepoType::Lerna;
            config.workspace_patterns = Self::parse_lerna_config(&lerna_json).await?;
            config.workspace_dirs = Self::resolve_workspace_patterns(path, &config.workspace_patterns)?;
            return Ok(config);
        }

        // Check for Turborepo
        let turbo_json = path.join("turbo.json");
        if turbo_json.exists() && package_json.exists() {
            info!("Detected Turborepo monorepo at {:?}", path);
            config.monorepo_type = MonorepoType::Turbo;
            // Turborepo uses package.json workspaces
            if let Some(patterns) = Self::parse_npm_workspace(&package_json).await? {
                config.workspace_patterns = patterns;
                config.workspace_dirs = Self::resolve_workspace_patterns(path, &config.workspace_patterns)?;
            }
            return Ok(config);
        }

        // Check for Cargo workspace
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Some(patterns) = Self::parse_cargo_workspace(&cargo_toml).await? {
                info!("Detected Cargo workspace at {:?}", path);
                config.monorepo_type = MonorepoType::CargoWorkspace;
                config.workspace_patterns = patterns;
                config.workspace_dirs = Self::resolve_workspace_patterns(path, &config.workspace_patterns)?;
                return Ok(config);
            }
        }

        // Check for generic monorepo patterns (packages/, apps/)
        let packages_dir = path.join("packages");
        let apps_dir = path.join("apps");
        if packages_dir.exists() || apps_dir.exists() {
            info!("Detected generic monorepo at {:?}", path);
            config.monorepo_type = MonorepoType::Generic;

            if packages_dir.exists() {
                config.workspace_patterns.push("packages/*".to_string());
                config.workspace_dirs.extend(Self::scan_directory(&packages_dir)?);
            }
            if apps_dir.exists() {
                config.workspace_patterns.push("apps/*".to_string());
                config.workspace_dirs.extend(Self::scan_directory(&apps_dir)?);
            }

            return Ok(config);
        }

        debug!("No monorepo pattern detected at {:?}", path);
        Ok(config)
    }

    /// Parse pnpm-workspace.yaml
    async fn parse_pnpm_workspace(path: &Path) -> Result<Vec<String>> {
        let content = tokio::fs::read_to_string(path).await?;

        // Simple YAML parsing for workspace patterns
        let mut patterns = Vec::new();
        let mut in_packages = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("packages:") {
                in_packages = true;
                continue;
            }
            if in_packages {
                if trimmed.is_empty() || (!trimmed.starts_with('-') && !trimmed.starts_with(' ')) {
                    break;
                }
                if let Some(pattern) = trimmed.strip_prefix('-').map(|s| s.trim()) {
                    let pattern = pattern.trim_matches(|c| c == '\'' || c == '"');
                    patterns.push(pattern.to_string());
                }
            }
        }

        Ok(patterns)
    }

    /// Parse package.json workspaces
    async fn parse_npm_workspace(path: &Path) -> Result<Option<Vec<String>>> {
        let content = tokio::fs::read_to_string(path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(workspaces) = json.get("workspaces") {
            if let Some(arr) = workspaces.as_array() {
                let patterns = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                return Ok(Some(patterns));
            } else if let Some(packages) = workspaces.get("packages").and_then(|p| p.as_array()) {
                let patterns = packages
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                return Ok(Some(patterns));
            }
        }

        Ok(None)
    }

    /// Parse lerna.json
    async fn parse_lerna_config(path: &Path) -> Result<Vec<String>> {
        let content = tokio::fs::read_to_string(path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let mut patterns = Vec::new();
        if let Some(packages) = json.get("packages").and_then(|p| p.as_array()) {
            patterns = packages
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else {
            // Default Lerna pattern
            patterns.push("packages/*".to_string());
        }

        Ok(patterns)
    }

    /// Parse Cargo.toml workspace
    async fn parse_cargo_workspace(path: &Path) -> Result<Option<Vec<String>>> {
        let content = tokio::fs::read_to_string(path).await?;

        // Simple TOML parsing for [workspace] section
        let mut in_workspace = false;
        let mut patterns = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "[workspace]" {
                in_workspace = true;
                continue;
            }

            if in_workspace {
                if trimmed.starts_with('[') {
                    break; // End of workspace section
                }

                if let Some(members_str) = trimmed.strip_prefix("members") {
                    let members_str = members_str.trim_start_matches('=').trim();
                    if members_str.starts_with('[') {
                        // Parse array
                        let members_str = members_str.trim_matches(|c| c == '[' || c == ']');
                        for member in members_str.split(',') {
                            let member = member.trim().trim_matches(|c| c == '"' || c == '\'');
                            if !member.is_empty() {
                                patterns.push(member.to_string());
                            }
                        }
                    }
                }
            }
        }

        if patterns.is_empty() {
            Ok(None)
        } else {
            Ok(Some(patterns))
        }
    }

    /// Resolve workspace patterns to actual directories
    fn resolve_workspace_patterns(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut dirs = Vec::new();

        for pattern in patterns {
            // Handle glob patterns like "packages/*" or "apps/*"
            if pattern.ends_with("/*") {
                let base = pattern.trim_end_matches("/*");
                let base_path = root.join(base);

                if base_path.exists() && base_path.is_dir() {
                    dirs.extend(Self::scan_directory(&base_path)?);
                }
            } else {
                // Direct path
                let path = root.join(pattern);
                if path.exists() && path.is_dir() {
                    dirs.push(path);
                }
            }
        }

        Ok(dirs)
    }

    /// Scan a directory for immediate subdirectories
    fn scan_directory(path: &Path) -> Result<Vec<PathBuf>> {
        let mut dirs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden directories and common ignore patterns
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !name.starts_with('.') && name != "node_modules" {
                            dirs.push(path);
                        }
                    }
                }
            }
        }

        Ok(dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_detect_pnpm_workspace() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create pnpm-workspace.yaml
        let workspace_file = root.join("pnpm-workspace.yaml");
        let mut file = std::fs::File::create(&workspace_file).unwrap();
        writeln!(file, "packages:").unwrap();
        writeln!(file, "  - 'apps/*'").unwrap();
        writeln!(file, "  - 'packages/*'").unwrap();

        // Create workspace directories
        std::fs::create_dir_all(root.join("apps/app1")).unwrap();
        std::fs::create_dir_all(root.join("packages/pkg1")).unwrap();

        let config = MonorepoConfig::detect(root).await.unwrap();

        assert_eq!(config.monorepo_type, MonorepoType::PnpmWorkspace);
        assert_eq!(config.workspace_patterns, vec!["apps/*", "packages/*"]);
        assert_eq!(config.workspace_dirs.len(), 2);
    }

    #[tokio::test]
    async fn test_detect_npm_workspace() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create package.json with workspaces
        let package_json = root.join("package.json");
        let mut file = std::fs::File::create(&package_json).unwrap();
        writeln!(file, r#"{{"workspaces": ["packages/*"]}}"#).unwrap();

        // Create workspace directory
        std::fs::create_dir_all(root.join("packages/pkg1")).unwrap();

        let config = MonorepoConfig::detect(root).await.unwrap();

        assert_eq!(config.monorepo_type, MonorepoType::NpmWorkspace);
        assert_eq!(config.workspace_patterns, vec!["packages/*"]);
    }

    #[tokio::test]
    async fn test_detect_generic_monorepo() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create packages and apps directories
        std::fs::create_dir_all(root.join("packages/pkg1")).unwrap();
        std::fs::create_dir_all(root.join("apps/app1")).unwrap();

        let config = MonorepoConfig::detect(root).await.unwrap();

        assert_eq!(config.monorepo_type, MonorepoType::Generic);
        assert!(config.workspace_patterns.contains(&"packages/*".to_string()));
        assert!(config.workspace_patterns.contains(&"apps/*".to_string()));
    }

    #[tokio::test]
    async fn test_detect_no_monorepo() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create a regular src directory
        std::fs::create_dir_all(root.join("src")).unwrap();

        let config = MonorepoConfig::detect(root).await.unwrap();

        assert_eq!(config.monorepo_type, MonorepoType::None);
        assert!(config.workspace_dirs.is_empty());
    }
}
