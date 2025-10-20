/// Query cache for frequently executed SurrealDB queries
///
/// This module provides caching for common graph queries to reduce
/// parsing overhead and improve performance.

use std::collections::HashMap;
use std::sync::RwLock;

/// Prepared query cache
pub struct QueryCache {
    queries: RwLock<HashMap<String, String>>,
}

impl QueryCache {
    /// Create a new query cache
    pub fn new() -> Self {
        Self {
            queries: RwLock::new(HashMap::new()),
        }
    }

    /// Get or insert a query template
    pub fn get_or_insert(&self, key: &str, template: impl Fn() -> String) -> String {
        // Fast path: read lock
        if let Ok(queries) = self.queries.read() {
            if let Some(query) = queries.get(key) {
                return query.clone();
            }
        }

        // Slow path: write lock
        if let Ok(mut queries) = self.queries.write() {
            queries
                .entry(key.to_string())
                .or_insert_with(template)
                .clone()
        } else {
            // Fallback if lock fails
            template()
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut queries) = self.queries.write() {
            queries.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(queries) = self.queries.read() {
            CacheStats {
                entry_count: queries.len(),
                memory_estimate: queries
                    .iter()
                    .map(|(k, v)| k.len() + v.len())
                    .sum::<usize>(),
            }
        } else {
            CacheStats {
                entry_count: 0,
                memory_estimate: 0,
            }
        }
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub memory_estimate: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new();

        let query1 = cache.get_or_insert("find_deps", || {
            "SELECT * FROM deps WHERE id = $id".to_string()
        });

        let query2 = cache.get_or_insert("find_deps", || {
            "This should not be used".to_string()
        });

        assert_eq!(query1, query2);
        assert_eq!(cache.stats().entry_count, 1);
    }

    #[test]
    fn test_cache_clear() {
        let cache = QueryCache::new();

        cache.get_or_insert("test", || "SELECT *".to_string());
        assert_eq!(cache.stats().entry_count, 1);

        cache.clear();
        assert_eq!(cache.stats().entry_count, 0);
    }
}
