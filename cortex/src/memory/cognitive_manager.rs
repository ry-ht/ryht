use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::embeddings::EmbeddingEngine;
use crate::storage::{deserialize, serialize, Storage};
use crate::types::{Outcome, TaskEpisode};

use super::episodic::EpisodicMemory;
use super::procedural::ProceduralMemory;
use super::semantic::SemanticMemory;
use super::working::WorkingMemory;

/// MemGPT-style hierarchical memory manager
/// Solves LLM amnesia through multi-tier memory architecture
pub struct CognitiveMemoryManager {
    db: Arc<Surreal<Db>>,
    storage: Arc<dyn Storage>,

    // MemGPT-style memory hierarchy
    pub core_memory: CoreMemory,
    pub working_memory: WorkingMemory,
    pub episodic_memory: EpisodicMemory,
    pub semantic_memory: SemanticMemory,
    pub procedural_memory: ProceduralMemory,

    // Embedding engine for semantic operations
    embedding_engine: Option<EmbeddingEngine>,
}

/// Core memory - always in context (~2K tokens)
/// This is the agent's "identity" and current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    /// Who the agent is and its role
    pub agent_persona: String,

    /// Who the user is and their preferences
    pub user_persona: String,

    /// Current system state and context
    pub system_context: String,

    /// Critical facts that must always be remembered
    pub key_facts: Vec<String>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl Default for CoreMemory {
    fn default() -> Self {
        Self {
            agent_persona: "I am Meridian, a cognitive memory system for LLM codebase interaction. \
                           I help maintain context across sessions and learn from past interactions."
                .to_string(),
            user_persona: "Developer working on a codebase".to_string(),
            system_context: "Initialized".to_string(),
            key_facts: Vec::new(),
            last_updated: Utc::now(),
        }
    }
}

impl CoreMemory {
    /// Add a key fact to core memory
    pub fn add_key_fact(&mut self, fact: String) {
        if !self.key_facts.contains(&fact) {
            self.key_facts.push(fact);
            self.last_updated = Utc::now();
        }
    }

    /// Remove a key fact from core memory
    pub fn remove_key_fact(&mut self, fact: &str) -> bool {
        if let Some(pos) = self.key_facts.iter().position(|f| f == fact) {
            self.key_facts.remove(pos);
            self.last_updated = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update system context
    pub fn update_context(&mut self, context: String) {
        self.system_context = context;
        self.last_updated = Utc::now();
    }

    /// Estimate token count for core memory
    pub fn token_estimate(&self) -> usize {
        // Rough estimate: ~4 chars per token
        let total_chars = self.agent_persona.len()
            + self.user_persona.len()
            + self.system_context.len()
            + self.key_facts.iter().map(|f| f.len()).sum::<usize>();

        total_chars / 4
    }
}

/// Memory item for working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub item_type: MemoryItemType,
    pub importance: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryItemType {
    FileContent,
    CodeSymbol,
    Documentation,
    Conversation,
    TaskContext,
}

impl std::fmt::Display for MemoryItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryItemType::FileContent => write!(f, "file"),
            MemoryItemType::CodeSymbol => write!(f, "symbol"),
            MemoryItemType::Documentation => write!(f, "docs"),
            MemoryItemType::Conversation => write!(f, "conversation"),
            MemoryItemType::TaskContext => write!(f, "task"),
        }
    }
}

/// Learning extracted from episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: String,
    pub pattern: String,
    pub confidence: f32,
    pub episode_ids: Vec<String>,
    pub applications: usize,
    pub last_applied: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub core_memory_tokens: usize,
    pub working_memory_items: usize,
    pub episodic_memory_count: usize,
    pub semantic_memory_count: usize,
    pub procedural_memory_count: usize,
    pub total_token_estimate: usize,
}

impl CognitiveMemoryManager {
    /// Create a new cognitive memory manager
    pub async fn new(
        db: Arc<Surreal<Db>>,
        storage: Arc<dyn Storage>,
        working_memory_size: usize,
        episodic_retention_days: u32,
        hnsw_index_path: Option<PathBuf>,
    ) -> Result<Self> {
        tracing::info!("Initializing cognitive memory manager with SurrealDB support");

        // Initialize embedding engine
        let embedding_engine = match EmbeddingEngine::new() {
            Ok(engine) => {
                tracing::info!("Embedding engine initialized: {}", engine.model_name());
                Some(engine)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize embedding engine: {}", e);
                None
            }
        };

        // Load or create core memory
        let core_memory = Self::load_core_memory(&storage).await?;

        // Initialize memory subsystems with SurrealDB support
        let working_memory = WorkingMemory::new(working_memory_size.to_string())?;
        let episodic_memory =
            EpisodicMemory::with_index_path(storage.clone(), episodic_retention_days, hnsw_index_path)?;
        let semantic_memory = SemanticMemory::with_surrealdb(storage.clone(), db.clone())?;
        let procedural_memory = ProceduralMemory::new(storage.clone())?;

        tracing::info!("Cognitive memory manager initialized with graph-enhanced semantic memory");

        Ok(Self {
            db,
            storage,
            core_memory,
            working_memory,
            episodic_memory,
            semantic_memory,
            procedural_memory,
            embedding_engine,
        })
    }

    /// Initialize the memory system (load from storage)
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Loading memory systems");

        // Load all memory subsystems
        self.episodic_memory.load().await?;
        self.semantic_memory.load().await?;
        self.procedural_memory.load().await?;

        tracing::info!("Memory systems loaded successfully");
        Ok(())
    }

    /// Load core memory from storage
    async fn load_core_memory(storage: &Arc<dyn Storage>) -> Result<CoreMemory> {
        match storage.get(b"core_memory").await? {
            Some(data) => {
                let core_memory: CoreMemory = deserialize(&data)
                    .with_context(|| "Failed to deserialize core memory")?;
                tracing::info!("Loaded existing core memory");
                Ok(core_memory)
            }
            None => {
                let core_memory = CoreMemory::default();
                tracing::info!("Created new core memory");
                Ok(core_memory)
            }
        }
    }

    /// Save core memory to storage
    pub async fn save_core_memory(&self) -> Result<()> {
        let data = serialize(&self.core_memory)
            .with_context(|| "Failed to serialize core memory")?;
        self.storage.put(b"core_memory", &data).await?;
        Ok(())
    }

    /// Record an interaction episode
    pub async fn record_episode(&mut self, episode: TaskEpisode) -> Result<()> {
        tracing::info!(
            episode_id = %episode.id.0,
            task = %episode.task_description,
            "Recording episode"
        );

        // Record in episodic memory
        self.episodic_memory.record_episode(episode.clone()).await?;

        // Extract learnings if successful
        if episode.outcome == Outcome::Success {
            self.extract_learnings_from_episode(&episode).await?;
        }

        // Update procedural memory with solution patterns
        if !episode.solution_path.is_empty() {
            self.procedural_memory
                .record_solution(&episode.task_description, &episode.solution_path)
                .await?;
        }

        Ok(())
    }

    /// Retrieve relevant memories for current context
    pub async fn retrieve_context(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        tracing::debug!(query = %query, limit = %limit, "Retrieving context");

        let mut memories = Vec::new();

        // Get similar episodes
        let similar_episodes = self.episodic_memory.find_similar(query, limit).await;

        for episode in similar_episodes {
            // Calculate relevance score based on semantic similarity and recency
            let recency_score = Self::calculate_recency_score(episode.timestamp);
            let keyword_overlap = Self::calculate_keyword_overlap(query, &episode.task_description);
            let relevance = (keyword_overlap * 0.7) + (recency_score * 0.3);

            memories.push(Memory {
                id: episode.id.0.clone(),
                content: format!(
                    "Task: {}\nSolution: {}\nFiles: {:?}",
                    episode.task_description, episode.solution_path, episode.files_touched
                ),
                memory_type: MemoryType::Episodic,
                relevance_score: relevance,
                timestamp: episode.timestamp,
            });
        }

        // Get relevant semantic knowledge
        let semantic_items = self.semantic_memory.find_relevant(query, limit).await;
        for item in semantic_items {
            memories.push(Memory {
                id: item.id,
                content: item.content,
                memory_type: MemoryType::Semantic,
                relevance_score: 1.0,
                timestamp: item.created_at,
            });
        }

        // Sort by relevance and timestamp
        memories.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.timestamp.cmp(&a.timestamp))
        });

        memories.truncate(limit);
        Ok(memories)
    }

    /// Update working memory with new information
    pub async fn update_working_memory(&mut self, items: Vec<MemoryItem>) -> Result<()> {
        // Working memory in current implementation uses symbol-based tracking
        // This is a placeholder for future enhancement with general memory items
        // For now, we can track this in core memory's key_facts
        for item in items {
            if item.importance > 0.7 {
                self.core_memory.add_key_fact(format!("{}: {}", item.item_type, item.content));
            }
        }
        Ok(())
    }

    /// Compress old memories (MemGPT-style summarization)
    pub async fn compress_memories(&mut self, older_than: Duration) -> Result<CompressionStats> {
        tracing::info!(
            days = older_than.num_days(),
            "Starting memory compression"
        );

        let cutoff = Utc::now() - older_than;
        let mut stats = CompressionStats {
            episodes_processed: 0,
            episodes_compressed: 0,
            semantic_memories_created: 0,
            space_saved_bytes: 0,
        };

        // Get old episodes
        let old_episodes: Vec<_> = self
            .episodic_memory
            .episodes()
            .iter()
            .filter(|e| e.timestamp < cutoff)
            .cloned()
            .collect();

        stats.episodes_processed = old_episodes.len();

        // Group similar episodes
        let episode_groups = self.group_similar_episodes(&old_episodes).await?;

        // Compress each group into semantic memory
        for group in episode_groups {
            if group.len() > 1 {
                let summary = self.summarize_episode_group(&group).await?;

                // Store in semantic memory
                self.semantic_memory
                    .add_knowledge(summary.title, summary.content)
                    .await?;

                stats.episodes_compressed += group.len();
                stats.semantic_memories_created += 1;
            }
        }

        // Consolidate episodic memory (removes old episodes)
        self.episodic_memory.consolidate().await?;

        tracing::info!(
            "Memory compression complete: {} episodes -> {} semantic memories",
            stats.episodes_compressed,
            stats.semantic_memories_created
        );

        Ok(stats)
    }

    /// Extract learnings from episodes
    pub async fn extract_learnings(&self) -> Result<Vec<Learning>> {
        let mut learnings = Vec::new();

        // Get all successful episodes
        let successful_episodes: Vec<_> = self
            .episodic_memory
            .episodes()
            .iter()
            .filter(|e| e.outcome == Outcome::Success)
            .collect();

        // Extract patterns
        let patterns = self.episodic_memory.extract_patterns(&successful_episodes);

        for pattern in patterns {
            learnings.push(Learning {
                id: pattern.id,
                pattern: pattern.description,
                confidence: pattern.success_rate,
                episode_ids: successful_episodes
                    .iter()
                    .take(pattern.frequency as usize)
                    .map(|e| e.id.0.clone())
                    .collect(),
                applications: pattern.frequency as usize,
                last_applied: None,
                created_at: Utc::now(),
            });
        }

        Ok(learnings)
    }

    /// Extract learnings from a single episode
    async fn extract_learnings_from_episode(&mut self, episode: &TaskEpisode) -> Result<()> {
        // Create a learning from the solution path if it's valuable
        if episode.pattern_value > 0.7 && !episode.solution_path.is_empty() {
            let learning = Learning {
                id: format!("learning_{}", episode.id.0),
                pattern: episode.solution_path.clone(),
                confidence: episode.pattern_value,
                episode_ids: vec![episode.id.0.clone()],
                applications: 1,
                last_applied: Some(episode.timestamp),
                created_at: episode.timestamp,
            };

            // Store in procedural memory
            self.procedural_memory
                .record_solution(&episode.task_description, &episode.solution_path)
                .await?;
        }

        Ok(())
    }

    /// Forget irrelevant information (selective forgetting)
    pub async fn prune_memories(&mut self, relevance_threshold: f32) -> Result<PruneStats> {
        tracing::info!(
            threshold = relevance_threshold,
            "Pruning low-relevance memories"
        );

        let mut stats = PruneStats {
            episodes_pruned: 0,
            semantic_items_pruned: 0,
        };

        // Prune episodic memories with low pattern value
        let to_remove: Vec<_> = self
            .episodic_memory
            .episodes()
            .iter()
            .filter(|e| e.pattern_value < relevance_threshold && e.access_count == 0)
            .map(|e| e.id.0.clone())
            .collect();

        for episode_id in to_remove {
            let key = format!("episode:{}", episode_id);
            self.storage.delete(key.as_bytes()).await?;
            stats.episodes_pruned += 1;
        }

        // Reload episodic memory
        let mut new_episodic = EpisodicMemory::new(self.storage.clone(), 30)?;
        new_episodic.load().await?;
        self.episodic_memory = new_episodic;

        tracing::info!(
            "Pruning complete: {} episodes removed",
            stats.episodes_pruned
        );

        Ok(stats)
    }

    /// Group similar episodes together for compression
    async fn group_similar_episodes(&self, episodes: &[TaskEpisode]) -> Result<Vec<Vec<TaskEpisode>>> {
        let mut groups: Vec<Vec<TaskEpisode>> = Vec::new();
        let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

        for episode in episodes {
            if processed.contains(&episode.id.0) {
                continue;
            }

            // Find similar episodes
            let similar = self
                .episodic_memory
                .find_similar(&episode.task_description, 5)
                .await;

            let mut group = vec![episode.clone()];
            processed.insert(episode.id.0.clone());

            for sim in similar {
                if !processed.contains(&sim.id.0) && episodes.iter().any(|e| e.id == sim.id) {
                    group.push(sim.clone());
                    processed.insert(sim.id.0.clone());
                }
            }

            if !group.is_empty() {
                groups.push(group);
            }
        }

        Ok(groups)
    }

    /// Summarize a group of episodes into semantic memory
    async fn summarize_episode_group(&self, episodes: &[TaskEpisode]) -> Result<EpisodeSummary> {
        // Extract common patterns
        let common_files: std::collections::HashSet<_> = episodes
            .iter()
            .flat_map(|e| e.files_touched.clone())
            .collect();

        let common_queries: std::collections::HashSet<_> = episodes
            .iter()
            .flat_map(|e| e.queries_made.clone())
            .collect();

        // Create summary
        let title = format!(
            "Pattern: {} related tasks",
            episodes.first().map(|e| &e.task_description).unwrap_or(&String::from("Unknown"))
        );

        let content = format!(
            "Summary of {} similar tasks:\n\
             Common files: {:?}\n\
             Common queries: {:?}\n\
             Success rate: {:.1}%\n\
             Average tokens: {}",
            episodes.len(),
            common_files,
            common_queries,
            episodes.iter().filter(|e| e.outcome == Outcome::Success).count() as f32
                / episodes.len() as f32
                * 100.0,
            episodes.iter().map(|e| e.tokens_used.0).sum::<u32>() / episodes.len() as u32
        );

        Ok(EpisodeSummary { title, content })
    }

    /// Get memory statistics
    pub fn get_statistics(&self) -> MemoryStats {
        let core_tokens = self.core_memory.token_estimate();
        let working_items = self.working_memory.get_active_count();
        let episodic_count = self.episodic_memory.episodes().len();
        let semantic_count = self.semantic_memory.knowledge_count();
        let procedural_count = self.procedural_memory.procedure_count();

        // Estimate total tokens (working memory tracks its own tokens)
        let working_tokens: usize = self.working_memory.estimate_tokens().into();
        let total_estimate = core_tokens + working_tokens + (episodic_count * 500);

        MemoryStats {
            core_memory_tokens: core_tokens,
            working_memory_items: working_items,
            episodic_memory_count: episodic_count,
            semantic_memory_count: semantic_count,
            procedural_memory_count: procedural_count,
            total_token_estimate: total_estimate,
        }
    }

    /// Consolidate all memory systems
    pub async fn consolidate_all(&mut self) -> Result<()> {
        tracing::info!("Consolidating all memory systems");

        self.episodic_memory.consolidate().await?;
        self.semantic_memory.consolidate().await?;
        self.save_core_memory().await?;

        // Save HNSW index for fast startup
        self.episodic_memory.save_index()?;

        tracing::info!("Memory consolidation complete");
        Ok(())
    }

    /// Calculate recency score (0.0 to 1.0, higher = more recent)
    fn calculate_recency_score(timestamp: DateTime<Utc>) -> f32 {
        let now = Utc::now();
        let age = now.signed_duration_since(timestamp);
        let age_hours = age.num_hours() as f32;

        // Exponential decay: half-life of 7 days (168 hours)
        let half_life_hours = 168.0;
        let decay_rate = 0.693 / half_life_hours; // ln(2) / half_life

        (-decay_rate * age_hours).exp()
    }

    /// Calculate keyword overlap score (0.0 to 1.0)
    fn calculate_keyword_overlap(query: &str, text: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let query_words: std::collections::HashSet<_> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let text_lower = text.to_lowercase();
        let text_words: std::collections::HashSet<_> = text_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if query_words.is_empty() || text_words.is_empty() {
            return 0.0;
        }

        let intersection = query_words.intersection(&text_words).count();
        let union = query_words.union(&text_words).count();

        intersection as f32 / union as f32
    }
}

/// Memory record for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub relevance_score: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    Core,
    Working,
    Episodic,
    Semantic,
    Procedural,
}

/// Compression statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub episodes_processed: usize,
    pub episodes_compressed: usize,
    pub semantic_memories_created: usize,
    pub space_saved_bytes: usize,
}

/// Pruning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStats {
    pub episodes_pruned: usize,
    pub semantic_items_pruned: usize,
}

/// Episode summary for compression
#[derive(Debug, Clone)]
struct EpisodeSummary {
    title: String,
    content: String,
}
