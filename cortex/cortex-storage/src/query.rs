//! Query builder and utilities for SurrealDB operations.

use serde::{Deserialize, Serialize};

/// Query builder for constructing SurrealQL queries.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    query: String,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }

    /// Add a SELECT statement
    pub fn select(mut self, fields: &str, from: &str) -> Self {
        self.query = format!("SELECT {} FROM {}", fields, from);
        self
    }

    /// Add a WHERE clause
    pub fn where_clause(mut self, condition: &str) -> Self {
        self.query.push_str(&format!(" WHERE {}", condition));
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, field: &str, desc: bool) -> Self {
        let direction = if desc { "DESC" } else { "ASC" };
        self.query.push_str(&format!(" ORDER BY {} {}", field, direction));
        self
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: usize) -> Self {
        self.query.push_str(&format!(" LIMIT {}", limit));
        self
    }

    /// Build the query string
    pub fn build(self) -> String {
        self.query
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Pagination parameters for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: usize,
    pub limit: usize,
}

impl Pagination {
    /// Create new pagination parameters
    pub fn new(offset: usize, limit: usize) -> Self {
        Self { offset, limit }
    }

    /// Get default pagination (first 20 items)
    pub fn default_page() -> Self {
        Self {
            offset: 0,
            limit: 20,
        }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self::default_page()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .select("*", "documents")
            .where_clause("project_id = $project_id")
            .order_by("created_at", true)
            .limit(10)
            .build();

        assert!(query.contains("SELECT * FROM documents"));
        assert!(query.contains("WHERE project_id = $project_id"));
        assert!(query.contains("ORDER BY created_at DESC"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_pagination() {
        let page = Pagination::new(20, 10);
        assert_eq!(page.offset, 20);
        assert_eq!(page.limit, 10);

        let default = Pagination::default_page();
        assert_eq!(default.offset, 0);
        assert_eq!(default.limit, 20);
    }
}
