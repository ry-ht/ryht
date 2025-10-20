/// MCP tools for graph-based code analysis
///
/// These tools provide access to the graph code analyzer for:
/// - Dependency traversal
/// - Semantic similarity search
/// - Impact analysis
/// - Pattern detection

use crate::mcp::tools::Tool;
use serde_json::json;

/// Get all graph analysis tools
pub fn get_graph_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "graph.find_dependencies".to_string(),
            description: Some(
                "Find all dependencies of a code symbol (transitive up to specified depth)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to analyze"
                    },
                    "depth": {
                        "type": "integer",
                        "default": 3,
                        "minimum": 1,
                        "maximum": 10,
                        "description": "Maximum depth of dependency traversal (1-10)"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.find_dependents".to_string(),
            description: Some(
                "Find all symbols that depend on the given symbol (reverse dependencies)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to analyze"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.semantic_search".to_string(),
            description: Some(
                "Search for code symbols using semantic similarity (embedding-based)".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing the code to find"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum number of results to return"
                    }
                },
                "required": ["query"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.find_similar_patterns".to_string(),
            description: Some(
                "Find symbols with similar graph patterns (similar dependencies and structure)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to find patterns similar to"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum number of patterns to return"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.impact_analysis".to_string(),
            description: Some(
                "Analyze the impact of changes to specified symbols (find all affected code)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "changed_symbols": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of symbol IDs that have changed or will change"
                    }
                },
                "required": ["changed_symbols"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.code_lineage".to_string(),
            description: Some(
                "Trace the evolution of a code symbol through historical episodes".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to trace"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 10,
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Maximum number of historical entries to return"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.get_call_graph".to_string(),
            description: Some("Get the call graph for a symbol (what it calls)".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to analyze"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.get_callers".to_string(),
            description: Some("Get all symbols that call the given symbol".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to analyze"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.get_stats".to_string(),
            description: Some("Get overall graph statistics (symbol count, connections, etc.)"
                .to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.find_hubs".to_string(),
            description: Some(
                "Find the most connected symbols (hubs with high in/out degree)".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum number of hubs to return"
                    }
                },
                "required": []
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.find_circular_dependencies".to_string(),
            description: Some("Find circular dependencies in the codebase".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            output_schema: None,
            _meta: None,
        },
        Tool {
            name: "graph.get_symbol_full".to_string(),
            description: Some(
                "Get complete symbol information with all relationships (dependencies, dependents, calls, documentation)"
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol_id": {
                        "type": "string",
                        "description": "ID of the symbol to retrieve"
                    }
                },
                "required": ["symbol_id"]
            }),
            output_schema: None,
            _meta: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_valid() {
        let tools = get_graph_tools();
        assert!(!tools.is_empty());

        for tool in tools {
            assert!(!tool.name.is_empty());
            assert!(tool.name.starts_with("graph."));
            assert!(tool.description.is_some());
        }
    }

    #[test]
    fn test_tool_count() {
        let tools = get_graph_tools();
        assert_eq!(tools.len(), 12); // We have 12 graph tools
    }
}
