# Qdrant Integration Test Suite

Comprehensive end-to-end tests and benchmarks for validating the Cortex system with Qdrant vector database integration.

## Overview

This test suite provides complete validation of:
- Full project ingestion to semantic search workflow
- Multi-language development scenarios
- Migration from HNSW to Qdrant
- Performance benchmarking and comparison
- Real-world development workflows
- Failure scenarios and recovery

## Test Files Created

### 1. **e2e_qdrant_integration.rs** (953 lines)
Complete workflow validation from project ingestion through semantic search.

**Test Coverage:**
- `test_1_complete_workflow_with_qdrant`: End-to-end workflow validation
  - Creates realistic Rust project (5 files: main.rs, lib.rs, database.rs, models.rs, api.rs)
  - Parses and extracts code units
  - Generates embeddings and indexes in Qdrant
  - Performs semantic searches with various queries
  - Validates search latency (<500ms threshold)
  - Tests filtered searches (by entity type, metadata)
  - Verifies data persistence and consistency

- `test_2_memory_consolidation_with_qdrant`: Memory system integration
  - Creates 20 semantic units with varying complexity
  - Tests memory recall from both cognitive manager and Qdrant
  - Validates semantic clustering

- `test_3_hnsw_to_qdrant_migration`: Migration workflow validation
  - Populates HNSW index with 50 vectors
  - Creates hybrid store for dual-write
  - Migrates data batch-by-batch
  - Verifies consistency between HNSW and Qdrant
  - Compares search results (>60% overlap threshold)
  - Switches to Qdrant as primary

- `test_4_performance_and_stress`: Performance validation
  - Bulk insertion: 500 vectors
  - Throughput requirements: >50 vectors/sec insert, >20 searches/sec
  - Concurrent search stress: 100 searches across 10 parallel threads
  - Mixed read/write operations

- `test_5_failure_scenarios`: Error handling and recovery
  - Dimension mismatch handling
  - Empty index searches
  - Concurrent write stress (20 parallel writes)
  - Recovery validation

**Key Metrics:**
- Search latency threshold: 500ms
- Minimum search recall: 80%
- Embedding dimension: 384 (MiniLM model)

### 2. **e2e_cortex_workflow.rs** (1,020 lines)
Real-world development workflow simulation across multiple languages.

**Test Coverage:**
- `test_1_multi_language_project`: Cross-language development
  - Creates Rust backend (2 files: HTTP server, database layer)
  - Creates TypeScript API client (2 files: client, validation)
  - Creates React TSX components (2 files: UserList, UserForm)
  - Tests cross-language semantic searches
  - Validates language-specific filtered searches

- `test_2_iterative_refactoring`: Refactoring workflow
  - Initial implementation of pricing logic
  - Refactoring iteration 1: Extract discount calculation
  - Refactoring iteration 2: Introduce discount tier enum
  - Verifies all versions are searchable

- `test_3_multi_agent_collaboration`: Concurrent multi-agent scenario
  - Creates 5 agent sessions
  - Each agent creates their own files concurrently
  - Verifies all agent contributions are indexed
  - Tests concurrent modifications

- `test_4_consistency_verification`: SurrealDB-Qdrant consistency
  - Creates 50 semantic units
  - Stores in both SurrealDB (cognitive) and Qdrant (search)
  - Verifies count consistency
  - Tests retrieval from both systems

- `test_5_large_batch_stress`: Large-scale operations
  - Batch insert: 1,000 documents
  - Throughput requirements: >100 docs/sec
  - Concurrent searches: 100 searches across 20 parallel threads
  - Mixed workload: 500 inserts + 500 searches

- `test_6_failure_recovery`: Recovery scenarios
  - Partial batch failure handling (>90% success rate)
  - Index clear and recovery
  - Re-population after clear

**Key Features:**
- Multi-language support: Rust, TypeScript, TSX
- Realistic code samples with proper syntax
- Concurrent operation testing
- Performance threshold validation

### 3. **migration_e2e.rs** (800 lines)
Complete migration workflow from HNSW to Qdrant with zero-downtime validation.

**Test Coverage:**
- `test_1_complete_migration_workflow`: Full migration lifecycle
  - Phase 1: Establish HNSW baseline (1,000 vectors)
  - Phase 2: Enable dual-write mode
  - Phase 3: Migrate existing data to Qdrant (batch size: 100)
  - Phase 4: Verify consistency (>90% consistency rate)
  - Phase 5: Test dual-write with new data (50 vectors)
  - Phase 6: Switch to Qdrant as primary
  - Phase 7: Measure post-migration performance
  - Phase 8: Verify search result quality (>60% similarity)
  - Phase 9: Generate comprehensive migration report

- `test_2_dual_verify_mode`: Dual verification testing
  - Populates both HNSW and Qdrant
  - Enables DualVerify mode
  - Performs 20 searches with verification
  - Validates consistency checks are performed

- `test_3_rollback_scenario`: Rollback capability validation
  - Starts migration with dual-write
  - Performs partial migration (50% of data)
  - Simulates issue and rolls back to SingleStore mode
  - Verifies HNSW still works correctly
  - Tests post-rollback insertions

- `test_4_incremental_migration_with_traffic`: Live traffic simulation
  - Migration thread: Batch migrates 1,000 vectors
  - 5 concurrent read threads: 20 searches each
  - 2 concurrent write threads: 10 inserts each
  - Verifies final state consistency

- `test_5_performance_regression_detection`: Performance comparison
  - Measures detailed HNSW performance (avg, P50, P95, P99)
  - Measures detailed Qdrant performance
  - Calculates regression percentage
  - Asserts <50% performance regression

**Key Metrics:**
- Migration dataset: 1,000 vectors
- Consistency check samples: 100 vectors
- Consistency threshold: 90%
- Result similarity threshold: 60%
- Performance regression limit: 50%

### 4. **qdrant_benchmark.rs** (569 lines)
Comprehensive performance benchmarks comparing HNSW and Qdrant.

**Benchmark Groups:**

1. **Insert Performance**
   - `bench_insert_single`: Single vector insertion
   - `bench_insert_batch`: Batch insertion (100, 1,000 vectors)
   - Compares HNSW vs Qdrant throughput

2. **Search Performance**
   - `bench_search_varying_k`: Search with k=[1, 5, 10, 20, 50]
   - `bench_search_dataset_size`: Search on [100, 1K, 10K] vectors
   - Measures latency at different scales

3. **Quantization Impact**
   - `bench_quantization_impact`: Three scenarios:
     - No quantization (baseline)
     - Scalar quantization (8-bit)
     - Product quantization (higher compression)
   - Measures search latency impact

4. **Concurrent Operations**
   - `bench_concurrent_operations`: Sequential baseline vs concurrent
   - Tests concurrency levels: [5, 10, 20] parallel searches
   - Measures throughput improvements

5. **Mixed Workload**
   - `bench_mixed_workload`: Interleaved inserts and searches
   - 100 inserts + 50 searches interleaved
   - Simulates real-world usage patterns

**Benchmark Configuration:**
- Sample size: 10-50 per benchmark
- Warm-up time: 2 seconds
- Measurement time: 10-20 seconds
- Embedding dimension: 384

## Prerequisites

### Required Services

1. **Qdrant Server**
   ```bash
   # Using Docker
   docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant

   # Or using Docker Compose (recommended)
   docker-compose up -d qdrant
   ```

2. **Environment Variables**
   ```bash
   export QDRANT_URL="http://localhost:6333"
   export QDRANT_API_KEY=""  # Optional, for cloud instances
   ```

### Optional Configuration

For custom Qdrant configurations:
```bash
export QDRANT_URL="http://your-qdrant-server:6333"
export QDRANT_API_KEY="your-api-key"
```

## Running Tests

### Run All Qdrant Tests
```bash
# Run all Qdrant integration tests (requires Qdrant server)
cargo test --test e2e_qdrant_integration --test e2e_cortex_workflow --test migration_e2e -- --ignored

# Or run individually:
cargo test --test e2e_qdrant_integration -- --ignored
cargo test --test e2e_cortex_workflow -- --ignored
cargo test --test migration_e2e -- --ignored
```

### Run Specific Tests
```bash
# Complete workflow test
cargo test --test e2e_qdrant_integration test_1_complete_workflow_with_qdrant -- --ignored --nocapture

# Migration workflow
cargo test --test migration_e2e test_1_complete_migration_workflow -- --ignored --nocapture

# Multi-language project
cargo test --test e2e_cortex_workflow test_1_multi_language_project -- --ignored --nocapture

# Performance stress test
cargo test --test e2e_cortex_workflow test_5_large_batch_stress -- --ignored --nocapture
```

### Run Benchmarks
```bash
# Run all Qdrant benchmarks
cargo bench --bench qdrant_benchmark

# Run specific benchmark group
cargo bench --bench qdrant_benchmark -- insert
cargo bench --bench qdrant_benchmark -- search
cargo bench --bench qdrant_benchmark -- quantization
cargo bench --bench qdrant_benchmark -- concurrent
cargo bench --bench qdrant_benchmark -- mixed

# Generate HTML reports (located in target/criterion/)
cargo bench --bench qdrant_benchmark -- --verbose
```

## Test Output and Reporting

### Test Metrics

Tests report the following metrics:
- **Throughput**: Vectors/second for insertions
- **Latency**: Milliseconds for search operations
- **Consistency Rate**: Percentage of consistent results between stores
- **Search Recall**: Percentage of successful searches
- **Performance Regression**: Percentage change in performance

### Example Output
```
========================================
TEST 1: Complete Workflow with Qdrant
========================================
Phase 1: Setting up Rust project
Created 5 files
Phase 2: Parsing code and extracting units
Extracted 5 code units
Phase 3: Indexing code units in Qdrant
Indexed 5 units in 1.23s (4.07 units/sec)
Phase 4: Testing semantic search queries
Searching for: 'database connection'
  Found 2 results in 45ms: Should find database.rs code
    [1] src/database.rs (score: 0.876)
    [2] src/main.rs (score: 0.654)
Average search latency: 52ms
✅ TEST 1 PASSED in 5.67s
  - Indexed: 5 units
  - Searches: 5/5 successful
  - Avg latency: 52ms
  - Recall: 100.0%
```

### Benchmark Reports

Criterion generates detailed HTML reports in `target/criterion/`:
- Performance graphs
- Statistical analysis
- Comparison between runs
- Regression detection

View reports:
```bash
open target/criterion/report/index.html
```

## Test Design Philosophy

### Deterministic Testing
- Uses fixed seed for random data generation
- Reproducible test vectors
- Consistent embedding generation (mock provider for tests)

### Performance Thresholds
All tests include reasonable performance thresholds:
- Search latency: <500ms
- Insert throughput: >50 vectors/sec
- Search throughput: >20 searches/sec
- Concurrent operations: >90% success rate

### Comprehensive Coverage
Tests cover:
- ✅ Happy path workflows
- ✅ Error scenarios
- ✅ Concurrent operations
- ✅ Performance characteristics
- ✅ Data consistency
- ✅ Migration workflows
- ✅ Recovery procedures

## Continuous Integration

### CI Configuration Example
```yaml
name: Qdrant Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      qdrant:
        image: qdrant/qdrant:latest
        ports:
          - 6333:6333
          - 6334:6334
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run Qdrant tests
        run: |
          export QDRANT_URL="http://localhost:6333"
          cargo test --test e2e_qdrant_integration -- --ignored
          cargo test --test e2e_cortex_workflow -- --ignored
          cargo test --test migration_e2e -- --ignored
```

## Troubleshooting

### Common Issues

1. **Qdrant connection refused**
   - Ensure Qdrant server is running: `docker ps`
   - Check QDRANT_URL environment variable
   - Verify port 6333 is accessible

2. **Tests timeout**
   - Increase test timeout: Add `#[tokio::test(flavor = "multi_thread")]`
   - Check system resources
   - Reduce dataset sizes for faster iteration

3. **Consistency check failures**
   - Wait longer for Qdrant indexing (adjust sleep duration)
   - Check Qdrant logs for errors
   - Verify collection configuration

4. **Performance regression**
   - Clear Qdrant collections between tests
   - Check system load
   - Verify quantization settings

### Debug Mode
Run tests with verbose logging:
```bash
RUST_LOG=debug cargo test --test e2e_qdrant_integration -- --ignored --nocapture
```

### Qdrant Health Check
```bash
curl http://localhost:6333/health
```

## Advanced Testing

### Custom Test Configuration

Create a custom config file `qdrant_test_config.toml`:
```toml
[qdrant]
url = "http://localhost:6333"
collection_prefix = "test_"
enable_quantization = true
quantization_type = "scalar"
write_batch_size = 100

[test]
dataset_size = 1000
search_sample_size = 50
embedding_dimension = 384
```

### Chaos Engineering

Tests include failure scenarios:
- Network partition simulation (via timeout testing)
- Partial write failures
- Concurrent modification conflicts
- Recovery from cleared indexes

### Performance Profiling

Profile specific test functions:
```bash
cargo flamegraph --test e2e_qdrant_integration -- test_1_complete_workflow_with_qdrant --ignored --exact
```

## Test Statistics

Total test coverage:
- **Test files**: 3
- **Benchmark file**: 1
- **Total lines**: 3,342
- **Test functions**: 17
- **Benchmark groups**: 7

Coverage breakdown:
- End-to-end workflows: 5 tests
- Migration scenarios: 5 tests
- Multi-language workflows: 6 tests
- Failure scenarios: 3 tests
- Performance benchmarks: 7 groups

## Contributing

When adding new tests:
1. Follow the existing test structure
2. Include comprehensive documentation
3. Add appropriate `#[ignore]` tags for integration tests
4. Include performance assertions
5. Document expected behavior in comments
6. Add to this README

## Future Enhancements

Planned additions:
- [ ] Distributed Qdrant cluster testing
- [ ] Network partition simulation (explicit chaos engineering)
- [ ] Data corruption recovery tests
- [ ] Performance regression detection automation
- [ ] Multi-tenancy testing
- [ ] Backup and restore workflow validation
- [ ] Cross-region replication testing

## License

MIT

## Support

For issues or questions:
- Open an issue on GitHub
- Check Qdrant documentation: https://qdrant.tech/documentation/
- Review test output logs
