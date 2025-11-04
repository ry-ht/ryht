//! Discovery caching for improved performance.
//!
//! This module provides in-memory caching of binary discovery results to avoid
//! repeated filesystem scans and command executions. The cache has a configurable
//! TTL and automatically invalidates stale entries.
//!
//! # Examples
//!
//! ```no_run
//! use crate::cc::binary::cache::DiscoveryCache;
//! use crate::cc::cache::CacheConfig;
//! use std::time::Duration;
//!
//! let config = CacheConfig::new(Duration::from_secs(3600), true);
//! let mut cache = DiscoveryCache::new(config);
//! ```

use std::collections::HashMap;
use std::hash::Hash;

use super::discovery::ClaudeInstallation;

/// Re-export generic cache types for convenience
pub use crate::cc::cache::{CachedEntry, CacheConfig};

/// Cache key based on discovery configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Custom paths to check
    custom_paths: Vec<String>,
    /// Whether NVM scanning is skipped
    skip_nvm: bool,
    /// Whether Homebrew scanning is skipped
    skip_homebrew: bool,
    /// Whether system scanning is skipped
    skip_system: bool,
}

impl CacheKey {
    /// Create a cache key from discovery options.
    fn from_options(
        custom_paths: &[String],
        skip_nvm: bool,
        skip_homebrew: bool,
        skip_system: bool,
    ) -> Self {
        Self {
            custom_paths: custom_paths.to_vec(),
            skip_nvm,
            skip_homebrew,
            skip_system,
        }
    }

    /// Create a default cache key for standard discovery.
    fn default_key() -> Self {
        Self {
            custom_paths: Vec::new(),
            skip_nvm: false,
            skip_homebrew: false,
            skip_system: false,
        }
    }
}

/// In-memory cache for binary discovery results.
///
/// This cache stores discovery results to avoid repeated filesystem scans
/// and command executions. Entries are automatically invalidated after their TTL expires.
///
/// # Thread Safety
///
/// This cache is not thread-safe by itself. If you need to share it across threads,
/// wrap it in a `Mutex` or `RwLock`.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::cache::DiscoveryCache;
///
/// let mut cache = DiscoveryCache::default();
///
/// // Cache is empty initially
/// assert!(cache.get_default().is_none());
/// ```
#[derive(Debug)]
pub struct DiscoveryCache {
    /// Configuration
    config: CacheConfig,
    /// Cached entries indexed by configuration hash
    entries: HashMap<CacheKey, CachedEntry<Vec<ClaudeInstallation>>>,
}

impl DiscoveryCache {
    /// Create a new cache with the given configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::{DiscoveryCache, CacheConfig};
    /// use std::time::Duration;
    ///
    /// let config = CacheConfig {
    ///     ttl: Duration::from_secs(1800), // 30 minutes
    ///     enabled: true,
    /// };
    ///
    /// let cache = DiscoveryCache::new(config);
    /// ```
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
        }
    }

    /// Get cached installations for the default discovery configuration.
    ///
    /// Returns `None` if:
    /// - Caching is disabled
    /// - No entry exists for this key
    /// - The cached entry has expired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// assert!(cache.get_default().is_none()); // Empty cache
    /// ```
    pub fn get_default(&self) -> Option<&Vec<ClaudeInstallation>> {
        if !self.config.enabled {
            return None;
        }

        let key = CacheKey::default_key();
        self.get_with_key(&key)
    }

    /// Get cached installations for a custom discovery configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// let custom_paths = vec!["/custom/path".to_string()];
    /// assert!(cache.get_custom(&custom_paths, false, false, false).is_none());
    /// ```
    pub fn get_custom(
        &self,
        custom_paths: &[String],
        skip_nvm: bool,
        skip_homebrew: bool,
        skip_system: bool,
    ) -> Option<&Vec<ClaudeInstallation>> {
        if !self.config.enabled {
            return None;
        }

        let key = CacheKey::from_options(custom_paths, skip_nvm, skip_homebrew, skip_system);
        self.get_with_key(&key)
    }

    /// Internal method to get cached entry by key.
    fn get_with_key(&self, key: &CacheKey) -> Option<&Vec<ClaudeInstallation>> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_valid(self.config.ttl) {
                Some(entry.data())
            } else {
                None
            }
        })
    }

    /// Cache installations for the default discovery configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// cache.set_default(vec![]); // Cache empty result
    /// ```
    pub fn set_default(&mut self, installations: Vec<ClaudeInstallation>) {
        if !self.config.enabled {
            return;
        }

        let key = CacheKey::default_key();
        self.set_with_key(key, installations);
    }

    /// Cache installations for a custom discovery configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// let custom_paths = vec!["/custom/path".to_string()];
    /// cache.set_custom(vec![], &custom_paths, false, false, false);
    /// ```
    pub fn set_custom(
        &mut self,
        installations: Vec<ClaudeInstallation>,
        custom_paths: &[String],
        skip_nvm: bool,
        skip_homebrew: bool,
        skip_system: bool,
    ) {
        if !self.config.enabled {
            return;
        }

        let key = CacheKey::from_options(custom_paths, skip_nvm, skip_homebrew, skip_system);
        self.set_with_key(key, installations);
    }

    /// Internal method to cache an entry.
    fn set_with_key(&mut self, key: CacheKey, installations: Vec<ClaudeInstallation>) {
        self.entries.insert(key, CachedEntry::new(installations));
    }

    /// Clear all cached entries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// cache.set_default(vec![]);
    /// cache.clear();
    /// assert!(cache.get_default().is_none());
    /// ```
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove expired entries from the cache.
    ///
    /// Returns the number of entries removed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// let removed = cache.cleanup();
    /// println!("Removed {} expired entries", removed);
    /// ```
    pub fn cleanup(&mut self) -> usize {
        let ttl = self.config.ttl;
        let initial_count = self.entries.len();

        self.entries.retain(|_, entry| entry.is_valid(ttl));

        initial_count - self.entries.len()
    }

    /// Get the number of cached entries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let cache = DiscoveryCache::default();
    /// assert_eq!(cache.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::DiscoveryCache;
    ///
    /// let cache = DiscoveryCache::default();
    /// assert!(cache.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Update the cache configuration.
    ///
    /// Note: This does not invalidate existing entries. Call `cleanup()` after
    /// changing the TTL if you want to remove entries that are now expired.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::cache::{DiscoveryCache, CacheConfig};
    /// use std::time::Duration;
    ///
    /// let mut cache = DiscoveryCache::default();
    /// let new_config = CacheConfig {
    ///     ttl: Duration::from_secs(1800),
    ///     enabled: true,
    /// };
    /// cache.set_config(new_config);
    /// cache.cleanup(); // Remove entries that are now expired
    /// ```
    pub fn set_config(&mut self, config: CacheConfig) {
        self.config = config;
    }

    /// Get the current cache configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        // Use longer TTL for binary discovery (1 hour) since binaries change infrequently
        Self::new(CacheConfig::new(std::time::Duration::from_secs(3600), true))
    }
}

/// Global cache instance for default discovery.
///
/// This provides a convenient way to cache default discovery results without
/// manually managing a cache instance.
static GLOBAL_CACHE: std::sync::OnceLock<std::sync::Mutex<DiscoveryCache>> =
    std::sync::OnceLock::new();

/// Get or initialize the global discovery cache.
fn global_cache() -> &'static std::sync::Mutex<DiscoveryCache> {
    GLOBAL_CACHE.get_or_init(|| std::sync::Mutex::new(DiscoveryCache::default()))
}

/// Get cached installations from the global cache.
///
/// This is a convenience function for accessing the global cache.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::cache;
///
/// if let Some(installations) = cache::get_cached_default() {
///     println!("Found {} cached installations", installations.len());
/// }
/// ```
pub fn get_cached_default() -> Option<Vec<ClaudeInstallation>> {
    global_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get_default().cloned())
}

/// Cache installations in the global cache.
///
/// This is a convenience function for updating the global cache.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::cache;
///
/// cache::set_cached_default(vec![]);
/// ```
pub fn set_cached_default(installations: Vec<ClaudeInstallation>) {
    if let Ok(mut cache) = global_cache().lock() {
        cache.set_default(installations);
    }
}

/// Clear the global cache.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::cache;
///
/// cache::clear_cache();
/// ```
pub fn clear_cache() {
    if let Ok(mut cache) = global_cache().lock() {
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_config_default() {
        // Generic cache default is 300 seconds
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(300));

        // DiscoveryCache default uses 1-hour TTL (binaries change infrequently)
        let discovery_cache = DiscoveryCache::default();
        assert_eq!(discovery_cache.config().ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_cache_key_equality() {
        let key1 = CacheKey::default_key();
        let key2 = CacheKey::default_key();
        assert_eq!(key1, key2);

        let key3 = CacheKey::from_options(&["/custom".to_string()], false, false, false);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = DiscoveryCache::default();

        // Initially empty
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get_default().is_none());

        // Set and get
        let installations = vec![];
        cache.set_default(installations.clone());
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        assert!(cache.get_default().is_some());

        // Clear
        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.get_default().is_none());
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig {
            ttl: Duration::from_millis(100), // Very short TTL for testing
            enabled: true,
        };
        let mut cache = DiscoveryCache::new(config);

        cache.set_default(vec![]);
        assert!(cache.get_default().is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache.get_default().is_none());
    }

    #[test]
    fn test_cache_cleanup() {
        let config = CacheConfig {
            ttl: Duration::from_millis(100),
            enabled: true,
        };
        let mut cache = DiscoveryCache::new(config);

        // Add entry
        cache.set_default(vec![]);
        assert_eq!(cache.len(), 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Cleanup should remove the expired entry
        let removed = cache.cleanup();
        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig {
            ttl: Duration::from_secs(3600),
            enabled: false,
        };
        let mut cache = DiscoveryCache::new(config);

        cache.set_default(vec![]);
        assert!(cache.get_default().is_none()); // Should be None when disabled
        assert_eq!(cache.len(), 0); // Should not cache
    }

    #[test]
    fn test_cache_custom_keys() {
        let mut cache = DiscoveryCache::default();

        let custom_paths = vec!["/custom".to_string()];
        cache.set_custom(vec![], &custom_paths, false, false, false);

        // Should not match default key
        assert!(cache.get_default().is_none());

        // Should match custom key
        assert!(cache
            .get_custom(&custom_paths, false, false, false)
            .is_some());

        // Should not match different custom key
        let different_paths = vec!["/other".to_string()];
        assert!(cache
            .get_custom(&different_paths, false, false, false)
            .is_none());
    }

    #[test]
    fn test_cache_config_update() {
        let mut cache = DiscoveryCache::default();

        cache.set_default(vec![]);
        assert!(cache.get_default().is_some());

        // Update config to disable caching
        let new_config = CacheConfig {
            ttl: Duration::from_secs(3600),
            enabled: false,
        };
        cache.set_config(new_config);

        // Should return None because caching is now disabled
        assert!(cache.get_default().is_none());
    }

    #[test]
    fn test_global_cache_functions() {
        // Clear any existing state
        clear_cache();

        // Initially empty
        assert!(get_cached_default().is_none());

        // Set and get
        set_cached_default(vec![]);
        assert!(get_cached_default().is_some());

        // Clear
        clear_cache();
        assert!(get_cached_default().is_none());
    }
}
