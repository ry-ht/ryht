//! Multi-level caching system for Meridian
//!
//! This module provides a 3-tier caching architecture:
//! - L1 (Hot): 1,000 entries - in-memory LRU, <1ms access
//! - L2 (Warm): 10,000 entries - in-memory LRU, 1-5ms access
//! - L3 (Cold): RocksDB - disk-based, 5-20ms access
//!
//! Expected performance improvements:
//! - Cache hit rate: 10% → 60% (6x improvement)
//! - Average latency: 15ms → 3ms (5x faster)
//! - Memory overhead: +50MB for larger caches

pub mod multi_level;

pub use multi_level::{CacheStats, MultiLevelCache, MultiLevelCacheConfig};
