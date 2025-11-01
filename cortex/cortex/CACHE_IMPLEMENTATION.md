# CodeUnitService LRU Cache Implementation

## Overview

This document describes the LRU (Least Recently Used) cache implementation for the `CodeUnitService` to optimize query performance. The cache uses the `moka` crate, a high-performance async-aware caching library for Rust.

## Implementation Details

### Architecture

The cache implementation follows the **cache-aside pattern** (lazy-loading):
1. Check cache first on read
2. On cache miss, fetch from database
3. Populate cache with fetched data
4. On writes, invalidate affected cache entries

### Components

#### 1. Cache Structure

- **Two-level cache**: Separate caches for `unit_id` and `qualified_name` lookups
- **Library**: `moka` v0.12 with async support
- **Thread-safe**: Built-in async-aware concurrency control
- **TTL**: Configurable time-to-live (default: 5 minutes)
- **TTI**: Configurable time-to-idle (default: 1 minute)
- **Capacity**: Configurable max entries (default: 10,000 units)

#### 2. Cache Configuration

```rust
pub struct CacheConfig {
    pub max_capacity: u64,    // Default: 10,000
    pub ttl_seconds: u64,     // Default: 300 (5 minutes)
    pub tti_seconds: u64,     // Default: 60 (1 minute)
}
```

#### 3. Cache Metrics

Thread-safe atomic counters track:
- **Hits**: Successful cache retrievals
- **Misses**: Cache misses requiring database queries
- **Invalidations**: Number of cache entries invalidated
- **Hit Rate**: Calculated as `(hits / total_requests) * 100`

```rust
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_requests: u64,
    pub hit_rate: f64,
    pub invalidations: u64,
}
```

### Key Features

#### 1. Dual Cache Indexes

```rust
cache_by_id: Cache<String, CodeUnitDetails>
cache_by_qualified_name: Cache<String, CodeUnitDetails>
```

Both caches are kept in sync:
- Fetching by ID populates both caches
- Fetching by qualified_name populates both caches
- Updates invalidate entries in both caches

#### 2. Cache Invalidation

Cache invalidation occurs when:
- `update_code_unit()` is called (invalidates both ID and qualified_name)
- `clear_cache()` is called (clears all entries)
- TTL expires (automatic)
- TTI expires due to inactivity (automatic)
- LRU eviction when capacity is reached (automatic)

#### 3. Concurrency

- `moka::future::Cache` is fully async-aware and thread-safe
- No manual locking required
- Safe for concurrent reads and writes
- Metrics use atomic operations for thread-safety

## API

### Public Methods

```rust
// Create service with default cache config
pub fn new(storage: Arc<ConnectionManager>) -> Self

// Create service with custom cache config
pub fn with_cache_config(storage: Arc<ConnectionManager>, config: CacheConfig) -> Self

// Get code unit by ID (cached)
pub async fn get_code_unit(&self, unit_id: &str) -> Result<CodeUnitDetails>

// Get code unit by qualified name (cached)
pub async fn get_by_qualified_name(&self, qualified_name: &str) -> Result<CodeUnitDetails>

// Update code unit (invalidates cache)
pub async fn update_code_unit(
    &self,
    unit_id: &str,
    body: Option<String>,
    docstring: Option<String>,
    expected_version: Option<u32>,
) -> Result<CodeUnitDetails>

// Cache management
pub async fn clear_cache(&self)
pub fn cache_stats(&self) -> CacheStats
pub fn reset_cache_stats(&self)
```

## Performance Characteristics

### Expected Performance Improvements

Based on similar implementations:

1. **Cache Hit Performance**: 10-100x faster than database queries
   - Cache hit: <1ms (in-memory lookup)
   - Database query: 5-50ms (depending on load)

2. **Read-Heavy Workloads**: Most significant improvement
   - 80/20 rule: 80% of requests access 20% of data
   - Expected hit rate: 60-90% for typical workloads

3. **Concurrent Access**: Linear scalability
   - No lock contention for reads
   - Efficient atomic operations for metrics

### Memory Usage

- **Per entry overhead**: ~200 bytes (moka metadata)
- **CodeUnitDetails size**: ~1-5 KB average
- **Total for 10,000 entries**: ~50-100 MB

### Eviction Strategy

- **LRU (Least Recently Used)**: When capacity is reached
- **TTL (Time-to-Live)**: Absolute expiration after 5 minutes
- **TTI (Time-to-Idle)**: Expiration after 1 minute of no access

## Configuration Recommendations

### Development Environment
```rust
CacheConfig {
    max_capacity: 1_000,
    ttl_seconds: 60,     // 1 minute
    tti_seconds: 30,     // 30 seconds
}
```

### Production Environment (Small)
```rust
CacheConfig {
    max_capacity: 10_000,  // Default
    ttl_seconds: 300,      // 5 minutes (default)
    tti_seconds: 60,       // 1 minute (default)
}
```

### Production Environment (Large)
```rust
CacheConfig {
    max_capacity: 50_000,
    ttl_seconds: 600,      // 10 minutes
    tti_seconds: 120,      // 2 minutes
}
```

### Memory-Constrained Environment
```rust
CacheConfig {
    max_capacity: 1_000,
    ttl_seconds: 120,      // 2 minutes
    tti_seconds: 30,       // 30 seconds
}
```

## Testing

### Unit Tests

Location: `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/tests/code_unit_cache_tests.rs`

Tests cover:
- Cache hit/miss behavior
- TTL expiration
- Cache invalidation on updates
- Concurrent access (thread safety)
- Cache size limits and eviction
- Cache statistics
- Custom configurations

Run tests:
```bash
cd cortex/cortex
cargo test code_unit_cache_tests
```

### Integration Tests

Location: `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/tests/code_unit_cache_integration.rs`

Tests cover:
- Realistic read-heavy workloads
- Update/invalidation workflows
- Memory pressure simulation
- Mixed access patterns (ID and qualified_name)
- Concurrent reads/writes
- Performance comparisons

Run tests:
```bash
cd cortex/cortex
cargo test code_unit_cache_integration
```

### Benchmarks

Location: `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/benches/code_unit_cache.rs`

Benchmarks:
1. **cache_hit_vs_miss**: Compare cache hit vs miss performance
2. **different_cache_sizes**: Test various cache capacities
3. **concurrent_reads**: Measure scalability under concurrent load
4. **cache_invalidation**: Measure invalidation overhead
5. **mixed_workload**: Realistic 80/15/5 read/read/write pattern
6. **qualified_name_lookup**: Compare ID vs qualified_name performance
7. **cache_stats_overhead**: Measure metrics collection overhead

Run benchmarks:
```bash
cd cortex/cortex
cargo bench --bench code_unit_cache
```

View results:
```bash
open target/criterion/report/index.html
```

## Monitoring

### Cache Statistics

Get current cache statistics:
```rust
let stats = service.cache_stats();
println!("Hit rate: {:.2}%", stats.hit_rate);
println!("Total requests: {}", stats.total_requests);
println!("Cache hits: {}", stats.hits);
println!("Cache misses: {}", stats.misses);
println!("Invalidations: {}", stats.invalidations);
```

### Recommended Metrics to Track

1. **Hit Rate**: Should be >60% for typical workloads
   - <50%: Consider increasing cache size or TTL
   - >90%: Cache is well-tuned

2. **Cache Size**: Monitor actual vs configured capacity
   - Use `cache_by_id.entry_count()` (available in moka)

3. **Invalidations**: Track update frequency
   - High invalidation rate may indicate cache thrashing

4. **Memory Usage**: Monitor with system tools
   - Expected: capacity * avg_entry_size

### Alerting Thresholds

- Hit rate < 40%: Investigate access patterns
- Invalidations > hits: Reconsider caching strategy
- Memory usage > expected: Check for memory leaks

## Migration Guide

### Existing Code

No changes required for existing code! The cache is transparent:

```rust
// This code works exactly the same
let unit = service.get_code_unit("unit:123").await?;
```

### New Features

To leverage new features:

```rust
// Custom cache configuration
let config = CacheConfig {
    max_capacity: 50_000,
    ttl_seconds: 600,
    tti_seconds: 120,
};
let service = CodeUnitService::with_cache_config(storage, config);

// Monitor cache performance
let stats = service.cache_stats();
if stats.hit_rate < 40.0 {
    warn!("Low cache hit rate: {:.2}%", stats.hit_rate);
}

// Clear cache after bulk operations
service.clear_cache().await;
```

## Troubleshooting

### Low Hit Rate

**Symptoms**: Cache statistics show <40% hit rate

**Causes**:
1. Access patterns are too random (no temporal locality)
2. Cache capacity too small for working set
3. TTL/TTI too aggressive (entries expire before reuse)

**Solutions**:
1. Increase `max_capacity`
2. Increase `ttl_seconds` and `tti_seconds`
3. Analyze access patterns with logging

### High Memory Usage

**Symptoms**: Service consuming more memory than expected

**Causes**:
1. Cache capacity set too high
2. Large CodeUnit bodies stored in cache
3. Memory leak in application code

**Solutions**:
1. Reduce `max_capacity`
2. Consider not caching unit bodies for large functions
3. Use memory profiler to identify leaks

### Cache Thrashing

**Symptoms**: High invalidation rate, frequent evictions

**Causes**:
1. High update frequency
2. Cache capacity too small for access pattern
3. TTL too short

**Solutions**:
1. Increase cache capacity
2. Increase TTL
3. Consider write-through caching for frequently updated units

## Future Enhancements

Potential improvements:

1. **Tiered Caching**: Add L1 (in-memory) and L2 (Redis) tiers
2. **Selective Caching**: Don't cache large function bodies
3. **Batch Operations**: Batch cache populate operations
4. **Cache Warming**: Pre-populate cache with frequently accessed units
5. **Metrics Export**: Expose metrics to Prometheus/monitoring systems
6. **Cache Partitioning**: Separate caches per workspace
7. **Compression**: Compress cached entries to reduce memory usage

## References

- [Moka Documentation](https://docs.rs/moka/)
- [Cache-Aside Pattern](https://docs.microsoft.com/en-us/azure/architecture/patterns/cache-aside)
- [LRU Cache Algorithm](https://en.wikipedia.org/wiki/Cache_replacement_policies#Least_recently_used_(LRU))

## Dependencies Added

```toml
[dependencies]
moka = { version = "0.12", features = ["future"] }

[dev-dependencies]
criterion = { workspace = true }
fastrand = "2.2.0"
```

## Files Modified/Created

### Modified
1. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/Cargo.toml` - Added dependencies
2. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/src/services/code_units.rs` - Added cache implementation

### Created
1. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/tests/code_unit_cache_tests.rs` - Unit tests
2. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/tests/code_unit_cache_integration.rs` - Integration tests
3. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/benches/code_unit_cache.rs` - Benchmarks
4. `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex/CACHE_IMPLEMENTATION.md` - This documentation
