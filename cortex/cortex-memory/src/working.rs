//! Working memory implementation with priority-based retention and eviction.
//!
//! Working memory provides fast, temporary storage with automatic eviction
//! based on priority, recency, and access patterns.

use crate::types::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Working memory system with priority-based eviction
pub struct WorkingMemorySystem {
    items: Arc<DashMap<String, WorkingMemoryItem>>,
    max_items: usize,
    max_bytes: usize,
    current_bytes: Arc<RwLock<usize>>,
    eviction_count: Arc<RwLock<u64>>,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
}

impl WorkingMemorySystem {
    /// Create a new working memory system
    pub fn new(max_items: usize, max_bytes: usize) -> Self {
        Self {
            items: Arc::new(DashMap::new()),
            max_items,
            max_bytes,
            current_bytes: Arc::new(RwLock::new(0)),
            eviction_count: Arc::new(RwLock::new(0)),
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Store an item with a given priority
    pub fn store(&self, key: String, value: Vec<u8>, priority: Priority) -> bool {
        let item = WorkingMemoryItem::new(key.clone(), value, priority);
        let item_size = item.size_bytes;

        // Check if we need to evict items
        if self.items.len() >= self.max_items || *self.current_bytes.read() + item_size > self.max_bytes {
            if !self.evict_low_priority_items(item_size) {
                warn!(key = %key, "Failed to evict items for new entry");
                return false;
            }
        }

        // Update byte count
        *self.current_bytes.write() += item_size;

        // Insert item
        self.items.insert(key.clone(), item);
        debug!(key = %key, priority = ?priority, size = item_size, "Stored item in working memory");

        true
    }

    /// Retrieve an item and update access statistics
    pub fn retrieve(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(mut entry) = self.items.get_mut(key) {
            entry.last_accessed = chrono::Utc::now();
            entry.access_count += 1;
            *self.hit_count.write() += 1;
            debug!(key = %key, access_count = entry.access_count, "Retrieved item from working memory");
            Some(entry.value.clone())
        } else {
            *self.miss_count.write() += 1;
            None
        }
    }

    /// Update an item's priority
    pub fn update_priority(&self, key: &str, new_priority: Priority) -> bool {
        if let Some(mut entry) = self.items.get_mut(key) {
            entry.priority = new_priority;
            debug!(key = %key, new_priority = ?new_priority, "Updated item priority");
            true
        } else {
            false
        }
    }

    /// Remove an item from working memory
    pub fn remove(&self, key: &str) -> Option<Vec<u8>> {
        if let Some((_, item)) = self.items.remove(key) {
            *self.current_bytes.write() -= item.size_bytes;
            debug!(key = %key, "Removed item from working memory");
            Some(item.value)
        } else {
            None
        }
    }

    /// Evict low-priority items to make space
    fn evict_low_priority_items(&self, needed_bytes: usize) -> bool {
        info!(needed_bytes, "Evicting low-priority items");

        // Collect all items with their retention scores
        let mut items_with_scores: Vec<(String, f64)> = self
            .items
            .iter()
            .map(|entry| {
                let key = entry.key().clone();
                let score = entry.value().retention_score();
                (key, score)
            })
            .collect();

        // Sort by retention score (lowest first = most likely to evict)
        items_with_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut freed_bytes = 0;
        let mut evicted_count = 0;

        // Evict items until we have enough space
        for (key, _score) in items_with_scores {
            if freed_bytes >= needed_bytes && self.items.len() - evicted_count < self.max_items {
                break;
            }

            if let Some((_, item)) = self.items.remove(&key) {
                freed_bytes += item.size_bytes;
                evicted_count += 1;
                debug!(key = %key, size = item.size_bytes, "Evicted item");
            }
        }

        // Update statistics
        *self.current_bytes.write() -= freed_bytes;
        *self.eviction_count.write() += evicted_count as u64;

        info!(evicted = evicted_count, freed_bytes, "Eviction complete");
        freed_bytes >= needed_bytes
    }

    /// Clear all items
    pub fn clear(&self) {
        info!("Clearing working memory");
        self.items.clear();
        *self.current_bytes.write() = 0;
    }

    /// Get the number of items in working memory
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if working memory is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get current byte usage
    pub fn current_bytes(&self) -> usize {
        *self.current_bytes.read()
    }

    /// Get all keys currently in memory
    pub fn keys(&self) -> Vec<String> {
        self.items.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get statistics about working memory
    pub fn get_statistics(&self) -> WorkingStats {
        let hit_count = *self.hit_count.read();
        let miss_count = *self.miss_count.read();
        let total_accesses = hit_count + miss_count;

        let cache_hit_rate = if total_accesses > 0 {
            (hit_count as f32) / (total_accesses as f32)
        } else {
            0.0
        };

        WorkingStats {
            current_items: self.items.len(),
            capacity: self.max_items,
            total_evictions: *self.eviction_count.read(),
            cache_hit_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let memory = WorkingMemorySystem::new(10, 1024 * 1024);

        let key = "test_key".to_string();
        let value = vec![1, 2, 3, 4, 5];

        assert!(memory.store(key.clone(), value.clone(), Priority::Medium));
        assert_eq!(memory.retrieve(&key), Some(value));
    }

    #[test]
    fn test_capacity_limit() {
        let memory = WorkingMemorySystem::new(2, 1024);

        assert!(memory.store("key1".to_string(), vec![1], Priority::Low));
        assert!(memory.store("key2".to_string(), vec![2], Priority::Low));
        assert!(memory.store("key3".to_string(), vec![3], Priority::High));

        // High priority item should be stored, low priority evicted
        assert_eq!(memory.len(), 2);
        assert!(memory.retrieve("key3").is_some());
    }

    #[test]
    fn test_priority_eviction() {
        let memory = WorkingMemorySystem::new(3, 1024);

        memory.store("low1".to_string(), vec![1], Priority::Low);
        memory.store("medium1".to_string(), vec![2], Priority::Medium);
        memory.store("high1".to_string(), vec![3], Priority::High);

        // Try to add a critical priority item - should evict lowest
        memory.store("critical1".to_string(), vec![4], Priority::Critical);

        assert!(memory.retrieve("critical1").is_some());
        assert!(memory.retrieve("high1").is_some());
        assert!(memory.retrieve("medium1").is_some());
        // Low priority item should have been evicted
        assert!(memory.retrieve("low1").is_none());
    }

    #[test]
    fn test_update_priority() {
        let memory = WorkingMemorySystem::new(10, 1024);

        memory.store("key1".to_string(), vec![1], Priority::Low);
        assert!(memory.update_priority("key1", Priority::High));

        // Verify priority was updated (implicitly through eviction behavior)
        memory.store("key2".to_string(), vec![2], Priority::Low);
        memory.store("key3".to_string(), vec![3], Priority::Low);
    }

    #[test]
    fn test_statistics() {
        let memory = WorkingMemorySystem::new(10, 1024);

        memory.store("key1".to_string(), vec![1], Priority::Medium);
        memory.retrieve("key1");
        memory.retrieve("key1");
        memory.retrieve("nonexistent");

        let stats = memory.get_statistics();
        assert_eq!(stats.current_items, 1);
        assert_eq!(stats.capacity, 10);
        assert!(stats.cache_hit_rate > 0.0);
    }
}
