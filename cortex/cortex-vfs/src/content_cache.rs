//! Content caching with LRU eviction and TTL support.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Content cache with LRU eviction policy.
///
/// Provides thread-safe caching of file content with:
/// - Automatic eviction based on size limits
/// - LRU (Least Recently Used) eviction policy
/// - TTL (Time To Live) support
/// - Reference counting for shared content
pub struct ContentCache {
    /// Cached entries
    entries: Arc<DashMap<String, CacheEntry>>,

    /// LRU queue for eviction
    lru_queue: Arc<RwLock<VecDeque<String>>>,

    /// Current cache size in bytes
    size_bytes: Arc<AtomicUsize>,

    /// Maximum cache size in bytes
    max_size: usize,

    /// Time-to-live for entries
    ttl: Option<Duration>,

    /// Cache statistics
    stats: CacheStats,
}

impl ContentCache {
    /// Create a new content cache with the given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
            size_bytes: Arc::new(AtomicUsize::new(0)),
            max_size,
            ttl: None,
            stats: CacheStats::new(),
        }
    }

    /// Create a cache with TTL support.
    pub fn with_ttl(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
            size_bytes: Arc::new(AtomicUsize::new(0)),
            max_size,
            ttl: Some(ttl),
            stats: CacheStats::new(),
        }
    }

    /// Get content from cache.
    pub fn get(&self, hash: &str) -> Option<Arc<Vec<u8>>> {
        // Check if entry exists
        if let Some(mut entry) = self.entries.get_mut(hash) {
            // Check TTL
            if let Some(ttl) = self.ttl {
                if entry.created_at.elapsed() > ttl {
                    // Entry expired
                    drop(entry);
                    self.remove(hash);
                    self.stats.record_miss();
                    return None;
                }
            }

            // Update access time and count
            entry.last_accessed = Instant::now();
            entry.access_count += 1;

            // Update LRU queue
            self.promote_in_lru(hash);

            self.stats.record_hit();
            Some(entry.content.clone())
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// Put content into cache.
    pub fn put(&self, hash: String, content: Vec<u8>) -> Arc<Vec<u8>> {
        let size = content.len();
        let arc_content = Arc::new(content);

        // Make room if needed
        self.evict_if_needed(size);

        // Create new entry
        let entry = CacheEntry {
            content: arc_content.clone(),
            size,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
        };

        // Insert into cache
        self.entries.insert(hash.clone(), entry);
        self.size_bytes.fetch_add(size, Ordering::Relaxed);

        // Add to LRU queue
        self.lru_queue.write().push_back(hash.clone());

        self.stats.record_put();

        arc_content
    }

    /// Remove content from cache.
    pub fn remove(&self, hash: &str) {
        if let Some((_, entry)) = self.entries.remove(hash) {
            self.size_bytes.fetch_sub(entry.size, Ordering::Relaxed);

            // Remove from LRU queue
            let mut queue = self.lru_queue.write();
            if let Some(pos) = queue.iter().position(|k| k == hash) {
                queue.remove(pos);
            }

            self.stats.record_eviction();
        }
    }

    /// Clear all entries from cache.
    pub fn clear(&self) {
        let count = self.entries.len();
        self.entries.clear();
        self.lru_queue.write().clear();
        self.size_bytes.store(0, Ordering::Relaxed);
        self.stats.evictions.fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Get current cache size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.size_bytes.load(Ordering::Relaxed)
    }

    /// Get number of entries in cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStatistics {
        self.stats.snapshot()
    }

    /// Evict entries if needed to make room for new content.
    fn evict_if_needed(&self, needed_size: usize) {
        let mut current_size = self.size_bytes.load(Ordering::Relaxed);

        // Evict until we have enough space
        while current_size + needed_size > self.max_size {
            if let Some(hash) = self.lru_queue.write().pop_front() {
                self.remove(&hash);
                current_size = self.size_bytes.load(Ordering::Relaxed);
            } else {
                // Nothing left to evict
                break;
            }
        }
    }

    /// Promote an entry in the LRU queue (mark as recently used).
    fn promote_in_lru(&self, hash: &str) {
        let mut queue = self.lru_queue.write();

        // Find and remove from current position
        if let Some(pos) = queue.iter().position(|k| k == hash) {
            queue.remove(pos);
        }

        // Add to back (most recently used)
        queue.push_back(hash.to_string());
    }

    /// Remove expired entries.
    pub fn cleanup_expired(&self) {
        if let Some(ttl) = self.ttl {
            let now = Instant::now();
            let mut expired = Vec::new();

            // Find expired entries
            for entry in self.entries.iter() {
                if now.duration_since(entry.created_at) > ttl {
                    expired.push(entry.key().clone());
                }
            }

            // Remove them
            for hash in expired {
                self.remove(&hash);
            }
        }
    }
}

impl Clone for ContentCache {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            lru_queue: Arc::clone(&self.lru_queue),
            size_bytes: Arc::clone(&self.size_bytes),
            max_size: self.max_size,
            ttl: self.ttl,
            stats: self.stats.clone(),
        }
    }
}

/// Cache entry with metadata.
struct CacheEntry {
    /// Cached content
    content: Arc<Vec<u8>>,

    /// Size in bytes
    size: usize,

    /// When entry was created
    created_at: Instant,

    /// When entry was last accessed
    last_accessed: Instant,

    /// Number of times accessed
    access_count: u64,
}

/// Cache statistics.
#[derive(Clone)]
struct CacheStats {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    puts: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            puts: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
        }
    }

    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_put(&self) {
        self.puts.fetch_add(1, Ordering::Relaxed);
    }

    fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> CacheStatistics {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheStatistics {
            hits,
            misses,
            puts: self.puts.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            hit_rate,
        }
    }
}

/// Snapshot of cache statistics.
#[derive(Debug, Clone, Copy)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub puts: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_basic_operations() {
        let cache = ContentCache::new(1024 * 1024); // 1MB

        // Put content
        let content = b"hello world".to_vec();
        let hash = "test_hash".to_string();
        cache.put(hash.clone(), content.clone());

        // Get content
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(&**retrieved, &content);

        // Statistics
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.puts, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ContentCache::new(1024 * 1024);

        let result = cache.get("nonexistent");
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = ContentCache::new(20); // Very small cache

        // Fill cache
        cache.put("key1".to_string(), vec![1, 2, 3, 4, 5]);
        cache.put("key2".to_string(), vec![6, 7, 8, 9, 10]);

        // This should evict key1
        cache.put("key3".to_string(), vec![11, 12, 13, 14, 15]);

        // key1 should be evicted
        assert!(cache.get("key1").is_none());

        // key2 and key3 should still be there
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_lru_order() {
        let cache = ContentCache::new(30);

        cache.put("key1".to_string(), vec![1; 10]);
        cache.put("key2".to_string(), vec![2; 10]);

        // Access key1 to make it more recent
        cache.get("key1");

        // Add key3, should evict key2 (least recently used)
        cache.put("key3".to_string(), vec![3; 10]);

        assert!(cache.get("key1").is_some());
        assert!(cache.get("key2").is_none()); // Evicted
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_cache_clear() {
        let cache = ContentCache::new(1024);

        cache.put("key1".to_string(), vec![1, 2, 3]);
        cache.put("key2".to_string(), vec![4, 5, 6]);

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
    }

    #[test]
    fn test_cache_ttl() {
        let cache = ContentCache::with_ttl(1024, Duration::from_millis(50));

        cache.put("key1".to_string(), vec![1, 2, 3]);

        // Should be available immediately
        assert!(cache.get("key1").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(100));

        // Should be expired
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_concurrent_access() {
        let cache = Arc::new(ContentCache::new(1024 * 1024));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let key = format!("key{}", i);
                let value = vec![i as u8; 100];
                cache_clone.put(key.clone(), value);
                cache_clone.get(&key)
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.join().unwrap().is_some());
        }
    }

    #[test]
    fn test_hit_rate_calculation() {
        let cache = ContentCache::new(1024);

        cache.put("key1".to_string(), vec![1, 2, 3]);

        cache.get("key1"); // Hit
        cache.get("key1"); // Hit
        cache.get("key2"); // Miss
        cache.get("key3"); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.hit_rate, 0.5);
    }
}
