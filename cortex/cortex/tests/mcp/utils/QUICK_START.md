# Quick Start Guide - MCP Test Infrastructure

## 30 Second Setup

```rust
mod utils;

use utils::{TestHarness, ToolResultAssertions};

#[tokio::test]
async fn my_first_test() {
    let harness = TestHarness::new().await;
    // You're ready to test!
}
```

## Common Test Patterns

### 1. Test a Workspace Tool

```rust
use utils::{TestHarness, ProjectFixture, LanguageType, ToolResultAssertions};

#[tokio::test]
async fn test_workspace_tool() {
    // Setup
    let harness = TestHarness::new().await;
    let fixture = ProjectFixture::new(LanguageType::Rust, "test");
    let path = fixture.write_to(harness.temp_path()).await.unwrap();

    // Execute
    let ctx = harness.workspace_context();
    let tool = WorkspaceCreateTool::new(ctx);
    let result = tool.execute(json!({
        "name": "Test",
        "root_path": path.to_string_lossy(),
    }), &ToolContext::default()).await.unwrap();

    // Verify
    result.assert_success().assert_has_field("workspace_id");
}
```

### 2. Measure Token Efficiency

```rust
use utils::TokenCounter;

#[tokio::test]
async fn test_efficiency() {
    let mut counter = TokenCounter::new();

    // Traditional: grep + read files
    // Cortex: semantic search
    counter.add_measurement("Find functions", "Search", 30000, 70);

    counter.print_summary();
    assert!(counter.total_savings().savings_percent > 95.0);
}
```

### 3. Test Multiple Languages

```rust
use utils::{TestHarness, ProjectFixture, LanguageType};

#[tokio::test]
async fn test_all_languages() {
    let harness = TestHarness::new().await;

    for lang in [LanguageType::Rust, LanguageType::TypeScript, LanguageType::Python] {
        let fixture = ProjectFixture::new(lang, format!("{:?}-test", lang));
        let path = fixture.write_to(harness.temp_path()).await.unwrap();
        // Test with this project...
    }
}
```

### 4. Complex Assertions

```rust
use utils::assertions::AssertionBuilder;

result
    .assert_success()
    .assert_has_field("count")
    .assert_array_min_length("items", 5)
    .assert_field_in_range("score", 0.0, 1.0);

// Or use builder
AssertionBuilder::new(&result)
    .success()
    .has_field("data")
    .assert(|json| json["count"] > 0, "count must be positive");
```

### 5. Semantic Search Validation

```rust
use utils::assertions::SemanticSearchMatcher;

let results = result.get_field("results").unwrap();
SemanticSearchMatcher::new(&results)
    .min_results(5)
    .min_relevance(0.8)
    .sorted_by_relevance();
```

### 6. Dependency Analysis

```rust
use utils::assertions::DependencyMatcher;

let deps = result.get_field("dependencies").unwrap();
DependencyMatcher::new(&deps)
    .min_dependencies(10)
    .contains_dependency("main", "utils")
    .no_circular();
```

## API Reference (Cheat Sheet)

### TestHarness
```rust
let harness = TestHarness::new().await;

// Paths
harness.temp_path()

// Contexts
harness.workspace_context()
harness.vfs_context()
harness.code_nav_context()
harness.code_manipulation_context()
harness.semantic_search_context()
harness.dependency_context()

// Operations
harness.create_test_workspace(name, path).await
harness.load_project(workspace_id, path).await
harness.ingest_file(workspace_id, path, content).await

// Metrics
harness.record_metric(name, duration_ms, tokens)
harness.record_traditional_tokens(tokens)
harness.record_cortex_tokens(tokens)
harness.print_summary()
```

### Assertions
```rust
result
    .assert_success()
    .assert_error()
    .assert_has_field("name")
    .assert_field_equals("status", &json!("ok"))
    .assert_field_in_range("count", 1.0, 100.0)
    .assert_array_min_length("items", 5)
    .assert_array_length("items", 10)
    .as_json()
    .get_field("name")
    .get_nested_field("user.email")
```

### TokenCounter
```rust
let mut counter = TokenCounter::new();

counter.add_measurement(scenario, category, traditional, cortex);
counter.measurements()
counter.total_savings()
counter.generate_report()
counter.print_summary()

// Helpers
estimate_tokens(text)
estimate_read_file_tokens(bytes)
estimate_grep_tokens(pattern, results, line_len)
estimate_semantic_search_tokens(query, results)
```

### Fixtures
```rust
// Project
let fixture = ProjectFixture::new(LanguageType::Rust, "name");
let path = fixture.write_to(dir).await.unwrap();

// Code snippet
let func = CodeFixture::function(lang, "name", "params", "body");
let class = CodeFixture::class(lang, "Name", &[("field", "type")]);

// Languages
LanguageType::Rust
LanguageType::TypeScript
LanguageType::JavaScript
LanguageType::Python
LanguageType::Go
```

## Best Practices

1. **Always use TestHarness** - Ensures clean environment
2. **Use fixtures** - Don't create files manually
3. **Chain assertions** - More readable
4. **Measure efficiency** - Track token savings
5. **Test multiple languages** - Where applicable
6. **Record metrics** - For performance tracking

## Common Imports

```rust
mod utils;

use utils::{
    TestHarness,
    ToolResultAssertions,
    TokenCounter,
    ProjectFixture,
    LanguageType,
    assert_token_efficiency,
};
use mcp_sdk::Tool;
use serde_json::json;
```

## Example Test File

```rust
mod utils;

use utils::{TestHarness, ToolResultAssertions, ProjectFixture, LanguageType};

#[tokio::test]
async fn test_my_tool() {
    // Setup
    let harness = TestHarness::new().await;
    let fixture = ProjectFixture::new(LanguageType::Rust, "test");
    let path = fixture.write_to(harness.temp_path()).await.unwrap();

    // Create workspace
    let workspace = harness.create_test_workspace("Test", &path).await;

    // Load project
    let result = harness.load_project(workspace.id, &path).await;
    assert!(result.files_loaded > 0);

    // Test your tool
    let ctx = harness.workspace_context();
    // ... test logic ...
}
```

## Troubleshooting

**Test fails with database error**
- Ensure you're using `TestHarness::new().await` (async)
- Check that tokio runtime is available

**Fixture creation fails**
- Verify temp directory is writable
- Check disk space

**Assertions fail unexpectedly**
- Use `result.as_json()` to inspect result
- Print the actual value with `println!("{:?}", result.as_json())`

**Token counts seem wrong**
- Token estimation is approximate
- Use real measurements for production

## Need More Help?

- See `README.md` for detailed documentation
- Check `test_infrastructure_example.rs` for examples
- Look at existing tests in `tests/mcp/` directory
