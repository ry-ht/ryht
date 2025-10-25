# Test Fixes Needed

This document outlines the compilation errors found and how to fix them.

## Summary

The newly created test files have compilation errors due to API mismatches. The tests were written based on expected APIs, but the actual implementations have slightly different signatures.

## Key API Differences

### 1. EnhancedProducerConsumer

**Issue**: The `EnhancedProducerConsumer::new()` signature is:
```rust
pub fn new(processor: F, config: ProducerConsumerConfig) -> Self
```

**Tests incorrectly use**: `new(config, processor)`

**No `process()` method exists**: The tests call `.process()` but this method doesn't exist in the implementation.

**Fix Needed**: Either:
1. Update tests to match the actual implementation's API
2. Or implement the missing `process()` method in `producer_consumer.rs`

### 2. ProgressTracker

**Issue**: `ProgressState` has field `processed` not `completed`

**Tests use**: `tracker.state().completed`
**Should be**: `tracker.state().processed`

**Also**: ProgressTracker methods include:
- `inc(n)` - not `increment(n)`
- `state()` returns `ProgressState` which has `processed` field
- `finish()` - exists
- Missing `increment()` method tests use

### 3. ParallelProcessor / BatchProcessor

**Missing fields/methods in result types**:
- Tests use `completed` field, should be `processed` or similar
- Need to verify exact result type structure

### 4. Analysis Module Traits

**Issue**: Some tests use methods/types that may not be publicly exported or don't exist:
- `NodeFilter` enum variants
- `CountFilter` enum variants
- Method signatures for `AstFinder`, `AstCounter` etc.

## Recommended Approach

Since the test files are comprehensive but have API mismatches, we have two options:

### Option A: Fix the Tests (Recommended for now)
Update the test files to match the actual implementation APIs. This is safer and ensures tests validate the real functionality.

### Option B: Extend the Implementation
Add the missing methods/APIs that the tests expect. This would be done in a follow-up PR after validating the tests work with the current APIs.

## Specific Fixes Needed

### concurrent_processing_test.rs

1. **Lines 24-44**: Fix `EnhancedProducerConsumer` usage:
```rust
// OLD:
let processor = EnhancedProducerConsumer::new(
    config,
    move |path: &PathBuf, _: &()| { ... },
);
let (results, stats) = processor.process(files, (), vec![], vec![])?;

// NEW: Need to check actual API and fix accordingly
```

2. **All ProgressTracker usage**: Replace `.completed` with `.processed`

3. **Check if `process()` method exists or needs to be `run()` or similar**

### ast_analysis_test.rs

1. **Verify `FindConfig` builder pattern matches implementation**
2. **Check `NodeFilter` enum construction**
3. **Verify `CountConfig` and `CountFilter` APIs**
4. **Check if `analyze_comments()` standalone function exists**

### metrics_enhanced_test.rs

1. **Verify `MetricsBuilder` API**
2. **Check `MetricsAggregator::aggregate()` signature**
3. **Verify all metric stat method names** (e.g., `nesting_level()`, `eval_boolean_operator()`)

### integration_test.rs

1. **Same fixes as above test files combined**
2. **Verify `CodeParser` API**
3. **Check `ParallelProcessor` result types**

## Next Steps

1. Comment out or fix the failing tests file by file
2. Run `cargo test` iteratively to identify remaining issues
3. Update tests to match actual APIs
4. Document any missing features that tests expect but aren't implemented
5. Create follow-up issues for missing functionality if needed

## Files Status

- ✅ `tests/ast_analysis_test.rs` - Created, needs API fixes
- ✅ `tests/concurrent_processing_test.rs` - Created, needs API fixes
- ✅ `tests/metrics_enhanced_test.rs` - Created, needs API fixes
- ✅ `tests/integration_test.rs` - Created, needs API fixes
- ✅ `tests/test_metrics.rs` - Updated with new tests, may need minor fixes

## Compilation Error Count

- ~110 errors in concurrent_processing_test.rs
- ~46 errors in test_metrics.rs
- Unknown count in other test files (not yet compiled past first failures)

Most errors are repeating patterns of the same API mismatches.
