//! Assertion helpers for MCP tool testing
//!
//! Provides specialized assertions for validating MCP tool results:
//! - Success/error validation
//! - Data structure validation
//! - Token efficiency checks
//! - Performance assertions
//! - Custom matchers for tool outputs

use mcp_sdk::prelude::*;
use serde_json::Value;
use std::fmt;

/// Extension trait for ToolResult assertions
pub trait ToolResultAssertions {
    /// Assert that the tool execution was successful
    fn assert_success(&self) -> &Self;

    /// Assert that the tool execution failed
    fn assert_error(&self) -> &Self;

    /// Assert that the result contains a specific field
    fn assert_has_field(&self, field: &str) -> &Self;

    /// Assert that a field has a specific value
    fn assert_field_equals(&self, field: &str, expected: &Value) -> &Self;

    /// Assert that a numeric field is within a range
    fn assert_field_in_range(&self, field: &str, min: f64, max: f64) -> &Self;

    /// Assert that an array field has a minimum length
    fn assert_array_min_length(&self, field: &str, min_len: usize) -> &Self;

    /// Assert that an array field has an exact length
    fn assert_array_length(&self, field: &str, len: usize) -> &Self;

    /// Get the result as a JSON value
    fn as_json(&self) -> Value;

    /// Get a field value
    fn get_field(&self, field: &str) -> Option<&Value>;

    /// Get a nested field using dot notation (e.g., "user.email")
    fn get_nested_field(&self, path: &str) -> Option<&Value>;
}

impl ToolResultAssertions for ToolResult {
    fn assert_success(&self) -> &Self {
        assert!(
            self.is_success(),
            "Expected successful tool execution, but got error: {:?}",
            self.content
        );
        self
    }

    fn assert_error(&self) -> &Self {
        assert!(
            !self.is_success(),
            "Expected error from tool execution, but got success"
        );
        self
    }

    fn assert_has_field(&self, field: &str) -> &Self {
        let json = self.as_json();
        assert!(
            json.get(field).is_some(),
            "Expected field '{}' in result, but it was not found. Result: {}",
            field,
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
        self
    }

    fn assert_field_equals(&self, field: &str, expected: &Value) -> &Self {
        let json = self.as_json();
        let actual = json.get(field).expect(&format!("Field '{}' not found", field));
        assert_eq!(
            actual, expected,
            "Field '{}' value mismatch.\nExpected: {}\nActual: {}",
            field,
            serde_json::to_string_pretty(expected).unwrap_or_default(),
            serde_json::to_string_pretty(actual).unwrap_or_default()
        );
        self
    }

    fn assert_field_in_range(&self, field: &str, min: f64, max: f64) -> &Self {
        let json = self.as_json();
        let value = json
            .get(field)
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
            .expect(&format!("Field '{}' not found or not a number", field));

        assert!(
            value >= min && value <= max,
            "Field '{}' value {} is not in range [{}, {}]",
            field, value, min, max
        );
        self
    }

    fn assert_array_min_length(&self, field: &str, min_len: usize) -> &Self {
        let json = self.as_json();
        let array = json
            .get(field)
            .and_then(|v| v.as_array())
            .expect(&format!("Field '{}' not found or not an array", field));

        assert!(
            array.len() >= min_len,
            "Array field '{}' has length {}, expected at least {}",
            field, array.len(), min_len
        );
        self
    }

    fn assert_array_length(&self, field: &str, len: usize) -> &Self {
        let json = self.as_json();
        let array = json
            .get(field)
            .and_then(|v| v.as_array())
            .expect(&format!("Field '{}' not found or not an array", field));

        assert_eq!(
            array.len(), len,
            "Array field '{}' has length {}, expected {}",
            field, array.len(), len
        );
        self
    }

    fn as_json(&self) -> Value {
        if self.content.is_empty() {
            return Value::Null;
        }

        serde_json::from_str(&self.content[0].text)
            .unwrap_or_else(|e| panic!("Failed to parse result as JSON: {}", e))
    }

    fn get_field(&self, field: &str) -> Option<&Value> {
        let json = self.as_json();
        json.get(field)
    }

    fn get_nested_field(&self, path: &str) -> Option<&Value> {
        let json = self.as_json();
        let parts: Vec<&str> = path.split('.').collect();

        let mut current = &json;
        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }
}

/// Assert that a tool execution was successful
pub fn assert_tool_success(result: &ToolResult) {
    result.assert_success();
}

/// Assert that a tool execution failed
pub fn assert_tool_error(result: &ToolResult) {
    result.assert_error();
}

/// Assert token efficiency meets a threshold
pub fn assert_token_efficiency(
    traditional_tokens: usize,
    cortex_tokens: usize,
    min_savings_percent: f64,
) {
    let savings = traditional_tokens.saturating_sub(cortex_tokens);
    let savings_percent = if traditional_tokens > 0 {
        100.0 * savings as f64 / traditional_tokens as f64
    } else {
        0.0
    };

    assert!(
        savings_percent >= min_savings_percent,
        "Token efficiency {:.1}% does not meet minimum threshold {:.1}%\n  Traditional: {} tokens\n  Cortex: {} tokens",
        savings_percent,
        min_savings_percent,
        traditional_tokens,
        cortex_tokens
    );
}

/// Assert performance meets a threshold
pub fn assert_performance(
    duration_ms: u64,
    max_duration_ms: u64,
    operation: &str,
) {
    assert!(
        duration_ms <= max_duration_ms,
        "Operation '{}' took {} ms, exceeds maximum {} ms",
        operation, duration_ms, max_duration_ms
    );
}

/// Custom assertions for specific data types

/// Assert that a value is a valid UUID string
pub fn assert_valid_uuid(value: &Value) {
    let uuid_str = value
        .as_str()
        .expect("Expected string value for UUID");

    uuid::Uuid::parse_str(uuid_str)
        .expect(&format!("Invalid UUID format: {}", uuid_str));
}

/// Assert that a value is a valid ISO timestamp
pub fn assert_valid_timestamp(value: &Value) {
    let timestamp_str = value
        .as_str()
        .expect("Expected string value for timestamp");

    chrono::DateTime::parse_from_rfc3339(timestamp_str)
        .expect(&format!("Invalid timestamp format: {}", timestamp_str));
}

/// Assert that a code unit has required fields
pub fn assert_valid_code_unit(unit: &Value) {
    assert!(unit.get("id").is_some(), "Code unit missing 'id' field");
    assert!(unit.get("name").is_some(), "Code unit missing 'name' field");
    assert!(unit.get("unit_type").is_some(), "Code unit missing 'unit_type' field");
    assert!(unit.get("file_path").is_some(), "Code unit missing 'file_path' field");
}

/// Assert that a file entry has required fields
pub fn assert_valid_file_entry(file: &Value) {
    assert!(file.get("path").is_some(), "File entry missing 'path' field");
    assert!(file.get("content_hash").is_some(), "File entry missing 'content_hash' field");
}

/// Assert that a dependency has required fields
pub fn assert_valid_dependency(dep: &Value) {
    assert!(dep.get("from").is_some(), "Dependency missing 'from' field");
    assert!(dep.get("to").is_some(), "Dependency missing 'to' field");
    assert!(dep.get("dep_type").is_some(), "Dependency missing 'dep_type' field");
}

/// Builder for complex assertions
pub struct AssertionBuilder<'a> {
    result: &'a ToolResult,
    json: Value,
}

impl<'a> AssertionBuilder<'a> {
    pub fn new(result: &'a ToolResult) -> Self {
        let json = result.as_json();
        Self { result, json }
    }

    /// Assert success
    pub fn success(self) -> Self {
        self.result.assert_success();
        self
    }

    /// Assert field exists
    pub fn has_field(self, field: &str) -> Self {
        self.result.assert_has_field(field);
        self
    }

    /// Assert field equals value
    pub fn field_equals(self, field: &str, expected: &Value) -> Self {
        self.result.assert_field_equals(field, expected);
        self
    }

    /// Assert array minimum length
    pub fn array_min_len(self, field: &str, min_len: usize) -> Self {
        self.result.assert_array_min_length(field, min_len);
        self
    }

    /// Assert with custom predicate
    pub fn assert(self, predicate: impl Fn(&Value) -> bool, message: &str) -> Self {
        assert!(
            predicate(&self.json),
            "Assertion failed: {}\nResult: {}",
            message,
            serde_json::to_string_pretty(&self.json).unwrap_or_default()
        );
        self
    }

    /// Get the JSON value for further processing
    pub fn json(&self) -> &Value {
        &self.json
    }
}

/// Matcher for semantic search results
pub struct SemanticSearchMatcher<'a> {
    results: &'a Value,
}

impl<'a> SemanticSearchMatcher<'a> {
    pub fn new(results: &'a Value) -> Self {
        Self { results }
    }

    /// Assert minimum number of results
    pub fn min_results(&self, min: usize) -> &Self {
        let results = self.results
            .as_array()
            .expect("Results should be an array");

        assert!(
            results.len() >= min,
            "Expected at least {} results, got {}",
            min, results.len()
        );
        self
    }

    /// Assert all results have minimum relevance score
    pub fn min_relevance(&self, min_score: f64) -> &Self {
        let results = self.results
            .as_array()
            .expect("Results should be an array");

        for (i, result) in results.iter().enumerate() {
            let score = result
                .get("relevance_score")
                .and_then(|v| v.as_f64())
                .expect(&format!("Result {} missing relevance_score", i));

            assert!(
                score >= min_score,
                "Result {} has relevance score {}, below minimum {}",
                i, score, min_score
            );
        }
        self
    }

    /// Assert results are sorted by relevance (descending)
    pub fn sorted_by_relevance(&self) -> &Self {
        let results = self.results
            .as_array()
            .expect("Results should be an array");

        if results.len() < 2 {
            return self;
        }

        for i in 0..results.len() - 1 {
            let score1 = results[i]
                .get("relevance_score")
                .and_then(|v| v.as_f64())
                .expect(&format!("Result {} missing relevance_score", i));

            let score2 = results[i + 1]
                .get("relevance_score")
                .and_then(|v| v.as_f64())
                .expect(&format!("Result {} missing relevance_score", i + 1));

            assert!(
                score1 >= score2,
                "Results not sorted by relevance at index {}: {} < {}",
                i, score1, score2
            );
        }
        self
    }
}

/// Matcher for dependency analysis results
pub struct DependencyMatcher<'a> {
    dependencies: &'a Value,
}

impl<'a> DependencyMatcher<'a> {
    pub fn new(dependencies: &'a Value) -> Self {
        Self { dependencies }
    }

    /// Assert minimum number of dependencies
    pub fn min_dependencies(&self, min: usize) -> &Self {
        let deps = self.dependencies
            .as_array()
            .expect("Dependencies should be an array");

        assert!(
            deps.len() >= min,
            "Expected at least {} dependencies, got {}",
            min, deps.len()
        );
        self
    }

    /// Assert contains specific dependency
    pub fn contains_dependency(&self, from: &str, to: &str) -> &Self {
        let deps = self.dependencies
            .as_array()
            .expect("Dependencies should be an array");

        let found = deps.iter().any(|dep| {
            let from_match = dep
                .get("from")
                .and_then(|v| v.as_str())
                .map(|s| s.contains(from))
                .unwrap_or(false);

            let to_match = dep
                .get("to")
                .and_then(|v| v.as_str())
                .map(|s| s.contains(to))
                .unwrap_or(false);

            from_match && to_match
        });

        assert!(
            found,
            "Expected dependency from '{}' to '{}' not found",
            from, to
        );
        self
    }

    /// Assert no circular dependencies
    pub fn no_circular(&self) -> &Self {
        // This is a simplified check - in practice, you'd need graph traversal
        let deps = self.dependencies
            .as_array()
            .expect("Dependencies should be an array");

        let mut seen = std::collections::HashSet::new();

        for dep in deps {
            let from = dep.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = dep.get("to").and_then(|v| v.as_str()).unwrap_or("");

            let forward = format!("{}→{}", from, to);
            let reverse = format!("{}→{}", to, from);

            assert!(
                !seen.contains(&reverse),
                "Circular dependency detected: {} ↔ {}",
                from, to
            );

            seen.insert(forward);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_sdk::content::Content;

    #[test]
    fn test_success_assertion() {
        let result = ToolResult {
            content: vec![Content::text(r#"{"status":"ok"}"#)],
            is_error: None,
        };

        result.assert_success();
    }

    #[test]
    #[should_panic(expected = "Expected successful tool execution")]
    fn test_success_assertion_fails() {
        let result = ToolResult {
            content: vec![Content::text("error")],
            is_error: Some(true),
        };

        result.assert_success();
    }

    #[test]
    fn test_field_assertions() {
        let result = ToolResult {
            content: vec![Content::text(r#"{"name":"test","count":5,"items":[]}"#)],
            is_error: None,
        };

        result
            .assert_has_field("name")
            .assert_has_field("count")
            .assert_field_equals("name", &Value::String("test".to_string()))
            .assert_field_in_range("count", 0.0, 10.0);
    }
}
