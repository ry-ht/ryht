# MCP Tools Test Infrastructure

Comprehensive testing utilities for Cortex MCP tools, providing setup, assertions, token counting, and fixtures.

## Overview

This test infrastructure provides everything needed to test MCP tools effectively:

- **TestHarness**: Complete test environment setup with in-memory database, VFS, and all Cortex components
- **Assertions**: Specialized assertions for MCP tool results, semantic search, dependencies, and more
- **TokenCounter**: Measure and compare token efficiency between traditional and Cortex approaches
- **Fixtures**: Pre-built project templates for Rust, TypeScript, JavaScript, Python, and Go

## Quick Start

```rust
use utils::{TestHarness, ToolResultAssertions, ProjectFixture, LanguageType};

#[tokio::test]
async fn my_test() {
    // Setup
    let harness = TestHarness::new().await;

    // Create test project
    let fixture = ProjectFixture::new(LanguageType::Rust, "my-project");
    let project_path = fixture.write_to(harness.temp_path()).await.unwrap();

    // Create workspace
    let workspace = harness
        .create_test_workspace("MyWorkspace", &project_path)
        .await;

    // Test your tool
    let ctx = harness.workspace_context();
    // ... use the context
}
```

## Components

### 1. TestHarness

Complete test environment with all Cortex components.

#### Features

- In-memory SurrealDB database
- Temporary directory management
- VFS, parser, semantic memory, ingestion pipeline
- Context creation for all tool categories
- Performance and token tracking

#### Usage

```rust
// Create harness
let mut harness = TestHarness::new().await;

// Get temp directory
let temp_path = harness.temp_path();

// Create contexts for different tool types
let workspace_ctx = harness.workspace_context();
let vfs_ctx = harness.vfs_context();
let code_nav_ctx = harness.code_nav_context();
let code_manip_ctx = harness.code_manipulation_context();
let semantic_ctx = harness.semantic_search_context();
let dep_ctx = harness.dependency_context();

// Create workspace
let workspace = harness
    .create_test_workspace("TestWS", &project_path)
    .await;

// Load project
let result = harness.load_project(workspace.id, &project_path).await;
println!("Loaded {} files, {} units",
    result.files_loaded, result.units_extracted);

// Track metrics
harness.record_metric("operation", duration_ms, tokens);
harness.record_traditional_tokens(30000);
harness.record_cortex_tokens(100);

// Print summary
harness.print_summary();
```

### 2. Assertions

Specialized assertions for MCP tool results.

#### Tool Result Assertions

```rust
use utils::ToolResultAssertions;

result
    .assert_success()
    .assert_has_field("workspace_id")
    .assert_field_equals("status", &json!("ok"))
    .assert_field_in_range("count", 1.0, 100.0)
    .assert_array_min_length("items", 5);

// Get values
let json = result.as_json();
let field = result.get_field("name");
let nested = result.get_nested_field("user.email");
```

#### Semantic Search Matcher

```rust
use utils::assertions::SemanticSearchMatcher;

let results = json!([...]);
SemanticSearchMatcher::new(&results)
    .min_results(5)
    .min_relevance(0.8)
    .sorted_by_relevance();
```

#### Dependency Matcher

```rust
use utils::assertions::DependencyMatcher;

let deps = json!([...]);
DependencyMatcher::new(&deps)
    .min_dependencies(10)
    .contains_dependency("main", "utils")
    .no_circular();
```

#### Assertion Builder

```rust
use utils::assertions::AssertionBuilder;

AssertionBuilder::new(&result)
    .success()
    .has_field("count")
    .field_equals("status", &json!("success"))
    .array_min_len("items", 3)
    .assert(|json| json["count"] > 0, "count must be positive");
```

#### Helper Functions

```rust
// Token efficiency
assert_token_efficiency(traditional, cortex, min_savings_percent);

// Performance
assert_performance(duration_ms, max_duration_ms, "operation");

// Data validation
assert_valid_uuid(&value);
assert_valid_timestamp(&value);
assert_valid_code_unit(&unit);
assert_valid_file_entry(&file);
assert_valid_dependency(&dep);
```

### 3. TokenCounter

Measure and compare token usage between traditional and Cortex approaches.

#### Basic Usage

```rust
use utils::TokenCounter;

let mut counter = TokenCounter::new();

// Add measurements
counter.add_measurement("Find functions", "Search", 30000, 70);
counter.add_measurement("Modify code", "Manipulation", 1200, 100);
counter.add_measurement("Analyze deps", "Analysis", 5000, 150);

// Get total savings
let total = counter.total_savings();
println!("Savings: {:.1}%", total.savings_percent);

// Print summary
counter.print_summary();

// Generate report
let report = counter.generate_report();
report.print_markdown();
report.save_to_file(Path::new("report.json")).unwrap();
```

#### Token Estimation Helpers

```rust
use utils::token_counter::*;

// Estimate tokens from text
let tokens = estimate_tokens("Hello world");

// Estimate file tokens
let tokens = estimate_read_file_tokens(file_size_bytes);

// Estimate operation tokens
let tokens = estimate_grep_tokens("pattern", num_results, avg_line_length);
let tokens = estimate_semantic_search_tokens("query", num_results);
let tokens = estimate_dependency_tokens(num_nodes, num_edges);
let tokens = estimate_code_unit_tokens("function_name", "function", true);
```

#### Example Output

```
================================================================================
                        TOKEN EFFICIENCY REPORT
================================================================================

Scenario                                 Category             Traditional       Cortex   Savings %
----------------------------------------------------------------------------------------------------
Find all functions                       Search                     30000           70       99.8%
Modify specific method                   Manipulation                1200          100       91.7%
Analyze dependencies                     Analysis                   10000          150       98.5%

================================================================================
CATEGORY SUMMARIES
----------------------------------------------------------------------------------------------------
Category                                            Traditional       Cortex        Saved   Savings %
----------------------------------------------------------------------------------------------------
Search                                                    30000           70        29930       99.8%
Manipulation                                               1200          100         1100       91.7%
Analysis                                                  10000          150         9850       98.5%

================================================================================
OVERALL SUMMARY
----------------------------------------------------------------------------------------------------
Total Traditional Tokens:           41200
Total Cortex Tokens:                  320
Total Tokens Saved:                 40880
Overall Savings:                    99.2%
================================================================================
```

### 4. Fixtures

Pre-built project templates for multiple languages.

#### Supported Languages

- Rust
- TypeScript
- JavaScript
- Python
- Go

#### Usage

```rust
use utils::{ProjectFixture, LanguageType};

// Create project fixture
let fixture = ProjectFixture::new(LanguageType::Rust, "my-project");

// Write to directory
let project_path = fixture.write_to(temp_dir).await.unwrap();

// Projects include:
// - Build configuration (Cargo.toml, package.json, etc.)
// - Source files with realistic code
// - Multiple modules/files
// - Tests
// - .gitignore
```

#### Language Examples

**Rust Project**
- `Cargo.toml` with dependencies
- `src/lib.rs`, `src/main.rs`, `src/models.rs`, `src/utils.rs`
- Structs, functions, tests
- Serde, tokio dependencies

**TypeScript Project**
- `package.json`, `tsconfig.json`
- `src/index.ts`, `src/models.ts`, `src/services.ts`, `src/utils.ts`
- Interfaces, classes, Express server
- Type definitions

**Python Project**
- `setup.py`, `pyproject.toml`
- `src/main.py`, `src/models.py`, `src/utils.py`
- Classes with Pydantic models
- Type hints

**Go Project**
- `go.mod`
- `main.go`, `models.go`, `utils.go`
- Structs, methods, interfaces

#### Code Fixtures

```rust
use utils::CodeFixture;

// Generate function
let func = CodeFixture::function(
    LanguageType::Rust,
    "calculate_total",
    "values: &[f64]",
    "values.iter().sum()"
);

// Generate class/struct
let class = CodeFixture::class(
    LanguageType::TypeScript,
    "User",
    &[("id", "number"), ("name", "string")]
);
```

## Common Patterns

### Testing Workspace Tools

```rust
#[tokio::test]
async fn test_workspace_creation() {
    let harness = TestHarness::new().await;
    let fixture = ProjectFixture::new(LanguageType::Rust, "test");
    let path = fixture.write_to(harness.temp_path()).await.unwrap();

    let ctx = harness.workspace_context();
    let tool = WorkspaceCreateTool::new(ctx);

    let result = tool.execute(json!({
        "name": "Test",
        "root_path": path.to_string_lossy(),
        "auto_import": true,
    }), &ToolContext::default()).await.unwrap();

    result
        .assert_success()
        .assert_has_field("workspace_id")
        .assert_field_in_range("files_imported", 1.0, 100.0);
}
```

### Measuring Token Efficiency

```rust
#[tokio::test]
async fn test_token_efficiency() {
    let mut counter = TokenCounter::new();

    // Scenario: Find all error handling code
    // Traditional: grep + read 100 files
    let traditional = 100 * 600; // 100 files * 600 tokens each

    // Cortex: semantic search
    let cortex = 80;

    counter.add_measurement(
        "Find error handling",
        "Search",
        traditional,
        cortex
    );

    counter.print_summary();

    let total = counter.total_savings();
    assert!(total.savings_percent > 95.0);
}
```

### Multi-Language Testing

```rust
#[tokio::test]
async fn test_multi_language() {
    let harness = TestHarness::new().await;

    for lang in [LanguageType::Rust, LanguageType::TypeScript, LanguageType::Python] {
        let fixture = ProjectFixture::new(lang, format!("{:?}-project", lang));
        let path = fixture.write_to(harness.temp_path()).await.unwrap();

        let workspace = harness.create_test_workspace(
            &format!("{:?}WS", lang),
            &path
        ).await;

        let result = harness.load_project(workspace.id, &path).await;

        assert!(result.files_loaded > 0);
        assert!(result.units_extracted > 0);
    }
}
```

### Performance Testing

```rust
#[tokio::test]
async fn test_performance() {
    use std::time::Instant;

    let harness = TestHarness::new().await;
    // ... setup ...

    let start = Instant::now();
    let result = tool.execute(input, &ctx).await.unwrap();
    let duration = start.elapsed().as_millis() as u64;

    assert_performance(duration, 1000, "Tool execution");
    result.assert_success();
}
```

## Best Practices

1. **Use TestHarness for all tests**: Ensures consistent environment
2. **Record metrics**: Track performance and token usage
3. **Use fixtures**: Don't manually create test files
4. **Assertion chaining**: Use fluent API for readability
5. **Test efficiency**: Always measure token savings
6. **Multi-language**: Test with different languages where applicable
7. **Clean assertions**: Use specialized matchers for complex data

## Examples

See `test_infrastructure_example.rs` for complete examples of:
- Workspace creation with assertions
- Token efficiency measurement
- Fixture usage
- Complete workflow testing
- Semantic search validation
- Dependency analysis validation
- Assertion builder usage

## Architecture

```
utils/
├── mod.rs              # Public API exports
├── test_harness.rs     # Test environment setup
├── assertions.rs       # Result validation helpers
├── token_counter.rs    # Token efficiency measurement
└── fixtures.rs         # Test data generation
```

## Integration

The test infrastructure integrates with:
- **tokio**: Async test runtime
- **tempfile**: Temporary directory management
- **serde_json**: JSON parsing and manipulation
- **cortex-storage**: In-memory database
- **cortex-vfs**: Virtual filesystem
- **cortex-code-analysis**: Code parsing
- **cortex-memory**: Semantic search
- **mcp-sdk**: MCP tool framework

## Performance

- In-memory database for fast tests
- Temporary directories automatically cleaned up
- Lazy initialization where possible
- Efficient fixture generation

## Future Enhancements

Potential additions:
- More language templates (Java, C++, C#)
- Benchmark comparison utilities
- Test data snapshots
- Mock HTTP servers for API testing
- Database seed data helpers
- Visual test reports
