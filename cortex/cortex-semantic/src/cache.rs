//! Caching layer for embeddings and search results.

use crate::types::Vector;
use moka::future::Cache;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

/// Cache key for embeddings.
#[derive(Clone, Eq, PartialEq)]
pub struct EmbeddingCacheKey {
    text: String,
    model: String,
}

impl Hash for EmbeddingCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.model.hash(state);
    }
}

impl EmbeddingCacheKey {
    pub fn new(text: String, model: String) -> Self {
        Self { text, model }
    }
}

/// Cache for embeddings.
pub struct EmbeddingCache {
    cache: Cache<EmbeddingCacheKey, Arc<Vector>>,
}

impl EmbeddingCache {
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { cache }
    }

    pub async fn get(&self, key: &EmbeddingCacheKey) -> Option<Arc<Vector>> {
        self.cache.get(key).await
    }

    pub async fn insert(&self, key: EmbeddingCacheKey, value: Vector) {
        self.cache.insert(key, Arc::new(value)).await;
    }

    pub async fn invalidate(&self, key: &EmbeddingCacheKey) {
        self.cache.invalidate(key).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
    }

    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

/// Cache key for search queries.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct QueryCacheKey {
    query: String,
    limit: usize,
    threshold: String, // Store as string for hashing
}

impl QueryCacheKey {
    pub fn new(query: String, limit: usize, threshold: f32) -> Self {
        Self {
            query,
            limit,
            threshold: format!("{:.6}", threshold),
        }
    }
}

/// Cached search result.
#[derive(Clone)]
pub struct CachedSearchResult {
    pub doc_ids: Vec<String>,
    pub scores: Vec<f32>,
}

/// Cache for search results.
pub struct QueryCache {
    cache: Cache<QueryCacheKey, Arc<CachedSearchResult>>,
}

impl QueryCache {
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { cache }
    }

    pub async fn get(&self, key: &QueryCacheKey) -> Option<Arc<CachedSearchResult>> {
        self.cache.get(key).await
    }

    pub async fn insert(&self, key: QueryCacheKey, value: CachedSearchResult) {
        self.cache.insert(key, Arc::new(value)).await;
    }

    pub async fn invalidate(&self, key: &QueryCacheKey) {
        self.cache.invalidate(key).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
    }

    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_cache() {
        let cache = EmbeddingCache::new(100, Duration::from_secs(60));

        let key = EmbeddingCacheKey::new("test".to_string(), "model".to_string());
        let vector = vec![1.0, 2.0, 3.0];

        // Insert
        cache.insert(key.clone(), vector.clone()).await;

        // Get
        let cached = cache.get(&key).await.unwrap();
        assert_eq!(*cached, vector);

        // Invalidate
        cache.invalidate(&key).await;
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_query_cache() {
        let cache = QueryCache::new(100, Duration::from_secs(60));

        let key = QueryCacheKey::new("test query".to_string(), 10, 0.5);
        let result = CachedSearchResult {
            doc_ids: vec!["doc1".to_string(), "doc2".to_string()],
            scores: vec![0.9, 0.8],
        };

        // Insert
        cache.insert(key.clone(), result.clone()).await;

        // Get
        let cached = cache.get(&key).await.unwrap();
        assert_eq!(cached.doc_ids, result.doc_ids);
        assert_eq!(cached.scores, result.scores);

        // Clear
        cache.clear().await;
        assert!(cache.get(&key).await.is_none());
    }
}
