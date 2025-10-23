# Rust Development Scenario Tests

Comprehensive test suite simulating real-world Rust development workflows using MCP tools.

## Overview

This test suite (`rust_development_tests.rs`) provides exhaustive tests that simulate how developers would use Cortex MCP tools in actual Rust development scenarios. Each test works with real code, measures token efficiency vs traditional approaches, and verifies correctness.

## Test Coverage

### 1. Implement New Feature (`test_implement_new_feature`)

**Scenario**: Create a new cache system from scratch with complex generics and lifetimes.

**Operations**:
- Create `Cache` trait with generic type parameters and lifetime bounds
- Implement `LruCache` struct with HashMap and access ordering
- Generate comprehensive tests (unit, integration, property-based)
- Verify compilation

**Tools Used**:
- `cortex.code.create_unit` - Create trait and implementation
- `cortex.testing.generate_tests` - Auto-generate test suite
- Cargo integration for compilation verification

**Token Savings**: >50%
- Traditional: 4,350 tokens (manual coding, test writing)
- Cortex: 430 tokens (structured MCP commands)

---

### 2. Refactor Complex Code (`test_refactor_complex_code`)

**Scenario**: Refactor deeply nested, complex code using AI assistance.

**Operations**:
- Analyze code for refactoring opportunities
- Apply extract function refactoring
- Rename symbols across codebase
- Verify code still compiles

**Tools Used**:
- `cortex.ai.suggest_refactoring` - AI-powered refactoring suggestions
- `cortex.code.extract_function` - Automated function extraction
- `cortex.code.rename_symbol` - Semantic rename across workspace

**Token Savings**: >60%
- Traditional: 5,050 tokens (manual review, editing, testing)
- Cortex: 310 tokens (AI-guided refactoring)

---

### 3. Fix Compilation Errors (`test_fix_compilation_errors`)

**Scenario**: Fix Rust borrow checker, lifetime, and type errors.

**Operations**:
- Detect compilation errors
- Apply borrow checker fixes
- Fix lifetime errors
- Verify all errors resolved

**Tools Used**:
- `cortex.ai.fix_compilation_errors` - AI error detection and fixes
- Compiler integration

**Token Savings**: >65%
- Traditional: 6,100 tokens (reading compiler output, debugging)
- Cortex: 400 tokens (AI-assisted fixes)

---

### 4. Optimize Performance (`test_optimize_performance`)

**Scenario**: Optimize performance-critical Rust code.

**Operations**:
- Analyze performance bottlenecks
- Reduce allocations
- Convert to iterator-based patterns
- Generate benchmarks

**Tools Used**:
- `cortex.ai.optimize_code` - Performance analysis
- `cortex.testing.generate_benchmarks` - Auto-generate Criterion benchmarks

**Token Savings**: >55%
- Traditional: 5,700 tokens (profiling, manual optimization)
- Cortex: 370 tokens (AI-guided optimization)

---

### 5. Security Audit (`test_security_audit`)

**Scenario**: Comprehensive security audit of Rust codebase.

**Operations**:
- Scan for unsafe blocks
- Check for hardcoded secrets
- Analyze dependency vulnerabilities
- Generate security report

**Tools Used**:
- `cortex.security.scan` - Unsafe code and vulnerability detection
- `cortex.security.check_secrets` - Secret scanning
- `cortex.security.check_dependencies` - Dependency audit
- `cortex.security.generate_report` - Report generation

**Token Savings**: >50%
- Traditional: 3,800 tokens (manual code review, tool usage)
- Cortex: 270 tokens (automated security scanning)

---

### 6. Generate Comprehensive Tests (`test_generate_comprehensive_tests`)

**Scenario**: Generate advanced test suites including property-based and fuzz tests.

**Operations**:
- Generate property-based tests (proptest)
- Generate fuzzing tests (cargo-fuzz)
- Generate mutation tests
- Analyze test coverage

**Tools Used**:
- `cortex.advanced_testing.generate_property_tests` - Property-based tests
- `cortex.advanced_testing.generate_fuzz_tests` - Fuzzing tests
- `cortex.advanced_testing.generate_mutation_tests` - Mutation testing
- `cortex.advanced_testing.analyze_coverage` - Coverage analysis

**Token Savings**: >60%
- Traditional: 6,100 tokens (manual test writing)
- Cortex: 330 tokens (auto-generated tests)

---

### 7. Analyze Architecture (`test_analyze_architecture`)

**Scenario**: Analyze and improve module architecture.

**Operations**:
- Visualize module dependencies
- Detect circular dependencies
- Suggest module boundaries
- Check architectural constraints

**Tools Used**:
- `cortex.architecture.visualize` - Dependency visualization
- `cortex.architecture.detect_cycles` - Circular dependency detection
- `cortex.architecture.suggest_boundaries` - Module boundary suggestions
- `cortex.architecture.check_constraints` - Constraint checking

**Token Savings**: >55%
- Traditional: 5,700 tokens (manual diagramming, analysis)
- Cortex: 360 tokens (automated architecture analysis)

---

### 8. Type System Analysis (`test_type_system_analysis`)

**Scenario**: Analyze and improve Rust type usage.

**Operations**:
- Infer types in generic code
- Check type coverage
- Suggest type improvements
- Analyze trait implementations

**Tools Used**:
- `cortex.code.infer_types` - Type inference
- `cortex.code.check_type_coverage` - Type coverage analysis
- `cortex.code.suggest_type_improvements` - Type improvement suggestions
- `cortex.code.analyze_traits` - Trait analysis

**Token Savings**: >50%
- Traditional: 5,100 tokens (manual type analysis)
- Cortex: 325 tokens (AI-powered type analysis)

---

## Running Tests

### Run All Tests

```bash
export PATH=/Users/taaliman/.cargo/bin:/usr/local/bin:/bin:/usr/bin:$PATH
cd cortex/cortex-cli
cargo test --test rust_development_tests -- --nocapture
```

### Run Specific Test

```bash
cargo test test_implement_new_feature -- --nocapture
cargo test test_refactor_complex_code -- --nocapture
cargo test test_fix_compilation_errors -- --nocapture
```

### Expected Output

Each test prints:
- Step-by-step operation progress
- Token usage comparison (Traditional vs Cortex)
- Duration metrics
- Savings percentage
- Summary report

Example output:
```
================================================================================
TEST 1: IMPLEMENT NEW RUST FEATURE FROM SCRATCH
================================================================================

[Setup] Creating Rust project...

[Step 1] Creating cache trait with generics and lifetimes...
  ✓ Created Cache trait

[Step 2] Creating LRU cache implementation...
  ✓ Created LruCache implementation

[Step 3] Generating comprehensive tests...
  ✓ Generated test cases

[Step 4] Verifying compilation...
  Compilation: ✓ Success

====================================================================================================
TEST SUMMARY: Implement New Feature
====================================================================================================

Operation                                          Duration   Traditional       Cortex  Savings %
----------------------------------------------------------------------------------------------------
Create Cache trait with generics                        12ms           800          120       85.0%
Create LRU implementation                               25ms          1500          200       86.7%
Generate comprehensive tests                            18ms          2000           80       96.0%
Verify compilation                                       5ms            50           30       40.0%
----------------------------------------------------------------------------------------------------
TOTAL                                                   60ms          4350          430       90.1%
====================================================================================================
```

## Test Infrastructure

### RustDevHarness

Main test harness providing:
- In-memory SurrealDB instance
- Virtual File System
- Code parser and semantic memory
- Workspace management
- Token counting and efficiency measurement

### TestMetrics

Tracks:
- Traditional token usage (manual approach)
- Cortex token usage (MCP tools)
- Operation duration
- Savings percentage

### Helper Functions

- `create_cache_project()` - Create cache system project
- `create_complex_rust_code()` - Create code needing refactoring
- `create_code_with_errors()` - Create code with intentional errors
- `create_performance_code()` - Create code needing optimization
- `create_security_test_code()` - Create code with security issues
- `create_multimodule_project()` - Create multi-module architecture
- `create_generic_code()` - Create code with complex generics
- `verify_rust_compiles()` - Verify Rust compilation

## Token Efficiency Methodology

### Traditional Approach Estimation

For each operation, we estimate traditional token usage by summing:

1. **Reading files**: 4 chars/token average
2. **Understanding context**: Estimated based on complexity
3. **Writing/editing code**: Full code tokens
4. **Verification**: Reading output, understanding errors

Example (Create Cache Trait):
- Read existing code: ~200 tokens
- Understand requirements: ~150 tokens
- Write trait definition: ~300 tokens
- Write documentation: ~100 tokens
- Verify compilation: ~50 tokens
- **Total**: 800 tokens

### Cortex Approach Measurement

Direct measurement of:
1. Tool invocation JSON payload
2. Result parsing
3. Minimal context switching

Example (Create Cache Trait):
- Tool call with specification: ~100 tokens
- Result confirmation: ~20 tokens
- **Total**: 120 tokens

**Savings**: (800 - 120) / 800 = 85%

## Verification

Each test verifies:

1. **Tool Execution Success**: Tools execute without errors (or gracefully handle "not implemented")
2. **Token Efficiency**: Savings exceed expected thresholds (50-65% depending on operation)
3. **Compilation**: Generated code compiles (when cargo available)
4. **Functionality**: Code works as expected (via tests)

## Future Enhancements

1. **Real Compilation Verification**: Run actual cargo build/test in CI
2. **Performance Benchmarking**: Measure actual performance improvements
3. **Integration with Real Projects**: Test on open-source Rust projects
4. **Multi-file Refactoring**: Complex cross-file refactorings
5. **Macro System Testing**: Test generation and analysis of Rust macros
6. **Async Code Patterns**: Test async/await pattern detection and optimization
7. **Error Recovery**: Test tool behavior when operations fail
8. **Incremental Development**: Test iterative development workflows

## Contributing

When adding new tests:

1. Follow the established pattern (Setup → Operations → Metrics → Summary)
2. Use real MCP tools (not mocks)
3. Estimate traditional tokens realistically
4. Measure actual Cortex tokens
5. Include comprehensive documentation
6. Verify compilation when possible
7. Add to test summary documentation

## Performance Targets

- **Each test**: < 5 seconds (excluding cargo compile)
- **Token savings**: > 50% minimum
- **Tool success rate**: > 90%
- **Compilation success**: 100% when cargo available

## Related Files

- `rust_development_tests.rs` - Main test implementation
- `../utils/test_harness.rs` - Shared test infrastructure
- `../utils/token_counter.rs` - Token counting utilities
- `../../src/mcp/tools/` - MCP tool implementations
