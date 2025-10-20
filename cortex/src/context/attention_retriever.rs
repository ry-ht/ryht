use crate::storage::Storage;
use crate::types::{
    AttentionHistoryEntry, AttentionPattern, CodeSymbol, ContextQuery, PredictedFocus, SymbolId,
    TokenCount,
};
use anyhow::{Context as AnyhowContext, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

const MAX_HISTORY_SIZE: usize = 1000;
const HISTORY_STORAGE_KEY: &[u8] = b"attention:history";
const PREDICTOR_MODEL_KEY: &[u8] = b"attention:predictor_model";

/// Attention history tracker
pub struct AttentionHistory {
    entries: VecDeque<AttentionHistoryEntry>,
    symbol_frequency: HashMap<SymbolId, f32>,
    co_occurrence: HashMap<(SymbolId, SymbolId), usize>,
    storage: Arc<dyn Storage>,
}

impl AttentionHistory {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            symbol_frequency: HashMap::new(),
            co_occurrence: HashMap::new(),
            storage,
        }
    }

    /// Add a new attention pattern to history
    pub async fn record(&mut self, pattern: AttentionPattern, query_context: String) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let entry = AttentionHistoryEntry {
            timestamp,
            pattern: pattern.clone(),
            query_context,
        };

        // Update symbol frequency
        for (symbol, weight) in &pattern.focused_symbols {
            *self.symbol_frequency.entry(symbol.clone()).or_insert(0.0) += weight;
        }

        // Update co-occurrence matrix
        let symbols: Vec<_> = pattern.focused_symbols.keys().collect();
        for i in 0..symbols.len() {
            for j in (i + 1)..symbols.len() {
                let pair = (symbols[i].clone(), symbols[j].clone());
                *self.co_occurrence.entry(pair).or_insert(0) += 1;
            }
        }

        // Add to history
        self.entries.push_back(entry);
        if self.entries.len() > MAX_HISTORY_SIZE {
            if let Some(old_entry) = self.entries.pop_front() {
                // Decay old symbol frequencies
                for (symbol, weight) in &old_entry.pattern.focused_symbols {
                    if let Some(freq) = self.symbol_frequency.get_mut(symbol) {
                        *freq = (*freq - weight).max(0.0);
                    }
                }
            }
        }

        // Persist to storage
        self.save_to_storage().await?;

        Ok(())
    }

    /// Analyze patterns based on query
    pub fn analyze_pattern(&self, query: &ContextQuery) -> AttentionPattern {
        let mut focused = HashMap::new();
        let mut predicted = Vec::new();

        // Analyze recent attention patterns
        let recent_entries: Vec<_> = self
            .entries
            .iter()
            .rev()
            .take(50)
            .collect();

        // Calculate attention weights based on frequency and recency
        for entry in recent_entries {
            let age_weight = Self::calculate_age_weight(entry.timestamp);

            for (symbol, weight) in &entry.pattern.focused_symbols {
                // Check if symbol is relevant to current query
                let relevance = if query.symbols.contains(symbol) {
                    1.0
                } else {
                    self.calculate_symbol_relevance(symbol, &query.symbols)
                };

                if relevance > 0.1 {
                    *focused.entry(symbol.clone()).or_insert(0.0) +=
                        weight * age_weight * relevance;
                }
            }

            // Add predicted symbols
            for symbol in &entry.pattern.predicted_next {
                if !predicted.contains(symbol) && !query.symbols.contains(symbol) {
                    predicted.push(symbol.clone());
                }
            }
        }

        AttentionPattern {
            focused_symbols: focused,
            predicted_next: predicted,
        }
    }

    /// Calculate relevance based on co-occurrence
    fn calculate_symbol_relevance(&self, symbol: &SymbolId, context_symbols: &[SymbolId]) -> f32 {
        let mut relevance = 0.0;
        let mut total_weight = 0.0;

        for context_symbol in context_symbols {
            let pair1 = (symbol.clone(), context_symbol.clone());
            let pair2 = (context_symbol.clone(), symbol.clone());

            let co_occur = self.co_occurrence.get(&pair1).or_else(|| self.co_occurrence.get(&pair2))
                .copied()
                .unwrap_or(0);

            if co_occur > 0 {
                relevance += co_occur as f32;
                total_weight += 1.0;
            }
        }

        if total_weight > 0.0 {
            (relevance / total_weight).min(1.0)
        } else {
            0.0
        }
    }

    /// Calculate weight based on entry age
    fn calculate_age_weight(timestamp: u64) -> f32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age_seconds = now.saturating_sub(timestamp) as f32;
        let age_hours = age_seconds / 3600.0;

        // Exponential decay: weight = e^(-t/24) where t is in hours
        (-age_hours / 24.0).exp()
    }

    /// Get most frequent symbols
    pub fn get_frequent_symbols(&self, limit: usize) -> Vec<(SymbolId, f32)> {
        let mut freq_vec: Vec<_> = self.symbol_frequency.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        freq_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        freq_vec.into_iter().take(limit).collect()
    }

    /// Save history to storage
    async fn save_to_storage(&self) -> Result<()> {
        let data = serde_json::to_vec(&self.entries)?;
        self.storage.put(HISTORY_STORAGE_KEY, &data).await?;
        Ok(())
    }

    /// Load history from storage
    pub async fn load_from_storage(storage: Arc<dyn Storage>) -> Result<Self> {
        let mut history = Self::new(storage);

        if let Some(data) = history.storage.get(HISTORY_STORAGE_KEY).await? {
            let entries: VecDeque<AttentionHistoryEntry> = serde_json::from_slice(&data)?;
            history.entries = entries;

            // Rebuild frequency and co-occurrence maps
            for entry in &history.entries {
                for (symbol, weight) in &entry.pattern.focused_symbols {
                    *history.symbol_frequency.entry(symbol.clone()).or_insert(0.0) += weight;
                }

                let symbols: Vec<_> = entry.pattern.focused_symbols.keys().collect();
                for i in 0..symbols.len() {
                    for j in (i + 1)..symbols.len() {
                        let pair = (symbols[i].clone(), symbols[j].clone());
                        *history.co_occurrence.entry(pair).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok(history)
    }
}

/// Simple attention predictor based on frequency and transitions
#[derive(Debug, Clone, Default)]
pub struct SimpleAttentionPredictorModel {
    /// Symbol access frequencies (normalized 0-1)
    symbol_frequencies: HashMap<SymbolId, f32>,
    /// Transition probabilities: (current_symbol, next_symbol) -> probability
    transition_matrix: HashMap<(SymbolId, SymbolId), f32>,
    /// Total number of observations for normalization
    total_observations: usize,
}

/// Serializable version of the predictor model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializablePredictorModel {
    symbol_frequencies: Vec<(String, f32)>,
    transition_matrix: Vec<(String, String, f32)>,
    total_observations: usize,
}

impl From<&SimpleAttentionPredictorModel> for SerializablePredictorModel {
    fn from(model: &SimpleAttentionPredictorModel) -> Self {
        let symbol_frequencies = model.symbol_frequencies.iter()
            .map(|(k, v)| (k.0.clone(), *v))
            .collect();

        let transition_matrix = model.transition_matrix.iter()
            .map(|((from, to), prob)| (from.0.clone(), to.0.clone(), *prob))
            .collect();

        SerializablePredictorModel {
            symbol_frequencies,
            transition_matrix,
            total_observations: model.total_observations,
        }
    }
}

impl From<SerializablePredictorModel> for SimpleAttentionPredictorModel {
    fn from(ser: SerializablePredictorModel) -> Self {
        let symbol_frequencies = ser.symbol_frequencies.into_iter()
            .map(|(k, v)| (SymbolId::new(k), v))
            .collect();

        let transition_matrix = ser.transition_matrix.into_iter()
            .map(|(from, to, prob)| ((SymbolId::new(from), SymbolId::new(to)), prob))
            .collect();

        SimpleAttentionPredictorModel {
            symbol_frequencies,
            transition_matrix,
            total_observations: ser.total_observations,
        }
    }
}

impl SimpleAttentionPredictorModel {
    /// Predict next k symbols based on current context
    pub fn predict_next(&self, current: &[SymbolId], k: usize) -> Vec<(SymbolId, f32)> {
        let mut predictions: HashMap<SymbolId, f32> = HashMap::new();

        // Use transition probabilities from current symbols
        for curr_symbol in current {
            for ((from, to), prob) in &self.transition_matrix {
                if from == curr_symbol {
                    *predictions.entry(to.clone()).or_insert(0.0) += prob;
                }
            }
        }

        // Boost with global frequency for symbols not in transitions
        for (symbol, freq) in &self.symbol_frequencies {
            if !current.contains(symbol) {
                *predictions.entry(symbol.clone()).or_insert(0.0) += freq * 0.3;
            }
        }

        // Sort by score descending and take top k
        let mut pred_vec: Vec<_> = predictions.into_iter().collect();
        pred_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        pred_vec.into_iter().take(k).collect()
    }

    /// Update model with new transition
    pub fn update(&mut self, transition: (SymbolId, SymbolId)) {
        let (from, to) = transition;

        // Update transition count
        *self.transition_matrix.entry((from.clone(), to.clone())).or_insert(0.0) += 1.0;

        // Update symbol frequencies
        *self.symbol_frequencies.entry(from.clone()).or_insert(0.0) += 1.0;
        *self.symbol_frequencies.entry(to.clone()).or_insert(0.0) += 1.0;

        self.total_observations += 1;

        // Normalize periodically (every 100 observations)
        if self.total_observations.is_multiple_of(100) {
            self.normalize();
        }
    }

    /// Normalize probabilities
    fn normalize(&mut self) {
        // Normalize symbol frequencies
        let max_freq = self.symbol_frequencies.values()
            .copied()
            .fold(0.0f32, f32::max);
        if max_freq > 0.0 {
            for freq in self.symbol_frequencies.values_mut() {
                *freq /= max_freq;
            }
        }

        // Normalize transitions by source symbol
        let mut symbol_transition_totals: HashMap<SymbolId, f32> = HashMap::new();
        for ((from, _), count) in &self.transition_matrix {
            *symbol_transition_totals.entry(from.clone()).or_insert(0.0) += count;
        }

        for ((from, _), prob) in self.transition_matrix.iter_mut() {
            if let Some(total) = symbol_transition_totals.get(from) {
                if *total > 0.0 {
                    *prob /= total;
                }
            }
        }
    }
}

/// Predictive cache with LRU eviction and prefetching
pub struct PredictiveCache {
    /// Cached symbols by ID
    cached_symbols: HashMap<SymbolId, CachedSymbol>,
    /// Access order for LRU eviction
    access_order: VecDeque<SymbolId>,
    /// Maximum cache size
    capacity: usize,
    /// Number of symbols to prefetch based on predictions
    prediction_horizon: usize,
    /// Cache hit/miss statistics
    hits: usize,
    misses: usize,
}

/// Cached symbol with metadata
#[derive(Debug, Clone)]
struct CachedSymbol {
    symbol: CodeSymbol,
    #[allow(dead_code)] // Reserved for future cache expiration logic
    cached_at: u64,
    access_count: usize,
    predicted_score: f32,
}

impl PredictiveCache {
    pub fn new(capacity: usize, prediction_horizon: usize) -> Self {
        Self {
            cached_symbols: HashMap::new(),
            access_order: VecDeque::with_capacity(capacity),
            capacity,
            prediction_horizon,
            hits: 0,
            misses: 0,
        }
    }

    /// Get cached symbol if available
    pub fn get_cached(&mut self, id: &SymbolId) -> Option<&CodeSymbol> {
        if let Some(cached) = self.cached_symbols.get_mut(id) {
            // Update LRU order
            self.access_order.retain(|sid| sid != id);
            self.access_order.push_back(id.clone());

            // Update access metadata
            cached.access_count += 1;
            self.hits += 1;

            debug!(
                "Cache HIT for symbol '{}' (access_count: {}, score: {:.3})",
                id,
                cached.access_count,
                cached.predicted_score
            );

            Some(&cached.symbol)
        } else {
            self.misses += 1;
            debug!("Cache MISS for symbol '{}'", id);
            None
        }
    }

    /// Prefetch symbols based on predictions
    pub fn prefetch(&mut self, predictions: Vec<(SymbolId, f32)>) {
        let prefetch_count = predictions.len().min(self.prediction_horizon);

        debug!(
            "Prefetching {} symbols based on predictions (horizon: {})",
            prefetch_count,
            self.prediction_horizon
        );

        for (i, (symbol_id, score)) in predictions.iter().take(prefetch_count).enumerate() {
            // Skip if already cached
            if self.cached_symbols.contains_key(symbol_id) {
                debug!("  [{}] Symbol '{}' already cached (score: {:.3})", i, symbol_id, score);
                continue;
            }

            debug!("  [{}] Marking '{}' for prefetch (score: {:.3})", i, symbol_id, score);

            // In a real implementation, this would trigger async loading
            // For now, we just reserve space and track the prediction
            // The actual symbol will be loaded on first access
        }
    }

    /// Put symbol into cache
    pub fn put(&mut self, symbol: CodeSymbol, predicted_score: f32) {
        let symbol_id = symbol.id.clone();

        // Remove if already exists
        if self.cached_symbols.contains_key(&symbol_id) {
            self.access_order.retain(|id| id != &symbol_id);
        }

        // Evict LRU if at capacity
        if self.cached_symbols.len() >= self.capacity && !self.cached_symbols.contains_key(&symbol_id) {
            if let Some(lru_id) = self.access_order.pop_front() {
                if let Some(evicted) = self.cached_symbols.remove(&lru_id) {
                    debug!(
                        "Evicting LRU symbol '{}' (access_count: {}, score: {:.3})",
                        lru_id,
                        evicted.access_count,
                        evicted.predicted_score
                    );
                }
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cached = CachedSymbol {
            symbol,
            cached_at: timestamp,
            access_count: 0,
            predicted_score,
        };

        debug!(
            "Caching symbol '{}' (score: {:.3}, cache_size: {})",
            symbol_id,
            predicted_score,
            self.cached_symbols.len() + 1
        );

        self.cached_symbols.insert(symbol_id.clone(), cached);
        self.access_order.push_back(symbol_id);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hit_rate = if self.hits + self.misses > 0 {
            self.hits as f32 / (self.hits + self.misses) as f32
        } else {
            0.0
        };

        CacheStats {
            size: self.cached_symbols.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            hit_rate,
        }
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.cached_symbols.clear();
        self.access_order.clear();
        debug!("Cache cleared");
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cached_symbols.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cached_symbols.is_empty()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f32,
}

/// Enhanced attention predictor with simple ML model
pub struct AttentionPredictor {
    model: SimpleAttentionPredictorModel,
    storage: Arc<dyn Storage>,
}

impl AttentionPredictor {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            model: SimpleAttentionPredictorModel::default(),
            storage,
        }
    }

    /// Train the predictor model from attention history
    pub fn train(&mut self, history: &AttentionHistory) {
        info!("Training attention predictor with {} history entries", history.entries.len());

        self.model = SimpleAttentionPredictorModel::default();

        // Calculate transitions from history
        let entries: Vec<_> = history.entries.iter().collect();
        for window in entries.windows(2) {
            let current = &window[0].pattern;
            let next = &window[1].pattern;

            // Create transitions between focused symbols
            for current_symbol in current.focused_symbols.keys() {
                for next_symbol in next.focused_symbols.keys() {
                    self.model.update((current_symbol.clone(), next_symbol.clone()));
                }
            }

            // Also track transitions to predicted symbols
            for predicted in &current.predicted_next {
                if next.focused_symbols.contains_key(predicted) {
                    for current_symbol in current.focused_symbols.keys() {
                        self.model.update((current_symbol.clone(), predicted.clone()));
                    }
                }
            }
        }

        // Final normalization
        self.model.normalize();

        info!(
            "Predictor trained: {} symbols, {} transitions",
            self.model.symbol_frequencies.len(),
            self.model.transition_matrix.len()
        );
    }

    /// Predict next symbols based on current attention pattern
    pub fn predict(&self, pattern: &AttentionPattern) -> PredictedFocus {
        let current_symbols: Vec<_> = pattern.focused_symbols.keys().cloned().collect();

        // Get predictions with scores
        let predictions = self.model.predict_next(&current_symbols, 20);

        debug!(
            "Predicting from {} current symbols, got {} predictions",
            current_symbols.len(),
            predictions.len()
        );

        let mut high_prob = Vec::new();
        let mut medium_prob = Vec::new();
        let mut context = Vec::new();

        let mut confidence_sum = 0.0;

        // Categorize predictions by confidence threshold
        for (symbol, score) in predictions {
            debug!("  - Symbol '{}': score {:.3}", symbol, score);

            if score > 0.6 {
                high_prob.push(symbol);
            } else if score > 0.3 {
                medium_prob.push(symbol);
            } else if score > 0.1 {
                context.push(symbol);
            }
            confidence_sum += score;
        }

        let total_predictions = (high_prob.len() + medium_prob.len() + context.len()) as f32;
        let confidence = if total_predictions > 0.0 {
            (confidence_sum / total_predictions).min(1.0)
        } else {
            0.0
        };

        debug!(
            "Prediction results: {} high, {} medium, {} context (confidence: {:.3})",
            high_prob.len(),
            medium_prob.len(),
            context.len(),
            confidence
        );

        PredictedFocus {
            high_probability: high_prob,
            medium_probability: medium_prob,
            context,
            confidence,
        }
    }

    /// Update model with a new observation
    pub fn update_online(&mut self, transition: (SymbolId, SymbolId)) {
        debug!("Online update: {} -> {}", transition.0, transition.1);
        self.model.update(transition);
    }

    /// Save model to storage
    pub async fn save_to_storage(&self) -> Result<()> {
        let serializable = SerializablePredictorModel::from(&self.model);
        let data = serde_json::to_vec(&serializable)?;
        self.storage.put(PREDICTOR_MODEL_KEY, &data).await?;
        debug!("Predictor model saved to storage");
        Ok(())
    }

    /// Load model from storage
    pub async fn load_from_storage(storage: Arc<dyn Storage>) -> Result<Self> {
        let mut predictor = Self::new(storage);

        if let Some(data) = predictor.storage.get(PREDICTOR_MODEL_KEY).await? {
            let serializable: SerializablePredictorModel = serde_json::from_slice(&data)?;
            predictor.model = SimpleAttentionPredictorModel::from(serializable);
            info!(
                "Predictor model loaded: {} symbols, {} transitions",
                predictor.model.symbol_frequencies.len(),
                predictor.model.transition_matrix.len()
            );
        } else {
            info!("No existing predictor model found, starting fresh");
        }

        Ok(predictor)
    }

    /// Get model statistics
    pub fn stats(&self) -> PredictorStats {
        PredictorStats {
            symbols_tracked: self.model.symbol_frequencies.len(),
            transitions_tracked: self.model.transition_matrix.len(),
            total_observations: self.model.total_observations,
        }
    }
}

/// Predictor statistics
#[derive(Debug, Clone)]
pub struct PredictorStats {
    pub symbols_tracked: usize,
    pub transitions_tracked: usize,
    pub total_observations: usize,
}

/// Priority level for symbol retrieval
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High,
    Medium,
    Context,
}

/// Attention-based retrieval system with real prediction
pub struct AttentionBasedRetriever {
    attention_history: Arc<RwLock<AttentionHistory>>,
    prediction_model: Arc<RwLock<AttentionPredictor>>,
    cache: Arc<RwLock<PredictiveCache>>,
    /// Storage reference kept for potential future direct operations
    #[allow(dead_code)]
    storage: Arc<dyn Storage>,
}

impl AttentionBasedRetriever {
    pub async fn new(storage: Arc<dyn Storage>) -> Result<Self> {
        info!("Initializing AttentionBasedRetriever");

        let history = AttentionHistory::load_from_storage(storage.clone())
            .await
            .context("Failed to load attention history")?;

        let predictor = AttentionPredictor::load_from_storage(storage.clone())
            .await
            .context("Failed to load attention predictor")?;

        let cache = PredictiveCache::new(1000, 10);

        Ok(Self {
            attention_history: Arc::new(RwLock::new(history)),
            prediction_model: Arc::new(RwLock::new(predictor)),
            cache: Arc::new(RwLock::new(cache)),
            storage,
        })
    }

    /// Record attention pattern
    pub async fn record_attention(
        &self,
        pattern: AttentionPattern,
        query_context: String,
    ) -> Result<()> {
        debug!("Recording attention pattern for query: {}", query_context);

        let mut history = self.attention_history.write().await;
        history.record(pattern.clone(), query_context).await?;

        // Update predictor online with new transitions
        let mut predictor = self.prediction_model.write().await;
        let symbols: Vec<_> = pattern.focused_symbols.keys().cloned().collect();
        for i in 0..symbols.len().saturating_sub(1) {
            predictor.update_online((symbols[i].clone(), symbols[i + 1].clone()));
        }

        // Retrain predictor periodically (every 10 patterns)
        if history.entries.len() % 10 == 0 {
            info!("Periodic predictor retraining (every 10 patterns)");
            predictor.train(&history);
            predictor.save_to_storage().await?;
        }

        Ok(())
    }

    /// Retrieve symbols based on attention patterns with predictive prefetching
    pub async fn retrieve(
        &self,
        query: ContextQuery,
        token_budget: TokenCount,
    ) -> Result<RetrievalResult> {
        debug!(
            "Retrieving context for query with {} symbols, budget: {}",
            query.symbols.len(),
            token_budget
        );

        let history = self.attention_history.read().await;
        let predictor = self.prediction_model.read().await;
        let mut cache = self.cache.write().await;

        // Analyze current attention pattern
        let attention_pattern = history.analyze_pattern(&query);

        debug!(
            "Analyzed pattern: {} focused symbols, {} predicted next",
            attention_pattern.focused_symbols.len(),
            attention_pattern.predicted_next.len()
        );

        // Predict next symbols
        let predicted_focus = predictor.predict(&attention_pattern);

        // Prefetch predicted symbols into cache
        let all_predictions: Vec<_> = predicted_focus.high_probability.iter()
            .chain(predicted_focus.medium_probability.iter())
            .chain(predicted_focus.context.iter())
            .enumerate()
            .map(|(i, sym)| {
                let score = if i < predicted_focus.high_probability.len() {
                    0.8
                } else if i < predicted_focus.high_probability.len() + predicted_focus.medium_probability.len() {
                    0.5
                } else {
                    0.2
                };
                (sym.clone(), score)
            })
            .collect();

        cache.prefetch(all_predictions);

        // Build retrieval result with token budget awareness
        let mut result = RetrievalResult {
            high_attention: Vec::new(),
            medium_attention: Vec::new(),
            context_symbols: Vec::new(),
            total_tokens: TokenCount::zero(),
            token_budget,
            truncated: false,
        };

        // Priority 1: High probability symbols
        result.add_symbols_with_priority(
            predicted_focus.high_probability,
            Priority::High,
        );

        // Priority 2: Medium probability symbols (if budget allows)
        if result.has_token_budget() {
            result.add_symbols_with_priority(
                predicted_focus.medium_probability,
                Priority::Medium,
            );
        }

        // Priority 3: Context symbols (if budget allows)
        if result.has_token_budget() {
            result.add_symbols_with_priority(
                predicted_focus.context,
                Priority::Context,
            );
        }

        info!(
            "Retrieval complete: {} high, {} medium, {} context (total tokens: {}, truncated: {})",
            result.high_attention.len(),
            result.medium_attention.len(),
            result.context_symbols.len(),
            result.total_tokens,
            result.truncated
        );

        Ok(result)
    }

    /// Train the prediction model
    pub async fn train(&self) -> Result<()> {
        info!("Training prediction model");
        let history = self.attention_history.read().await;
        let mut predictor = self.prediction_model.write().await;
        predictor.train(&history);
        predictor.save_to_storage().await?;
        Ok(())
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get statistics
    pub async fn get_stats(&self) -> RetrievalStats {
        let history = self.attention_history.read().await;
        let cache = self.cache.read().await;
        let predictor = self.prediction_model.read().await;

        RetrievalStats {
            history_size: history.entries.len(),
            cache_size: cache.len(),
            frequent_symbols: history.get_frequent_symbols(10),
            cache_stats: cache.stats(),
            predictor_stats: predictor.stats(),
        }
    }
}

/// Result of attention-based retrieval
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub high_attention: Vec<SymbolId>,
    pub medium_attention: Vec<SymbolId>,
    pub context_symbols: Vec<SymbolId>,
    pub total_tokens: TokenCount,
    pub token_budget: TokenCount,
    pub truncated: bool,
}

impl RetrievalResult {
    fn add_symbols_with_priority(&mut self, symbols: Vec<SymbolId>, priority: Priority) {
        let estimated_tokens_per_symbol = 100; // Conservative estimate

        for symbol in symbols {
            let estimated_cost = TokenCount::new(estimated_tokens_per_symbol);

            if self.total_tokens.0 + estimated_cost.0 <= self.token_budget.0 {
                match priority {
                    Priority::High => self.high_attention.push(symbol),
                    Priority::Medium => self.medium_attention.push(symbol),
                    Priority::Context => self.context_symbols.push(symbol),
                }
                self.total_tokens.0 += estimated_cost.0;
            } else {
                self.truncated = true;
                break;
            }
        }
    }

    fn has_token_budget(&self) -> bool {
        self.total_tokens.0 < self.token_budget.0
    }

    /// Get all symbols in priority order
    pub fn all_symbols(&self) -> Vec<SymbolId> {
        let mut result = Vec::new();
        result.extend(self.high_attention.iter().cloned());
        result.extend(self.medium_attention.iter().cloned());
        result.extend(self.context_symbols.iter().cloned());
        result
    }
}

/// Statistics about retrieval system
#[derive(Debug, Clone)]
pub struct RetrievalStats {
    pub history_size: usize,
    pub cache_size: usize,
    pub frequent_symbols: Vec<(SymbolId, f32)>,
    pub cache_stats: CacheStats,
    pub predictor_stats: PredictorStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::types::symbol::{CodeSymbol, SymbolKind, SymbolMetadata};
    use tempfile::TempDir;

    async fn create_test_storage() -> (Arc<dyn Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MemoryStorage::new();
        (Arc::new(storage), temp_dir)
    }

    fn create_test_symbol(id: &str) -> CodeSymbol {
        CodeSymbol {
            id: SymbolId::new(id),
            name: format!("Symbol_{}", id),
            kind: SymbolKind::Function,
            signature: format!("fn {}()", id),
            body_hash: crate::types::Hash::from_string(format!("hash_{}", id)),
            location: crate::types::Location::new("test.rs".to_string(), 1, 1, 0, 0),
            references: vec![],
            dependencies: vec![],
            metadata: SymbolMetadata {
                complexity: 1,
                token_cost: TokenCount::new(100),
                last_modified: None,
                authors: vec![],
                doc_comment: None,
                test_coverage: 0.0,
                usage_frequency: 0,
            },
            embedding: None,
        }
    }

    #[tokio::test]
    async fn test_simple_attention_predictor() {
        let mut model = SimpleAttentionPredictorModel::default();

        // Train with transitions
        model.update((SymbolId::new("sym1"), SymbolId::new("sym2")));
        model.update((SymbolId::new("sym2"), SymbolId::new("sym3")));
        model.update((SymbolId::new("sym1"), SymbolId::new("sym3")));
        model.update((SymbolId::new("sym1"), SymbolId::new("sym2"))); // Reinforce

        // Predict next from sym1
        let predictions = model.predict_next(&[SymbolId::new("sym1")], 5);

        assert!(!predictions.is_empty());
        assert_eq!(predictions[0].0, SymbolId::new("sym2")); // Most frequent transition
        assert!(predictions[0].1 > 0.0);
    }

    #[tokio::test]
    async fn test_predictive_cache_prefetch() {
        let mut cache = PredictiveCache::new(10, 5);

        let predictions = vec![
            (SymbolId::new("sym1"), 0.9),
            (SymbolId::new("sym2"), 0.7),
            (SymbolId::new("sym3"), 0.5),
        ];

        cache.prefetch(predictions);

        // Prefetch marks symbols for loading
        // Cache should still be empty until symbols are actually loaded
        assert_eq!(cache.len(), 0);

        // Now add a symbol
        cache.put(create_test_symbol("sym1"), 0.9);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_predictive_cache_lru_eviction() {
        let mut cache = PredictiveCache::new(3, 5);

        // Fill cache
        cache.put(create_test_symbol("sym1"), 0.8);
        cache.put(create_test_symbol("sym2"), 0.7);
        cache.put(create_test_symbol("sym3"), 0.6);
        assert_eq!(cache.len(), 3);

        // Add one more - should evict LRU (sym1)
        cache.put(create_test_symbol("sym4"), 0.9);
        assert_eq!(cache.len(), 3);

        // sym1 should be evicted
        assert!(cache.get_cached(&SymbolId::new("sym1")).is_none());
        assert!(cache.get_cached(&SymbolId::new("sym2")).is_some());
    }

    #[tokio::test]
    async fn test_predictive_cache_stats() {
        let mut cache = PredictiveCache::new(10, 5);

        cache.put(create_test_symbol("sym1"), 0.8);

        // Hit
        let _ = cache.get_cached(&SymbolId::new("sym1"));

        // Miss
        let _ = cache.get_cached(&SymbolId::new("sym2"));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_attention_history_record() {
        let (storage, _temp) = create_test_storage().await;
        let mut history = AttentionHistory::new(storage);

        let mut focused = HashMap::new();
        focused.insert(SymbolId::new("sym1"), 0.8);
        focused.insert(SymbolId::new("sym2"), 0.6);

        let pattern = AttentionPattern {
            focused_symbols: focused,
            predicted_next: vec![SymbolId::new("sym3")],
        };

        history.record(pattern, "test query".to_string()).await.unwrap();

        assert_eq!(history.entries.len(), 1);
        assert!(history.symbol_frequency.contains_key(&SymbolId::new("sym1")));
    }

    #[tokio::test]
    async fn test_attention_predictor_train_and_predict() {
        let (storage, _temp) = create_test_storage().await;
        let mut history = AttentionHistory::new(storage.clone());

        // Add sequential patterns
        for i in 0..5 {
            let mut focused = HashMap::new();
            focused.insert(SymbolId::new(format!("sym{}", i)), 0.8);
            focused.insert(SymbolId::new(format!("sym{}", i + 1)), 0.5);

            let pattern = AttentionPattern {
                focused_symbols: focused,
                predicted_next: vec![],
            };

            history.record(pattern, format!("query {}", i)).await.unwrap();
        }

        let mut predictor = AttentionPredictor::new(storage);
        predictor.train(&history);

        // Predict from sym2
        let pattern = AttentionPattern {
            focused_symbols: [(SymbolId::new("sym2"), 0.8)].into_iter().collect(),
            predicted_next: vec![],
        };

        let predictions = predictor.predict(&pattern);
        assert!(!predictions.high_probability.is_empty() ||
                !predictions.medium_probability.is_empty() ||
                !predictions.context.is_empty());
    }

    #[tokio::test]
    async fn test_attention_retriever_integration() {
        let (storage, _temp) = create_test_storage().await;
        let retriever = AttentionBasedRetriever::new(storage).await.unwrap();

        // Record some patterns
        for i in 0..3 {
            let mut focused = HashMap::new();
            focused.insert(SymbolId::new(format!("sym{}", i)), 0.8);
            focused.insert(SymbolId::new(format!("sym{}", i + 1)), 0.6);

            let pattern = AttentionPattern {
                focused_symbols: focused,
                predicted_next: vec![SymbolId::new(format!("sym{}", i + 2))],
            };

            retriever.record_attention(pattern, format!("query {}", i))
                .await
                .unwrap();
        }

        // Retrieve with query
        let query = ContextQuery {
            text: "test query".to_string(),
            symbols: vec![SymbolId::new("sym1")],
            context_size: TokenCount::new(0),
            timestamp: 0,
        };

        let result = retriever.retrieve(query, TokenCount::new(1000))
            .await
            .unwrap();

        // Should have some predictions
        let total_predictions = result.high_attention.len() +
                               result.medium_attention.len() +
                               result.context_symbols.len();
        assert!(total_predictions > 0);
    }

    #[tokio::test]
    async fn test_retrieval_stats() {
        let (storage, _temp) = create_test_storage().await;
        let retriever = AttentionBasedRetriever::new(storage).await.unwrap();

        let stats = retriever.get_stats().await;
        assert_eq!(stats.history_size, 0);
        assert_eq!(stats.cache_size, 0);
    }

    #[tokio::test]
    async fn test_online_learning() {
        let (storage, _temp) = create_test_storage().await;
        let mut predictor = AttentionPredictor::new(storage);

        // Online updates
        predictor.update_online((SymbolId::new("a"), SymbolId::new("b")));
        predictor.update_online((SymbolId::new("b"), SymbolId::new("c")));
        predictor.update_online((SymbolId::new("a"), SymbolId::new("c")));

        let stats = predictor.stats();
        assert!(stats.symbols_tracked >= 3);
        assert!(stats.transitions_tracked >= 3);
    }

    #[tokio::test]
    async fn test_predictor_persistence() {
        let (storage, _temp) = create_test_storage().await;

        // Train and save
        {
            let mut predictor = AttentionPredictor::new(storage.clone());
            predictor.update_online((SymbolId::new("a"), SymbolId::new("b")));
            predictor.save_to_storage().await.unwrap();
        }

        // Load in new instance
        {
            let predictor = AttentionPredictor::load_from_storage(storage)
                .await
                .unwrap();
            let stats = predictor.stats();
            assert!(stats.symbols_tracked > 0);
        }
    }
}
