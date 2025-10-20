use crate::types::{AttentionPattern, SymbolId, TokenCount, WorkingContext};
use anyhow::Result;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

/// Symbol with metadata for LRU tracking
#[derive(Debug, Clone)]
struct SymbolMetadata {
    /// Last access timestamp
    last_access: u64,
    /// Attention weight
    attention_weight: f32,
    /// Estimated token cost
    token_cost: TokenCount,
    /// Access frequency
    access_count: u32,
}

impl SymbolMetadata {
    fn new(token_cost: TokenCount) -> Self {
        Self {
            last_access: Self::current_timestamp(),
            attention_weight: 0.0,
            token_cost,
            access_count: 1,
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn update_access(&mut self, weight: f32) {
        self.last_access = Self::current_timestamp();
        self.attention_weight += weight;
        self.access_count += 1;
    }

    /// Calculate combined score for eviction (higher is better)
    fn score(&self) -> f32 {
        let now = Self::current_timestamp();
        let age_seconds = now.saturating_sub(self.last_access) as f32;
        let recency_score = 1.0 / (1.0 + age_seconds / 60.0); // Decay over minutes

        // Combine recency, attention, and frequency
        // Higher weight = keep longer
        recency_score * 0.4 + self.attention_weight * 0.4 + (self.access_count as f32 / 100.0) * 0.2
    }
}

/// Working memory - active symbols for current task
pub struct WorkingMemory {
    capacity: TokenCount,
    active_symbols: BTreeSet<SymbolId>,
    symbol_metadata: HashMap<SymbolId, SymbolMetadata>,
    current_usage: TokenCount,
    prefetch_queue: VecDeque<SymbolId>,
    eviction_history: VecDeque<SymbolId>,
}

impl WorkingMemory {
    pub fn new(capacity_str: String) -> Result<Self> {
        let capacity = Self::parse_capacity(&capacity_str)?;

        Ok(Self {
            capacity,
            active_symbols: BTreeSet::new(),
            symbol_metadata: HashMap::new(),
            current_usage: TokenCount::zero(),
            prefetch_queue: VecDeque::new(),
            eviction_history: VecDeque::with_capacity(100),
        })
    }

    fn parse_capacity(s: &str) -> Result<TokenCount> {
        // Simple parser for "10MB" -> token count
        // For now, assume 1 token ~= 4 bytes
        if s.ends_with("MB") {
            let mb: f32 = s.trim_end_matches("MB").parse()?;
            let bytes = mb * 1024.0 * 1024.0;
            let tokens = (bytes / 4.0) as u32;
            Ok(TokenCount::new(tokens))
        } else if s.ends_with("KB") {
            let kb: f32 = s.trim_end_matches("KB").parse()?;
            let bytes = kb * 1024.0;
            let tokens = (bytes / 4.0) as u32;
            Ok(TokenCount::new(tokens))
        } else {
            Ok(TokenCount::new(s.parse()?))
        }
    }

    /// Add a symbol with its token cost
    pub fn add_symbol(&mut self, symbol: SymbolId, token_cost: TokenCount) -> bool {
        // If already present, just update
        if self.active_symbols.contains(&symbol) {
            if let Some(metadata) = self.symbol_metadata.get_mut(&symbol) {
                metadata.update_access(0.1);
            }
            return true;
        }

        // Check if we have capacity
        let needed = token_cost.0;
        while self.current_usage.0 + needed > self.capacity.0 {
            if !self.evict_one() {
                return false; // Can't evict any more
            }
        }

        // Add symbol
        self.active_symbols.insert(symbol.clone());
        self.symbol_metadata.insert(symbol, SymbolMetadata::new(token_cost));
        self.current_usage.0 += needed;

        true
    }

    /// Update working memory based on attention pattern
    pub fn update(&mut self, attention: AttentionPattern) {
        // Update attention weights
        for (symbol, weight) in attention.focused_symbols {
            if let Some(metadata) = self.symbol_metadata.get_mut(&symbol) {
                metadata.update_access(weight);
            } else {
                // Symbol not in working memory - estimate cost and try to add
                let estimated_cost = TokenCount::new(100); // Default estimate
                self.add_symbol(symbol.clone(), estimated_cost);
                if let Some(metadata) = self.symbol_metadata.get_mut(&symbol) {
                    metadata.attention_weight = weight;
                }
            }
        }

        // Decay attention weights over time
        self.decay_attention_weights();

        // Add predicted symbols to prefetch queue
        for symbol in attention.predicted_next {
            if !self.active_symbols.contains(&symbol) && !self.prefetch_queue.contains(&symbol) {
                self.prefetch_queue.push_back(symbol);
            }
        }

        // Prefetch if we have capacity
        self.process_prefetch_queue();
    }

    /// Decay attention weights to implement forgetting
    fn decay_attention_weights(&mut self) {
        const DECAY_FACTOR: f32 = 0.95;
        for metadata in self.symbol_metadata.values_mut() {
            metadata.attention_weight *= DECAY_FACTOR;
        }
    }

    /// Process prefetch queue
    fn process_prefetch_queue(&mut self) {
        while let Some(symbol) = self.prefetch_queue.pop_front() {
            let estimated_cost = TokenCount::new(100);
            if !self.add_symbol(symbol, estimated_cost) {
                break; // No more capacity
            }
        }
    }

    /// Evict one symbol based on combined score
    fn evict_one(&mut self) -> bool {
        if self.active_symbols.is_empty() {
            return false;
        }

        // Find symbol with lowest score
        let mut min_score = f32::MAX;
        let mut to_evict = None;

        for symbol in &self.active_symbols {
            if let Some(metadata) = self.symbol_metadata.get(symbol) {
                let score = metadata.score();
                if score < min_score {
                    min_score = score;
                    to_evict = Some(symbol.clone());
                }
            }
        }

        if let Some(symbol) = to_evict {
            self.evict_symbol(&symbol);
            true
        } else {
            false
        }
    }

    /// Evict a specific symbol
    fn evict_symbol(&mut self, symbol: &SymbolId) {
        if let Some(metadata) = self.symbol_metadata.remove(symbol) {
            self.active_symbols.remove(symbol);
            self.current_usage.0 = self.current_usage.0.saturating_sub(metadata.token_cost.0);

            // Track eviction history
            self.eviction_history.push_back(symbol.clone());
            if self.eviction_history.len() > 100 {
                self.eviction_history.pop_front();
            }

            tracing::trace!(
                "Evicted symbol {} (score: {:.3})",
                symbol.0,
                metadata.score()
            );
        }
    }

    /// Get compact representation
    pub fn compact_representation(&self) -> WorkingContext {
        let mut symbols: Vec<_> = self.active_symbols.iter().cloned().collect();

        // Sort by attention weight (descending)
        symbols.sort_by(|a, b| {
            let weight_a = self
                .symbol_metadata
                .get(a)
                .map(|m| m.attention_weight)
                .unwrap_or(0.0);
            let weight_b = self
                .symbol_metadata
                .get(b)
                .map(|m| m.attention_weight)
                .unwrap_or(0.0);
            weight_b.partial_cmp(&weight_a).unwrap()
        });

        let attention_weights = self
            .symbol_metadata
            .iter()
            .map(|(id, meta)| (id.clone(), meta.attention_weight))
            .collect();

        WorkingContext {
            symbols,
            attention_weights,
            total_tokens: self.current_usage,
        }
    }

    /// Clear working memory
    pub fn clear(&mut self) {
        self.active_symbols.clear();
        self.symbol_metadata.clear();
        self.current_usage = TokenCount::zero();
        self.prefetch_queue.clear();
    }

    /// Get active symbols
    pub fn active_symbols(&self) -> &BTreeSet<SymbolId> {
        &self.active_symbols
    }

    /// Get current usage
    pub fn current_usage(&self) -> TokenCount {
        self.current_usage
    }

    /// Get capacity
    pub fn capacity(&self) -> TokenCount {
        self.capacity
    }

    /// Get eviction history
    pub fn eviction_history(&self) -> &VecDeque<SymbolId> {
        &self.eviction_history
    }

    /// Get attention weight for a symbol
    pub fn get_attention_weight(&self, symbol: &SymbolId) -> Option<f32> {
        self.symbol_metadata.get(symbol).map(|m| m.attention_weight)
    }

    /// Update token cost for a symbol
    pub fn update_token_cost(&mut self, symbol: &SymbolId, new_cost: TokenCount) {
        if let Some(metadata) = self.symbol_metadata.get_mut(symbol) {
            let old_cost = metadata.token_cost;
            metadata.token_cost = new_cost;

            // Update current usage
            self.current_usage.0 = self.current_usage.0.saturating_sub(old_cost.0) + new_cost.0;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> WorkingMemoryStats {
        WorkingMemoryStats {
            active_symbols: self.active_symbols.len(),
            current_usage: self.current_usage,
            capacity: self.capacity,
            utilization: self.current_usage.0 as f32 / self.capacity.0 as f32,
            prefetch_queue_size: self.prefetch_queue.len(),
            total_attention_weight: self
                .symbol_metadata
                .values()
                .map(|m| m.attention_weight)
                .sum(),
        }
    }

    /// Update attention weight for a symbol
    pub fn update_attention_weight(&mut self, symbol: &SymbolId, weight: f32) {
        if let Some(metadata) = self.symbol_metadata.get_mut(symbol) {
            metadata.update_access(weight);
        } else {
            // Symbol not in working memory - estimate cost and try to add
            let estimated_cost = TokenCount::new(100);
            self.add_symbol(symbol.clone(), estimated_cost);
            if let Some(metadata) = self.symbol_metadata.get_mut(symbol) {
                metadata.attention_weight = weight;
            }
        }
    }

    /// Evict symbols if needed based on capacity
    pub fn evict_if_needed(&mut self) -> Result<()> {
        while self.current_usage.0 > self.capacity.0 {
            if !self.evict_one() {
                return Err(anyhow::anyhow!("Failed to evict enough symbols"));
            }
        }
        Ok(())
    }

    /// Get count of active symbols
    pub fn get_active_count(&self) -> usize {
        self.active_symbols.len()
    }

    /// Estimate total tokens for active symbols
    pub fn estimate_tokens(&self) -> TokenCount {
        self.current_usage
    }
}

/// Statistics about working memory
#[derive(Debug, Clone)]
pub struct WorkingMemoryStats {
    pub active_symbols: usize,
    pub current_usage: TokenCount,
    pub capacity: TokenCount,
    pub utilization: f32,
    pub prefetch_queue_size: usize,
    pub total_attention_weight: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_capacity() {
        let wm = WorkingMemory::new("10MB".to_string()).unwrap();
        assert!(wm.capacity.0 > 0);

        let wm2 = WorkingMemory::new("500KB".to_string()).unwrap();
        assert!(wm2.capacity.0 > 0);
    }

    #[test]
    fn test_add_symbol() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();
        let symbol = SymbolId::new("test_symbol");

        assert!(wm.add_symbol(symbol.clone(), TokenCount::new(100)));
        assert!(wm.active_symbols.contains(&symbol));
        assert_eq!(wm.current_usage().0, 100);
    }

    #[test]
    fn test_eviction() {
        let mut wm = WorkingMemory::new("300".to_string()).unwrap();

        // Add symbols until capacity
        let sym1 = SymbolId::new("sym1");
        let sym2 = SymbolId::new("sym2");
        let sym3 = SymbolId::new("sym3");

        wm.add_symbol(sym1.clone(), TokenCount::new(100));
        wm.add_symbol(sym2.clone(), TokenCount::new(100));
        wm.add_symbol(sym3.clone(), TokenCount::new(100));

        assert_eq!(wm.active_symbols.len(), 3);

        // Add another symbol - should trigger eviction
        let sym4 = SymbolId::new("sym4");
        wm.add_symbol(sym4.clone(), TokenCount::new(100));

        // Should have evicted one symbol
        assert_eq!(wm.active_symbols.len(), 3);
        assert!(wm.active_symbols.contains(&sym4));
    }

    #[test]
    fn test_attention_update() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();
        let symbol = SymbolId::new("test_symbol");

        wm.add_symbol(symbol.clone(), TokenCount::new(100));

        let mut focused = HashMap::new();
        focused.insert(symbol.clone(), 0.8);

        let attention = AttentionPattern {
            focused_symbols: focused,
            predicted_next: vec![],
        };

        wm.update(attention);

        let weight = wm.get_attention_weight(&symbol);
        assert!(weight.is_some());
        assert!(weight.unwrap() > 0.0);
    }

    #[test]
    fn test_prefetch_queue() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();

        let symbol1 = SymbolId::new("existing");
        wm.add_symbol(symbol1.clone(), TokenCount::new(100));

        let symbol2 = SymbolId::new("predicted");

        let attention = AttentionPattern {
            focused_symbols: HashMap::new(),
            predicted_next: vec![symbol2.clone()],
        };

        wm.update(attention);

        // Should have prefetched the predicted symbol
        assert!(wm.active_symbols.contains(&symbol2) || wm.prefetch_queue.contains(&symbol2));
    }

    #[test]
    fn test_compact_representation() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();

        let sym1 = SymbolId::new("sym1");
        let sym2 = SymbolId::new("sym2");

        wm.add_symbol(sym1.clone(), TokenCount::new(100));
        wm.add_symbol(sym2.clone(), TokenCount::new(150));

        // Update with different attention weights
        let mut focused = HashMap::new();
        focused.insert(sym1.clone(), 0.5);
        focused.insert(sym2.clone(), 0.9);

        wm.update(AttentionPattern {
            focused_symbols: focused,
            predicted_next: vec![],
        });

        let context = wm.compact_representation();
        assert_eq!(context.symbols.len(), 2);
        // sym2 should be first (higher attention)
        assert_eq!(context.symbols[0].0, "sym2");
    }

    #[test]
    fn test_clear() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();
        wm.add_symbol(SymbolId::new("test"), TokenCount::new(100));

        assert!(!wm.active_symbols.is_empty());

        wm.clear();

        assert!(wm.active_symbols.is_empty());
        assert_eq!(wm.current_usage().0, 0);
    }

    #[test]
    fn test_stats() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();
        wm.add_symbol(SymbolId::new("test"), TokenCount::new(200));

        let stats = wm.stats();
        assert_eq!(stats.active_symbols, 1);
        assert_eq!(stats.current_usage.0, 200);
        assert_eq!(stats.capacity.0, 1000);
        assert_eq!(stats.utilization, 0.2);
    }

    #[test]
    fn test_attention_decay() {
        let mut wm = WorkingMemory::new("1000".to_string()).unwrap();
        let symbol = SymbolId::new("test");

        wm.add_symbol(symbol.clone(), TokenCount::new(100));

        // Set initial attention
        let mut focused = HashMap::new();
        focused.insert(symbol.clone(), 1.0);
        wm.update(AttentionPattern {
            focused_symbols: focused,
            predicted_next: vec![],
        });

        let initial_weight = wm.get_attention_weight(&symbol).unwrap();

        // Update again without focusing - should decay
        wm.update(AttentionPattern {
            focused_symbols: HashMap::new(),
            predicted_next: vec![],
        });

        let decayed_weight = wm.get_attention_weight(&symbol).unwrap();
        assert!(decayed_weight < initial_weight);
    }
}
