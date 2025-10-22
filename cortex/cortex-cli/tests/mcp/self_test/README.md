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

### Phase 2: Deep Analysis 🔄 (Planned)

**Objective:** Validate advanced querying and analysis capabilities.

**What it will test:**
- Semantic search across cortex codebase
- Find all callers of specific functions
- Dependency analysis and impact assessment
- Code complexity metrics
- Cross-crate reference resolution
- Type hierarchy navigation

**Planned Features:**
- Query: "Find all functions that use VirtualFileSystem"
- Query: "What depends on ConnectionManager::new?"
- Query: "Show me all async functions in cortex-storage"
- Query: "Find similar code to this function"

### Phase 3: Self-Modification 🚀 (Future)

**Objective:** Use cortex to improve cortex itself.

**What it will test:**
- Generate missing documentation
- Suggest refactoring opportunities
- Identify code duplication
- Propose architecture improvements
- Auto-generate test cases

**Vision:**
- Cortex analyzes its own code quality
- Suggests improvements via MCP tools
- Can be used by AI agents to enhance cortex

## Running the Tests

### Quick Test (without actual ingestion)
```bash
cargo test --package cortex-cli self_test::
```

This runs the lightweight validation tests (workspace detection, etc.)

### Full Phase 1 Ingestion Test
```bash
cargo test --package cortex-cli --test phase1_ingestion \
  -- --ignored --nocapture --test-threads=1
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

## Next Steps

After Phase 1 passes:

1. **Phase 2 Planning**: Design deep analysis tests
2. **Benchmark Tracking**: Set up automated performance monitoring
3. **CI Integration**: Add to continuous integration pipeline
4. **Phase 3 Vision**: Explore self-improvement capabilities

## Contributing

When adding new cortex features:

1. Update expected counts if adding significant code
2. Add new "known functions" if adding key APIs
3. Run self-test to ensure no regressions
4. Document any new warnings that appear

## Philosophy

> "The best test for a code analysis tool is: can it analyze itself?"

This self-test embodies that principle. When cortex can successfully understand, navigate, and potentially improve its own codebase, we know it's ready to handle any user's code.
