//! Query processing, expansion, and intent detection.

use crate::error::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
}

impl QueryProcessor {
    pub fn new() -> Self {
        Self {
            expander: QueryExpander::new(),
        }
    }

    /// Process a raw query string.
    pub fn process(&self, query: &str) -> Result<ProcessedQuery> {
        let normalized = self.normalize(query);
        let intent = self.detect_intent(&normalized);
        let keywords = self.extract_keywords(&normalized);
        let filters = self.extract_filters(query);
        let expanded = self.expander.expand(&normalized, &intent);

        Ok(ProcessedQuery {
            original: query.to_string(),
            normalized,
            expanded,
            intent,
            keywords,
            filters,
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
        assert_eq!(intent, QueryIntent::Examples);
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
        assert_eq!(processed.intent, QueryIntent::Examples);
        assert!(!processed.keywords.is_empty());
        assert!(!processed.expanded.is_empty());
    }
}
