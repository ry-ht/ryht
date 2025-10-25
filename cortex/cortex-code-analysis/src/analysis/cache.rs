//! High-Performance Caching Module for AST Operations
//!
//! This module provides efficient caching strategies for:
//! - Parsed AST trees
//! - Computed metrics
//! - Search results
//! - Transformation results
//!
//! Uses LRU (Least Recently Used) eviction policy with configurable size limits.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::analysis::cache::{AstCache, MetricsCache};
//!
//! // Create a cache with capacity for 100 entries
//! let cache = AstCache::new(100);
//!
//! // Cache is thread-safe and can be shared
//! let cache = std::sync::Arc::new(cache);
//! ```

use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

/// Compute a hash for a given value
fn compute_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Generic LRU cache with thread-safe access
#[derive(Clone)]
pub struct Cache<K: Hash + Eq, V: Clone> {
    cache: Arc<Mutex<LruCache<K, V>>>,
}

impl<K: Hash + Eq, V: Clone> Cache<K, V> {
    /// Create a new cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    /// Put a value into the cache
    pub fn put(&self, key: K, value: V) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, value);
    }

    /// Check if the cache contains a key
    pub fn contains(&self, key: &K) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.contains(key)
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.lock().unwrap();
        cache.pop(key)
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get the current size of the cache
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.is_empty()
    }

    /// Get or insert a value using a closure
    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        let mut cache = self.cache.lock().unwrap();
        if let Some(value) = cache.get(&key) {
            return value.clone();
        }
        let value = f();
        cache.put(key, value.clone());
        value
    }
}

/// Cache key for source code
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SourceKey {
    /// Hash of the source code
    pub content_hash: u64,
    /// Language of the source
    pub language: String,
}

impl SourceKey {
    /// Create a new source key
    pub fn new(content: &[u8], language: &str) -> Self {
        Self {
            content_hash: compute_hash(&content),
            language: language.to_string(),
        }
    }
}

/// Cached parsed AST result
#[derive(Debug, Clone)]
pub struct CachedAst {
    /// Serialized AST representation
    pub ast_data: Vec<u8>,
    /// Timestamp when cached
    pub timestamp: std::time::SystemTime,
}

/// Cache specifically for parsed ASTs
pub type AstCache = Cache<SourceKey, CachedAst>;

/// Cached metrics result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedMetrics {
    /// JSON-serialized metrics
    pub metrics_json: String,
    /// Timestamp when cached
    pub timestamp: std::time::SystemTime,
}

/// Cache specifically for computed metrics
pub type MetricsCache = Cache<SourceKey, CachedMetrics>;

/// Cache key for search operations
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SearchKey {
    /// Source content hash
    pub content_hash: u64,
    /// Search query hash
    pub query_hash: u64,
}

impl SearchKey {
    /// Create a new search key
    pub fn new(content: &[u8], query: &str) -> Self {
        Self {
            content_hash: compute_hash(&content),
            query_hash: compute_hash(&query),
        }
    }
}

/// Cached search results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedSearch {
    /// Number of results found
    pub count: usize,
    /// Serialized result data
    pub results_json: String,
}

/// Cache specifically for search results
pub type SearchCache = Cache<SearchKey, CachedSearch>;

/// Multi-level cache manager
///
/// Manages multiple caches for different operation types.
#[derive(Clone)]
pub struct CacheManager {
    /// Cache for parsed ASTs
    pub ast_cache: AstCache,
    /// Cache for computed metrics
    pub metrics_cache: MetricsCache,
    /// Cache for search results
    pub search_cache: SearchCache,
}

impl CacheManager {
    /// Create a new cache manager with default capacities
    pub fn new() -> Self {
        Self {
            ast_cache: AstCache::new(50),
            metrics_cache: MetricsCache::new(100),
            search_cache: SearchCache::new(200),
        }
    }

    /// Create a cache manager with custom capacities
    pub fn with_capacities(ast_cap: usize, metrics_cap: usize, search_cap: usize) -> Self {
        Self {
            ast_cache: AstCache::new(ast_cap),
            metrics_cache: MetricsCache::new(metrics_cap),
            search_cache: SearchCache::new(search_cap),
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.ast_cache.clear();
        self.metrics_cache.clear();
        self.search_cache.clear();
    }

    /// Get statistics about cache usage
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            ast_entries: self.ast_cache.len(),
            metrics_entries: self.metrics_cache.len(),
            search_entries: self.search_cache.len(),
        }
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about cache usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Number of AST entries cached
    pub ast_entries: usize,
    /// Number of metrics entries cached
    pub metrics_entries: usize,
    /// Number of search entries cached
    pub search_entries: usize,
}

impl CacheStats {
    /// Get total number of cached entries
    pub fn total_entries(&self) -> usize {
        self.ast_entries + self.metrics_entries + self.search_entries
    }
}

/// Builder for cache configuration
#[derive(Debug, Clone)]
pub struct CacheBuilder {
    ast_capacity: usize,
    metrics_capacity: usize,
    search_capacity: usize,
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self {
            ast_capacity: 50,
            metrics_capacity: 100,
            search_capacity: 200,
        }
    }
}

impl CacheBuilder {
    /// Create a new cache builder with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set AST cache capacity
    pub fn ast_capacity(mut self, capacity: usize) -> Self {
        self.ast_capacity = capacity;
        self
    }

    /// Set metrics cache capacity
    pub fn metrics_capacity(mut self, capacity: usize) -> Self {
        self.metrics_capacity = capacity;
        self
    }

    /// Set search cache capacity
    pub fn search_capacity(mut self, capacity: usize) -> Self {
        self.search_capacity = capacity;
        self
    }

    /// Build the cache manager
    pub fn build(self) -> CacheManager {
        CacheManager::with_capacities(
            self.ast_capacity,
            self.metrics_capacity,
            self.search_capacity,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache: Cache<String, i32> = Cache::new(10);

        cache.put("key1".to_string(), 42);
        assert_eq!(cache.get(&"key1".to_string()), Some(42));
        assert_eq!(cache.len(), 1);

        cache.put("key2".to_string(), 100);
        assert_eq!(cache.len(), 2);

        cache.remove(&"key1".to_string());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"key1".to_string()), None);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache: Cache<String, i32> = Cache::new(2);

        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.put("key3".to_string(), 3); // Should evict key1

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(2));
        assert_eq!(cache.get(&"key3".to_string()), Some(3));
    }

    #[test]
    fn test_get_or_insert_with() {
        let cache: Cache<String, i32> = Cache::new(10);

        let value = cache.get_or_insert_with("key".to_string(), || 42);
        assert_eq!(value, 42);

        let value = cache.get_or_insert_with("key".to_string(), || 100);
        assert_eq!(value, 42); // Should return cached value
    }

    #[test]
    fn test_source_key() {
        let content1 = b"fn main() {}";
        let content2 = b"fn main() {}";
        let content3 = b"fn test() {}";

        let key1 = SourceKey::new(content1, "rust");
        let key2 = SourceKey::new(content2, "rust");
        let key3 = SourceKey::new(content3, "rust");

        assert_eq!(key1, key2); // Same content
        assert_ne!(key1, key3); // Different content
    }

    #[test]
    fn test_cache_manager() {
        let manager = CacheManager::new();

        assert_eq!(manager.stats().total_entries(), 0);

        let key = SourceKey::new(b"test", "rust");
        manager.metrics_cache.put(
            key,
            CachedMetrics {
                metrics_json: "{}".to_string(),
                timestamp: std::time::SystemTime::now(),
            },
        );

        assert_eq!(manager.stats().metrics_entries, 1);

        manager.clear_all();
        assert_eq!(manager.stats().total_entries(), 0);
    }

    #[test]
    fn test_cache_builder() {
        let manager = CacheBuilder::new()
            .ast_capacity(25)
            .metrics_capacity(50)
            .search_capacity(100)
            .build();

        // Caches should be created with specified capacities
        assert_eq!(manager.ast_cache.len(), 0);
        assert_eq!(manager.metrics_cache.len(), 0);
        assert_eq!(manager.search_cache.len(), 0);
    }

    #[test]
    fn test_search_key() {
        let content = b"fn main() {}";
        let query1 = "function";
        let query2 = "function";
        let query3 = "variable";

        let key1 = SearchKey::new(content, query1);
        let key2 = SearchKey::new(content, query2);
        let key3 = SearchKey::new(content, query3);

        assert_eq!(key1, key2); // Same query
        assert_ne!(key1, key3); // Different query
    }
}
