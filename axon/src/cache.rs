//! Generic caching infrastructure.
//!
//! This module provides a generic, reusable caching mechanism that can be used
//! throughout the SDK for caching various types of data with TTL support.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::cache::{CachedEntry, CacheConfig};
//! use std::time::Duration;
//!
//! let config = CacheConfig {
//!     ttl: Duration::from_secs(300),
//!     enabled: true,
//! };
//!
//! let entry = CachedEntry::new(vec!["data".to_string()]);
//! assert!(entry.is_valid(config.ttl));
//! ```

use std::time::{Duration, Instant};

/// Configuration for caching behavior.
///
/// This configuration can be used across different cache types to maintain
/// consistent caching behavior throughout the SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheConfig {
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl CacheConfig {
    /// Create a new cache configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CacheConfig;
    /// use std::time::Duration;
    ///
    /// let config = CacheConfig::new(Duration::from_secs(600), true);
    /// assert_eq!(config.ttl, Duration::from_secs(600));
    /// assert!(config.enabled);
    /// ```
    pub fn new(ttl: Duration, enabled: bool) -> Self {
        Self { ttl, enabled }
    }

    /// Create a disabled cache configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CacheConfig;
    ///
    /// let config = CacheConfig::disabled();
    /// assert!(!config.enabled);
    /// ```
    pub fn disabled() -> Self {
        Self {
            ttl: Duration::from_secs(0),
            enabled: false,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300), // 5 minutes default
            enabled: true,
        }
    }
}

/// A cached entry with automatic expiration.
///
/// This generic type wraps any data with a timestamp, allowing for TTL-based
/// cache invalidation.
///
/// # Type Parameters
///
/// * `T` - The type of data being cached
///
/// # Examples
///
/// ```
/// use cc_sdk::cache::CachedEntry;
/// use std::time::Duration;
///
/// let entry = CachedEntry::new(vec![1, 2, 3]);
/// assert!(entry.is_valid(Duration::from_secs(300)));
///
/// let data = entry.into_inner();
/// assert_eq!(data, vec![1, 2, 3]);
/// ```
#[derive(Debug, Clone)]
pub struct CachedEntry<T> {
    /// The cached data
    data: T,
    /// When this entry was cached
    cached_at: Instant,
}

impl<T> CachedEntry<T> {
    /// Create a new cached entry with the current timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    ///
    /// let entry = CachedEntry::new("cached data".to_string());
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            data,
            cached_at: Instant::now(),
        }
    }

    /// Check if this entry is still valid based on TTL.
    ///
    /// Returns `true` if the elapsed time since caching is less than the TTL.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    /// use std::time::Duration;
    ///
    /// let entry = CachedEntry::new(42);
    /// assert!(entry.is_valid(Duration::from_secs(300)));
    /// ```
    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() < ttl
    }

    /// Get a reference to the cached data.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    ///
    /// let entry = CachedEntry::new(vec![1, 2, 3]);
    /// assert_eq!(entry.data(), &vec![1, 2, 3]);
    /// ```
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Get a mutable reference to the cached data.
    ///
    /// Note: This does not update the cached_at timestamp. If you want to
    /// update both the data and timestamp, create a new `CachedEntry`.
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Get the timestamp when this entry was cached.
    pub fn cached_at(&self) -> Instant {
        self.cached_at
    }

    /// Get the age of this cached entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    ///
    /// let entry = CachedEntry::new("data");
    /// let age = entry.age();
    /// assert!(age.as_millis() < 100); // Should be very fresh
    /// ```
    pub fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }

    /// Consume the entry and return the inner data.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    ///
    /// let entry = CachedEntry::new(vec![1, 2, 3]);
    /// let data = entry.into_inner();
    /// assert_eq!(data, vec![1, 2, 3]);
    /// ```
    pub fn into_inner(self) -> T {
        self.data
    }

    /// Refresh the entry's timestamp to the current time.
    ///
    /// This is useful when you want to extend the lifetime of an entry without
    /// changing its data.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    /// use std::time::Duration;
    ///
    /// let mut entry = CachedEntry::new(42);
    /// std::thread::sleep(Duration::from_millis(100));
    /// entry.refresh();
    /// assert!(entry.age().as_millis() < 50);
    /// ```
    pub fn refresh(&mut self) {
        self.cached_at = Instant::now();
    }

    /// Update the cached data and refresh the timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    ///
    /// let mut entry = CachedEntry::new(42);
    /// entry.update(100);
    /// assert_eq!(entry.data(), &100);
    /// ```
    pub fn update(&mut self, data: T) {
        self.data = data;
        self.cached_at = Instant::now();
    }
}

impl<T: Clone> CachedEntry<T> {
    /// Get a clone of the cached data if the entry is still valid.
    ///
    /// Returns `Some(data)` if the entry is valid according to the TTL,
    /// `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::cache::CachedEntry;
    /// use std::time::Duration;
    ///
    /// let entry = CachedEntry::new(vec![1, 2, 3]);
    /// let data = entry.get_if_valid(Duration::from_secs(300));
    /// assert_eq!(data, Some(vec![1, 2, 3]));
    /// ```
    pub fn get_if_valid(&self, ttl: Duration) -> Option<T> {
        if self.is_valid(ttl) {
            Some(self.data.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config() {
        let config = CacheConfig::new(Duration::from_secs(600), true);
        assert_eq!(config.ttl, Duration::from_secs(600));
        assert!(config.enabled);

        let disabled = CacheConfig::disabled();
        assert!(!disabled.enabled);
        assert_eq!(disabled.ttl, Duration::from_secs(0));

        let default = CacheConfig::default();
        assert!(default.enabled);
        assert_eq!(default.ttl, Duration::from_secs(300));
    }

    #[test]
    fn test_cached_entry_basic() {
        let entry = CachedEntry::new(42);
        assert_eq!(entry.data(), &42);
        assert!(entry.is_valid(Duration::from_secs(300)));
    }

    #[test]
    fn test_cached_entry_expiration() {
        let entry = CachedEntry::new("data");
        assert!(entry.is_valid(Duration::from_millis(100)));

        std::thread::sleep(Duration::from_millis(150));
        assert!(!entry.is_valid(Duration::from_millis(100)));
    }

    #[test]
    fn test_cached_entry_age() {
        let entry = CachedEntry::new(vec![1, 2, 3]);
        assert!(entry.age().as_millis() < 100);

        std::thread::sleep(Duration::from_millis(100));
        assert!(entry.age().as_millis() >= 100);
    }

    #[test]
    fn test_cached_entry_into_inner() {
        let entry = CachedEntry::new(vec![1, 2, 3]);
        let data = entry.into_inner();
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn test_cached_entry_refresh() {
        let mut entry = CachedEntry::new(42);
        std::thread::sleep(Duration::from_millis(100));

        entry.refresh();
        assert!(entry.age().as_millis() < 50);
    }

    #[test]
    fn test_cached_entry_update() {
        let mut entry = CachedEntry::new(42);
        std::thread::sleep(Duration::from_millis(100));

        entry.update(100);
        assert_eq!(entry.data(), &100);
        assert!(entry.age().as_millis() < 50);
    }

    #[test]
    fn test_cached_entry_get_if_valid() {
        let entry = CachedEntry::new(vec![1, 2, 3]);
        assert_eq!(entry.get_if_valid(Duration::from_secs(300)), Some(vec![1, 2, 3]));

        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(entry.get_if_valid(Duration::from_millis(100)), None);
    }

    #[test]
    fn test_cached_entry_data_mut() {
        let mut entry = CachedEntry::new(vec![1, 2, 3]);
        entry.data_mut().push(4);
        assert_eq!(entry.data(), &vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_cache_config_equality() {
        let config1 = CacheConfig::new(Duration::from_secs(300), true);
        let config2 = CacheConfig::new(Duration::from_secs(300), true);
        assert_eq!(config1, config2);

        let config3 = CacheConfig::new(Duration::from_secs(600), true);
        assert_ne!(config1, config3);
    }
}
