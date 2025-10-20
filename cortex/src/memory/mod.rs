pub mod cognitive_manager;
pub mod compression;
pub mod episode_recorder;
pub mod episodic;
pub mod episodic_surreal;
pub mod learning_extractor;
pub mod procedural;
pub mod retrieval;
pub mod semantic;
pub mod working;

use crate::config::MemoryConfig;
use crate::storage::Storage;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

pub use cognitive_manager::{CognitiveMemoryManager, CoreMemory, Memory, MemoryType};
pub use compression::{Checkpoint, CheckpointId, CompressionStats, MemoryCompressor, Summary};
pub use episode_recorder::{Action, ActionType, Episode, EpisodeHandle, EpisodeRecorder, Pattern};
pub use episodic::EpisodicMemory;
pub use episodic_surreal::{EpisodicMemorySurreal, EpisodeSearchResult, EpisodeStatistics};
pub use learning_extractor::{Learning, LearningCategory, LearningExtractor, Suggestion};
pub use procedural::ProceduralMemory;
pub use retrieval::{MemoryRetrieval, RetrievalStrategy, ScoredMemory};
pub use semantic::SemanticMemory;
pub use working::WorkingMemory;

/// Complete memory system
pub struct MemorySystem {
    pub episodic: EpisodicMemory,
    pub working: WorkingMemory,
    pub semantic: SemanticMemory,
    pub procedural: ProceduralMemory,
    storage: Arc<dyn Storage>,
}

impl MemorySystem {
    pub fn new(storage: Arc<dyn Storage>, config: MemoryConfig) -> Result<Self> {
        Self::with_index_path(storage, config, None)
    }

    pub fn with_index_path(
        storage: Arc<dyn Storage>,
        config: MemoryConfig,
        hnsw_index_path: Option<PathBuf>,
    ) -> Result<Self> {
        Ok(Self {
            episodic: EpisodicMemory::with_index_path(
                storage.clone(),
                config.episodic_retention_days,
                hnsw_index_path,
            )?,
            working: WorkingMemory::new(config.working_memory_size)?,
            semantic: SemanticMemory::new(storage.clone())?,
            procedural: ProceduralMemory::new(storage.clone())?,
            storage,
        })
    }

    /// Get the database instance (for advanced operations)
    pub fn get_db(&self) -> Result<Arc<Surreal<Db>>> {
        // Downcast storage to SurrealDBStorage and get db
        use crate::storage::SurrealDBStorage;
        use std::any::Any;

        // Try to downcast to SurrealDBStorage
        if let Some(surreal_storage) = (self.storage.as_ref() as &dyn Any).downcast_ref::<SurrealDBStorage>() {
            Ok(surreal_storage.db())
        } else {
            Err(anyhow::anyhow!("Storage is not SurrealDBStorage"))
        }
    }

    /// Initialize the memory system
    pub async fn init(&mut self) -> Result<()> {
        // Load existing data from storage
        self.episodic.load().await?;
        self.semantic.load().await?;
        self.procedural.load().await?;
        Ok(())
    }

    /// Periodic consolidation
    pub async fn consolidate(&mut self) -> Result<()> {
        self.episodic.consolidate().await?;
        self.semantic.consolidate().await?;
        Ok(())
    }

    /// Save HNSW index to disk for fast startup
    pub fn save_index(&self) -> Result<()> {
        self.episodic.save_index()
    }
}
