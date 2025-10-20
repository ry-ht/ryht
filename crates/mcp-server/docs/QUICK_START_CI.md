# Quick Start: CI & Benchmarks

## Pre-Commit Checklist

Before committing code, run these commands:

```bash
# 1. Format code
cargo fmt

# 2. Check lints
cargo clippy --all-targets --all-features -- -D warnings

# 3. Run tests
cargo test --all-features

# 4. Build documentation
cargo doc --all-features --no-deps

# 5. Run benchmarks (optional)
cargo bench
```

## One-Line CI Check

```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
```

## Running Benchmarks

### Quick Benchmark

```bash
# Run all benchmarks
cargo bench

# Run specific group
cargo bench tool_registration
```

### Baseline Comparison

```bash
# Save baseline
cargo bench -- --save-baseline main

# After changes, compare
cargo bench -- --baseline main
```

## CI Pipeline Status

View pipeline: `https://github.com/[user]/[repo]/actions`

### Jobs

1. **Check** - Format & Lint (~2 min)
2. **Test** - Build & Test on 9 platforms (~5-8 min)
3. **Features** - Test feature flags (~3 min)
4. **Benchmarks** - Performance tracking (~5 min)
5. **Coverage** - Code coverage report (~4 min)
6. **Docs** - Build documentation (~2 min)
7. **Security** - Dependency audit (~1 min)
8. **MSRV** - Rust 1.70.0 compatibility (~3 min)

**Total**: ~8-10 minutes (parallel)

## Benchmark Targets

| Operation | Target |
|-----------|--------|
| Tool registration | < 1μs |
| Tool lookup | < 1μs |
| Schema generation | < 10μs |
| Request parsing | < 100μs |
| Tool execution | < 50μs |
| Middleware chain | < 10μs |
| Hook emission | < 5μs |

## Coverage Targets

| Metric | Target |
|--------|--------|
| Line coverage | > 80% |
| Branch coverage | > 70% |
| Function coverage | > 85% |

## Common Issues

### Formatting Fails

```bash
cargo fmt
```

### Clippy Warnings

```bash
cargo clippy --fix --all-targets --all-features
```

### Test Failures

```bash
# Run specific test
cargo test test_name

# Show output
cargo test -- --nocapture

# Run single-threaded
cargo test -- --test-threads=1
```

### Benchmark Fails

```bash
# Clean and rebuild
cargo clean
cargo bench --no-run
```

## File Structure

```
mcp-server/
├── .github/
│   └── workflows/
│       └── ci.yml           # CI pipeline configuration
├── benches/
│   └── benchmarks.rs        # Performance benchmarks
├── .gitignore               # Git ignore patterns
├── clippy.toml              # Clippy configuration
├── rustfmt.toml             # Rustfmt configuration
├── BENCHMARKING.md          # Benchmark documentation
├── CI_CD.md                 # CI/CD documentation
└── QUICK_START_CI.md        # This file
```

## Resources

- [BENCHMARKING.md](./BENCHMARKING.md) - Full benchmark guide
- [CI_CD.md](./CI_CD.md) - Complete CI/CD documentation
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) - Benchmark framework
- [GitHub Actions](https://docs.github.com/en/actions) - CI platform
