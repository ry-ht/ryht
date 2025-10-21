# Benchmarking Guide

This document describes the benchmark suite for the MCP Server framework and how to use it.

## Overview

The benchmark suite measures performance of critical operations to ensure they meet target metrics:

| Operation | Target | Benchmark Group |
|-----------|--------|-----------------|
| Tool registration | < 1μs | `tool_registration` |
| Tool lookup | < 1μs | `tool_lookup` |
| Schema generation | < 10μs | `schema_generation` |
| Request parsing | < 100μs | `request_parsing` |
| Tool execution overhead | < 50μs | `tool_execution_overhead` |
| Middleware chain | < 10μs | `middleware_chain` |
| Hook emission | < 5μs | `hook_emission` |
| Full request cycle | < 500μs | `full_request_cycle` |

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmark Group

```bash
cargo bench --bench benchmarks tool_registration
cargo bench --bench benchmarks request_parsing
```

### Run with Baseline Comparison

```bash
# Save current performance as baseline
cargo bench --bench benchmarks -- --save-baseline main

# Make changes...

# Compare against baseline
cargo bench --bench benchmarks -- --baseline main
```

## Benchmark Groups

### 1. Tool Registration

Measures the overhead of registering tools in the registry.

```bash
cargo bench tool_registration
```

Tests:
- Single tool registration
- Batch registration (10 tools)

### 2. Tool Lookup

Measures tool retrieval performance with varying registry sizes.

```bash
cargo bench tool_lookup
```

Tests:
- `get()` - Retrieve tool by name
- `has()` - Check tool existence
- `list()` - List all tools

Registry sizes tested: 1, 10, 50, 100 tools

### 3. Schema Generation

Measures JSON schema generation performance.

```bash
cargo bench schema_generation
```

Tests:
- Simple schema (minimal properties)
- Complex schema (nested objects, arrays, enums)
- Schema with output validation

### 4. Request Parsing

Measures JSON-RPC request deserialization.

```bash
cargo bench request_parsing
```

Tests:
- Simple request (method only)
- Request with parameters
- Complex request (initialize with full params)

### 5. Tool Execution Overhead

Measures the framework overhead separate from tool logic.

```bash
cargo bench tool_execution_overhead
```

Tests:
- Minimal tool (returns immediately)
- Tool with JSON serialization
- Tool with complex input/output

### 6. Middleware Chain

Measures middleware processing performance.

```bash
cargo bench middleware_chain
```

Tests:
- No middleware baseline
- Single middleware (logging)
- Single middleware (metrics)
- Multiple middlewares chained

### 7. Hook Emission

Measures event hook system performance.

```bash
cargo bench hook_emission
```

Tests:
- No hooks registered (baseline)
- Single hook
- Multiple hooks (5)
- Different event types

### 8. Full Request Cycle

Measures end-to-end request handling.

```bash
cargo bench full_request_cycle
```

Tests:
- `initialize` request
- `tools/list` request
- `tools/call` with simple tool
- `tools/call` with complex tool

### 9. Concurrent Requests

Measures performance under concurrent load.

```bash
cargo bench concurrent_requests
```

Tests different concurrency levels: 1, 5, 10, 20 simultaneous requests

## Analyzing Results

### Understanding Output

```
tool_registration/single_tool
                        time:   [487.23 ns 491.45 ns 496.12 ns]
                        change: [-2.1234% -0.5678% +1.2345%] (p = 0.23 > 0.05)
                        No change in performance detected.
```

- **time**: Mean execution time with confidence interval
- **change**: Performance change vs previous run
- **p-value**: Statistical significance (< 0.05 is significant)

### Performance Regression Detection

If you see:

```
change: [+15.234% +17.456% +19.678%] (p = 0.00 < 0.05)
Performance has regressed.
```

This indicates a significant performance degradation that should be investigated.

## Continuous Benchmarking

The CI pipeline runs benchmarks on every commit and tracks results over time.

### Viewing Historical Data

GitHub Actions stores benchmark results. Check the "Benchmarks" job in CI runs.

### Performance Alerts

The CI is configured to alert on:
- Performance regression > 200%
- Significant degradation in critical paths

## Profiling

For deeper performance analysis:

### CPU Profiling with Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bench benchmarks -- --bench tool_registration
```

### Memory Profiling

```bash
cargo install cargo-instruments
cargo instruments -t alloc --bench benchmarks
```

### Performance Analysis with Valgrind

```bash
cargo build --release --bench benchmarks
valgrind --tool=callgrind ./target/release/deps/benchmarks-*
kcachegrind callgrind.out.*
```

## Adding New Benchmarks

1. Add benchmark function to `benches/benchmarks.rs`:

```rust
fn bench_my_feature(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("my_feature");

    group.bench_function("test_case", |b| {
        b.to_async(&rt).iter(|| async {
            // Benchmark code here
            black_box(result);
        });
    });

    group.finish();
}
```

2. Add to criterion group:

```rust
criterion_group!(
    benches,
    // ... existing benchmarks
    bench_my_feature,
);
```

3. Run and verify:

```bash
cargo bench my_feature
```

## Best Practices

1. **Use `black_box()`**: Prevents compiler optimization from eliminating code
2. **Async benchmarks**: Use `b.to_async(&rt)` for async operations
3. **Setup/teardown**: Exclude from measurements using `iter_batched`
4. **Warmup**: Criterion handles warmup automatically
5. **Sample size**: Adjust with `sample_size()` for slow benchmarks
6. **Statistical significance**: Default 100 samples, 5% significance

## Troubleshooting

### Benchmarks Fail to Compile

```bash
cargo clean
cargo bench --no-run
```

### Unstable Results

- Close other applications
- Use `sample_size()` to increase samples
- Run on isolated hardware
- Check for background processes

### CI Benchmark Failures

- Check for platform-specific issues
- Verify GitHub Actions runner has resources
- Review timeout settings

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
