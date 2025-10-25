//! File Caching and Memory Pools for Concurrent Processing
//!
//! Provides efficient caching mechanisms for repeated file analysis:
//! - LRU cache for parsed results
//! - Content-based caching with hash keys
//! - Memory pools for parser allocation
//! - Cache statistics and metrics
//! - Thread-safe concurrent access
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::concurrent::file_cache::{FileCache, CacheConfig};
//! use std::path::PathBuf;
//!
//! let cache = FileCache::<String>::new(CacheConfig::default());
//! let path = PathBuf::from("example.rs");
//! cache.insert(path.clone(), "cached data".to_string());
//!
//! if let Some(data) = cache.get(&path) {
//!     println!("Cache hit: {}", data);
//! }
//! ```

use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Configuration for file cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in cache
    pub max_entries: usize,

    /// Enable content-based hashing (slower but more accurate)
    pub content_hash: bool,

    /// Cache entry TTL (time to live) in seconds (0 = infinite)
    pub ttl_seconds: u64,

    /// Enable cache statistics
    pub enable_stats: bool,

    /// Preload capacity (number of entries to pre-allocate)
    pub preload_capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            content_hash: false,
            ttl_seconds: 0,
            enable_stats: true,
            preload_capacity: 100,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,

    /// Total cache misses
    pub misses: u64,

    /// Total insertions
    pub insertions: u64,

    /// Total evictions
    pub evictions: u64,

    /// Current size
    pub current_size: usize,

    /// Peak size
    pub peak_size: usize,

    /// Total bytes cached (estimated)
    pub bytes_cached: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }
}

/// Cached entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    inserted_at: SystemTime,
    last_accessed: SystemTime,
    access_count: usize,
    size_bytes: usize,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T, size_bytes: usize) -> Self {
        let now = SystemTime::now();
        Self {
            value,
            inserted_at: now,
            last_accessed: now,
            access_count: 0,
            size_bytes,
        }
    }

    fn access(&mut self) -> T {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
        self.value.clone()
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        if ttl.is_zero() {
            return false;
        }

        SystemTime::now()
            .duration_since(self.inserted_at)
            .map(|age| age > ttl)
            .unwrap_or(false)
    }
}

/// LRU-based file cache
pub struct FileCache<T: Clone + Send + Sync> {
    cache: Arc<Mutex<LruCache<PathBuf, CacheEntry<T>>>>,
    config: CacheConfig,
    stats: Arc<Mutex<CacheStats>>,
    hits: AtomicU64,
    misses: AtomicU64,
    insertions: AtomicU64,
}

impl<T: Clone + Send + Sync> FileCache<T> {
    /// Create a new file cache
    pub fn new(config: CacheConfig) -> Self {
        let capacity = NonZeroUsize::new(config.max_entries).unwrap_or(NonZeroUsize::new(1000).unwrap());

        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            config,
            stats: Arc::new(Mutex::new(CacheStats::new())),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            insertions: AtomicU64::new(0),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Create with specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let mut config = CacheConfig::default();
        config.max_entries = capacity;
        Self::new(config)
    }

    /// Get value from cache
    pub fn get(&self, path: &Path) -> Option<T> {
        let mut cache = self.cache.lock();

        if let Some(entry) = cache.get_mut(&path.to_path_buf()) {
            // Check if expired
            let ttl = Duration::from_secs(self.config.ttl_seconds);
            if entry.is_expired(ttl) {
                cache.pop(&path.to_path_buf());
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.access())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert value into cache
    pub fn insert(&self, path: PathBuf, value: T) {
        self.insert_with_size(path, value, 0)
    }

    /// Insert with estimated size
    pub fn insert_with_size(&self, path: PathBuf, value: T, size_bytes: usize) {
        let entry = CacheEntry::new(value, size_bytes);
        let mut cache = self.cache.lock();

        // Track evictions
        if cache.len() >= cache.cap().get() {
            if self.config.enable_stats {
                let mut stats = self.stats.lock();
                stats.evictions += 1;
            }
        }

        cache.put(path, entry);
        self.insertions.fetch_add(1, Ordering::Relaxed);

        if self.config.enable_stats {
            let mut stats = self.stats.lock();
            stats.insertions += 1;
            stats.current_size = cache.len();
            stats.peak_size = stats.peak_size.max(cache.len());
            stats.bytes_cached += size_bytes as u64;
        }
    }

    /// Check if path exists in cache
    pub fn contains(&self, path: &Path) -> bool {
        let cache = self.cache.lock();
        cache.contains(&path.to_path_buf())
    }

    /// Remove entry from cache
    pub fn remove(&self, path: &Path) -> Option<T> {
        let mut cache = self.cache.lock();
        cache.pop(&path.to_path_buf()).map(|entry| entry.value)
    }

    /// Clear entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();

        if self.config.enable_stats {
            let mut stats = self.stats.lock();
            stats.current_size = 0;
        }
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.lock().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.lock().is_empty()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if self.config.enable_stats {
            let mut stats = self.stats.lock().clone();
            stats.hits = self.hits.load(Ordering::Relaxed);
            stats.misses = self.misses.load(Ordering::Relaxed);
            stats
        } else {
            CacheStats {
                hits: self.hits.load(Ordering::Relaxed),
                misses: self.misses.load(Ordering::Relaxed),
                insertions: self.insertions.load(Ordering::Relaxed),
                ..Default::default()
            }
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.insertions.store(0, Ordering::Relaxed);

        if self.config.enable_stats {
            *self.stats.lock() = CacheStats::new();
        }
    }
}

/// Content-hash based cache (concurrent)
pub struct ContentHashCache<T: Clone + Send + Sync> {
    cache: DashMap<u64, CacheEntry<T>>,
    config: CacheConfig,
    hits: AtomicU64,
    misses: AtomicU64,
    size: AtomicUsize,
}

impl<T: Clone + Send + Sync> ContentHashCache<T> {
    /// Create a new content hash cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: DashMap::with_capacity(config.preload_capacity),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            size: AtomicUsize::new(0),
        }
    }

    /// Create with defaults
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Hash content to create cache key
    pub fn hash_content(content: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash file path to create cache key
    pub fn hash_path(path: &Path) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    }

    /// Get value by content hash
    pub fn get(&self, hash: u64) -> Option<T> {
        if let Some(mut entry) = self.cache.get_mut(&hash) {
            // Check expiration
            let ttl = Duration::from_secs(self.config.ttl_seconds);
            if entry.is_expired(ttl) {
                drop(entry);
                self.cache.remove(&hash);
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.access())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert value with hash key
    pub fn insert(&self, hash: u64, value: T, size_bytes: usize) {
        let entry = CacheEntry::new(value, size_bytes);

        // Check size limit
        let current_size = self.size.load(Ordering::Relaxed);
        if current_size >= self.config.max_entries {
            // Evict random entry (DashMap doesn't have LRU built-in)
            if let Some(first) = self.cache.iter().next() {
                let key = *first.key();
                drop(first);
                self.cache.remove(&key);
                self.size.fetch_sub(1, Ordering::Relaxed);
            }
        }

        self.cache.insert(hash, entry);
        self.size.fetch_add(1, Ordering::Relaxed);
    }

    /// Get value by file content
    pub fn get_by_content(&self, content: &[u8]) -> Option<T> {
        let hash = Self::hash_content(content);
        self.get(hash)
    }

    /// Insert value with file content
    pub fn insert_by_content(&self, content: &[u8], value: T, size_bytes: usize) {
        let hash = Self::hash_content(content);
        self.insert(hash, value, size_bytes);
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear cache
    pub fn clear(&self) {
        self.cache.clear();
        self.size.store(0, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            current_size: self.size.load(Ordering::Relaxed),
            ..Default::default()
        }
    }
}

/// Multi-level cache combining path and content caching
pub struct MultiLevelCache<T: Clone + Send + Sync> {
    path_cache: FileCache<T>,
    content_cache: ContentHashCache<T>,
    config: CacheConfig,
}

impl<T: Clone + Send + Sync> MultiLevelCache<T> {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            path_cache: FileCache::new(config.clone()),
            content_cache: ContentHashCache::new(config.clone()),
            config,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get from cache, checking path first, then content
    pub fn get(&self, path: &Path, content: Option<&[u8]>) -> Option<T> {
        // Try path cache first (faster)
        if let Some(value) = self.path_cache.get(path) {
            return Some(value);
        }

        // Try content cache if content provided
        if let Some(content) = content {
            if let Some(value) = self.content_cache.get_by_content(content) {
                // Store in path cache for faster future access
                self.path_cache.insert(path.to_path_buf(), value.clone());
                return Some(value);
            }
        }

        None
    }

    /// Insert into both caches
    pub fn insert(&self, path: PathBuf, content: Option<&[u8]>, value: T, size_bytes: usize) {
        self.path_cache.insert_with_size(path, value.clone(), size_bytes);

        if let Some(content) = content {
            self.content_cache.insert_by_content(content, value, size_bytes);
        }
    }

    /// Clear both caches
    pub fn clear(&self) {
        self.path_cache.clear();
        self.content_cache.clear();
    }

    /// Get combined statistics
    pub fn stats(&self) -> (CacheStats, CacheStats) {
        (self.path_cache.stats(), self.content_cache.stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_cache_basic() {
        let cache = FileCache::<String>::with_capacity(10);
        let path = PathBuf::from("test.rs");

        cache.insert(path.clone(), "test data".to_string());
        assert!(cache.contains(&path));

        let value = cache.get(&path);
        assert_eq!(value, Some("test data".to_string()));
    }

    #[test]
    fn test_cache_stats() {
        let cache = FileCache::<i32>::with_capacity(10);
        let path = PathBuf::from("test.rs");

        cache.insert(path.clone(), 42);
        cache.get(&path); // Hit
        cache.get(&PathBuf::from("missing.rs")); // Miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate() > 0.0);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = FileCache::<i32>::with_capacity(2);

        cache.insert(PathBuf::from("file1.rs"), 1);
        cache.insert(PathBuf::from("file2.rs"), 2);
        cache.insert(PathBuf::from("file3.rs"), 3); // Should evict file1

        assert_eq!(cache.len(), 2);
        assert!(cache.get(&PathBuf::from("file1.rs")).is_none());
    }

    #[test]
    fn test_content_hash_cache() {
        let cache = ContentHashCache::<String>::with_defaults();
        let content = b"fn main() {}";

        let hash = ContentHashCache::<String>::hash_content(content);
        cache.insert(hash, "parsed".to_string(), 12);

        assert_eq!(cache.get(hash), Some("parsed".to_string()));
    }

    #[test]
    fn test_multi_level_cache() {
        let cache = MultiLevelCache::<String>::with_defaults();
        let path = PathBuf::from("test.rs");
        let content = b"fn main() {}";

        cache.insert(path.clone(), Some(content), "result".to_string(), 12);

        // Should hit path cache
        assert_eq!(cache.get(&path, None), Some("result".to_string()));

        // Should hit content cache
        let path2 = PathBuf::from("test2.rs");
        assert_eq!(cache.get(&path2, Some(content)), Some("result".to_string()));
    }
}
