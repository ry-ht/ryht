//! Context optimization for token efficiency

use super::*;

#[derive(Debug, Clone)]
pub struct OptimizedContext {
    pub content: String,
    pub token_count: usize,
    pub optimization_ratio: f32,
}

pub struct ContextOptimizer {
    cache: Arc<RwLock<HashMap<u64, OptimizedContext>>>,
}

impl ContextOptimizer {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn optimize(&self, content: String, target_tokens: usize) -> Result<OptimizedContext> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Calculate hash for caching
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        // Check cache
        if let Some(cached) = self.cache.read().await.get(&hash) {
            return Ok(cached.clone());
        }

        // Simple optimization: truncate and remove whitespace
        let original_len = content.len();
        let optimized = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let optimized_len = optimized.len();
        let estimated_tokens = optimized_len / 4; // Rough estimate

        let result = OptimizedContext {
            content: optimized,
            token_count: estimated_tokens,
            optimization_ratio: optimized_len as f32 / original_len as f32,
        };

        // Cache result
        self.cache.write().await.insert(hash, result.clone());

        Ok(result)
    }
}

impl Default for ContextOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
