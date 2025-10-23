//! Mock LLM Responses for Deterministic Testing

use std::collections::HashMap;

/// Mock LLM response provider for testing
pub struct MockLlm {
    responses: HashMap<String, String>,
}

impl MockLlm {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Register a canned response for a query
    pub fn register_response(&mut self, query: &str, response: &str) {
        self.responses.insert(query.to_string(), response.to_string());
    }

    /// Get response for a query
    pub fn get_response(&self, query: &str) -> Option<&str> {
        self.responses.get(query).map(|s| s.as_str())
    }

    /// Create a mock with common responses
    pub fn with_common_responses() -> Self {
        let mut mock = Self::new();

        mock.register_response(
            "authentication",
            r#"{"functions": ["authenticate", "validate_token", "logout"]}"#,
        );

        mock.register_response(
            "database operations",
            r#"{"functions": ["query", "insert", "update", "delete"]}"#,
        );

        mock
    }
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_llm() {
        let mut mock = MockLlm::new();
        mock.register_response("test", "response");
        assert_eq!(mock.get_response("test"), Some("response"));
        assert_eq!(mock.get_response("other"), None);
    }
}
