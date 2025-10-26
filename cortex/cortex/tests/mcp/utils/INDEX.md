# MCP Tools Test Infrastructure - Complete Index

## Overview

Comprehensive testing infrastructure for Cortex MCP tools with 2,298 lines of production-ready code across 5 core modules plus documentation.

**Location**: `cortex/cortex/tests/mcp/utils/`

## Files

### Core Modules (5 files, 2,298 LOC)

| File | Lines | Purpose |
|------|------:|---------|
| **mod.rs** | 18 | Public API, re-exports |
| **test_harness.rs** | 524 | Test environment setup, contexts |
| **assertions.rs** | 500 | Result validation, matchers |
| **token_counter.rs** | 397 | Efficiency measurement |
| **fixtures.rs** | 859 | Project templates, test data |

### Documentation (3 files, ~30KB)

| File | Size | Purpose |
|------|-----:|---------|
| **README.md** | 13KB | Complete usage guide |
| **QUICK_START.md** | 5KB | Fast reference, patterns |
| **SUMMARY.txt** | 8KB | Implementation overview |
| **INDEX.md** | This | Complete index |

### Examples

| File | Lines | Purpose |
|------|------:|---------|
| **test_infrastructure_example.rs** | 250+ | Usage demonstrations |

## Module Details

### 1. test_harness.rs

**Main Type**: `TestHarness`

**Features**:
- In-memory SurrealDB setup
- Temporary directory management
- VFS, parser, semantic memory, ingestion pipeline
- 6 context creators (workspace, vfs, code_nav, manipulation, semantic, deps)
- Workspace/project creation helpers
- Metrics tracking (performance, tokens)

**Key APIs**:
```rust
TestHarness::new() -> TestHarness
harness.workspace_context() -> WorkspaceContext
harness.create_test_workspace(name, path) -> TestWorkspace
harness.load_project(id, path) -> LoadResult
harness.record_metric(name, duration, tokens)
harness.print_summary()
```

**Helper Functions**:
- `create_rust_project(dir, name)` - Generate Rust project
- `create_typescript_project(dir, name)` - Generate TS project

### 2. assertions.rs

**Main Trait**: `ToolResultAssertions`

**Features**:
- Fluent assertion API
- Specialized matchers
- Data validation helpers
- Custom predicates

**Key Types**:
- `ToolResultAssertions` - Extension trait for ToolResult
- `SemanticSearchMatcher` - Validate search results
- `DependencyMatcher` - Validate dependency graphs
- `AssertionBuilder` - Fluent builder pattern

**Key APIs**:
```rust
result.assert_success()
result.assert_has_field("name")
result.assert_field_equals("status", &json!("ok"))
result.assert_field_in_range("count", 0.0, 100.0)
result.assert_array_min_length("items", 5)

SemanticSearchMatcher::new(&results)
    .min_results(5)
    .min_relevance(0.8)
    .sorted_by_relevance()

DependencyMatcher::new(&deps)
    .contains_dependency("from", "to")
    .no_circular()

AssertionBuilder::new(&result)
    .success()
    .has_field("count")
    .assert(predicate, message)
```

**Helper Functions**:
- `assert_token_efficiency(trad, cortex, min_pct)`
- `assert_performance(dur_ms, max_ms, op)`
- `assert_valid_uuid(value)`
- `assert_valid_timestamp(value)`
- `assert_valid_code_unit(unit)`

### 3. token_counter.rs

**Main Type**: `TokenCounter`

**Features**:
- Measurement tracking
- Category aggregation
- Multi-format reporting
- Token estimation utilities

**Key Types**:
- `TokenCounter` - Main counter
- `TokenMeasurement` - Single measurement
- `TokenComparison` - Traditional vs Cortex
- `EfficiencyReport` - Complete report

**Key APIs**:
```rust
counter.add_measurement(scenario, category, trad, cortex)
counter.total_savings() -> TokenComparison
counter.generate_report() -> EfficiencyReport
counter.print_summary()

report.to_json() -> String
report.save_to_file(path)
report.print_markdown()
```

**Estimation Functions**:
- `estimate_tokens(text)` - Basic estimation
- `estimate_read_file_tokens(bytes)` - File reading
- `estimate_grep_tokens(pattern, results, len)` - Grep operations
- `estimate_semantic_search_tokens(query, results)` - Semantic search
- `estimate_dependency_tokens(nodes, edges)` - Dependency analysis
- `estimate_code_unit_tokens(name, type, has_body)` - Code units

### 4. fixtures.rs

**Main Types**: `ProjectFixture`, `CodeFixture`

**Features**:
- 5 language support
- Complete project templates
- Realistic code samples
- Build configurations

**Languages**:
- **Rust**: Cargo.toml, lib.rs, main.rs, models.rs, utils.rs
- **TypeScript**: package.json, tsconfig.json, index.ts, models.ts, services.ts
- **JavaScript**: package.json, index.js, utils.js
- **Python**: setup.py, pyproject.toml, main.py, models.py, utils.py
- **Go**: go.mod, main.go, models.go, utils.go

**Key APIs**:
```rust
ProjectFixture::new(LanguageType::Rust, "name")
fixture.write_to(dir) -> PathBuf

CodeFixture::function(lang, name, params, body)
CodeFixture::class(lang, name, fields)
```

**Templates Include**:
- Dependencies (serde, tokio, express, pydantic, etc.)
- Models (User, Task with full implementations)
- Utilities (validation, formatting, calculations)
- Tests
- Build configs

### 5. mod.rs

Public API module that re-exports:
- `TestHarness`, `TestContext`, `TestWorkspace`
- `ToolResultAssertions`, `assert_tool_success`, `assert_tool_error`
- `TokenCounter`, `TokenComparison`, `EfficiencyReport`
- `ProjectFixture`, `CodeFixture`, `LanguageType`

## Usage Patterns

### Pattern 1: Basic Test
```rust
let harness = TestHarness::new().await;
let fixture = ProjectFixture::new(LanguageType::Rust, "test");
let path = fixture.write_to(harness.temp_path()).await.unwrap();
// Test...
```

### Pattern 2: With Assertions
```rust
result
    .assert_success()
    .assert_has_field("workspace_id")
    .assert_field_in_range("count", 1.0, 100.0);
```

### Pattern 3: Token Efficiency
```rust
let mut counter = TokenCounter::new();
counter.add_measurement("scenario", "category", 30000, 70);
counter.print_summary();
```

### Pattern 4: Multi-Language
```rust
for lang in [Rust, TypeScript, Python] {
    let fixture = ProjectFixture::new(lang, format!("{:?}-test", lang));
    // Test with fixture...
}
```

## Integration Points

### Dependencies
- `tokio` - Async runtime
- `tempfile` - Temporary directories
- `serde_json` - JSON parsing
- `cortex-storage` - Database
- `cortex-vfs` - Virtual filesystem
- `cortex-code-analysis` - Code parsing
- `cortex-memory` - Semantic search
- `mcp-sdk` - MCP framework

### Components Used
- ConnectionManager (in-memory DB)
- VirtualFileSystem
- ExternalProjectLoader
- MaterializationEngine
- CodeParser
- SemanticMemorySystem
- FileIngestionPipeline

## Statistics

**Code**:
- 5 modules: 2,298 lines
- 1 example: 250+ lines
- Total executable: ~2,550 lines

**Documentation**:
- 3 guides: ~30KB
- Coverage: All APIs documented
- Examples: 15+ complete examples

**Test Coverage**:
- 6 tool contexts
- 5 language templates
- 15+ assertion types
- 10+ token estimators
- 50+ helper functions

## Quick Reference

### Common Imports
```rust
mod utils;
use utils::{
    TestHarness, ToolResultAssertions, TokenCounter,
    ProjectFixture, LanguageType, assert_token_efficiency,
};
```

### Context Creation
```rust
harness.workspace_context()           // Workspace tools
harness.vfs_context()                 // VFS tools
harness.code_nav_context()            // Navigation tools
harness.code_manipulation_context()   // Manipulation tools
harness.semantic_search_context()     // Search tools
harness.dependency_context()          // Dependency tools
```

### Assertions
```rust
// Basic
result.assert_success()

// Field checks
result.assert_has_field("name")
result.assert_field_equals("status", &json!("ok"))

// Numeric
result.assert_field_in_range("count", 0.0, 100.0)

// Arrays
result.assert_array_min_length("items", 5)

// Custom
assert_token_efficiency(trad, cortex, 75.0)
assert_performance(duration_ms, 1000, "operation")
```

### Languages
```rust
LanguageType::Rust
LanguageType::TypeScript
LanguageType::JavaScript
LanguageType::Python
LanguageType::Go
```

## See Also

- **README.md** - Complete documentation
- **QUICK_START.md** - Fast reference
- **SUMMARY.txt** - Implementation details
- **test_infrastructure_example.rs** - Working examples

## Version

Created: 2025-10-23
Version: 1.0.0
Status: Production Ready
