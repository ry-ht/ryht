//! Session discovery caching for improved performance.
//!
//! This module provides in-memory caching of session discovery results to avoid
//! repeated filesystem scans and parsing operations. The cache has a configurable
//! TTL and automatically invalidates stale entries.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::session::cache::{SessionCache, CacheConfig};
//! use std::time::Duration;
//!
//! let config = CacheConfig {
//!     ttl: Duration::from_secs(300), // 5 minutes
//!     enabled: true,
//! };
//!
//! let mut cache = SessionCache::new(config);
//! ```

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::types::{Project, Session};

/// Configuration for session caching.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300), // 5 minutes default (sessions change less frequently)
            enabled: true,
        }
    }
}

/// Cached entry for projects.
#[derive(Debug, Clone)]
struct CachedProjectsEntry {
    /// The cached projects
    projects: Vec<Project>,
    /// When this entry was cached
    cached_at: Instant,
}

impl CachedProjectsEntry {
    /// Check if this entry is still valid based on TTL.
    fn is_valid(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() < ttl
    }
}

/// Cached entry for sessions.
#[derive(Debug, Clone)]
struct CachedSessionsEntry {
    /// The cached sessions
    sessions: Vec<Session>,
    /// When this entry was cached
    cached_at: Instant,
}

impl CachedSessionsEntry {
    /// Check if this entry is still valid based on TTL.
    fn is_valid(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() < ttl
    }
}

/// Cache key for session queries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CacheKey {
    /// All projects
    AllProjects,
    /// Sessions for a specific project
    ProjectSessions(String),
}

/// In-memory cache for session discovery results.
///
/// This cache stores both projects and sessions to avoid repeated filesystem scans
/// and parsing operations. Entries are automatically invalidated after their TTL expires.
///
/// # Thread Safety
///
/// This cache is thread-safe and can be shared across threads using `Arc`.
/// Internally, it uses `RwLock` for concurrent access.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache::SessionCache;
///
/// let cache = SessionCache::default();
///
/// // Cache is empty initially
/// assert!(cache.get_projects().is_none());
/// ```
#[derive(Debug, Clone)]
pub struct SessionCache {
    /// Configuration
    config: CacheConfig,
    /// Cached projects
    projects_cache: Arc<RwLock<Option<CachedProjectsEntry>>>,
    /// Cached sessions by project ID
    sessions_cache: Arc<RwLock<HashMap<String, CachedSessionsEntry>>>,
}

impl SessionCache {
    /// Create a new cache with the given configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::{SessionCache, CacheConfig};
    /// use std::time::Duration;
    ///
    /// let config = CacheConfig {
    ///     ttl: Duration::from_secs(600), // 10 minutes
    ///     enabled: true,
    /// };
    ///
    /// let cache = SessionCache::new(config);
    /// ```
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            projects_cache: Arc::new(RwLock::new(None)),
            sessions_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get cached projects.
    ///
    /// Returns `None` if:
    /// - Caching is disabled
    /// - No entry exists
    /// - The cached entry has expired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// assert!(cache.get_projects().is_none()); // Empty cache
    /// ```
    pub fn get_projects(&self) -> Option<Vec<Project>> {
        if !self.config.enabled {
            return None;
        }

        let cache = self.projects_cache.read().ok()?;
        cache.as_ref().and_then(|entry| {
            if entry.is_valid(self.config.ttl) {
                Some(entry.projects.clone())
            } else {
                None
            }
        })
    }

    /// Cache projects.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// cache.set_projects(vec![]); // Cache empty result
    /// ```
    pub fn set_projects(&self, projects: Vec<Project>) {
        if !self.config.enabled {
            return;
        }

        if let Ok(mut cache) = self.projects_cache.write() {
            *cache = Some(CachedProjectsEntry {
                projects,
                cached_at: Instant::now(),
            });
        }
    }

    /// Get cached sessions for a project.
    ///
    /// Returns `None` if:
    /// - Caching is disabled
    /// - No entry exists for this project
    /// - The cached entry has expired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// assert!(cache.get_sessions("project-id").is_none());
    /// ```
    pub fn get_sessions(&self, project_id: &str) -> Option<Vec<Session>> {
        if !self.config.enabled {
            return None;
        }

        let cache = self.sessions_cache.read().ok()?;
        cache.get(project_id).and_then(|entry| {
            if entry.is_valid(self.config.ttl) {
                Some(entry.sessions.clone())
            } else {
                None
            }
        })
    }

    /// Cache sessions for a project.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// cache.set_sessions("project-id", vec![]);
    /// ```
    pub fn set_sessions(&self, project_id: String, sessions: Vec<Session>) {
        if !self.config.enabled {
            return;
        }

        if let Ok(mut cache) = self.sessions_cache.write() {
            cache.insert(
                project_id,
                CachedSessionsEntry {
                    sessions,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Clear all cached entries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// cache.set_projects(vec![]);
    /// cache.clear();
    /// assert!(cache.get_projects().is_none());
    /// ```
    pub fn clear(&self) {
        if let Ok(mut cache) = self.projects_cache.write() {
            *cache = None;
        }
        if let Ok(mut cache) = self.sessions_cache.write() {
            cache.clear();
        }
    }

    /// Clear cached projects only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// cache.set_projects(vec![]);
    /// cache.clear_projects();
    /// assert!(cache.get_projects().is_none());
    /// ```
    pub fn clear_projects(&self) {
        if let Ok(mut cache) = self.projects_cache.write() {
            *cache = None;
        }
    }

    /// Clear cached sessions for a specific project.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// cache.set_sessions("project-id".to_string(), vec![]);
    /// cache.clear_sessions("project-id");
    /// assert!(cache.get_sessions("project-id").is_none());
    /// ```
    pub fn clear_sessions(&self, project_id: &str) {
        if let Ok(mut cache) = self.sessions_cache.write() {
            cache.remove(project_id);
        }
    }

    /// Remove expired entries from the cache.
    ///
    /// Returns the number of entries removed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// let removed = cache.cleanup();
    /// println!("Removed {} expired entries", removed);
    /// ```
    pub fn cleanup(&self) -> usize {
        let mut removed = 0;
        let ttl = self.config.ttl;

        // Clean projects cache
        if let Ok(mut cache) = self.projects_cache.write() {
            if let Some(entry) = cache.as_ref() {
                if !entry.is_valid(ttl) {
                    *cache = None;
                    removed += 1;
                }
            }
        }

        // Clean sessions cache
        if let Ok(mut cache) = self.sessions_cache.write() {
            let initial_count = cache.len();
            cache.retain(|_, entry| entry.is_valid(ttl));
            removed += initial_count - cache.len();
        }

        removed
    }

    /// Get the number of cached entries.
    ///
    /// Returns a tuple of (projects_count, sessions_count).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// let (projects, sessions) = cache.len();
    /// println!("Cached {} projects and {} session lists", projects, sessions);
    /// ```
    pub fn len(&self) -> (usize, usize) {
        let projects_count = self
            .projects_cache
            .read()
            .ok()
            .map(|cache| if cache.is_some() { 1 } else { 0 })
            .unwrap_or(0);

        let sessions_count = self
            .sessions_cache
            .read()
            .ok()
            .map(|cache| cache.len())
            .unwrap_or(0);

        (projects_count, sessions_count)
    }

    /// Check if the cache is empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::SessionCache;
    ///
    /// let cache = SessionCache::default();
    /// assert!(cache.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let (projects, sessions) = self.len();
        projects == 0 && sessions == 0
    }

    /// Update the cache configuration.
    ///
    /// Note: This does not invalidate existing entries. Call `cleanup()` after
    /// changing the TTL if you want to remove entries that are now expired.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::session::cache::{SessionCache, CacheConfig};
    /// use std::time::Duration;
    ///
    /// let cache = SessionCache::default();
    /// let new_config = CacheConfig {
    ///     ttl: Duration::from_secs(600),
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

impl Default for SessionCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Global cache instance for session discovery.
///
/// This provides a convenient way to cache session discovery results without
/// manually managing a cache instance.
static GLOBAL_CACHE: std::sync::OnceLock<SessionCache> = std::sync::OnceLock::new();

/// Get or initialize the global session cache.
fn global_cache() -> &'static SessionCache {
    GLOBAL_CACHE.get_or_init(SessionCache::default)
}

/// Get cached projects from the global cache.
///
/// This is a convenience function for accessing the global cache.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache;
///
/// if let Some(projects) = cache::get_cached_projects() {
///     println!("Found {} cached projects", projects.len());
/// }
/// ```
pub fn get_cached_projects() -> Option<Vec<Project>> {
    global_cache().get_projects()
}

/// Cache projects in the global cache.
///
/// This is a convenience function for updating the global cache.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache;
///
/// cache::set_cached_projects(vec![]);
/// ```
pub fn set_cached_projects(projects: Vec<Project>) {
    global_cache().set_projects(projects);
}

/// Get cached sessions from the global cache.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache;
///
/// if let Some(sessions) = cache::get_cached_sessions("project-id") {
///     println!("Found {} cached sessions", sessions.len());
/// }
/// ```
pub fn get_cached_sessions(project_id: &str) -> Option<Vec<Session>> {
    global_cache().get_sessions(project_id)
}

/// Cache sessions in the global cache.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache;
///
/// cache::set_cached_sessions("project-id".to_string(), vec![]);
/// ```
pub fn set_cached_sessions(project_id: String, sessions: Vec<Session>) {
    global_cache().set_sessions(project_id, sessions);
}

/// Clear the global cache.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::cache;
///
/// cache::clear_cache();
/// ```
pub fn clear_cache() {
    global_cache().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::SessionId;
    use chrono::Utc;
    use std::path::PathBuf;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(300));
    }

    #[test]
    fn test_cache_basic_operations() {
        let cache = SessionCache::default();

        // Initially empty
        assert!(cache.is_empty());
        let (p, s) = cache.len();
        assert_eq!(p, 0);
        assert_eq!(s, 0);
        assert!(cache.get_projects().is_none());

        // Set and get projects
        let projects = vec![Project::new(
            "test-id".to_string(),
            PathBuf::from("/test"),
            vec![],
        )];
        cache.set_projects(projects.clone());
        let (p, s) = cache.len();
        assert_eq!(p, 1);
        assert_eq!(s, 0);
        assert!(!cache.is_empty());
        assert!(cache.get_projects().is_some());

        // Clear
        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.get_projects().is_none());
    }

    #[test]
    fn test_cache_sessions() {
        let cache = SessionCache::default();

        let sessions = vec![Session::new(
            SessionId::new("session-1"),
            PathBuf::from("/test"),
            Utc::now(),
            Some("Test message".to_string()),
        )];

        cache.set_sessions("project-1".to_string(), sessions.clone());
        assert!(cache.get_sessions("project-1").is_some());
        assert!(cache.get_sessions("project-2").is_none());

        // Clear specific project
        cache.clear_sessions("project-1");
        assert!(cache.get_sessions("project-1").is_none());
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig {
            ttl: Duration::from_millis(100), // Very short TTL for testing
            enabled: true,
        };
        let cache = SessionCache::new(config);

        cache.set_projects(vec![]);
        assert!(cache.get_projects().is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache.get_projects().is_none());
    }

    #[test]
    fn test_cache_cleanup() {
        let config = CacheConfig {
            ttl: Duration::from_millis(100),
            enabled: true,
        };
        let cache = SessionCache::new(config);

        // Add entries
        cache.set_projects(vec![]);
        cache.set_sessions("project-1".to_string(), vec![]);
        let (p, s) = cache.len();
        assert_eq!(p, 1);
        assert_eq!(s, 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Cleanup should remove the expired entries
        let removed = cache.cleanup();
        assert_eq!(removed, 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig {
            ttl: Duration::from_secs(300),
            enabled: false,
        };
        let cache = SessionCache::new(config);

        cache.set_projects(vec![]);
        assert!(cache.get_projects().is_none()); // Should be None when disabled
        assert!(cache.is_empty()); // Should not cache
    }

    #[test]
    fn test_cache_config_update() {
        let mut cache = SessionCache::default();

        cache.set_projects(vec![]);
        assert!(cache.get_projects().is_some());

        // Update config to disable caching
        let new_config = CacheConfig {
            ttl: Duration::from_secs(300),
            enabled: false,
        };
        cache.set_config(new_config);

        // Should return None because caching is now disabled
        assert!(cache.get_projects().is_none());
    }

    #[test]
    fn test_global_cache_functions() {
        // Clear any existing state
        clear_cache();

        // Initially empty
        assert!(get_cached_projects().is_none());

        // Set and get
        set_cached_projects(vec![]);
        assert!(get_cached_projects().is_some());

        // Sessions
        set_cached_sessions("project-1".to_string(), vec![]);
        assert!(get_cached_sessions("project-1").is_some());

        // Clear
        clear_cache();
        assert!(get_cached_projects().is_none());
        assert!(get_cached_sessions("project-1").is_none());
    }
}
