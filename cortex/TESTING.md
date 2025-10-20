# Cortex System Testing Guide

## Overview

This document describes the comprehensive testing strategy for the Cortex cognitive memory system. The test suite includes unit tests, integration tests, end-to-end workflow tests, and performance benchmarks.

## Test Organization

```
cortex/
├── cortex-core/
│   └── tests/
│       ├── config_integration.rs  (38 tests)
│       └── unit_tests.rs          (60+ tests - NEW)
├── cortex-storage/
│   └── tests/
│       ├── connection_pool_integration_tests.rs  (80+ tests)
│       ├── connection_pool_load_tests.rs
│       ├── surrealdb_manager_tests.rs
│       └── unit_tests.rs                        (40+ tests - NEW)
├── cortex-vfs/
│   └── tests/
│       └── integration_tests.rs
├── cortex-ingestion/
│   └── tests/
│       └── integration_tests.rs
├── cortex-memory/
│   └── tests/
│       ├── integration_tests.rs  (20 tests)
│       └── edge_case_tests.rs    (25 tests)
├── cortex-semantic/
│   └── tests/
│       └── integration_tests.rs
├── cortex-mcp/
│   └── tests/
│       └── integration_tests.rs
├── cortex-cli/
│   └── tests/
│       └── integration_tests.rs
└── tests/
    └── e2e_workflow_tests.rs  (13 comprehensive E2E tests - NEW)
```

## Test Categories

### 1. Unit Tests

Unit tests verify individual components in isolation.

#### cortex-core Unit Tests (`cortex-core/tests/unit_tests.rs`)

- **Error Handling** (13 tests)
  - Error creation for all error types
  - Error type checking (is_storage, is_database, is_not_found)
  - Error display formatting
  - Conversion from std::io::Error and serde_json::Error

- **ID Management** (8 tests)
  - ID uniqueness verification
  - UUID parsing and serialization
  - String conversion and roundtrip
  - Hash map usage

- **Core Types** (20+ tests)
  - Project, Document, Chunk, Embedding creation
  - Symbol and Relation types
  - SearchQuery builder pattern
  - All enum serialization (EntityType, SymbolKind, RelationType)

- **Metadata** (8 tests)
  - MetadataBuilder functionality
  - MetadataExtractor from paths and content
  - Option handling

#### cortex-storage Unit Tests (`cortex-storage/tests/unit_tests.rs`)

- **Query Builder** (12 tests)
  - SELECT statement construction
  - WHERE, ORDER BY, LIMIT clauses
  - Query chaining and composition

- **Pagination** (8 tests)
  - Pagination parameter creation
  - Serialization/deserialization
  - Edge cases (zero offset, large values)

- **Schema Validation** (15 tests)
  - Table definitions verification
  - Field definitions for all tables
  - Index verification
  - Type definitions

### 2. Integration Tests

Integration tests verify interactions between components within a crate.

#### cortex-storage Integration Tests (80+ tests)

- **Connection Pool** (`connection_pool_integration_tests.rs`)
  - Connection acquisition and reuse
  - Pool exhaustion handling
  - Concurrent access
  - Health monitoring
  - Circuit breaker
  - Retry policies
  - Graceful shutdown
  - Transaction support

- **Agent Sessions**
  - Session creation and management
  - Transaction recording
  - Resource limits
  - Concurrent sessions

- **Load Balancing**
  - Round-robin strategy
  - Least connections
  - Health-based routing

#### cortex-memory Integration Tests (45 tests)

- **Episodic Memory** (`integration_tests.rs`)
  - Episode storage and retrieval
  - Episode metadata tracking
  - Success/failure tracking

- **Semantic Memory**
  - Code unit storage
  - Dependency tracking
  - Complexity analysis
  - Quality metrics

- **Working Memory**
  - Priority-based eviction
  - Cache statistics
  - Capacity limits

- **Procedural Memory**
  - Pattern learning
  - Success rate tracking

- **Consolidation**
  - Memory consolidation workflows
  - Pattern extraction (dreaming)
  - Memory decay

- **Edge Cases** (`edge_case_tests.rs`)
  - Empty memories
  - Maximum data sizes
  - Circular dependencies
  - Concurrent access
  - Statistics accuracy

### 3. End-to-End Workflow Tests

E2E tests verify complete user workflows across all system components.

#### Workflow Tests (`cortex/tests/e2e_workflow_tests.rs`)

1. **Complete Workflow: Workspace to Search** (test_complete_workflow_workspace_creation_to_search)
   - Create workspace
   - Initialize database
   - Create project
   - Initialize cognitive memory
   - Store episodes
   - Retrieve and verify

2. **Multi-Agent Workflow** (test_multi_agent_workflow)
   - Multiple agents working concurrently
   - Shared connection pool
   - Independent session management
   - Combined statistics

3. **Memory Consolidation** (test_memory_consolidation_workflow)
   - Create multiple episodes
   - Consolidate memories
   - Extract patterns through dreaming
   - Verify statistics

4. **Semantic Code Analysis** (test_semantic_code_analysis_workflow)
   - Store semantic units
   - Retrieve code structure
   - Find complex units
   - Check quality metrics

5. **Working Memory to Long-term** (test_working_memory_to_longterm_workflow)
   - Store items in working memory
   - Priority management
   - Transfer to long-term storage

6. **Pattern Learning and Application** (test_pattern_learning_and_application_workflow)
   - Learn patterns from episodes
   - Apply patterns
   - Track success/failure rates

7. **Dependency Graph** (test_dependency_graph_workflow)
   - Create dependency chains
   - Multiple dependency types
   - Graph queries

8. **Incremental Consolidation** (test_incremental_consolidation_workflow)
   - Batch episode creation
   - Incremental processing
   - Performance verification

9. **Forget Low Importance** (test_forget_low_importance_workflow)
   - Varying importance episodes
   - Selective forgetting
   - Memory optimization

## Running Tests

### Run All Tests

```bash
cd cortex
cargo test --workspace
```

### Run Tests for Specific Crate

```bash
# Core tests
cargo test -p cortex-core

# Storage tests
cargo test -p cortex-storage

# Memory tests
cargo test -p cortex-memory

# E2E tests
cargo test --test e2e_workflow_tests
```

### Run Specific Test

```bash
cargo test test_complete_workflow_workspace_creation_to_search
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

### Run Tests in Parallel (default)

```bash
cargo test
```

### Run Tests Serially

```bash
cargo test -- --test-threads=1
```

### Run Only Integration Tests

```bash
cargo test --test '*_integration_tests'
```

### Run Only Unit Tests

```bash
cargo test --lib
```

## Test Infrastructure

### Test Utilities

- **tempfile**: Temporary directories for filesystem isolation
- **tokio::test**: Async test runtime
- **Arc and Clone**: Shared test resources
- **In-memory Database**: `mem://` for fast, isolated tests

### Test Helpers

Common patterns used across tests:

```rust
// Create test database config
fn create_test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "test".to_string(),
        database: "test".to_string(),
    }
}

// Create test workspace
async fn create_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_workspace");
    fs::create_dir_all(&workspace_path).await.unwrap();
    (temp_dir, workspace_path)
}

// Create cognitive manager
async fn create_test_manager() -> CognitiveManager {
    let config = create_test_db_config();
    let manager = Arc::new(
        ConnectionManager::new(config).await.unwrap()
    );
    CognitiveManager::new(manager)
}
```

## Coverage Goals

### Current Coverage (Estimated)

| Crate            | Unit Tests | Integration Tests | E2E Tests | Coverage |
|------------------|------------|-------------------|-----------|----------|
| cortex-core      | 60+        | 38                | ✓         | ~85%     |
| cortex-storage   | 40+        | 80+               | ✓         | ~90%     |
| cortex-memory    | -          | 45+               | ✓         | ~80%     |
| cortex-vfs       | -          | Basic             | ✓         | ~60%     |
| cortex-ingestion | -          | Basic             | ✓         | ~60%     |
| cortex-semantic  | -          | Basic             | ✓         | ~60%     |
| cortex-mcp       | -          | Basic             | ✓         | ~50%     |
| cortex-cli       | -          | Basic             | ✓         | ~50%     |

### Coverage Target: 80%+

To achieve 80%+ code coverage:

1. **Add missing unit tests**:
   - cortex-vfs modules
   - cortex-ingestion processors
   - cortex-semantic search algorithms
   - cortex-mcp tool handlers

2. **Expand integration tests**:
   - Cross-crate workflows
   - Error scenarios
   - Edge cases

3. **Add performance benchmarks**:
   - Embedding generation
   - Search performance
   - Connection pool throughput

## Generating Coverage Reports

### Using cargo-tarpaulin

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cd cortex
cargo tarpaulin --workspace --out Html --output-dir ./coverage

# View report
open coverage/index.html
```

### Using cargo-llvm-cov

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Generate coverage
cd cortex
cargo llvm-cov --workspace --html

# View report
open target/llvm-cov/html/index.html
```

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cd cortex && cargo test --workspace
      - name: Generate coverage
        run: cargo tarpaulin --workspace --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

## Best Practices

### 1. Test Isolation

- Each test uses its own temporary directory
- In-memory database for speed and isolation
- No shared mutable state between tests

### 2. Async Testing

- Use `#[tokio::test]` for async tests
- Properly await all operations
- Clean up resources with Drop or explicit cleanup

### 3. Error Testing

- Test both success and failure paths
- Verify error types and messages
- Test error propagation

### 4. Property-Based Testing

- Use `proptest` for property-based tests (future enhancement)
- Test invariants across random inputs
- Catch edge cases automatically

### 5. Performance Testing

- Benchmark critical paths
- Set performance budgets
- Regression testing for performance

## Test Maintenance

### Adding New Tests

1. Identify component to test
2. Choose appropriate test type (unit/integration/e2e)
3. Create test file in appropriate location
4. Follow naming convention: `test_<component>_<scenario>`
5. Add test documentation
6. Update this TESTING.md

### Test Naming Convention

- Unit tests: `test_<function>_<scenario>`
- Integration tests: `test_<component>_<workflow>`
- E2E tests: `test_<feature>_workflow`

### Documentation

Each test should have:
- Clear name describing what is tested
- Comments explaining complex setup
- Assertions with meaningful messages

## Troubleshooting

### Common Issues

1. **Tests timeout**
   - Increase timeout: `#[tokio::test(flavor = "multi_thread")]`
   - Check for deadlocks
   - Verify async operations complete

2. **Flaky tests**
   - Avoid time-dependent assertions
   - Ensure proper cleanup
   - Check for race conditions

3. **Database errors**
   - Verify SurrealDB is running (if not using mem://)
   - Check connection configuration
   - Ensure schema is initialized

4. **File system errors**
   - Use tempfile for isolation
   - Clean up resources
   - Check permissions

## Future Enhancements

### Planned Test Additions

1. **Property-Based Tests**
   - ID generation properties
   - Query builder correctness
   - Search result consistency

2. **Fuzz Testing**
   - Input validation
   - Parser robustness
   - Error handling

3. **Load Testing**
   - Connection pool under load
   - Memory consolidation performance
   - Search throughput

4. **Chaos Testing**
   - Network failures
   - Database crashes
   - Resource exhaustion

5. **Integration with External Systems**
   - Real SurrealDB clusters
   - External embedding services
   - File system watchers

## Metrics and Reporting

### Test Execution Metrics

Track these metrics in CI:

- Total test count
- Test execution time
- Coverage percentage
- Failed test count
- Flaky test detection

### Quality Gates

Before merging:
- All tests must pass
- Coverage must be ≥ 80%
- No new clippy warnings
- Documentation updated

## Summary

The Cortex test suite provides comprehensive coverage of the system through:

- **200+ total tests** across all components
- **Unit tests** for individual functions and types
- **Integration tests** for component interactions
- **E2E tests** for complete workflows
- **Proper isolation** using temporary resources
- **Fast execution** with in-memory databases
- **Clear documentation** for maintainability

The test suite ensures reliability, catches regressions, and enables confident refactoring of the Cortex cognitive memory system.
