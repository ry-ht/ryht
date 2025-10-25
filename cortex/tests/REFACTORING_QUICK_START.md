# Refactoring Scenarios - Quick Start Guide

## Overview

This is a comprehensive test suite with 20+ test cases covering real-world code refactoring scenarios. Each test validates AST correctness, token efficiency, and proper refactoring transformations.

## Quick Test Commands

```bash
# Compile all tests (fast check)
cargo test --test test_refactoring_scenarios --no-run

# Run all refactoring tests
cargo test --test test_refactoring_scenarios

# Run with detailed output
cargo test --test test_refactoring_scenarios -- --nocapture

# Run specific scenario by number
cargo test --test test_refactoring_scenarios -- test_scenario_1
cargo test --test test_refactoring_scenarios -- test_scenario_2
# ... up to test_scenario_10

# Run edge case tests only
cargo test --test test_refactoring_scenarios -- edge_case

# Run complete integration workflow
cargo test --test test_refactoring_scenarios -- complete_workflow
```

## Test Scenarios at a Glance

| # | Scenario | Test Name | Key Features |
|---|----------|-----------|--------------|
| 1A | API Migration - Basic | `test_scenario_1_api_migration_basic` | Old API → New API, 3 files |
| 1B | API Migration - Breaking | `test_scenario_1_api_migration_with_breaking_changes` | Sync → Async API |
| 2 | Procedural to OOP | `test_scenario_2_procedural_to_oop` | Remove global state |
| 3 | Sync to Async | `test_scenario_3_sync_to_async` | Add async/await |
| 4 | Error Handling | `test_scenario_4_error_handling_standardization` | Remove unwrap() |
| 5 | Module Reorganization | `test_scenario_5_module_reorganization` | Split large module |
| 6 | Add Generics | `test_scenario_6_add_generics` | Generic types + traits |
| 7 | Performance Optimization | `test_scenario_7_performance_optimization` | O(n²) → O(n) |
| 8 | Dead Code Elimination | `test_scenario_8_dead_code_elimination` | Remove unused code |
| 9 | Naming Convention | `test_scenario_9_naming_convention_update` | snake_case → camelCase |
| 10 | Dependency Injection | `test_scenario_10_dependency_injection` | Add DI pattern |

### Edge Cases

| # | Test | Description |
|---|------|-------------|
| E1 | `test_edge_case_circular_dependencies` | Break circular module deps |
| E2 | `test_edge_case_partial_refactoring` | Handle partial success |
| E3 | `test_complete_refactoring_workflow` | Full end-to-end workflow |

## Expected Output Format

Each test produces metrics like this:

```
================================================================================
SCENARIO X: [Scenario Name]
================================================================================
Total Duration:        XXXms
Files Modified:        N
Lines Changed:         N
Operations:            N (XX.X% success)

Token Efficiency:
  Traditional:         XXXX tokens
  Cortex:              XXX tokens
  Savings:             XX.X%

AST Validation:
  Total Validations:   N
  Failures:            N
  Success Rate:        100.0%

Operation Details:
  ✓ Operation 1 - XXms
  ✓ Operation 2 - XXms
================================================================================
```

## Performance Benchmarks

Expected performance on typical hardware:

- Single scenario: 100-500ms
- Complete workflow: <5 seconds
- Token savings: 70-95% vs traditional
- AST validation: 100% success rate

## Troubleshooting

### Test hangs or times out

The tests use in-memory SurrealDB which can be slow on first run. Try:

```bash
# Run with longer timeout
cargo test --test test_refactoring_scenarios -- --test-threads=1

# Or run a simpler test first
cargo test --test test_refactoring_scenarios -- test_scenario_8
```

### Compilation errors

Ensure you have the latest dependencies:

```bash
cargo clean
cargo build --test test_refactoring_scenarios
```

### AST validation failures

Check that the language parser is working:

```bash
cargo test --package cortex-code-analysis
```

## File Locations

- **Test File:** `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/tests/test_refactoring_scenarios.rs`
- **Documentation:** `./REFACTORING_SCENARIOS_README.md`
- **Quick Start:** `./REFACTORING_QUICK_START.md` (this file)

## Metrics Tracked

Each test tracks:

✅ **Correctness** - AST validation, semantic checks
✅ **Performance** - Operation latency, total duration
✅ **Efficiency** - Token usage vs traditional approach
✅ **Coverage** - Files modified, lines changed
✅ **Reliability** - Success rate, error handling

## Languages Tested

- **Rust** - Most scenarios
- **TypeScript** - Scenario 9 (naming), React components
- **JavaScript** - Supported via TypeScript parser

## Next Steps

1. Run the complete test suite:
   ```bash
   cargo test --test test_refactoring_scenarios -- --nocapture
   ```

2. Review the detailed README:
   ```bash
   cat tests/REFACTORING_SCENARIOS_README.md
   ```

3. Examine specific test code:
   ```bash
   # View the source
   less tests/test_refactoring_scenarios.rs
   ```

4. Add your own scenario:
   - Copy an existing `test_scenario_N` function
   - Modify the code samples and refactoring logic
   - Update metrics and assertions
   - Add documentation to README

## Contributing

To add a new scenario:

1. Follow the naming pattern: `test_scenario_N_descriptive_name`
2. Use `RefactoringMetrics` for tracking
3. Validate AST after transformations
4. Calculate token efficiency
5. Document in README

## Support

- Issues: Open on Cortex repository
- Questions: Check existing test patterns
- Examples: See `test_real_world_development.rs`

---

**Quick Reference**
- **Test Count:** 14 scenarios, 20+ test cases
- **Test File:** 2,400+ lines
- **Language Coverage:** Rust, TypeScript, JavaScript
- **Metrics:** Timing, tokens, AST, success rate
- **Documentation:** 300+ lines

**Last Updated:** October 23, 2025
