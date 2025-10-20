//! Content deduplication using content hashing.

use dashmap::DashMap;
use std::sync::Arc;

/// Deduplication manager using content-addressable storage
pub struct Deduplicator {
    /// Maps content hash to reference count
    hash_refs: Arc<DashMap<String, usize>>,
}

impl Deduplicator {
    /// Create a new deduplicator
    pub fn new() -> Self {
        Self {
            hash_refs: Arc::new(DashMap::new()),
        }
    }

    /// Add a reference to content with the given hash
    pub fn add_ref(&self, hash: &str) {
        self.hash_refs
            .entry(hash.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Remove a reference to content with the given hash
    /// Returns true if the content should be deleted (no more references)
    pub fn remove_ref(&self, hash: &str) -> bool {
        if let Some(mut entry) = self.hash_refs.get_mut(hash) {
            *entry -= 1;
            if *entry == 0 {
                drop(entry);
                self.hash_refs.remove(hash);
                return true;
            }
        }
        false
    }

    /// Get the number of references for a hash
    pub fn ref_count(&self, hash: &str) -> usize {
        self.hash_refs.get(hash).map(|e| *e).unwrap_or(0)
    }

    /// Check if content with the given hash exists
    pub fn has_content(&self, hash: &str) -> bool {
        self.hash_refs.contains_key(hash)
    }

    /// Get total number of unique content items
    pub fn unique_count(&self) -> usize {
        self.hash_refs.len()
    }
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication() {
        let dedup = Deduplicator::new();

        dedup.add_ref("hash1");
        assert_eq!(dedup.ref_count("hash1"), 1);

        dedup.add_ref("hash1");
        assert_eq!(dedup.ref_count("hash1"), 2);

        assert!(!dedup.remove_ref("hash1"));
        assert_eq!(dedup.ref_count("hash1"), 1);

        assert!(dedup.remove_ref("hash1"));
        assert_eq!(dedup.ref_count("hash1"), 0);
        assert!(!dedup.has_content("hash1"));
    }

    #[test]
    fn test_unique_count() {
        let dedup = Deduplicator::new();

        dedup.add_ref("hash1");
        dedup.add_ref("hash2");
        dedup.add_ref("hash1");

        assert_eq!(dedup.unique_count(), 2);
    }
}
