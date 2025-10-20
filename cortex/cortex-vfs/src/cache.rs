//! Caching layer for file system operations.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache entry with expiration
#[derive(Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

/// LRU cache with TTL support
pub struct Cache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    store: Arc<DashMap<K, CacheEntry<V>>>,
    ttl: Duration,
}

impl<K, V> Cache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    /// Create a new cache with TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            ttl,
        }
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };
        self.store.insert(key, entry);
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let entry = self.store.get(key)?;

        if entry.expires_at < Instant::now() {
            drop(entry);
            self.store.remove(key);
            return None;
        }

        Some(entry.value.clone())
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) {
        self.store.remove(key);
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        self.store.clear();
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Evict expired entries
    pub fn evict_expired(&self) {
        let now = Instant::now();
        self.store.retain(|_, entry| entry.expires_at >= now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_basic() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.insert("key1", "value1");

        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = Cache::new(Duration::from_millis(100));
        cache.insert("key1", "value1");

        assert_eq!(cache.get(&"key1"), Some("value1"));

        thread::sleep(Duration::from_millis(150));
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");

        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
