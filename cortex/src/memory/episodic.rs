use crate::storage::{deserialize, serialize, Storage};
use crate::types::{CodePattern, Outcome, TaskEpisode};
use crate::indexer::vector::{HnswIndex, VectorIndex};
use crate::embeddings::EmbeddingEngine;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Pattern index for fast lookup
#[derive(Debug, Clone)]
struct PatternIndex {
    /// Maps from keywords to episode IDs
    keyword_index: HashMap<String, Vec<String>>,
    /// Maps from episode ID to extracted patterns
    episode_patterns: HashMap<String, Vec<CodePattern>>,
}

impl PatternIndex {
    fn new() -> Self {
        Self {
            keyword_index: HashMap::new(),
            episode_patterns: HashMap::new(),
        }
    }

    /// Extract and index patterns from an episode
    fn extract_and_index(&mut self, episode: &TaskEpisode) {
        let patterns = Self::extract_patterns(episode);

        // Index keywords from task description
        let keywords = Self::extract_keywords(&episode.task_description);
        for keyword in keywords {
            self.keyword_index
                .entry(keyword.to_lowercase())
                .or_default()
                .push(episode.id.0.clone());
        }

        // Store patterns for this episode
        if !patterns.is_empty() {
            self.episode_patterns.insert(episode.id.0.clone(), patterns);
        }
    }

    /// Extract patterns from an episode
    fn extract_patterns(episode: &TaskEpisode) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Pattern 1: File access patterns
        if !episode.files_touched.is_empty() {
            patterns.push(CodePattern {
                id: format!("file_pattern_{}", episode.id.0),
                name: "File Access Pattern".to_string(),
                description: format!("Files typically accessed for: {}", episode.task_description),
                typical_actions: episode.files_touched.clone(),
                frequency: 1,
                success_rate: if episode.outcome == Outcome::Success { 1.0 } else { 0.0 },
                context_markers: Self::extract_keywords(&episode.task_description),
            });
        }

        // Pattern 2: Query patterns
        if !episode.queries_made.is_empty() {
            patterns.push(CodePattern {
                id: format!("query_pattern_{}", episode.id.0),
                name: "Query Pattern".to_string(),
                description: format!("Common queries for: {}", episode.task_description),
                typical_actions: episode.queries_made.clone(),
                frequency: 1,
                success_rate: if episode.outcome == Outcome::Success { 1.0 } else { 0.0 },
                context_markers: Self::extract_keywords(&episode.task_description),
            });
        }

        // Pattern 3: Solution path patterns
        if !episode.solution_path.is_empty() {
            patterns.push(CodePattern {
                id: format!("solution_pattern_{}", episode.id.0),
                name: "Solution Path Pattern".to_string(),
                description: format!("Solution approach for: {}", episode.task_description),
                typical_actions: vec![episode.solution_path.clone()],
                frequency: 1,
                success_rate: if episode.outcome == Outcome::Success { 1.0 } else { 0.0 },
                context_markers: Self::extract_keywords(&episode.task_description),
            });
        }

        patterns
    }

    /// Extract keywords from text
    fn extract_keywords(text: &str) -> Vec<String> {
        // Simple keyword extraction - split on whitespace and filter common words
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "is", "are", "was", "were", "be", "been", "being",
        ]
        .iter()
        .copied()
        .collect();

        text.split_whitespace()
            .filter(|w| w.len() > 2 && !stop_words.contains(&w.to_lowercase().as_str()))
            .map(|w| w.to_lowercase())
            .collect()
    }

    /// Find episodes matching keywords
    fn find_matching_episodes(&self, keywords: &[String]) -> Vec<String> {
        let mut episode_scores: HashMap<String, usize> = HashMap::new();

        for keyword in keywords {
            if let Some(episode_ids) = self.keyword_index.get(&keyword.to_lowercase()) {
                for id in episode_ids {
                    *episode_scores.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }

        // Sort by score (number of matching keywords)
        let mut results: Vec<_> = episode_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(id, _)| id).collect()
    }
}

/// Episodic memory - records of specific tasks and solutions
pub struct EpisodicMemory {
    storage: Arc<dyn Storage>,
    episodes: Vec<TaskEpisode>,
    retention_days: u32,
    pattern_index: PatternIndex,
    /// HNSW vector index for O(log n) similarity search (vs O(n) linear)
    vector_index: Option<HnswIndex<'static>>,
    /// Embedding engine for 384-dim Sentence-BERT embeddings
    embedding_engine: Option<EmbeddingEngine>,
    /// Path to persist HNSW index for fast startup
    hnsw_index_path: Option<PathBuf>,
}

impl std::fmt::Debug for EpisodicMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpisodicMemory")
            .field("episodes_count", &self.episodes.len())
            .field("retention_days", &self.retention_days)
            .field("has_vector_index", &self.vector_index.is_some())
            .field("has_embedding_engine", &self.embedding_engine.is_some())
            .field("hnsw_index_path", &self.hnsw_index_path)
            .finish()
    }
}

impl EpisodicMemory {
    pub fn new(storage: Arc<dyn Storage>, retention_days: u32) -> Result<Self> {
        Self::with_index_path(storage, retention_days, None)
    }

    /// Create with custom HNSW index path for persistence
    pub fn with_index_path(
        storage: Arc<dyn Storage>,
        retention_days: u32,
        hnsw_index_path: Option<PathBuf>,
    ) -> Result<Self> {
        // Initialize embedding engine
        let embedding_engine = match EmbeddingEngine::new() {
            Ok(engine) => {
                tracing::info!("Episodic memory: embedding engine initialized ({})", engine.model_name());
                Some(engine)
            }
            Err(e) => {
                tracing::warn!("Failed to init embedding engine, using keyword search only: {}", e);
                None
            }
        };

        // Initialize HNSW if embeddings available
        let vector_index = embedding_engine.as_ref().map(|engine| {
            tracing::info!("Episodic memory: initializing HNSW index (dim={})", engine.dimension());
            HnswIndex::new(engine.dimension(), 100_000) // Support 100K episodes
        });

        Ok(Self {
            storage,
            episodes: Vec::new(),
            retention_days,
            pattern_index: PatternIndex::new(),
            vector_index,
            embedding_engine,
            hnsw_index_path,
        })
    }

    /// Load episodes from storage
    pub async fn load(&mut self) -> Result<()> {
        // Try to load persisted HNSW index first for fast startup
        let mut loaded_from_disk = false;
        if let Some(ref index_path) = self.hnsw_index_path {
            if index_path.exists() {
                tracing::info!("Loading HNSW index from disk: {:?}", index_path);
                match HnswIndex::load(index_path) {
                    Ok(loaded_index) => {
                        tracing::info!("Successfully loaded HNSW index from disk ({} vectors)", loaded_index.len());
                        self.vector_index = Some(loaded_index);
                        loaded_from_disk = true;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load HNSW index from disk, will rebuild: {}", e);
                    }
                }
            }
        }

        let keys = self.storage.get_keys_with_prefix(b"episode:").await?;

        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                let episode: TaskEpisode = deserialize(&data)?;
                self.pattern_index.extract_and_index(&episode);

                // Generate embedding and add to HNSW (only if not loaded from disk)
                if !loaded_from_disk {
                    if let (Some(ref mut vi), Some(ref eng)) = (&mut self.vector_index, &self.embedding_engine) {
                        if let Ok(emb) = eng.generate_embedding(&episode.task_description) {
                            let _ = vi.add_vector(&episode.id.0, &emb);
                        }
                    }
                }

                self.episodes.push(episode);
            }
        }

        let with_emb = if self.vector_index.is_some() { self.episodes.len() } else { 0 };
        let load_source = if loaded_from_disk { "from disk" } else { "rebuilt" };
        tracing::info!("Loaded {} episodes ({} with HNSW embeddings {})", self.episodes.len(), with_emb, load_source);
        Ok(())
    }

    /// Record a new episode
    pub async fn record_episode(&mut self, episode: TaskEpisode) -> Result<()> {
        let key = format!("episode:{}", episode.id.0);
        let value = serialize(&episode)?;

        self.storage.put(key.as_bytes(), &value).await?;
        self.pattern_index.extract_and_index(&episode);

        // Generate embedding and add to HNSW for fast similarity search
        if let (Some(ref mut vi), Some(ref eng)) = (&mut self.vector_index, &self.embedding_engine) {
            match eng.generate_embedding(&episode.task_description) {
                Ok(emb) => {
                    if let Err(e) = vi.add_vector(&episode.id.0, &emb) {
                        tracing::warn!("Failed to index episode {} in HNSW: {}", episode.id.0, e);
                    }
                }
                Err(e) => tracing::warn!("Failed to generate embedding for episode {}: {}", episode.id.0, e),
            }
        }

        self.episodes.push(episode);
        Ok(())
    }

    /// Find similar episodes - HNSW vector search (O(log n)) or fallback to keyword (O(n))
    pub async fn find_similar(&self, task_description: &str, limit: usize) -> Vec<TaskEpisode> {
        let search_start = std::time::Instant::now();

        // Try HNSW vector search first (10-50x faster than linear!)
        if let (Some(ref vi), Some(ref eng)) = (&self.vector_index, &self.embedding_engine) {
            let emb_start = std::time::Instant::now();
            match eng.generate_embedding(task_description) {
                Ok(query_emb) => {
                    let emb_time = emb_start.elapsed();
                    let search_hnsw_start = std::time::Instant::now();

                    // Search for more candidates to filter by outcome
                    match vi.search(&query_emb, limit * 3) {
                        Ok(sim_ids) => {
                            let search_time = search_hnsw_start.elapsed();
                            let mut results = Vec::new();

                            for (id, score) in sim_ids {
                                if let Some(ep) = self.episodes.iter().find(|e| e.id.0 == id) {
                                    // Only return successful episodes with good similarity
                                    if ep.outcome == Outcome::Success && score > 0.3 {
                                        results.push(ep.clone());
                                        if results.len() >= limit { break; }
                                    }
                                }
                            }

                            if !results.is_empty() {
                                let total_time = search_start.elapsed();
                                tracing::info!(
                                    "HNSW search: found {} episodes in {:.2}ms (embed: {:.2}ms, search: {:.2}ms, index_size: {})",
                                    results.len(),
                                    total_time.as_secs_f64() * 1000.0,
                                    emb_time.as_secs_f64() * 1000.0,
                                    search_time.as_secs_f64() * 1000.0,
                                    vi.len()
                                );
                                return results;
                            }
                        }
                        Err(e) => tracing::warn!("HNSW search failed: {}", e),
                    }
                }
                Err(e) => tracing::warn!("Query embedding failed: {}", e),
            }
        }

        // Fallback: keyword search (slower but always works)
        tracing::info!("Using keyword-based episode search (HNSW not available)");
        let keywords = PatternIndex::extract_keywords(task_description);
        let matching_ids = self.pattern_index.find_matching_episodes(&keywords);

        let mut results = Vec::new();
        for id in matching_ids {
            if let Some(episode) = self.episodes.iter().find(|e| e.id.0 == id) {
                if episode.outcome == Outcome::Success {
                    results.push(episode.clone());
                    if results.len() >= limit { break; }
                }
            }
        }

        // Jaccard similarity fallback
        if results.len() < limit {
            let mut additional: Vec<_> = self.episodes.iter()
                .filter(|e| e.outcome == Outcome::Success
                    && !results.iter().any(|r| r.id == e.id)
                    && self.calculate_similarity(&e.task_description, task_description) > 0.3)
                .take(limit - results.len())
                .cloned()
                .collect();
            results.append(&mut additional);
        }

        results
    }

    /// Calculate simple text similarity using Jaccard similarity
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: HashSet<_> = text1.split_whitespace().map(|s| s.to_lowercase()).collect();
        let words2: HashSet<_> = text2.split_whitespace().map(|s| s.to_lowercase()).collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        intersection as f32 / union as f32
    }

    /// Extract patterns from a set of episodes
    pub fn extract_patterns(&self, episodes: &[&TaskEpisode]) -> Vec<CodePattern> {
        let mut patterns = Vec::new();
        let mut pattern_groups: HashMap<String, Vec<CodePattern>> = HashMap::new();

        // Group patterns by their markers
        for episode in episodes {
            let episode_patterns = PatternIndex::extract_patterns(episode);
            for pattern in episode_patterns {
                let key = pattern.context_markers.join("_");
                pattern_groups.entry(key).or_default().push(pattern);
            }
        }

        // Consolidate patterns in each group
        for (_, group) in pattern_groups {
            if let Some(consolidated) = self.consolidate_pattern_group(&group) {
                patterns.push(consolidated);
            }
        }

        patterns
    }

    /// Consolidate a group of similar patterns
    fn consolidate_pattern_group(&self, patterns: &[CodePattern]) -> Option<CodePattern> {
        if patterns.is_empty() {
            return None;
        }

        // Merge all patterns in the group
        let mut consolidated = patterns[0].clone();
        consolidated.frequency = patterns.len() as u32;

        // Average success rate
        let total_success: f32 = patterns.iter().map(|p| p.success_rate).sum();
        consolidated.success_rate = total_success / patterns.len() as f32;

        // Merge typical actions (deduplicated)
        let mut all_actions = HashSet::new();
        for pattern in patterns {
            all_actions.extend(pattern.typical_actions.iter().cloned());
        }
        consolidated.typical_actions = all_actions.into_iter().collect();

        Some(consolidated)
    }

    /// Consolidate and clean up old episodes
    pub async fn consolidate(&mut self) -> Result<()> {
        let now = chrono::Utc::now();
        let retention = chrono::Duration::days(self.retention_days as i64);

        // Remove episodes that are old and not frequently accessed
        let mut to_remove = Vec::new();

        for (i, episode) in self.episodes.iter().enumerate() {
            let age = now - episode.timestamp;

            // Keep recent episodes
            if age < retention {
                continue;
            }

            // Keep frequently accessed episodes
            if episode.access_count > 10 {
                continue;
            }

            // Keep high-value patterns
            if episode.outcome == Outcome::Success && episode.pattern_value > 0.8 {
                continue;
            }

            to_remove.push(i);
        }

        // Remove from storage and memory
        for i in to_remove.iter().rev() {
            let episode = &self.episodes[*i];
            let key = format!("episode:{}", episode.id.0);
            self.storage.delete(key.as_bytes()).await?;
            self.episodes.remove(*i);
        }

        // Rebuild pattern index after removal
        self.pattern_index = PatternIndex::new();
        for episode in &self.episodes {
            self.pattern_index.extract_and_index(episode);
        }

        tracing::info!("Consolidated episodes, removed {}", to_remove.len());
        Ok(())
    }

    /// Increment access count for an episode
    pub async fn increment_access(&mut self, episode_id: &str) -> Result<()> {
        if let Some(episode) = self.episodes.iter_mut().find(|e| e.id.0 == episode_id) {
            episode.access_count += 1;

            // Update in storage
            let key = format!("episode:{}", episode.id.0);
            let value = serialize(&episode)?;
            self.storage.put(key.as_bytes(), &value).await?;
        }
        Ok(())
    }

    /// Get all episodes
    pub fn episodes(&self) -> &[TaskEpisode] {
        &self.episodes
    }

    /// Get episode by ID
    pub fn get_episode(&self, id: &str) -> Option<&TaskEpisode> {
        self.episodes.iter().find(|e| e.id.0 == id)
    }

    /// Get patterns for an episode
    pub fn get_episode_patterns(&self, episode_id: &str) -> Option<&[CodePattern]> {
        self.pattern_index
            .episode_patterns
            .get(episode_id)
            .map(|v| v.as_slice())
    }

    /// Save HNSW index to disk for fast startup
    pub fn save_index(&self) -> Result<()> {
        if let (Some(ref index_path), Some(ref vi)) = (&self.hnsw_index_path, &self.vector_index) {
            tracing::info!("Saving HNSW index to disk: {:?} ({} vectors)", index_path, vi.len());
            vi.save(index_path)?;
            tracing::info!("HNSW index saved successfully");
        } else if self.hnsw_index_path.is_none() {
            tracing::debug!("No HNSW index path configured, skipping save");
        } else {
            tracing::debug!("No HNSW vector index to save");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::types::EpisodeId;
    use chrono::Utc;
    use tempfile::TempDir;

    async fn create_test_storage() -> (Arc<dyn Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MemoryStorage::new();
        (Arc::new(storage), temp_dir)
    }

    #[tokio::test]
    async fn test_record_and_load_episode() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = EpisodicMemory::new(storage.clone(), 30).unwrap();

        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Add authentication middleware".to_string(),
            initial_context: crate::types::ContextSnapshot {
                active_files: vec!["auth.ts".to_string()],
                active_symbols: vec![],
                working_directory: None,
            },
            queries_made: vec!["find middleware".to_string()],
            files_touched: vec!["src/middleware/auth.ts".to_string()],
            solution_path: "Created JWT middleware".to_string(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::new(1000),
            access_count: 0,
            pattern_value: 0.9,
        };

        memory.record_episode(episode.clone()).await.unwrap();
        assert_eq!(memory.episodes().len(), 1);

        // Create new memory instance and load
        let mut memory2 = EpisodicMemory::new(storage, 30).unwrap();
        memory2.load().await.unwrap();
        assert_eq!(memory2.episodes().len(), 1);
        assert_eq!(memory2.episodes()[0].task_description, episode.task_description);
    }

    #[tokio::test]
    async fn test_find_similar_episodes() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = EpisodicMemory::new(storage, 30).unwrap();

        // Add multiple episodes
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Add authentication middleware".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["src/middleware/auth.ts".to_string()],
            solution_path: String::new(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Fix authentication bug".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["src/middleware/auth.ts".to_string()],
            solution_path: String::new(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.7,
        };

        memory.record_episode(episode1).await.unwrap();
        memory.record_episode(episode2).await.unwrap();

        // Find similar episodes
        let similar = memory.find_similar("authentication middleware fix", 5).await;
        assert!(!similar.is_empty());
    }

    #[tokio::test]
    async fn test_pattern_extraction() {
        let (storage, _temp) = create_test_storage().await;
        let memory = EpisodicMemory::new(storage, 30).unwrap();

        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Add authentication".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec!["find auth".to_string()],
            files_touched: vec!["auth.ts".to_string()],
            solution_path: "Created middleware".to_string(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.9,
        };

        let patterns = memory.extract_patterns(&[&episode]);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.name.contains("Pattern")));
    }

    #[tokio::test]
    async fn test_consolidation() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = EpisodicMemory::new(storage, 30).unwrap();

        // Add old episode
        let old_episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now() - chrono::Duration::days(40),
            task_description: "Old task".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: String::new(),
            outcome: Outcome::Partial,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.1,
        };

        memory.record_episode(old_episode).await.unwrap();
        assert_eq!(memory.episodes().len(), 1);

        // Consolidate should remove old episode
        memory.consolidate().await.unwrap();
        assert_eq!(memory.episodes().len(), 0);
    }

    #[tokio::test]
    async fn test_increment_access() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = EpisodicMemory::new(storage, 30).unwrap();

        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Test task".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: String::new(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.5,
        };

        let episode_id = episode.id.0.clone();
        memory.record_episode(episode).await.unwrap();

        memory.increment_access(&episode_id).await.unwrap();
        assert_eq!(memory.get_episode(&episode_id).unwrap().access_count, 1);
    }
}
