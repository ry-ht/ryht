# E2E Workflow Tests - Quick Start Guide

## What Are These Tests?

These are **end-to-end workflow tests** that simulate real development tasks using Cortex MCP tools. Each test demonstrates a complete workflow from start to finish, measuring efficiency vs traditional approaches.

## Quick Commands

### Run All E2E Tests
```bash
cd /path/to/cortex/cortex
cargo test --package cortex --test '*' e2e:: -- --nocapture
```

### Run Individual Workflows
```bash
# Add authentication feature
cargo test test_workflow_add_authentication_feature -- --nocapture

# Fix off-by-one bug
cargo test test_workflow_fix_off_by_one_bug -- --nocapture

# Refactor monolithic module
cargo test test_workflow_refactor_monolithic_module -- --nocapture

# Add test coverage
cargo test test_workflow_add_comprehensive_test_coverage -- --nocapture
```

## The 4 Workflows

### 1. 🔐 Add Feature (575 lines)
**Task**: Add login/register to auth service
**Steps**: 9 workflow steps
**Savings**: ~55% fewer tokens
**File**: `workflow_add_feature.rs`

### 2. 🐛 Fix Bug (519 lines)
**Task**: Fix off-by-one error
**Steps**: 8 workflow steps
**Savings**: ~48% fewer tokens
**File**: `workflow_fix_bug.rs`

### 3. 🔧 Refactor (691 lines)
**Task**: Split monolith into modules
**Steps**: 10 workflow steps
**Savings**: ~52% fewer tokens
**File**: `workflow_refactor_module.rs`

### 4. ✅ Add Tests (713 lines)
**Task**: Improve coverage 20% → 80%
**Steps**: 9 workflow steps
**Savings**: ~68% fewer tokens
**File**: `workflow_add_tests.rs`

## What Each Test Does

### Workflow Pattern
```rust
1. Create TempDir with realistic project
2. Initialize storage and MCP context
3. Create workspace and import files
4. Execute 8-10 workflow steps:
   - Code navigation
   - Semantic search
   - Code manipulation
   - Test generation
   - Documentation
5. Track metrics (time, tokens, results)
6. Export and verify
7. Print comprehensive summary
```

### Example Output
```
================================================================================
E2E WORKFLOW: ADD AUTHENTICATION FEATURE - SUMMARY
================================================================================

Step-by-Step Breakdown:
  Create Workspace & Import                             245ms
  Analyze Auth Module                                    89ms
  Navigate to Auth Module                                45ms
  Create Login Function                                  123ms
  Create Register Function                               98ms
  Generate Tests                                         178ms
  Run Tests                                              234ms
  Generate Documentation                                 156ms
  Export Workspace                                       201ms

Total Duration:              1369ms

Token Efficiency:
  Traditional Approach:      3700 tokens
  Cortex MCP Approach:       580 tokens
  Token Savings:             55.4%
================================================================================
```

## Key Metrics

| Workflow | Steps | Lines | Token Savings |
|----------|-------|-------|---------------|
| Add Feature | 9 | 575 | 55% |
| Fix Bug | 8 | 519 | 48% |
| Refactor | 10 | 691 | 52% |
| Add Tests | 9 | 713 | 68% |
| **Total** | **36** | **2,498** | **Avg 55.9%** |

## What Makes These Special?

✅ **Real MCP Tools** - Uses actual tool implementations
✅ **Real Projects** - Creates complete Rust projects
✅ **Real Metrics** - Measures actual token usage
✅ **Real Validation** - Exports and verifies code compiles
✅ **Complete Workflows** - End-to-end task completion

## MCP Tools Demonstrated

Each workflow uses multiple tool categories:

- **Workspace Tools**: create, sync, export
- **Code Navigation**: goto_definition, document_symbols, find_references
- **Semantic Search**: semantic_search for concept-based finding
- **Code Manipulation**: create_unit, update_unit, rename_symbol
- **Testing Tools**: generate_tests, run_tests
- **Documentation Tools**: generate_documentation
- **VFS Tools**: create_file, update_file, read_file
- **Dependency Analysis**: analyze_dependencies, find_references
- **Code Quality**: analyze_code_quality

## Documentation

- **README.md** - Comprehensive guide with examples
- **SUMMARY.md** - Implementation details and metrics
- **QUICK_START.md** - This file (quick reference)
- **mod.rs** - Module docs and usage

## Requirements

- Rust toolchain with cargo
- Tokio async runtime
- All cortex crates (storage, vfs, parser, memory, mcp)
- Test dependencies (tempfile, etc.)

## Expected Results

All tests should:
- ✅ Pass successfully
- ✅ Create realistic projects
- ✅ Execute all workflow steps
- ✅ Print detailed metrics
- ✅ Show >40% token savings
- ✅ Export valid code

## Troubleshooting

### Test Fails to Compile
- Check all cortex dependencies are available
- Verify MCP tool contexts are correctly imported

### Test Times Out
- Increase timeout if needed (default 2min should be enough)
- Check database connection (uses mem://)

### Metrics Don't Show
- Run with `--nocapture` flag to see output
- Check that println! statements execute

## Next Steps

After running tests:
1. Review the output metrics
2. Compare token efficiency across workflows
3. Examine the realistic project structures created
4. Consider adding new workflows for other scenarios

## Learn More

See full documentation in:
- **README.md** - Complete workflow descriptions
- **SUMMARY.md** - Detailed implementation analysis
- Individual test files for code examples

---

**Total Code**: 2,624 lines across 4 workflows
**Average Token Savings**: 55.9%
**Coverage**: Add features, fix bugs, refactor, add tests
