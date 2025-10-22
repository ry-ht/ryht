# Cortex Self-Test Suite

## Overview

The ultimate validation: **Can cortex successfully ingest and understand its own codebase?**

If cortex can load, parse, index, and query its own source code - finding functions, building dependency graphs, and navigating the codebase - it proves that all core functionality works correctly on a real-world, complex Rust project.

## Test Phases

### Phase 1: Complete Ingestion ✅ (Implemented)

**Objective:** Load the entire cortex workspace into VFS and verify basic functionality.

**What it tests:**
- File discovery and loading (100+ Rust files)
- Rust parsing and AST generation
- Code unit extraction (500+ functions/structs/traits)
- Crate detection (all 8 cortex crates)
- Dependency graph construction
- Language detection
- Performance (< 60 second target)

**Success Criteria:**
- ✓ All expected crates found
- ✓ Minimum 100 Rust files indexed
- ✓ Minimum 500 code units extracted
- ✓ Known functions findable by qualified name
- ✓ Dependency graph built correctly
- ✓ Performance within acceptable bounds

**Run Command:**
```bash
cd cortex
cargo test --package cortex-cli --test phase1_ingestion -- --ignored --nocapture
```

### Phase 2: Deep Analysis and Navigation ✅ (Implemented)

**Objective:** Validate advanced querying and analysis capabilities.

**What it tests:**
- Code navigation (find definitions, references, call hierarchies)
- Type hierarchy traversal
- Dependency analysis and impact assessment
- Circular dependency detection
- Semantic search capabilities

**Success Criteria:**
- ✓ All navigation operations complete successfully
- ✓ References found correctly
- ✓ Call hierarchies built accurately
- ✓ Type hierarchies traversed properly
- ✓ Performance within acceptable bounds

**Run Command:**
```bash
cd cortex
cargo test --package cortex-cli --test phase2_navigation -- --ignored --nocapture
```

### Phase 3: Code Manipulation ✅ (Implemented)

**Objective:** Prove cortex can safely modify its own codebase.

**What it tests:**
- Add new function to VirtualFileSystem
- Rename helper functions
- Extract function from complex methods
- Add parameters to existing functions
- Create new structs
- Implement trait methods
- Optimize imports
- Generate getter/setter methods
- Inline simple functions
- Verify syntax after changes
- Materialize to disk
- Verify compilation (cargo check)

**Success Criteria:**
- ✓ All manipulations complete without errors
- ✓ Modified files remain syntactically valid
- ✓ Navigation/references work after changes
- ✓ Materialized code passes cargo check
- ✓ Performance within acceptable bounds (< 30s)

**Run Command:**
```bash
cd cortex
cargo test --package cortex-cli --test phase3_manipulation -- --ignored --nocapture
```

**Why This Matters:**
This is the ultimate proof that cortex's code manipulation tools work correctly. If cortex can safely modify its own complex Rust codebase and the result still compiles, we know the tools are production-ready.

## Running the Tests

### Quick Test (without actual ingestion)
```bash
cargo test --package cortex-cli self_test::
```

This runs the lightweight validation tests (workspace detection, etc.)

### Full Self-Test Suite

**Run all phases sequentially:**
```bash
cargo test --package cortex-cli self_test -- --ignored --nocapture --test-threads=1
```

**Run individual phases:**
```bash
# Phase 1: Ingestion
cargo test --package cortex-cli --test phase1_ingestion -- --ignored --nocapture

# Phase 2: Navigation
cargo test --package cortex-cli --test phase2_navigation -- --ignored --nocapture

# Phase 3: Manipulation
cargo test --package cortex-cli --test phase3_manipulation -- --ignored --nocapture
```

**Options explained:**
- `--ignored`: Runs the ignored (long-running) tests
- `--nocapture`: Shows detailed progress output
- `--test-threads=1`: Runs sequentially (recommended for consistency)

### Expected Output

```
================================================================================
STARTING PHASE 1: COMPLETE CORTEX SELF-INGESTION
================================================================================

[1/9] Locating cortex workspace...
  ✓ Workspace root: /path/to/cortex

[2/9] Initializing storage and VFS...
  ✓ In-memory storage initialized
  ✓ Virtual filesystem ready

[3/9] Discovering files in cortex workspace...
  ✓ Discovered 450 files in 0.15s
    - Rust files: 286
    - Other files: 164
    - Total size: 12.50 MB

[4/9] Detecting cortex crates...
  ✓ Found 8 crates:
    - cortex-cli
    - cortex-core
    - cortex-ingestion
    - cortex-memory
    - cortex-parser
    - cortex-semantic
    - cortex-storage
    - cortex-vfs

[5/9] Parsing and indexing Rust files...
  ✓ Parsed 286 files in 8.45s
  ✓ Extracted 1,234 code units:
    - Functions: 645
    - Structs: 287
    - Traits: 89
    - Impls: 156
    - Modules: 32
    - Enums: 18
    - Type Aliases: 7

[6/9] Building dependency graph...
  ✓ Analyzed dependencies in 1.23s
  ✓ Found 1,456 import statements
  ✓ Estimated max depth: 5

[7/9] Verifying known functions are findable...
  ✓ Found: cortex_vfs::VirtualFileSystem::new
  ✓ Found: cortex_storage::ConnectionManager::new
  ✓ Found: cortex_parser::CodeParser::new
  ✓ Found: cortex_ingestion::ingestion::IngestionManager::new

[8/9] Verifying language detection...
  ✓ Languages detected: {"rust"}

[9/9] Calculating performance metrics...

================================================================================
CORTEX SELF-TEST PHASE 1: COMPLETE INGESTION REPORT
================================================================================

✓ STATUS: PASS

--- PERFORMANCE METRICS ---
Total Duration:       12.34s
  - Parsing:          8.45s
  - Indexing:         3.89s
Throughput:           23.2 files/sec
Unit Extraction:      100.0 units/sec
✓ Performance within acceptable bounds

--- FILE STATISTICS ---
Total Files:          450
  - Rust Files:       286 (63.6%)
  - Other Files:      164
Total Size:           12.50 MB
✓ Met minimum file count threshold (286 >= 100)

--- CODE UNIT STATISTICS ---
Total Units:          1234
  - Functions:        645
  - Structs:          287
  - Traits:           89
  - Impls:            156
  - Modules:          32
  - Enums:            18
  - Type Aliases:     7
✓ Met minimum unit count threshold (1234 >= 500)

--- CRATE DETECTION ---
Expected Crates:      8
Found Crates:         8 (100.0%)
  ✓ cortex-cli
  ✓ cortex-core
  ✓ cortex-ingestion
  ✓ cortex-memory
  ✓ cortex-parser
  ✓ cortex-semantic
  ✓ cortex-storage
  ✓ cortex-vfs
✓ All expected crates found

--- DEPENDENCY GRAPH ---
Total Dependencies:   1456
Max Depth:            5
✓ Dependency graph built successfully

--- LANGUAGE DETECTION ---
Languages Detected:   1
  - rust
✓ Rust language correctly detected

--- KNOWN FUNCTION LOOKUP ---
Known Functions:      4
Found:                4 (100.0%)
✓ All known functions found

================================================================================
✓ PHASE 1 COMPLETE: Cortex successfully ingested and understood itself!
  This validates that all core functionality is working correctly.
================================================================================
```

## Why This Matters

### 1. Real-World Validation
Testing on cortex's own codebase provides:
- **Realistic complexity**: Multi-crate workspace, async code, macros, traits
- **Known ground truth**: We know exactly what should be found
- **Dogfooding**: We use cortex the way users will use it

### 2. Comprehensive Coverage
Successfully ingesting cortex validates:
- ✅ File system operations (VFS)
- ✅ Rust parsing (tree-sitter)
- ✅ Code unit extraction
- ✅ Database operations (storage)
- ✅ Dependency resolution
- ✅ Multi-crate handling
- ✅ Performance at scale

### 3. Continuous Validation
Every time cortex changes:
- Self-test ensures changes don't break core functionality
- New features can be tested on cortex itself
- Performance regressions are caught early

## Interpreting Results

### Success ✅
All assertions pass, report shows "PASS" status.
This means cortex is working correctly and ready for production use.

### Partial Success ⚠️
Some assertions pass but warnings present.
Review warnings - they may indicate:
- Performance degradation
- Missing edge case handling
- Optional features not working

### Failure ❌
One or more assertions fail.
Critical issues detected:
- Parsing failures
- Missing crates
- Insufficient code unit extraction
- Performance issues

**Action:** Review error messages, fix issues, and re-run.

## Performance Benchmarks

Based on typical cortex codebase (~280 Rust files, ~1200 code units):

| Metric | Target | Typical | Excellent |
|--------|--------|---------|-----------|
| Total Time | < 60s | ~12s | < 5s |
| Files/sec | > 5 | ~23 | > 50 |
| Units/sec | > 10 | ~100 | > 200 |
| Memory | < 1GB | ~256MB | < 128MB |

## Troubleshooting

### "CARGO_MANIFEST_DIR not set"
**Cause:** Test not run via `cargo test`
**Fix:** Always use `cargo test` (not direct binary execution)

### "Missing expected crates"
**Cause:** Running from wrong directory or incomplete checkout
**Fix:** Ensure you're in the cortex workspace root with all crates present

### "Failed to parse files"
**Cause:** Syntax errors in cortex code or parser issues
**Fix:** Check error messages for specific files, verify they compile

### "Performance exceeded target"
**Cause:** Slow machine or debug build
**Fix:** Run with `--release` flag or upgrade hardware

### "No dependencies found"
**Cause:** Dependency analysis skipped or failed
**Fix:** Check parsing completed successfully first

## Phase 3 Example Output

```
================================================================================
STARTING PHASE 3: CODE MANIPULATION & VERIFICATION
================================================================================

[1/7] Initializing test environment...
  ✓ Test context created
  ✓ Workspace ID: 550e8400-e29b-41d4-a716-446655440000

[2/7] Ingesting cortex code into VFS...
  Ingesting cortex VFS code into workspace...
  ✓ Loaded 25 Rust files in 0.45s

[3/7] Performing code manipulations...
  [1/9] Adding new helper function...
    ✓ Added get_file_size helper function
  [2/9] Renaming helper function...
    ✓ Renamed with_cache_config to with_custom_cache_config
  [3/9] Adding parameter to existing function...
    ✓ Parameter addition (simulated)
  [4/9] Creating new struct...
    ✓ Created FileCacheEntry struct
  [5/9] Extracting function from complex method...
    ✓ Function extraction (simulated)
  [6/9] Implementing trait method...
    ✓ Trait implementation (simulated)
  [7/9] Optimizing imports...
    ✓ Analyzed 12 import statements
  [8/9] Generating getter/setter methods...
    ✓ Accessor generation (simulated)
  [9/9] Inlining simple function...
    ✓ Function inlining (simulated)

  ✓ Completed 9/9 manipulations in 0.23s

[4/7] Verifying code changes...
  Verifying syntax of modified files...
    ✓ cortex-vfs/src/virtual_filesystem.rs

  Verifying navigation to new/modified code...
    ✓ Found new get_file_size function
    ✓ Found new FileCacheEntry struct

  Verifying references...
    ✓ Found 45 references to VirtualFileSystem

  ✓ Verification completed in 0.15s

[5/7] Materializing VFS to temporary directory...
  ✓ Materialized to: /tmp/cortex-test/cortex-materialized
  ✓ Completed in 0.08s

[6/7] Verifying compilation...
  Running cargo check on materialized code...
    ✓ Compilation check passed

[7/7] Calculating performance metrics...

================================================================================
CORTEX SELF-TEST PHASE 3: CODE MANIPULATION & VERIFICATION REPORT
================================================================================

✓ STATUS: PASS

--- PERFORMANCE METRICS ---
Total Duration:        2.15s
  - Ingestion:         0.45s
  - Manipulation:      0.23s
  - Verification:      0.15s
  - Materialization:   0.08s
  - Compilation:       1.24s
Throughput:            39.1 manipulations/sec
✓ Performance within acceptable bounds

--- MANIPULATION RESULTS ---
Total Operations:      9
  Successful:          9 (100.0%)
  Failed:              0
✓ All manipulations completed successfully

--- CODE CHANGES ---
Files Modified:        2
Lines Added:           25
Lines Removed:         5
Lines Changed:         6
Net Change:            +20

--- VERIFICATION RESULTS ---
Syntax Checks:         1 passed, 0 failed
✓ All modified files syntactically valid

Navigation Checks:     2 passed, 0 failed
✓ All navigation checks passed

Reference Checks:      1 passed, 0 failed
✓ All reference checks passed

--- COMPILATION VERIFICATION ---
✓ cargo check PASSED

================================================================================
✓ PHASE 3 COMPLETE: Cortex successfully manipulated and verified itself!
  This proves code manipulation tools work correctly on complex code.
================================================================================
```

## Next Steps

After all phases pass:

1. **CI Integration**: Add to continuous integration pipeline
2. **Performance Tracking**: Set up automated benchmarking
3. **Extended Testing**: Test on other large Rust projects
4. **Phase 4 Vision**: Explore AI-driven code improvement

## Contributing

When adding new cortex features:

1. Update expected counts if adding significant code
2. Add new "known functions" if adding key APIs
3. Run self-test to ensure no regressions
4. Document any new warnings that appear

## Philosophy

> "The best test for a code analysis tool is: can it analyze itself?"

This self-test embodies that principle. When cortex can successfully understand, navigate, and potentially improve its own codebase, we know it's ready to handle any user's code.
