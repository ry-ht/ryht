# Testing Guide for cc-sdk

## Quick Start

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test file
cargo test --test error_tests

# Run with output
cargo test -- --nocapture

# Run ignored tests (require Claude binary)
cargo test -- --ignored
```

## Test Organization

### Unit Tests (in `tests/`)

1. **error_tests.rs** - Error system tests
   - All error variants and conversions
   - Error chaining and sources
   - Result type integration

2. **message_tests.rs** - Message type tests
   - Message serialization/deserialization
   - Content block variants
   - Message equality and cloning

3. **options_tests.rs** - Configuration tests
   - ClaudeCodeOptions builder
   - MCP server configurations
   - Tool filtering

4. **permissions_tests.rs** - Permission system tests
   - Permission modes and behaviors
   - Permission updates and rules
   - Permission results

5. **requests_tests.rs** - Control protocol tests
   - SDK control requests
   - Request/response serialization
   - All request subtypes

### Integration Tests

6. **session_writer_tests.rs** - Session writing
   - File operations
   - JSONL format
   - Concurrent writes

7. **session_filter_tests.rs** - Session filtering
   - Filter builder
   - Search logic
   - Sorting and pagination

8. **session_management_tests.rs** - Session operations
   - Export formats
   - Fork/merge concepts
   - Backup operations

9. **client_builder_tests.rs** - Client builder
   - Builder pattern
   - Configuration options
   - State transitions

10. **session_tests.rs** - Session module (existing)
    - Cache operations
    - Types and metadata
    - Project/Session structures

## Test Patterns

### Unit Test Pattern

```rust
#[test]
fn test_feature_name() {
    // Arrange
    let input = create_test_data();

    // Act
    let result = function_under_test(input);

    // Assert
    assert_eq!(result, expected_value);
}
```

### Serialization Test Pattern

```rust
#[test]
fn test_type_serialization() {
    let obj = MyType { field: "value" };

    // Serialize
    let json = serde_json::to_string(&obj).unwrap();
    assert!(json.contains("expected_json"));

    // Deserialize
    let deserialized: MyType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, obj);
}
```

### Integration Test Pattern

```rust
#[tokio::test]
async fn test_integration_scenario() {
    // Setup
    let temp_dir = TempDir::new().unwrap();

    // Execute
    let result = async_operation(&temp_dir).await;

    // Verify
    assert!(result.is_ok());

    // Cleanup (automatic with TempDir)
}
```

### Ignored Test Pattern (Requires Claude Binary)

```rust
#[tokio::test]
#[ignore = "Requires Claude binary to be installed"]
async fn test_with_real_binary() {
    match ClaudeClient::builder().discover_binary().await {
        Ok(builder) => {
            // Test with real binary
        }
        Err(_) => {
            println!("Claude binary not found (expected in test environment)");
        }
    }
}
```

## Testing Best Practices

### 1. Test Naming

- Use descriptive names: `test_error_conversion_from_binary_to_top_level`
- Group related tests: `test_permission_mode_*`, `test_session_filter_*`
- Indicate test type: `test_serialize_*`, `test_create_*`, `test_validate_*`

### 2. Test Independence

- Each test should be independent
- Use `TempDir` for file operations
- Don't rely on test execution order
- Clean up resources (or use RAII types)

### 3. Test Coverage

- Test success paths ✅
- Test error paths ✅
- Test edge cases ✅
- Test boundary conditions ✅

### 4. Mock vs Real

- Unit tests: Use mocks/stubs
- Integration tests: Use temporary files/dirs
- E2E tests: Mark as `#[ignore]` if requiring external dependencies

### 5. Assertions

```rust
// Good: Specific assertions
assert_eq!(actual, expected);
assert!(condition, "Descriptive message");
assert!(result.is_ok());

// Good: Pattern matching for enums
match error {
    Error::Binary(BinaryError::NotFound { .. }) => {
        // Expected
    }
    _ => panic!("Unexpected error variant"),
}
```

## Test Utilities

### Tempfile for File Tests

```rust
use tempfile::TempDir;

#[test]
fn test_with_temp_dir() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Use temp_dir for tests
    fs::write(&file_path, "test data").unwrap();

    // Cleanup is automatic
}
```

### Creating Test Data

```rust
fn create_test_message() -> Message {
    Message::User {
        message: UserMessage {
            content: "Test".to_string(),
        },
    }
}

fn create_test_session() -> Session {
    Session::new(
        SessionId::new("test"),
        PathBuf::from("/test"),
        Utc::now(),
        Some("Test message".to_string()),
    )
}
```

## Running Specific Test Categories

```bash
# Run all error tests
cargo test error_

# Run all serialization tests
cargo test serialization

# Run all builder tests
cargo test builder

# Run tests matching pattern
cargo test --test '*_tests' -- message
```

## Coverage Analysis

```bash
# Install tarpaulin (coverage tool)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Open coverage report
open coverage/index.html
```

## Debugging Tests

```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run with full backtrace
RUST_BACKTRACE=full cargo test

# Run single test with output
cargo test test_name -- --nocapture --exact

# Run with logging
RUST_LOG=debug cargo test
```

## Common Test Scenarios

### Testing Async Code

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Testing Error Conversion

```rust
#[test]
fn test_error_conversion() {
    let specific_error = BinaryError::NotFound { searched_paths: vec![] };
    let general_error: Error = specific_error.into();

    assert!(matches!(general_error, Error::Binary(_)));
}
```

### Testing Serialization

```rust
#[test]
fn test_roundtrip_serialization() {
    let original = create_test_object();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}
```

### Testing Builder Pattern

```rust
#[test]
fn test_builder_chaining() {
    let result = Builder::new()
        .with_option_a("value_a")
        .with_option_b("value_b")
        .build();

    assert_eq!(result.option_a, Some("value_a".to_string()));
}
```

## CI/CD Integration

```yaml
# Example GitHub Actions workflow
- name: Run tests
  run: cargo test --all-features

- name: Run ignored tests
  run: cargo test -- --ignored
  continue-on-error: true

- name: Generate coverage
  run: cargo tarpaulin --out Xml

- name: Upload coverage
  uses: codecov/codecov-action@v3
```

## Test Maintenance

- Update tests when API changes
- Add tests for new features
- Remove tests for deprecated features
- Keep test data realistic
- Document complex test scenarios
- Review test failures promptly

## Getting Help

- Check test output for specific error messages
- Use `--nocapture` to see println! output
- Use `RUST_BACKTRACE=1` for stack traces
- Check TEST_COVERAGE_REPORT.md for known issues
- Consult crate documentation for API details
