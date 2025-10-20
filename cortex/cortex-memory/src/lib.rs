//! Cognitive memory systems for Cortex.
//!
//! This crate implements episodic, semantic, working, and procedural memory systems
//! based on cognitive architecture principles from the specification.
//!
//! # Architecture
//!
//! The memory system is organized into five tiers:
//!
//! 1. **Working Memory**: Fast, temporary storage with capacity limits (7±2 items)
//! 2. **Episodic Memory**: Development session episodes with full context
//! 3. **Semantic Memory**: Code structures, patterns, and relationships
//! 4. **Procedural Memory**: Learned procedures and workflows
//! 5. **Memory Consolidation**: Transfer and optimization between tiers
//!
//! # Usage
//!
//! ```rust,no_run
//! use cortex_memory::prelude::*;
//! use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};
//! use std::sync::Arc;
//!
//! # async fn example() -> cortex_core::error::Result<()> {
//! // Create connection manager
//! let config = ConnectionConfig::memory();
//! let pool_config = PoolConfig::default();
//! let manager = Arc::new(ConnectionManager::new(config).await?);
//!
//! // Create cognitive manager
//! let cognitive = CognitiveManager::new(manager);
//!
//! // Store an episode
//! let episode = EpisodicMemory::new(
//!     "Implement feature".to_string(),
//!     "agent-001".to_string(),
//!     cortex_core::id::CortexId::new(),
//!     EpisodeType::Feature,
//! );
//! cognitive.remember_episode(&episode).await?;
//!
//! // Consolidate memories
//! let report = cognitive.consolidate().await?;
//! println!("Consolidated {} patterns", report.patterns_extracted);
//! # Ok(())
//! # }
//! ```

pub mod types;
pub mod episodic;
pub mod semantic;
pub mod working;
pub mod procedural;
pub mod consolidation;
pub mod cognitive;
pub mod query;

pub use episodic::EpisodicMemorySystem;
pub use semantic::SemanticMemorySystem;
pub use working::WorkingMemorySystem;
pub use procedural::ProceduralMemorySystem;
pub use consolidation::MemoryConsolidator;
pub use cognitive::CognitiveManager;
pub use query::CrossMemoryQuery;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::types::*;
    pub use crate::episodic::EpisodicMemorySystem;
    pub use crate::semantic::SemanticMemorySystem;
    pub use crate::working::WorkingMemorySystem;
    pub use crate::procedural::ProceduralMemorySystem;
    pub use crate::consolidation::MemoryConsolidator;
    pub use crate::cognitive::CognitiveManager;
    pub use crate::query::CrossMemoryQuery;
}
