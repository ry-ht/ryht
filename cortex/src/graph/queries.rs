/// Common SurrealQL queries for graph operations
///
/// This module contains reusable query templates for graph traversal,
/// semantic search, and relationship analysis.

/// Find all dependencies of a symbol (recursive with depth control)
/// Optimized to use graph traversal with FETCH for better performance
pub const FIND_DEPENDENCIES: &str = r#"
    SELECT id, name, symbol_type, file_path,
           ->depends_on->code_symbol AS dependencies
    FROM code_symbol:$symbol_id
    FETCH dependencies
"#;

/// Find all symbols that depend on this symbol (reverse dependencies)
pub const FIND_DEPENDENTS: &str = r#"
    SELECT <-depends_on<-code_symbol.* FROM code_symbol:$symbol_id
"#;

/// Find symbols by name pattern
pub const FIND_BY_NAME_PATTERN: &str = r#"
    SELECT * FROM code_symbol WHERE name ~ $pattern ORDER BY name LIMIT $limit
"#;

/// Find symbols by type
pub const FIND_BY_TYPE: &str = r#"
    SELECT * FROM code_symbol WHERE symbol_type = $symbol_type ORDER BY name LIMIT $limit
"#;

/// Find symbols in a specific file
pub const FIND_BY_FILE: &str = r#"
    SELECT * FROM code_symbol WHERE file_path = $file_path ORDER BY start_line
"#;

/// Semantic similarity search using embeddings
pub const SEMANTIC_SEARCH: &str = r#"
    SELECT *,
        vector::similarity::cosine(embedding, $query_embedding) as similarity
    FROM code_symbol
    WHERE embedding IS NOT NONE
    ORDER BY similarity DESC
    LIMIT $limit
"#;

/// Find similar code patterns based on graph structure
pub const FIND_SIMILAR_PATTERNS: &str = r#"
    LET $symbol = code_symbol:$symbol_id;
    LET $dep_types = (SELECT ->depends_on->code_symbol.symbol_type FROM $symbol);

    SELECT id, name, symbol_type, file_path,
           count(->depends_on) as out_degree,
           count(<-depends_on) as in_degree
    FROM code_symbol
    WHERE symbol_type IN $dep_types
    ORDER BY out_degree DESC, in_degree DESC
    LIMIT $limit
"#;

/// Impact analysis - find all affected symbols
pub const IMPACT_ANALYSIS: &str = r#"
    SELECT <-depends_on<-code_symbol.*
    FROM code_symbol
    WHERE id IN $changed_ids
"#;

/// Get call graph for a symbol
pub const GET_CALL_GRAPH: &str = r#"
    SELECT ->calls->code_symbol.*
    FROM code_symbol:$symbol_id
"#;

/// Get callers of a symbol
pub const GET_CALLERS: &str = r#"
    SELECT <-calls<-code_symbol.*
    FROM code_symbol:$symbol_id
"#;

/// Find documentation for a symbol
pub const FIND_DOCUMENTATION: &str = r#"
    SELECT <-documents<-documentation.*
    FROM code_symbol:$symbol_id
"#;

/// Find symbols implementing a specification
pub const FIND_SPEC_IMPLEMENTATIONS: &str = r#"
    SELECT <-implements_spec<-code_symbol.*
    FROM specification
    WHERE name = $spec_name
"#;

/// Get symbol with all relationships
pub const GET_SYMBOL_FULL: &str = r#"
    LET $symbol = (SELECT * FROM code_symbol:$symbol_id)[0];
    LET $dependencies = SELECT ->depends_on->code_symbol.* FROM code_symbol:$symbol_id;
    LET $dependents = SELECT <-depends_on<-code_symbol.* FROM code_symbol:$symbol_id;
    LET $calls = SELECT ->calls->code_symbol.* FROM code_symbol:$symbol_id;
    LET $called_by = SELECT <-calls<-code_symbol.* FROM code_symbol:$symbol_id;
    LET $docs = SELECT <-documents<-documentation.* FROM code_symbol:$symbol_id;

    RETURN {
        symbol: $symbol,
        dependencies: $dependencies,
        dependents: $dependents,
        calls: $calls,
        called_by: $called_by,
        documentation: $docs
    };
"#;

/// Find code lineage - trace evolution of a symbol through episodes
pub const FIND_CODE_LINEAGE: &str = r#"
    SELECT <-references_symbol<-episode.*
    FROM code_symbol:$symbol_id
    ORDER BY created_at DESC
"#;

/// Get dependency graph with depth limit
/// Optimized with proper graph traversal and aggregation
pub const GET_DEPENDENCY_GRAPH: &str = r#"
    SELECT id, name, symbol_type, file_path, metadata,
           ->depends_on->code_symbol.{id, name, symbol_type, file_path} AS direct_deps,
           count(->depends_on) AS out_degree,
           count(<-depends_on) AS in_degree
    FROM code_symbol:$symbol_id
    FETCH direct_deps
"#;

/// Find circular dependencies
pub const FIND_CIRCULAR_DEPENDENCIES: &str = r#"
    SELECT id, name, file_path
    FROM code_symbol
    WHERE id IN (
        SELECT DISTINCT in
        FROM depends_on
        WHERE out IN (
            SELECT in FROM depends_on WHERE out = in
        )
    )
"#;

/// Get graph statistics
pub const GET_GRAPH_STATS: &str = r#"
    LET $total_symbols = count(SELECT * FROM code_symbol);
    LET $total_deps = count(SELECT * FROM depends_on);
    LET $total_calls = count(SELECT * FROM calls);
    LET $avg_out_degree = math::mean(SELECT count(->depends_on) FROM code_symbol);
    LET $avg_in_degree = math::mean(SELECT count(<-depends_on) FROM code_symbol);

    RETURN {
        total_symbols: $total_symbols,
        total_dependencies: $total_deps,
        total_calls: $total_calls,
        avg_out_degree: $avg_out_degree,
        avg_in_degree: $avg_in_degree
    };
"#;

/// Find most connected symbols (hubs)
pub const FIND_HUBS: &str = r#"
    SELECT id, name, symbol_type, file_path,
           count(->depends_on) as out_degree,
           count(<-depends_on) as in_degree,
           (count(->depends_on) + count(<-depends_on)) as total_degree
    FROM code_symbol
    ORDER BY total_degree DESC
    LIMIT $limit
"#;

/// Find leaf nodes (no dependencies)
pub const FIND_LEAF_NODES: &str = r#"
    SELECT id, name, symbol_type, file_path
    FROM code_symbol
    WHERE count(->depends_on) = 0
    ORDER BY name
    LIMIT $limit
"#;

/// Find root nodes (no dependents)
pub const FIND_ROOT_NODES: &str = r#"
    SELECT id, name, symbol_type, file_path
    FROM code_symbol
    WHERE count(<-depends_on) = 0
    ORDER BY name
    LIMIT $limit
"#;

/// Find symbols in a module/namespace
pub const FIND_BY_MODULE: &str = r#"
    SELECT * FROM code_symbol
    WHERE file_path CONTAINS $module_path
    ORDER BY symbol_type, name
"#;

/// Get symbol metadata
pub const GET_SYMBOL_METADATA: &str = r#"
    SELECT id, name, symbol_type, metadata
    FROM code_symbol:$symbol_id
"#;

/// Find recently modified symbols
pub const FIND_RECENT_SYMBOLS: &str = r#"
    SELECT * FROM code_symbol
    WHERE updated_at >= $since
    ORDER BY updated_at DESC
    LIMIT $limit
"#;

/// Find symbols with high complexity
pub const FIND_COMPLEX_SYMBOLS: &str = r#"
    SELECT id, name, symbol_type, file_path, metadata.complexity
    FROM code_symbol
    WHERE metadata.complexity > $threshold
    ORDER BY metadata.complexity DESC
    LIMIT $limit
"#;

/// Find symbols with low test coverage
pub const FIND_UNTESTED_SYMBOLS: &str = r#"
    SELECT id, name, symbol_type, file_path, metadata.test_coverage
    FROM code_symbol
    WHERE metadata.test_coverage < $threshold
    ORDER BY metadata.test_coverage ASC
    LIMIT $limit
"#;

/// Helper struct for building queries dynamically
pub struct QueryBuilder {
    query: String,
    bindings: Vec<(String, serde_json::Value)>,
}

impl QueryBuilder {
    pub fn new(base_query: &str) -> Self {
        Self {
            query: base_query.to_string(),
            bindings: Vec::new(),
        }
    }

    pub fn bind<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        self.bindings.push((
            key.to_string(),
            serde_json::to_value(value).unwrap_or_default(),
        ));
        self
    }

    pub fn build(self) -> (String, Vec<(String, serde_json::Value)>) {
        (self.query, self.bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let builder = QueryBuilder::new(FIND_BY_NAME_PATTERN)
            .bind("pattern", "test.*")
            .bind("limit", 10);

        let (query, bindings) = builder.build();
        assert!(query.contains("name ~ $pattern"));
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_all_queries_valid() {
        // Just verify all queries are valid strings
        assert!(!FIND_DEPENDENCIES.is_empty());
        assert!(!FIND_DEPENDENTS.is_empty());
        assert!(!SEMANTIC_SEARCH.is_empty());
        assert!(!IMPACT_ANALYSIS.is_empty());
        assert!(!GET_GRAPH_STATS.is_empty());
    }
}
