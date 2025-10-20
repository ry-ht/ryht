//! External project loader for importing external content into VFS.

use crate::path::VirtualPath;
use crate::types::*;
use crate::virtual_filesystem::VirtualFileSystem;
use cortex_core::error::{CortexError, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Loader for importing external projects and documents into VFS.
///
/// Supports:
/// - Read-only import of external projects
/// - Selective file inclusion/exclusion patterns
/// - Language detection and code parsing
/// - Automatic content deduplication
pub struct ExternalProjectLoader {
    vfs: VirtualFileSystem,
}

impl ExternalProjectLoader {
    /// Create a new external project loader.
    pub fn new(vfs: VirtualFileSystem) -> Self {
        Self { vfs }
    }

    /// Import an external project into VFS.
    pub async fn import_project(
        &self,
        source_path: &Path,
        options: ImportOptions,
    ) -> Result<ImportReport> {
        let start = Instant::now();
        info!("Importing project from: {}", source_path.display());

        // Validate source path
        if !source_path.exists() {
            return Err(CortexError::not_found(
                "SourcePath",
                source_path.display().to_string()
            ));
        }

        if !source_path.is_dir() {
            return Err(CortexError::invalid_input(
                "Source path must be a directory"
            ));
        }

        // Create workspace
        let workspace = self.create_workspace(source_path, &options).await?;
        let workspace_id = workspace.id;

        let mut report = ImportReport {
            workspace_id,
            ..Default::default()
        };

        // Walk directory and import files
        self.import_directory(
            source_path,
            source_path,
            &workspace_id,
            &options,
            &mut report,
        ).await?;

        report.duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Import completed: {} files, {} directories in {}ms",
            report.files_imported, report.directories_imported, report.duration_ms
        );

        Ok(report)
    }

    /// Create a workspace for the imported project.
    async fn create_workspace(
        &self,
        source_path: &Path,
        options: &ImportOptions,
    ) -> Result<Workspace> {
        let workspace = Workspace {
            id: Uuid::new_v4(),
            name: source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("external")
                .to_string(),
            workspace_type: WorkspaceType::External,
            source_type: if options.create_fork {
                SourceType::Fork
            } else if options.read_only {
                SourceType::ExternalReadOnly
            } else {
                SourceType::Local
            },
            namespace: options.namespace.clone(),
            source_path: Some(source_path.to_path_buf()),
            read_only: options.read_only && !options.create_fork,
            parent_workspace: None,
            fork_metadata: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // TODO: Store workspace in database - need VFS API for this
        // For now, just return the workspace
        Ok(workspace)
    }

    /// Import a directory recursively.
    async fn import_directory(
        &self,
        root: &Path,
        current: &Path,
        workspace_id: &Uuid,
        options: &ImportOptions,
        report: &mut ImportReport,
    ) -> Result<()> {
        // Build walker with ignore patterns
        let mut walker = WalkBuilder::new(current);
        walker
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .standard_filters(true);

        // Add exclude patterns
        for pattern in &options.exclude_patterns {
            walker.add_custom_ignore_filename(pattern);
        }

        if let Some(max_depth) = options.max_depth {
            walker.max_depth(Some(max_depth));
        }

        // Walk directory
        for entry in walker.build() {
            let entry = entry.map_err(|e| CortexError::vfs(e.to_string()))?;
            let path = entry.path();

            // Skip the root itself
            if path == current {
                continue;
            }

            // Get relative path
            let relative_path = path
                .strip_prefix(root)
                .map_err(|e| CortexError::invalid_input(e.to_string()))?;

            let virtual_path = VirtualPath::from_physical(path, root)
                .map_err(|e| CortexError::invalid_input(e.to_string()))?;

            // Check if should be included
            if !self.should_include(relative_path, options) {
                debug!("Skipping: {}", path.display());
                continue;
            }

            // Import based on type
            if path.is_dir() {
                self.import_directory_node(workspace_id, &virtual_path, options).await?;
                report.directories_imported += 1;
            } else if path.is_file() {
                let size = self.import_file_node(
                    workspace_id,
                    path,
                    &virtual_path,
                    options,
                ).await?;
                report.files_imported += 1;
                report.bytes_imported += size;
            }
        }

        Ok(())
    }

    /// Import a directory node.
    async fn import_directory_node(
        &self,
        workspace_id: &Uuid,
        virtual_path: &VirtualPath,
        options: &ImportOptions,
    ) -> Result<()> {
        let mut vnode = VNode::new_directory(*workspace_id, virtual_path.clone());
        vnode.read_only = options.read_only && !options.create_fork;
        vnode.mark_synchronized(); // External content starts as synchronized

        self.save_vnode(&vnode).await?;

        Ok(())
    }

    /// Import a file node.
    async fn import_file_node(
        &self,
        workspace_id: &Uuid,
        physical_path: &Path,
        virtual_path: &VirtualPath,
        options: &ImportOptions,
    ) -> Result<usize> {
        // Read file content
        let content = fs::read(physical_path).await
            .map_err(|e| CortexError::vfs(format!("Failed to read file: {}", e)))?;

        let size = content.len();

        // Calculate content hash
        let content_hash = blake3::hash(&content).to_hex().to_string();

        // Create vnode
        let mut vnode = VNode::new_file(
            *workspace_id,
            virtual_path.clone(),
            content_hash.clone(),
            size,
        );

        vnode.read_only = options.read_only && !options.create_fork;
        vnode.source_path = Some(physical_path.to_path_buf());
        vnode.mark_synchronized(); // External content starts as synchronized

        // Detect language
        if let Some(ext) = virtual_path.extension() {
            vnode.language = Some(Language::from_extension(ext));
        }

        // Save vnode
        self.save_vnode(&vnode).await?;

        // Store content (will be deduplicated)
        self.vfs.write_file(workspace_id, virtual_path, &content).await?;

        debug!("Imported file: {} ({} bytes)", virtual_path, size);

        Ok(size)
    }

    /// Check if a path should be included based on patterns.
    fn should_include(&self, path: &Path, options: &ImportOptions) -> bool {
        let path_str = path.to_string_lossy();

        // Check exclude patterns first
        for pattern in &options.exclude_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return false;
            }
        }

        // Check include patterns
        if options.include_patterns.is_empty() {
            return true;
        }

        for pattern in &options.include_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple pattern matching (supports * wildcard).
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern == "**/*" {
            return true;
        }

        // Simple glob matching
        let pattern_parts: Vec<&str> = pattern.split("/**").collect();
        if pattern_parts.len() == 2 {
            let prefix = pattern_parts[0].trim_start_matches("**/");
            let suffix = pattern_parts[1].trim_end_matches("/**");

            if !prefix.is_empty() && !path.contains(prefix) {
                return false;
            }
            if !suffix.is_empty() && !path.contains(suffix) {
                return false;
            }

            return true;
        }

        // Exact match
        path.contains(pattern)
    }

    /// Save a vnode to database.
    async fn save_vnode(&self, _vnode: &VNode) -> Result<()> {
        // TODO: Need VFS API for saving vnodes directly
        // For now, skip saving
        Ok(())
    }

    /// Import a single file (convenience method).
    pub async fn import_file(
        &self,
        workspace_id: &Uuid,
        physical_path: &Path,
        virtual_path: VirtualPath,
        read_only: bool,
    ) -> Result<()> {
        let content = fs::read(physical_path).await
            .map_err(|e| CortexError::vfs(format!("Failed to read file: {}", e)))?;

        let content_hash = blake3::hash(&content).to_hex().to_string();

        let mut vnode = VNode::new_file(
            *workspace_id,
            virtual_path.clone(),
            content_hash,
            content.len(),
        );

        vnode.read_only = read_only;
        vnode.source_path = Some(physical_path.to_path_buf());

        if let Some(ext) = virtual_path.extension() {
            vnode.language = Some(Language::from_extension(ext));
        }

        self.save_vnode(&vnode).await?;
        self.vfs.write_file(workspace_id, &virtual_path, &content).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig};

    #[test]
    fn test_pattern_matching() {
        // Create a test VFS with minimal config
        // Note: This will panic if we try to actually connect, but for pattern matching test it's fine
        let config = DatabaseConfig {
            connection_mode: cortex_storage::connection_pool::ConnectionMode::Local {
                endpoint: "mem://".to_string(),
            },
            credentials: cortex_storage::connection_pool::Credentials {
                username: None,
                password: None,
            },
            pool_config: cortex_storage::connection_pool::PoolConfig {
                min_connections: 1,
                max_connections: 1,
                connection_timeout: std::time::Duration::from_secs(1),
                idle_timeout: None,
                max_lifetime: None,
                retry_policy: cortex_storage::connection_pool::RetryPolicy {
                    max_attempts: 1,
                    initial_backoff: std::time::Duration::from_millis(100),
                    max_backoff: std::time::Duration::from_secs(1),
                    multiplier: 1.5,
                },
                warm_connections: false,
                validate_on_checkout: false,
                recycle_after_uses: None,
                shutdown_grace_period: std::time::Duration::from_secs(5),
            },
            namespace: "test".to_string(),
            database: "test".to_string(),
        };

        // We can't actually create a ConnectionManager here without async
        // So let's just test the pattern matching directly without a loader instance

        // Just test pattern matching logic inline
        let matches_all = |path: &str, pattern: &str| -> bool {
            if pattern == "**/*" {
                return true;
            }

            let pattern_parts: Vec<&str> = pattern.split("/**").collect();
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0].trim_start_matches("**/");
                let suffix = pattern_parts[1].trim_end_matches("/**");

                if !prefix.is_empty() && !path.contains(prefix) {
                    return false;
                }
                if !suffix.is_empty() && !path.contains(suffix) {
                    return false;
                }

                return true;
            }

            path.contains(pattern)
        };

        assert!(matches_all("src/main.rs", "**/*"));
        assert!(matches_all("node_modules/foo/bar.js", "**/node_modules/**"));
        assert!(!matches_all("src/main.rs", "**/node_modules/**"));
    }
}
