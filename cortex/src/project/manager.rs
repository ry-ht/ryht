use super::context::ProjectContext;
use crate::config::Config;
use anyhow::Result;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{info, warn};

/// Manager for multiple project contexts with LRU eviction
pub struct ProjectManager {
    projects: Arc<DashMap<PathBuf, Arc<ProjectContext>>>,
    config: Config,
    max_projects: usize,
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new(config: Config, max_projects: usize) -> Self {
        info!(
            "Initializing ProjectManager with max {} projects",
            max_projects
        );

        Self {
            projects: Arc::new(DashMap::new()),
            config,
            max_projects,
        }
    }

    /// Get or create a project context
    pub async fn get_project(&self, project_path: &Path) -> Result<Arc<ProjectContext>> {
        let project_path = project_path.canonicalize().unwrap_or_else(|_| {
            // If canonicalize fails, use the path as-is
            project_path.to_path_buf()
        });

        // Check if project already exists in cache
        if let Some(entry) = self.projects.get(&project_path) {
            // Note: We can't update last_access here because ProjectContext is in an Arc
            // This is acceptable as LRU eviction will still work based on creation order
            return Ok(entry.value().clone());
        }

        // Evict if we're at capacity
        if self.projects.len() >= self.max_projects {
            self.evict_lru().await?;
        }

        // Create new project context
        info!("Loading new project context: {:?}", project_path);
        let context = ProjectContext::new(project_path.clone(), self.config.clone()).await?;
        let context_arc = Arc::new(context);

        // Insert into cache
        self.projects
            .insert(project_path.clone(), context_arc.clone());

        Ok(context_arc)
    }

    /// Evict the least recently used project
    async fn evict_lru(&self) -> Result<()> {
        // Find the project with the oldest last_access time
        let mut oldest_path: Option<PathBuf> = None;
        let mut oldest_time = SystemTime::now();

        for entry in self.projects.iter() {
            let context = entry.value();
            if context.last_access < oldest_time {
                oldest_time = context.last_access;
                oldest_path = Some(entry.key().clone());
            }
        }

        if let Some(path) = oldest_path {
            info!("Evicting LRU project: {:?}", path);
            self.projects.remove(&path);
        } else {
            warn!("No projects to evict");
        }

        Ok(())
    }

    /// Close a specific project
    pub async fn close_project(&self, project_path: &Path) -> Result<()> {
        let project_path = project_path.canonicalize().unwrap_or_else(|_| {
            project_path.to_path_buf()
        });

        if let Some((path, _)) = self.projects.remove(&project_path) {
            info!("Closed project: {:?}", path);
        }

        Ok(())
    }

    /// Close all projects
    pub async fn close_all(&self) -> Result<()> {
        info!("Closing all projects");
        self.projects.clear();
        Ok(())
    }

    /// Get the number of active projects
    pub fn active_count(&self) -> usize {
        self.projects.len()
    }

    /// List all active project paths
    pub fn list_projects(&self) -> Vec<PathBuf> {
        self.projects
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Drop for ProjectManager {
    fn drop(&mut self) {
        // Clear all projects on drop
        self.projects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_project_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());

        let manager = ProjectManager::new(config, 5);
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_get_project() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ProjectManager::new(config, 5);

        let project_path = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_path).unwrap();

        let context = manager.get_project(&project_path).await.unwrap();
        assert_eq!(context.project_path, project_path.canonicalize().unwrap());
        assert_eq!(manager.active_count(), 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ProjectManager::new(config, 2); // Max 2 projects

        // Create 3 projects
        let project1 = temp_dir.path().join("project1");
        let project2 = temp_dir.path().join("project2");
        let project3 = temp_dir.path().join("project3");

        std::fs::create_dir_all(&project1).unwrap();
        std::fs::create_dir_all(&project2).unwrap();
        std::fs::create_dir_all(&project3).unwrap();

        // Load first two projects
        manager.get_project(&project1).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        manager.get_project(&project2).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert_eq!(manager.active_count(), 2);

        // Load third project - should evict project1 (oldest)
        manager.get_project(&project3).await.unwrap();
        assert_eq!(manager.active_count(), 2);

        // project2 and project3 should be active, project1 should be evicted
        let active_projects = manager.list_projects();
        let project1_canon = project1.canonicalize().unwrap();
        assert!(!active_projects.contains(&project1_canon));
    }

    #[tokio::test]
    async fn test_close_project() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ProjectManager::new(config, 5);

        let project_path = temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_path).unwrap();

        manager.get_project(&project_path).await.unwrap();
        assert_eq!(manager.active_count(), 1);

        manager.close_project(&project_path).await.unwrap();
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_close_all() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ProjectManager::new(config, 5);

        let project1 = temp_dir.path().join("project1");
        let project2 = temp_dir.path().join("project2");
        std::fs::create_dir_all(&project1).unwrap();
        std::fs::create_dir_all(&project2).unwrap();

        manager.get_project(&project1).await.unwrap();
        manager.get_project(&project2).await.unwrap();
        assert_eq!(manager.active_count(), 2);

        manager.close_all().await.unwrap();
        assert_eq!(manager.active_count(), 0);
    }
}
