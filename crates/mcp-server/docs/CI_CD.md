# CI/CD Documentation

This document describes the Continuous Integration and Continuous Deployment pipeline for the MCP Server framework.

## Overview

The CI/CD pipeline ensures code quality, correctness, and performance through automated checks on every commit and pull request.

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     CI Pipeline                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Check   │  │   Test   │  │ Features │  │   Bench  │   │
│  │          │  │          │  │          │  │          │   │
│  │ • Format │  │ • stable │  │ • stdio  │  │ • Perf   │   │
│  │ • Clippy │  │ • beta   │  │ • http   │  │ • Track  │   │
│  └──────────┘  │ • nightly│  │ • websoc │  └──────────┘   │
│                 │          │  │ • all    │                 │
│                 │ • Linux  │  └──────────┘                 │
│                 │ • macOS  │                               │
│                 │ • Windows│  ┌──────────┐  ┌──────────┐  │
│                 └──────────┘  │ Coverage │  │   Docs   │  │
│                               │          │  │          │  │
│                               │ • Report │  │ • Build  │  │
│                               │ • Upload │  │ • Deploy │  │
│                               └──────────┘  └──────────┘  │
│                                                             │
│  ┌──────────┐  ┌──────────┐                               │
│  │ Security │  │   MSRV   │                               │
│  │          │  │          │                               │
│  │ • Audit  │  │ • 1.70.0 │                               │
│  └──────────┘  └──────────┘                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Jobs

### 1. Check (Format & Lint)

**Platforms**: Ubuntu latest
**Duration**: ~2 minutes

Validates code style and catches common mistakes.

```yaml
- Check formatting (rustfmt)
- Run clippy lints (all warnings as errors)
```

**Failure causes**:
- Unformatted code
- Clippy warnings
- Clippy errors

**Fix**:
```bash
cargo fmt
cargo clippy --fix --all-targets --all-features
```

### 2. Test (Build & Test)

**Platforms**: Ubuntu, macOS, Windows
**Rust versions**: stable, beta, nightly
**Duration**: ~5-8 minutes per matrix cell

Runs comprehensive test suite across platforms and Rust versions.

```yaml
- Build all features
- Run unit tests
- Run integration tests
- Run doc tests
```

**Matrix** (9 combinations):
- stable × (ubuntu, macos, windows)
- beta × (ubuntu, macos, windows)
- nightly × (ubuntu, macos, windows)

**Failure causes**:
- Compilation errors
- Test failures
- Platform-specific issues

**Fix**:
```bash
cargo test --all-features
cargo test --doc
```

### 3. Features (Feature Flag Testing)

**Platform**: Ubuntu latest
**Duration**: ~3 minutes

Tests all feature flag combinations.

```yaml
- No default features
- stdio only
- http only
- websocket only
- All features
```

**Failure causes**:
- Feature-specific compilation errors
- Missing feature gates
- Broken feature combinations

**Fix**:
```bash
cargo test --no-default-features --features stdio
cargo test --no-default-features --features http
cargo test --all-features
```

### 4. Benchmarks

**Platform**: Ubuntu latest
**Duration**: ~5 minutes

Runs performance benchmarks and tracks regression.

```yaml
- Run all benchmarks
- Store results
- Track performance history
- Alert on regression (> 200%)
```

**Failure causes**:
- Significant performance regression
- Benchmark compilation errors

**Configuration**:
- Alert threshold: 200%
- Fail on alert: false (warning only)
- Auto-push results: true

### 5. Coverage

**Platform**: Ubuntu latest
**Duration**: ~4 minutes

Generates code coverage report.

```yaml
- Install tarpaulin
- Generate coverage
- Upload to Codecov
```

**Targets**:
- Line coverage: > 80%
- Branch coverage: > 70%
- Function coverage: > 85%

**Reports**: Uploaded to Codecov

### 6. Documentation

**Platform**: Ubuntu latest
**Duration**: ~2 minutes

Builds and validates documentation.

```yaml
- Build documentation
- Check for broken links
- Deploy to GitHub Pages (main branch only)
```

**Failure causes**:
- Broken documentation links
- Invalid doc comments
- Missing documentation

**Deployment**:
- Main branch only
- Publishes to GitHub Pages
- URL: https://[username].github.io/[repo]/

### 7. Security Audit

**Platform**: Ubuntu latest
**Duration**: ~1 minute

Checks dependencies for known vulnerabilities.

```yaml
- Install cargo-audit
- Scan dependencies
- Report vulnerabilities
```

**Failure causes**:
- Known CVEs in dependencies
- Unmaintained dependencies

### 8. MSRV (Minimum Supported Rust Version)

**Platform**: Ubuntu latest
**Rust version**: 1.70.0
**Duration**: ~3 minutes

Ensures compatibility with minimum Rust version.

```yaml
- Build with Rust 1.70.0
- Verify compilation
```

**Failure causes**:
- Using newer Rust features
- Incompatible dependencies

## Triggers

### On Push

```yaml
branches: [main, develop]
```

Runs full CI pipeline on:
- Commits to main branch
- Commits to develop branch

### On Pull Request

```yaml
branches: [main, develop]
```

Runs full CI pipeline on:
- New pull requests
- Pull request updates

### Manual Trigger

```yaml
workflow_dispatch:
```

Can be triggered manually from GitHub Actions UI.

## Caching Strategy

Aggressive caching to speed up builds:

```yaml
~/.cargo/registry  # Dependency registry
~/.cargo/git       # Git dependencies
target/            # Build artifacts
```

Cache keys based on:
- Operating system
- Rust version
- Cargo.lock hash

## Expected Runtime

| Job | Duration | Critical Path |
|-----|----------|---------------|
| Check | 2 min | No |
| Test (per matrix) | 5-8 min | Yes |
| Features | 3 min | No |
| Benchmarks | 5 min | No |
| Coverage | 4 min | No |
| Docs | 2 min | No |
| Security | 1 min | No |
| MSRV | 3 min | No |

**Total pipeline time**: ~8-10 minutes (parallel execution)

## Status Badges

Add to README.md:

```markdown
[![CI](https://github.com/[user]/[repo]/workflows/CI/badge.svg)](https://github.com/[user]/[repo]/actions)
[![codecov](https://codecov.io/gh/[user]/[repo]/branch/main/graph/badge.svg)](https://codecov.io/gh/[user]/[repo])
[![Crates.io](https://img.shields.io/crates/v/mcp-server.svg)](https://crates.io/crates/mcp-server)
[![docs.rs](https://docs.rs/mcp-server/badge.svg)](https://docs.rs/mcp-server)
```

## Failure Handling

### Auto-Retry

Failed jobs can be manually retried from GitHub Actions UI.

### Fail-Fast

Test matrix uses `fail-fast: false` to run all combinations even if one fails.

### Critical Path

The `ci-success` job depends on all others and serves as the final status check for:
- Branch protection rules
- Merge requirements
- Status badges

## Local Development

Run CI checks locally before pushing:

```bash
# Format check
cargo fmt --all -- --check

# Lint check
cargo clippy --all-targets --all-features -- -D warnings

# Build
cargo build --all-features

# Test
cargo test --all-features

# Benchmarks
cargo bench --no-fail-fast

# Documentation
cargo doc --all-features --no-deps

# Security audit
cargo install cargo-audit
cargo audit
```

## Optimizing CI

### Speed Up Builds

1. **Use sccache**:
```yaml
- uses: mozilla-actions/sccache-action@v0.0.3
```

2. **Reduce matrix**:
- Remove nightly if not needed
- Test only on Ubuntu for most checks

3. **Parallel tests**:
```bash
cargo test -- --test-threads=4
```

### Reduce Costs

1. **Skip redundant jobs** on docs-only changes
2. **Use self-hosted runners** for private repos
3. **Cache aggressively**

## Secrets Required

For full functionality, configure these secrets:

| Secret | Purpose | Required |
|--------|---------|----------|
| `GITHUB_TOKEN` | Automatic (provided) | Yes |
| `CODECOV_TOKEN` | Coverage upload | Optional |

## Branch Protection

Recommended settings for main branch:

```yaml
- Require status checks to pass before merging
  ✓ ci-success
- Require branches to be up to date before merging
- Require linear history
- Include administrators
```

## Troubleshooting

### CI Fails Locally Works

1. Check Rust version matches CI
2. Clear cargo cache: `cargo clean`
3. Check for platform-specific code

### Slow CI Runs

1. Review cache hit rates
2. Check for test timeouts
3. Consider splitting large test files

### Flaky Tests

1. Add retry logic for network tests
2. Use fixed seeds for random tests
3. Increase timeouts

## Monitoring

### GitHub Actions Dashboard

View pipeline status:
- https://github.com/[user]/[repo]/actions

### Metrics

Track:
- Success rate
- Average duration
- Cache hit rate
- Failure patterns

## Contributing

Before submitting PRs:

1. Ensure all CI checks pass locally
2. Add tests for new features
3. Update documentation
4. Run benchmarks if changing performance-critical code

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI Best Practices](https://github.com/actions-rs)
- [Codecov Documentation](https://docs.codecov.io/)
