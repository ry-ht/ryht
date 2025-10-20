use crate::config::Config;
use crate::context::ContextManager;
use crate::indexer::CodeIndexer;
use crate::links::{LinksStorage, RocksDBLinksStorage};
use crate::memory::MemorySystem;
use crate::tasks::{TaskManager, TaskStorage};
use crate::session::SessionManager;
use crate::storage::{Storage, create_default_storage};
use crate::types::LLMAdapter;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::info;

/// Components for a single project context
pub struct ProjectContext {
    pub project_path: PathBuf,
    pub storage: Arc<dyn Storage>,
    pub memory_system: Arc<tokio::sync::RwLock<MemorySystem>>,
    pub context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
    pub indexer: Arc<tokio::sync::RwLock<CodeIndexer>>,
    pub session_manager: Arc<SessionManager>,
    pub doc_indexer: Arc<crate::docs::DocIndexer>,
    pub spec_manager: Arc<tokio::sync::RwLock<crate::specs::SpecificationManager>>,
    pub progress_manager: Arc<tokio::sync::RwLock<TaskManager>>,
    pub links_storage: Arc<tokio::sync::RwLock<dyn LinksStorage>>,
    pub last_access: SystemTime,
}

impl ProjectContext {
    /// Create a new project context
    pub async fn new(project_path: PathBuf, config: Config) -> Result<Self> {
        info!("Initializing project context for {:?}", project_path);

        // Create database path based on project path hash
        let db_path = Self::get_db_path(&project_path)?;

        // Initialize storage with project-specific path (uses SurrealDB by default)
        let storage = create_default_storage(&db_path).await?;

        // Initialize memory system
        let mut memory_system = MemorySystem::new(storage.clone(), config.memory.clone())?;
        memory_system.init().await?;

        // Initialize context manager
        let context_manager = ContextManager::new(LLMAdapter::claude3());

        // Initialize indexer
        let mut indexer = CodeIndexer::new(storage.clone(), config.index.clone())?;
        indexer.load().await?;

        // Initialize session manager
        let session_config = crate::session::SessionConfig {
            max_sessions: config.session.max_sessions,
            timeout: chrono::Duration::hours(1),
            auto_cleanup: true,
        };
        let session_manager = SessionManager::new(storage.clone(), session_config)?;

        // Initialize documentation indexer
        let doc_indexer = Arc::new(crate::docs::DocIndexer::new());

        // Initialize specification manager
        // Try multiple locations to find specs directory:
        // 1. Environment variable MERIDIAN_SPECS_PATH
        // 2. Project path/specs
        // 3. Current working directory/specs (fallback for stdio mode)
        let specs_path = std::env::var("MERIDIAN_SPECS_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                // Try project path first
                let project_specs = project_path.join("specs");
                if project_specs.exists() && project_specs.is_dir() {
                    info!("Using specs directory from project path: {:?}", project_specs);
                    Some(project_specs)
                } else {
                    None
                }
            })
            .or_else(|| {
                // Try current working directory as fallback
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| {
                        let cwd_specs = cwd.join("specs");
                        if cwd_specs.exists() && cwd_specs.is_dir() {
                            info!("Using specs directory from current working directory: {:?}", cwd_specs);
                            Some(cwd_specs)
                        } else {
                            None
                        }
                    })
            })
            .unwrap_or_else(|| {
                // Fallback to project path (will create if needed)
                let fallback = project_path.join("specs");
                info!("Using specs directory fallback: {:?}", fallback);
                fallback
            });

        info!("Initializing SpecificationManager with path: {:?}", specs_path);
        let spec_manager = crate::specs::SpecificationManager::new(specs_path);

        // Initialize progress manager
        let progress_storage = Arc::new(TaskStorage::new(storage.clone()));
        let progress_manager = TaskManager::new(progress_storage);

        // Initialize links storage
        let links_storage = RocksDBLinksStorage::new(storage.clone());

        Ok(Self {
            project_path,
            storage,
            memory_system: Arc::new(tokio::sync::RwLock::new(memory_system)),
            context_manager: Arc::new(tokio::sync::RwLock::new(context_manager)),
            indexer: Arc::new(tokio::sync::RwLock::new(indexer)),
            session_manager: Arc::new(session_manager),
            doc_indexer,
            spec_manager: Arc::new(tokio::sync::RwLock::new(spec_manager)),
            progress_manager: Arc::new(tokio::sync::RwLock::new(progress_manager)),
            links_storage: Arc::new(tokio::sync::RwLock::new(links_storage)),
            last_access: SystemTime::now(),
        })
    }

    /// Update last access time
    pub fn update_access(&mut self) {
        self.last_access = SystemTime::now();
    }

    /// Get database path for a project
    fn get_db_path(project_path: &Path) -> Result<PathBuf> {
        use crate::config::get_meridian_home;

        // Hash the project path to create a unique database directory
        let path_str = project_path.to_string_lossy();
        let hash = blake3::hash(path_str.as_bytes());
        let hash_str = hash.to_hex();

        // Create ~/.meridian/db/{hash}/index directory
        let db_path = get_meridian_home()
            .join("db")
            .join(&hash_str[..16]) // Use first 16 chars of hash
            .join("index");

        // Ensure the directory exists
        std::fs::create_dir_all(&db_path)?;

        Ok(db_path)
    }
}
