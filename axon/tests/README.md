# Axon Multi-Agent System Test Suite

This directory contains comprehensive unit and integration tests for the Axon multi-agent system.

## Test Organization

### Unit Tests

#### 1. **unit_orchestrator_worker.rs**
Tests for the Orchestrator-Worker pattern implementation following Anthropic's research:

- **Query Complexity Tests**: Validates complexity analysis (Simple/Medium/Complex) and resource allocation
- **Strategy Library Tests**: Tests strategy selection, pattern matching, and custom strategy registration
- **Worker Registry Tests**: Validates worker registration, status transitions, capability matching, and pool management
- **Task Delegation Tests**: Tests task builder, validation, boundaries, and resource limits
- **Result Synthesizer Tests**: Tests result aggregation and quality metrics

**Key Features Tested:**
- Resource allocation based on query complexity (1 worker for simple, 4 for medium, 10+ for complex)
- Dynamic worker spawning and capability matching
- Parallel execution strategies
- Worker lifecycle management

#### 2. **unit_parallel_executor.rs**
Tests for the Parallel Tool Execution system:

- **Tool Call Tests**: Validates tool call creation, dependencies, and priority ordering
- **Parallel Execution Tests**: Tests independent tool parallelization and performance
- **Dependency Resolution Tests**: Tests topological sorting and DAG execution
- **Error Handling Tests**: Tests partial failures and error propagation
- **Performance Tests**: Validates 70-90% time reduction for 3+ independent tools

**Key Features Tested:**
- Dependency graph construction and cycle detection
- Concurrent execution with semaphore-based concurrency control
- Performance optimization for independent tools
- Max concurrent limit enforcement

#### 3. **unit_runtime.rs**
Tests for the Agent Runtime System:

- **Runtime Configuration Tests**: Tests default configs, custom configs, and validation
- **Process Manager Tests**: Tests process spawning, killing, health checks, and resource limits
- **MCP Server Pool Tests**: Tests MCP server registration and tool execution
- **Agent Executor Tests**: Tests task execution, statistics, and status transitions
- **Runtime Lifecycle Tests**: Tests start/stop cycles, state management, and statistics tracking

**Key Features Tested:**
- Process isolation and resource management
- MCP integration for tool execution
- Health monitoring and recovery
- Graceful shutdown with active agents

#### 4. **unit_rest_api.rs**
Tests for the REST API endpoints:

- **API Info Tests**: Tests API documentation endpoints
- **Health Check Tests**: Tests health status and uptime reporting
- **Agent Management Tests**: Tests create, list, get, pause, resume, restart, delete endpoints
- **Workflow Management Tests**: Tests workflow execution, listing, and control endpoints
- **Metrics Tests**: Tests metrics collection and telemetry endpoints
- **Configuration Tests**: Tests config get, update, and validation
- **Error Handling Tests**: Tests invalid endpoints, methods, and malformed JSON

**Key Features Tested:**
- RESTful API surface
- Request/response validation
- Error handling and status codes
- CORS and content-type headers

### Integration Tests

#### 5. **integration_agent_lifecycle.rs**
End-to-end tests for complete agent lifecycle:

- **Full Lifecycle Tests**: Tests agent creation → initialization → execution → shutdown
- **State Transition Tests**: Tests all valid state transitions
- **Task Execution Tests**: Tests single and multiple task executions
- **Resource Management Tests**: Tests memory limits, CPU limits, and concurrent agent limits
- **Error Recovery Tests**: Tests failure handling and auto-restart
- **Graceful Shutdown Tests**: Tests shutdown with active agents and timeout enforcement
- **Statistics Tests**: Tests metrics collection and tracking
- **Concurrent Operations Tests**: Tests parallel agent operations

**Integration Points:**
- Runtime ↔ Process Manager
- Runtime ↔ Message Bus
- Runtime ↔ Agent Executor
- Runtime ↔ MCP Server Pool

#### 6. **integration_workflow_execution.rs**
Integration tests for workflow execution:

- **Workflow Creation Tests**: Tests simple, parallel, and complex DAG workflows
- **Validation Tests**: Tests cycle detection and missing dependency detection
- **Status Tests**: Tests task and workflow status transitions
- **Metadata Tests**: Tests priority ordering and timeout enforcement
- **Error Handling Tests**: Tests workflow timeout and retry logic
- **Dependency Tests**: Tests empty graphs, linear chains, and multiple dependencies
- **Result Tests**: Tests success and failure result structures

**Workflow Patterns Tested:**
- Sequential execution (A → B → C)
- Parallel execution (A, B, C all independent)
- Diamond pattern (A → B,C → D)
- Complex DAGs with multiple levels

#### 7. **integration_websocket_multiagent.rs**
Integration tests for WebSocket and multi-agent coordination:

- **WebSocket Manager Tests**: Tests connection management and broadcasting
- **Message Bus Tests**: Tests pub/sub, topics, and multiple subscribers
- **Message Coordinator Tests**: Tests agent registration and message routing
- **Coordination Pattern Tests**: Tests Star, Mesh, and Pipeline patterns
- **Consensus Protocol Tests**: Tests voting, majority, and unanimous consensus
- **Multi-Agent Coordination Tests**: Tests discovery, broadcast, and hierarchical patterns
- **Conflict Resolution Tests**: Tests conflicting votes and tie-breaking
- **Real-time Communication Tests**: Tests message delivery and ordering
- **Scalability Tests**: Tests 100+ agent broadcasting and concurrent publishing

**Coordination Patterns:**
- **Star**: Central coordinator with worker agents
- **Mesh**: Fully connected agent network
- **Pipeline**: Sequential processing pipeline

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test File
```bash
# Unit tests
cargo test --test unit_orchestrator_worker
cargo test --test unit_parallel_executor
cargo test --test unit_runtime
cargo test --test unit_rest_api

# Integration tests
cargo test --test integration_agent_lifecycle
cargo test --test integration_workflow_execution
cargo test --test integration_websocket_multiagent
```

### Run Specific Test
```bash
cargo test test_query_complexity_simple_allocation
cargo test test_parallel_workflow_execution
cargo test test_full_agent_lifecycle
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Tests in Parallel
```bash
cargo test -- --test-threads=4
```

## Test Coverage

### Unit Test Coverage
- **Orchestrator-Worker Pattern**: ~30 tests
- **Parallel Tool Executor**: ~20 tests
- **Agent Runtime**: ~40 tests
- **REST API**: ~35 tests

### Integration Test Coverage
- **Agent Lifecycle**: ~20 tests
- **Workflow Execution**: ~25 tests
- **Multi-Agent Coordination**: ~35 tests

**Total**: ~205 tests covering all major components

## Test Environment

### Requirements
- Rust 1.70+ (2024 edition)
- Tokio async runtime
- Mock Cortex server (for some tests)
- Test fixtures in `tests/fixtures/`

### Environment Variables
Some tests may require:
- `AXON_API_KEY`: For API authentication tests
- `TEST_CORTEX_PATH`: Path to test Cortex binary

## Test Helpers

### Common Test Utilities
Located in `tests/common/mod.rs`:

- `MockCortexServer`: Mock Cortex for testing
- `create_test_agent()`: Create agents for testing
- `create_test_vote()`: Create consensus votes
- `create_test_episode()`: Create episodic memories
- `create_test_pattern()`: Create learned patterns
- `wait_for_condition()`: Async condition waiter
- `create_test_capability_matcher()`: Capability matching setup

## Notes

### API Compatibility
These tests were written against the Axon multi-agent system API as of October 2025. Some tests may need updates if the API evolves.

### Mock vs Real Services
- Most unit tests use mocks and don't require external services
- Integration tests may spawn real processes (using `echo` or `sleep` for safety)
- WebSocket tests work with in-memory message bus
- MCP tests may require actual MCP server for full validation

### Performance Tests
Some tests include timing assertions for parallelization benefits:
- Parallel tool executor should achieve 70-90% time reduction
- Sequential timing tests may be flaky on slow systems

### Known Limitations
- Some tests use simple binaries (`echo`, `sleep`) instead of actual agent processes
- WebSocket connection tests don't establish actual WebSocket connections (manager API only)
- Full end-to-end tests with real Cortex require external setup

## Contributing

When adding new tests:

1. **Follow the naming convention**: `test_<feature>_<scenario>`
2. **Use descriptive assertions**: Make failures easy to diagnose
3. **Clean up resources**: Use proper shutdown/cleanup in async tests
4. **Document complex scenarios**: Add comments for non-obvious test logic
5. **Group related tests**: Use `mod` blocks for organization
6. **Update this README**: Add documentation for new test files

## Continuous Integration

These tests are designed to run in CI environments:
- All tests should pass without external dependencies (for unit tests)
- Integration tests may be skipped if required binaries are unavailable
- Performance tests have generous timeouts for CI variability

## Performance Benchmarks

For performance testing, see `benches/` directory:
- `binary_discovery_bench.rs`
- `session_bench.rs`
- `serialization_bench.rs`
- `version_bench.rs`

Run with:
```bash
cargo bench
```

## Test Metrics

Expected test execution times (on modern hardware):
- **Unit tests**: < 1 second per file
- **Integration tests**: 2-5 seconds per file
- **Full test suite**: < 30 seconds

## Troubleshooting

### Tests timing out
- Increase timeout with `--test-threads=1`
- Check for deadlocks in async code
- Verify cleanup code runs properly

### Flaky tests
- Tests involving timing may fail on slow systems
- Increase duration tolerances if needed
- Use `cargo test -- --test-threads=1` to reduce contention

### Compilation errors
- Ensure all dependencies are up to date
- Check that feature flags are correct
- Verify Rust edition is 2024

## References

- [Anthropic Multi-Agent Research](https://www.anthropic.com/research/multi-agent-systems)
- [Axon Architecture Documentation](../docs/)
- [API Documentation](../src/commands/api/README.md)
- [Cortex Integration Guide](../CORTEX_INTEGRATION_SUMMARY.md)
