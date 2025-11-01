//! Query processing, expansion, and intent detection.
//!
//! Enhanced with query decomposition and multi-step reasoning based on 2025 RAG research.
//!
//! # References
//! - "Decomposed Prompting: A Modular Approach for Solving Complex Tasks" (Khot et al., 2023)
//! - "Self-Ask: Eliciting Reasoning via Self-Questioning" (Press et al., 2023)
//! - "Least-to-Most Prompting Enables Complex Reasoning in Large Language Models" (Zhou et al., 2023)

use crate::error::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use unicode_segmentation::UnicodeSegmentation;

/// Query intent classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// Looking for code (function, class, etc.)
    Code,
    /// Looking for documentation
    Documentation,
    /// Looking for examples
    Examples,
    /// General search
    General,
    /// Looking for similar items
    Similarity,
    /// Looking for definitions
    Definition,
}

/// Processed query with metadata.
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    pub original: String,
    pub normalized: String,
    pub expanded: Vec<String>,
    pub intent: QueryIntent,
    pub keywords: Vec<String>,
    pub filters: QueryFilters,
    /// Decomposed sub-queries for complex reasoning
    pub sub_queries: Vec<SubQuery>,
    /// Dependency graph for sub-queries
    pub query_graph: Option<QueryDependencyGraph>,
}

/// A sub-query extracted from a complex query.
///
/// Reference: "Decomposed Prompting" (Khot et al., 2023)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQuery {
    /// The sub-query text
    pub text: String,
    /// Priority/order of execution (lower = higher priority)
    pub priority: usize,
    /// IDs of sub-queries this depends on
    pub dependencies: Vec<usize>,
    /// Expected answer type
    pub expected_type: AnswerType,
}

/// Expected type of answer for a sub-query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnswerType {
    /// Factual information
    Fact,
    /// Code snippet
    Code,
    /// Explanation
    Explanation,
    /// List of items
    List,
    /// Yes/No answer
    Boolean,
}

/// Dependency graph for query execution planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDependencyGraph {
    /// Adjacency list representation
    pub edges: HashMap<usize, Vec<usize>>,
    /// Execution order (topologically sorted)
    pub execution_order: Vec<usize>,
}

/// Query filters extracted from the query.
#[derive(Debug, Clone, Default)]
pub struct QueryFilters {
    pub language: Option<String>,
    pub file_type: Option<String>,
    pub entity_type: Option<String>,
    pub exclude_terms: Vec<String>,
}

/// Query processor for parsing and expanding queries.
pub struct QueryProcessor {
    expander: QueryExpander,
    decomposer: QueryDecomposer,
}

impl QueryProcessor {
    pub fn new() -> Self {
        Self {
            expander: QueryExpander::new(),
            decomposer: QueryDecomposer::new(),
        }
    }

    /// Process a raw query string.
    pub fn process(&self, query: &str) -> Result<ProcessedQuery> {
        let normalized = self.normalize(query);
        let intent = self.detect_intent(&normalized);
        let keywords = self.extract_keywords(&normalized);
        let filters = self.extract_filters(query);
        let expanded = self.expander.expand(&normalized, &intent);

        // Decompose complex queries into sub-queries
        let (sub_queries, query_graph) = self.decomposer.decompose(&normalized, &intent);

        Ok(ProcessedQuery {
            original: query.to_string(),
            normalized,
            expanded,
            intent,
            keywords,
            filters,
            sub_queries,
            query_graph,
        })
    }

    /// Normalize query text.
    fn normalize(&self, query: &str) -> String {
        // Convert to lowercase
        let mut normalized = query.to_lowercase();

        // Remove extra whitespace
        let re = Regex::new(r"\s+").unwrap();
        normalized = re.replace_all(&normalized, " ").to_string();

        // Trim
        normalized.trim().to_string()
    }

    /// Detect query intent.
    fn detect_intent(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();

        // Code-related keywords
        if query_lower.contains("function")
            || query_lower.contains("class")
            || query_lower.contains("method")
            || query_lower.contains("implement")
            || query_lower.contains("code for")
        {
            return QueryIntent::Code;
        }

        // Documentation keywords
        if query_lower.contains("documentation")
            || query_lower.contains("docs")
            || query_lower.starts_with("what is")
            || query_lower.starts_with("how does")
            || query_lower.starts_with("explain")
        {
            return QueryIntent::Documentation;
        }

        // Example keywords
        if query_lower.contains("example")
            || query_lower.contains("sample")
            || query_lower.contains("how to")
            || query_lower.contains("usage")
        {
            return QueryIntent::Examples;
        }

        // Similarity keywords
        if query_lower.starts_with("similar to")
            || query_lower.contains("like this")
            || query_lower.contains("related to")
        {
            return QueryIntent::Similarity;
        }

        // Definition keywords
        if query_lower.starts_with("define")
            || query_lower.starts_with("what is")
            || query_lower.contains("definition")
        {
            return QueryIntent::Definition;
        }

        QueryIntent::General
    }

    /// Extract keywords from query.
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        // Simple keyword extraction based on word importance
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "should", "could", "may", "might",
            "can", "this", "that", "these", "those", "what", "which", "who", "when", "where",
            "why", "how",
        ]
        .iter()
        .cloned()
        .collect();

        query
            .unicode_words()
            .filter(|word| !stop_words.contains(word) && word.len() > 2)
            .map(|word| word.to_string())
            .collect()
    }

    /// Extract filters from query (language:rust, type:function, etc.).
    fn extract_filters(&self, query: &str) -> QueryFilters {
        let mut filters = QueryFilters::default();

        // Extract language filter
        let lang_re = Regex::new(r"language:(\w+)").unwrap();
        if let Some(caps) = lang_re.captures(query) {
            filters.language = Some(caps[1].to_string());
        }

        // Extract file type filter
        let type_re = Regex::new(r"type:(\w+)").unwrap();
        if let Some(caps) = type_re.captures(query) {
            filters.file_type = Some(caps[1].to_string());
        }

        // Extract entity type filter
        let entity_re = Regex::new(r"entity:(\w+)").unwrap();
        if let Some(caps) = entity_re.captures(query) {
            filters.entity_type = Some(caps[1].to_string());
        }

        // Extract exclusions
        let exclude_re = Regex::new(r"-(\w+)").unwrap();
        for caps in exclude_re.captures_iter(query) {
            filters.exclude_terms.push(caps[1].to_string());
        }

        filters
    }
}

impl Default for QueryProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Query expander for generating variations of queries.
pub struct QueryExpander {
    // In production, this would use a synonym database or word embeddings
}

impl QueryExpander {
    pub fn new() -> Self {
        Self {}
    }

    /// Expand query with synonyms and variations.
    pub fn expand(&self, query: &str, intent: &QueryIntent) -> Vec<String> {
        let mut expansions = vec![query.to_string()];

        // Add intent-specific expansions
        match intent {
            QueryIntent::Code => {
                self.expand_code_query(query, &mut expansions);
            }
            QueryIntent::Documentation => {
                self.expand_doc_query(query, &mut expansions);
            }
            QueryIntent::Examples => {
                self.expand_example_query(query, &mut expansions);
            }
            _ => {}
        }

        // Add common synonyms
        self.add_common_synonyms(query, &mut expansions);

        expansions
    }

    fn expand_code_query(&self, query: &str, expansions: &mut Vec<String>) {
        // Add code-specific variations
        if !query.contains("function") && !query.contains("method") {
            expansions.push(format!("{} function", query));
            expansions.push(format!("{} method", query));
        }

        if !query.contains("implement") {
            expansions.push(format!("implement {}", query));
        }
    }

    fn expand_doc_query(&self, query: &str, expansions: &mut Vec<String>) {
        // Add documentation-specific variations
        let query_clean = query
            .replace("what is", "")
            .replace("how does", "")
            .trim()
            .to_string();

        if !query_clean.is_empty() {
            expansions.push(format!("{} documentation", query_clean));
            expansions.push(format!("{} overview", query_clean));
            expansions.push(format!("{} guide", query_clean));
        }
    }

    fn expand_example_query(&self, query: &str, expansions: &mut Vec<String>) {
        // Add example-specific variations
        let query_clean = query.replace("example", "").replace("how to", "").trim().to_string();

        if !query_clean.is_empty() {
            expansions.push(format!("{} example", query_clean));
            expansions.push(format!("{} usage", query_clean));
            expansions.push(format!("{} tutorial", query_clean));
        }
    }

    fn add_common_synonyms(&self, query: &str, expansions: &mut Vec<String>) {
        // Common programming synonyms
        let synonyms = [
            ("function", "method"),
            ("method", "function"),
            ("class", "type"),
            ("struct", "type"),
            ("error", "exception"),
            ("bug", "issue"),
            ("create", "make"),
            ("delete", "remove"),
            ("update", "modify"),
        ];

        for (from, to) in &synonyms {
            if query.contains(from) {
                expansions.push(query.replace(from, to));
            }
        }
    }
}

impl Default for QueryExpander {
    fn default() -> Self {
        Self::new()
    }
}

/// Query decomposer for breaking complex queries into sub-queries.
///
/// Implements multi-step reasoning by decomposing complex queries into
/// simpler, sequential sub-queries that can be answered independently.
///
/// # Example
/// ```
/// use cortex_semantic::query::{QueryDecomposer, QueryIntent};
///
/// let decomposer = QueryDecomposer::new();
/// let (sub_queries, graph) = decomposer.decompose(
///     "How do I implement authentication and then integrate it with the database?",
///     &QueryIntent::Code
/// );
///
/// assert!(sub_queries.len() > 1);
/// ```
pub struct QueryDecomposer {
    // Future: Could include LLM client for more sophisticated decomposition
}

impl QueryDecomposer {
    pub fn new() -> Self {
        Self {}
    }

    /// Decompose a query into sub-queries with dependency tracking.
    ///
    /// Reference: "Least-to-Most Prompting" (Zhou et al., 2023)
    /// Breaks down complex queries into simpler steps that build on each other.
    pub fn decompose(
        &self,
        query: &str,
        intent: &QueryIntent,
    ) -> (Vec<SubQuery>, Option<QueryDependencyGraph>) {
        // Detect if query is complex (contains conjunctions, sequential steps)
        if !self.is_complex_query(query) {
            return (vec![], None);
        }

        let sub_queries = match intent {
            QueryIntent::Code => self.decompose_code_query(query),
            QueryIntent::Documentation => self.decompose_doc_query(query),
            QueryIntent::Examples => self.decompose_example_query(query),
            _ => self.decompose_general_query(query),
        };

        let graph = if !sub_queries.is_empty() {
            Some(self.build_dependency_graph(&sub_queries))
        } else {
            None
        };

        (sub_queries, graph)
    }

    /// Check if a query is complex enough to warrant decomposition.
    fn is_complex_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Look for conjunctions and sequential markers
        query_lower.contains(" and ")
            || query_lower.contains(" then ")
            || query_lower.contains(" after ")
            || query_lower.contains(" also ")
            || query_lower.contains(" first ")
            || query_lower.contains(" next ")
            || query_lower.contains(" finally ")
            || query_lower.matches('?').count() > 1
    }

    /// Decompose a code-related query.
    ///
    /// Pattern: "Implement X and Y" -> ["Implement X", "Implement Y", "Integrate X and Y"]
    fn decompose_code_query(&self, query: &str) -> Vec<SubQuery> {
        let mut sub_queries = Vec::new();

        // Split on "and then", "then", "and", "also"
        let parts = self.split_on_conjunctions(query);

        for (i, part) in parts.iter().enumerate() {
            let dependencies = if i > 0 {
                vec![i - 1] // Each step depends on the previous one
            } else {
                vec![]
            };

            sub_queries.push(SubQuery {
                text: part.trim().to_string(),
                priority: i,
                dependencies,
                expected_type: AnswerType::Code,
            });
        }

        // Add integration step if multiple parts
        if sub_queries.len() > 1 {
            let integration = format!(
                "How to integrate {}?",
                parts.join(" and ")
            );
            sub_queries.push(SubQuery {
                text: integration,
                priority: sub_queries.len(),
                dependencies: (0..sub_queries.len()).collect(),
                expected_type: AnswerType::Explanation,
            });
        }

        sub_queries
    }

    /// Decompose a documentation query.
    fn decompose_doc_query(&self, query: &str) -> Vec<SubQuery> {
        let mut sub_queries = Vec::new();
        let parts = self.split_on_conjunctions(query);

        for (i, part) in parts.iter().enumerate() {
            sub_queries.push(SubQuery {
                text: part.trim().to_string(),
                priority: i,
                dependencies: vec![],
                expected_type: AnswerType::Explanation,
            });
        }

        sub_queries
    }

    /// Decompose an example query.
    fn decompose_example_query(&self, query: &str) -> Vec<SubQuery> {
        let mut sub_queries = Vec::new();

        // Pattern: "How to X and Y" -> ["How to X", "How to Y", "Complete example"]
        let parts = self.split_on_conjunctions(query);

        for (i, part) in parts.iter().enumerate() {
            sub_queries.push(SubQuery {
                text: part.trim().to_string(),
                priority: i,
                dependencies: if i > 0 { vec![i - 1] } else { vec![] },
                expected_type: AnswerType::Code,
            });
        }

        // Add synthesis step
        if sub_queries.len() > 1 {
            sub_queries.push(SubQuery {
                text: format!("Complete example combining all steps"),
                priority: sub_queries.len(),
                dependencies: (0..sub_queries.len()).collect(),
                expected_type: AnswerType::Code,
            });
        }

        sub_queries
    }

    /// Decompose a general query.
    fn decompose_general_query(&self, query: &str) -> Vec<SubQuery> {
        let parts = self.split_on_conjunctions(query);

        parts
            .into_iter()
            .enumerate()
            .map(|(i, part)| SubQuery {
                text: part.trim().to_string(),
                priority: i,
                dependencies: vec![],
                expected_type: AnswerType::Fact,
            })
            .collect()
    }

    /// Split query on common conjunctions and sequential markers.
    fn split_on_conjunctions(&self, query: &str) -> Vec<String> {
        // Try to intelligently split on conjunctions while preserving meaning
        // First try "and then" or "then"
        if query.contains(" and then ") {
            query.split(" and then ").map(|s| s.to_string()).collect()
        } else if query.contains(" then ") {
            query.split(" then ").map(|s| s.to_string()).collect()
        } else if query.contains(" and ") {
            // Split on "and" but be careful not to split things like "functions and methods"
            // This is a simple heuristic - in production, use an LLM for better parsing
            let potential_parts: Vec<&str> = query.split(" and ").collect();
            if potential_parts.len() <= 3 {
                // Only split if reasonable number of parts
                potential_parts.into_iter().map(|s| s.to_string()).collect()
            } else {
                vec![query.to_string()]
            }
        } else {
            vec![query.to_string()]
        }
    }

    /// Build dependency graph from sub-queries.
    ///
    /// Creates a topological ordering for execution planning.
    fn build_dependency_graph(&self, sub_queries: &[SubQuery]) -> QueryDependencyGraph {
        let mut edges: HashMap<usize, Vec<usize>> = HashMap::new();

        // Build adjacency list
        for (i, sub_query) in sub_queries.iter().enumerate() {
            for &dep in &sub_query.dependencies {
                edges.entry(dep).or_insert_with(Vec::new).push(i);
            }
        }

        // Topological sort using Kahn's algorithm
        let execution_order = self.topological_sort(sub_queries, &edges);

        QueryDependencyGraph {
            edges,
            execution_order,
        }
    }

    /// Perform topological sort on the dependency graph.
    fn topological_sort(
        &self,
        sub_queries: &[SubQuery],
        edges: &HashMap<usize, Vec<usize>>,
    ) -> Vec<usize> {
        let mut in_degree = vec![0; sub_queries.len()];

        // Calculate in-degrees
        for sub_query in sub_queries {
            for &dep in &sub_query.dependencies {
                if dep < sub_queries.len() {
                    in_degree[sub_query.priority] += 1;
                }
            }
        }

        // Queue of nodes with no incoming edges
        let mut queue: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter_map(|(i, &deg)| if deg == 0 { Some(i) } else { None })
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node);

            // Reduce in-degree for neighbors
            if let Some(neighbors) = edges.get(&node) {
                for &neighbor in neighbors {
                    in_degree[neighbor] -= 1;
                    if in_degree[neighbor] == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }

        result
    }
}

impl Default for QueryDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let processor = QueryProcessor::new();

        let query = "  How   to  create   a   function  ";
        let normalized = processor.normalize(query);
        assert_eq!(normalized, "how to create a function");
    }

    #[test]
    fn test_detect_intent_code() {
        let processor = QueryProcessor::new();

        let intent = processor.detect_intent("find function to parse JSON");
        assert_eq!(intent, QueryIntent::Code);

        let intent = processor.detect_intent("implement authentication");
        assert_eq!(intent, QueryIntent::Code);
    }

    #[test]
    fn test_detect_intent_documentation() {
        let processor = QueryProcessor::new();

        let intent = processor.detect_intent("what is a closure");
        assert_eq!(intent, QueryIntent::Documentation);

        let intent = processor.detect_intent("explain async await");
        assert_eq!(intent, QueryIntent::Documentation);
    }

    #[test]
    fn test_detect_intent_examples() {
        let processor = QueryProcessor::new();

        let intent = processor.detect_intent("how to use regex");
        assert_eq!(intent, QueryIntent::Examples);

        let intent = processor.detect_intent("example of async function");
        assert_eq!(intent, QueryIntent::Code);
    }

    #[test]
    fn test_extract_keywords() {
        let processor = QueryProcessor::new();

        let keywords = processor.extract_keywords("find the best implementation for sorting");
        assert!(keywords.contains(&"find".to_string()));
        assert!(keywords.contains(&"best".to_string()));
        assert!(keywords.contains(&"implementation".to_string()));
        assert!(keywords.contains(&"sorting".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"for".to_string()));
    }

    #[test]
    fn test_extract_filters() {
        let processor = QueryProcessor::new();

        let filters = processor.extract_filters("find function language:rust type:async -deprecated");
        assert_eq!(filters.language, Some("rust".to_string()));
        assert_eq!(filters.file_type, Some("async".to_string()));
        assert!(filters.exclude_terms.contains(&"deprecated".to_string()));
    }

    #[test]
    fn test_query_expansion() {
        let expander = QueryExpander::new();

        let expanded = expander.expand("authentication", &QueryIntent::Code);
        assert!(expanded.len() > 1);
        assert!(expanded.contains(&"authentication".to_string()));
    }

    #[test]
    fn test_process_query() {
        let processor = QueryProcessor::new();

        let processed = processor.process("How to implement authentication?").unwrap();
        assert_eq!(processed.intent, QueryIntent::Code);
        assert!(!processed.keywords.is_empty());
        assert!(!processed.expanded.is_empty());
    }

    #[test]
    fn test_query_decomposition() {
        let decomposer = QueryDecomposer::new();

        let (sub_queries, graph) = decomposer.decompose(
            "How to implement authentication and then integrate it with the database?",
            &QueryIntent::Code,
        );

        assert!(sub_queries.len() > 1);
        assert!(graph.is_some());

        let graph = graph.unwrap();
        assert!(!graph.execution_order.is_empty());
    }

    #[test]
    fn test_simple_query_no_decomposition() {
        let decomposer = QueryDecomposer::new();

        let (sub_queries, graph) = decomposer.decompose(
            "What is authentication?",
            &QueryIntent::Documentation,
        );

        assert_eq!(sub_queries.len(), 0);
        assert!(graph.is_none());
    }

    #[test]
    fn test_dependency_graph_building() {
        let decomposer = QueryDecomposer::new();

        let (sub_queries, graph) = decomposer.decompose(
            "First create a user model, then add authentication, and finally test it",
            &QueryIntent::Code,
        );

        assert!(sub_queries.len() >= 3);
        assert!(graph.is_some());

        // Check that execution order respects dependencies
        let graph = graph.unwrap();
        for (i, &node_id) in graph.execution_order.iter().enumerate() {
            let node = &sub_queries[node_id];
            for &dep in &node.dependencies {
                // All dependencies should appear earlier in execution order
                assert!(graph.execution_order[..i].contains(&dep));
            }
        }
    }
}
