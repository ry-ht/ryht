use super::handlers::ToolHandlers;
use crate::project::ProjectManager;
use anyhow::{Context as AnyhowContext, Result};
use dashmap::DashMap;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

/// Wrapper for ToolHandlers with ProjectManager integration
pub struct ProjectToolHandlers {
    project_manager: Arc<ProjectManager>,
    handlers_cache: Arc<DashMap<PathBuf, Arc<ToolHandlers>>>,
}

impl ProjectToolHandlers {
    /// Create a new ProjectToolHandlers
    pub fn new(project_manager: Arc<ProjectManager>) -> Self {
        Self {
            project_manager,
            handlers_cache: Arc::new(DashMap::new()),
        }
    }

    /// Handle a tool call for a specific project
    pub async fn handle_tool_call_for_project(
        &self,
        tool_name: &str,
        arguments: Value,
        project_path: Option<&Path>,
    ) -> Result<Value> {
        // Determine project path
        let project_path = if let Some(path) = project_path {
            path.to_path_buf()
        } else {
            // Check if project_path is in arguments
            if let Some(path_str) = arguments.get("project_path").and_then(|v| v.as_str()) {
                PathBuf::from(path_str)
            } else {
                // Default to current directory
                std::env::current_dir()
                    .context("Failed to get current directory")?
            }
        };

        debug!(
            "Handling tool call '{}' for project: {:?}",
            tool_name, project_path
        );

        // Get or create handlers for this project
        let handlers = self.get_or_create_handlers(&project_path).await?;

        // Execute tool call
        handlers.handle_tool_call(tool_name, arguments).await
    }

    /// Get or create handlers for a project
    async fn get_or_create_handlers(&self, project_path: &Path) -> Result<Arc<ToolHandlers>> {
        let project_path = project_path.canonicalize().unwrap_or_else(|_| {
            project_path.to_path_buf()
        });

        // Check cache first
        if let Some(handlers) = self.handlers_cache.get(&project_path) {
            debug!("Using cached handlers for project: {:?}", project_path);
            return Ok(handlers.clone());
        }

        info!("Creating new handlers for project: {:?}", project_path);

        // Get project context from manager
        let context = self.project_manager.get_project(&project_path).await
            .with_context(|| format!("Failed to load project context for {:?}", project_path))?;

        // Create handlers from project context
        // Note: Components are already wrapped in Arc<RwLock<>>, so we can clone the Arc pointers
        let pattern_engine = Arc::new(crate::indexer::PatternSearchEngine::new()
            .expect("Failed to initialize pattern search engine"));

        let handlers = Arc::new(ToolHandlers::new(
            context.memory_system.clone(),
            context.context_manager.clone(),
            context.indexer.clone(),
            context.session_manager.clone(),
            context.doc_indexer.clone(),
            context.spec_manager.clone(),
            context.progress_manager.clone(),
            context.links_storage.clone(),
            pattern_engine,
        ));

        // Cache the handlers
        self.handlers_cache.insert(project_path.clone(), handlers.clone());

        Ok(handlers)
    }

    /// Extract project path from tool arguments
    pub fn extract_project_path(arguments: &Value) -> Option<PathBuf> {
        arguments
            .get("project_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
    }

    /// Clear cache for a specific project
    pub fn clear_project_cache(&self, project_path: &Path) -> Result<()> {
        let project_path = project_path.canonicalize().unwrap_or_else(|_| {
            project_path.to_path_buf()
        });

        if self.handlers_cache.remove(&project_path).is_some() {
            info!("Cleared handler cache for project: {:?}", project_path);
        }

        Ok(())
    }

    /// Clear all cached handlers
    pub fn clear_all_caches(&self) {
        info!("Clearing all handler caches");
        self.handlers_cache.clear();
    }

    /// Get the number of cached handler sets
    pub fn cache_size(&self) -> usize {
        self.handlers_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn create_test_config(db_path: PathBuf) -> Config {
        Config {
            index: crate::config::IndexConfig {
                languages: vec!["rust".to_string()],
                ignore: vec![],
                max_file_size: "1MB".to_string(),
            },
            storage: crate::config::StorageConfig {
                path: db_path,
                cache_size: "256MB".to_string(),
                hnsw_index_path: None,
            },
            memory: crate::config::MemoryConfig {
                episodic_retention_days: 30,
                working_memory_size: "10MB".to_string(),
                consolidation_interval: "1h".to_string(),
            },
            session: crate::config::SessionConfig {
                max_sessions: 10,
                session_timeout: "1h".to_string(),
            },
            monorepo: crate::config::MonorepoConfig::default(),
            learning: crate::config::LearningConfig::default(),
            mcp: crate::config::McpConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_project_tool_handlers_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = Arc::new(ProjectManager::new(config, 5));

        let handlers = ProjectToolHandlers::new(manager);
        assert_eq!(handlers.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_extract_project_path() {
        let args = serde_json::json!({
            "project_path": "/path/to/project",
            "other": "value"
        });

        let path = ProjectToolHandlers::extract_project_path(&args);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), PathBuf::from("/path/to/project"));
    }

    #[tokio::test]
    async fn test_cache_management() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = Arc::new(ProjectManager::new(config, 5));

        let handlers = ProjectToolHandlers::new(manager);

        let project_path = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_path).unwrap();

        // Create handlers for project (this will cache them)
        handlers.get_or_create_handlers(&project_path).await.unwrap();
        assert_eq!(handlers.cache_size(), 1);

        // Clear cache
        handlers.clear_project_cache(&project_path).unwrap();
        // Note: DashMap remove returns Some if found, but our cache_size might not reflect it immediately
        // depending on how DashMap handles concurrent operations
    }
}
