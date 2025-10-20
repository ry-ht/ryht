use crate::types::{ContextFragment, SemanticBridge, TokenCount, UnifiedContext};
use anyhow::Result;

/// Context defragmenter for combining scattered fragments
pub struct ContextDefragmenter {
    // Configuration options can be added here
}

impl ContextDefragmenter {
    pub fn new() -> Self {
        Self {}
    }

    /// Defragment scattered context into unified narrative
    pub fn defragment(
        &self,
        fragments: Vec<ContextFragment>,
        target_tokens: usize,
    ) -> Result<UnifiedContext> {
        if fragments.is_empty() {
            return Ok(UnifiedContext {
                main_narrative: String::new(),
                support_fragments: Vec::new(),
                total_tokens: TokenCount::zero(),
            });
        }

        // 1. Group fragments by semantic similarity
        let clusters = self.cluster_by_semantics(&fragments);

        // 2. Create semantic bridges between clusters
        let bridges = self.create_semantic_bridges(&clusters);

        // 3. Linearize for sequential presentation
        let narrative = self.linearize_for_llm(&clusters, &bridges);

        // 4. Compress support fragments if needed
        let (main, support) = self.split_main_and_support(&narrative, &fragments, target_tokens);

        let main_tokens = self.count_tokens(&main);
        let mut total_tokens = main_tokens;
        for fragment in &support {
            total_tokens.add(fragment.tokens);
        }

        Ok(UnifiedContext {
            main_narrative: main,
            support_fragments: support,
            total_tokens,
        })
    }

    /// Group fragments by semantic similarity
    fn cluster_by_semantics(&self, fragments: &[ContextFragment]) -> Vec<Cluster> {
        let mut clusters: Vec<Cluster> = Vec::new();

        for fragment in fragments {
            let mut best_cluster_idx = None;
            let mut best_similarity = 0.0;

            // Find most similar cluster
            for (idx, cluster) in clusters.iter().enumerate() {
                let similarity = self.calculate_similarity(fragment, cluster);
                if similarity > best_similarity && similarity > 0.3 {
                    best_similarity = similarity;
                    best_cluster_idx = Some(idx);
                }
            }

            // Add to best cluster or create new one
            if let Some(idx) = best_cluster_idx {
                clusters[idx].fragments.push(fragment.clone());
            } else {
                clusters.push(Cluster {
                    id: clusters.len(),
                    fragments: vec![fragment.clone()],
                    centroid: fragment.content.clone(),
                });
            }
        }

        clusters
    }

    /// Calculate semantic similarity between fragment and cluster
    fn calculate_similarity(&self, fragment: &ContextFragment, cluster: &Cluster) -> f32 {
        // Simple keyword-based similarity
        let fragment_words: Vec<&str> = fragment.content.split_whitespace().collect();
        let cluster_words: Vec<&str> = cluster.centroid.split_whitespace().collect();

        if fragment_words.is_empty() || cluster_words.is_empty() {
            return 0.0;
        }

        let mut common_words = 0;
        for word in &fragment_words {
            if cluster_words.contains(word) {
                common_words += 1;
            }
        }

        let max_len = fragment_words.len().max(cluster_words.len());
        common_words as f32 / max_len as f32
    }

    /// Create semantic bridges between clusters
    fn create_semantic_bridges(&self, clusters: &[Cluster]) -> Vec<SemanticBridge> {
        let mut bridges = Vec::new();

        for window in clusters.windows(2) {
            let from_cluster = &window[0];
            let to_cluster = &window[1];

            let connection = self.find_connection(from_cluster, to_cluster);
            let transition = self.generate_transition(from_cluster, to_cluster);

            bridges.push(SemanticBridge {
                from: from_cluster.id.to_string(),
                to: to_cluster.id.to_string(),
                connection,
                transition_text: transition,
            });
        }

        bridges
    }

    /// Find logical connection between clusters
    fn find_connection(&self, from: &Cluster, to: &Cluster) -> String {
        // Analyze content to determine relationship
        let from_keywords = self.extract_keywords(&from.centroid);
        let to_keywords = self.extract_keywords(&to.centroid);

        let common: Vec<_> = from_keywords
            .iter()
            .filter(|k| to_keywords.contains(k))
            .collect();

        if !common.is_empty() {
            format!(
                "Related through: {}",
                common
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            "Sequential flow".to_string()
        }
    }

    /// Generate transition text between clusters
    fn generate_transition(&self, from: &Cluster, to: &Cluster) -> String {
        let from_type = self.infer_cluster_type(from);
        let to_type = self.infer_cluster_type(to);

        match (from_type.as_str(), to_type.as_str()) {
            ("definition", "implementation") => {
                "The following implementation provides the concrete details:".to_string()
            }
            ("interface", "implementation") => {
                "This is implemented as follows:".to_string()
            }
            ("setup", "execution") => {
                "With the setup complete, the execution proceeds:".to_string()
            }
            _ => format!("Moving from {} to {}:", from_type, to_type),
        }
    }

    /// Linearize clusters and bridges into narrative
    fn linearize_for_llm(&self, clusters: &[Cluster], bridges: &[SemanticBridge]) -> String {
        let mut narrative = String::new();

        for (i, cluster) in clusters.iter().enumerate() {
            // Add cluster content
            narrative.push_str(&format!("## Section {}: {}\n\n", i + 1, cluster.id));

            for fragment in &cluster.fragments {
                narrative.push_str(&format!("### From {}\n", fragment.source));
                narrative.push_str(&fragment.content);
                narrative.push_str("\n\n");
            }

            // Add bridge to next cluster
            if i < bridges.len() {
                narrative.push_str(&format!("\n{}\n\n", bridges[i].transition_text));
            }
        }

        narrative
    }

    /// Split into main narrative and support fragments
    fn split_main_and_support(
        &self,
        narrative: &str,
        fragments: &[ContextFragment],
        target_tokens: usize,
    ) -> (String, Vec<ContextFragment>) {
        let narrative_tokens = self.count_tokens(narrative);
        let narrative_token_count: usize = narrative_tokens.into();

        if narrative_token_count <= target_tokens {
            // Everything fits
            return (narrative.to_string(), Vec::new());
        }

        // Need to split - keep most important in main, rest in support
        let main_budget = (target_tokens as f32 * 0.7) as usize;
        let main = self.truncate_to_tokens(narrative, main_budget);

        // Remaining fragments go to support
        let support = fragments
            .iter()
            .filter(|f| !main.contains(&f.content))
            .cloned()
            .collect();

        (main, support)
    }

    /// Extract keywords from text
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let important_patterns = [
            "fn ", "struct ", "impl ", "trait ", "enum ", "pub ", "class ", "interface ",
            "function ",
        ];

        let mut keywords = Vec::new();
        for line in text.lines() {
            for pattern in &important_patterns {
                if line.contains(pattern) {
                    // Extract the identifier after the keyword
                    if let Some(rest) = line.split(pattern).nth(1) {
                        if let Some(word) = rest.split_whitespace().next() {
                            keywords.push(
                                word.trim_end_matches(|c: char| !c.is_alphanumeric())
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }
        keywords
    }

    /// Infer the type of cluster from its content
    fn infer_cluster_type(&self, cluster: &Cluster) -> String {
        let content = &cluster.centroid;

        if content.contains("trait ") || content.contains("interface ") {
            "interface".to_string()
        } else if content.contains("impl ") || content.contains("class ") {
            "implementation".to_string()
        } else if content.contains("struct ") || content.contains("enum ") {
            "definition".to_string()
        } else if content.contains("fn main") || content.contains("async fn") {
            "execution".to_string()
        } else {
            "general".to_string()
        }
    }

    /// Truncate text to approximate token count
    fn truncate_to_tokens(&self, text: &str, target_tokens: usize) -> String {
        let chars_per_token = 4;
        let target_chars = target_tokens * chars_per_token;

        if text.len() <= target_chars {
            return text.to_string();
        }

        let mut truncated = String::from(&text[..target_chars]);
        truncated.push_str("\n\n... [Additional content available in support fragments]");
        truncated
    }

    /// Count tokens (rough estimation)
    fn count_tokens(&self, content: &str) -> TokenCount {
        TokenCount::new((content.len() / 4) as u32)
    }
}

impl Default for ContextDefragmenter {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal cluster structure
#[derive(Debug, Clone)]
struct Cluster {
    id: usize,
    fragments: Vec<ContextFragment>,
    centroid: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fragment(id: &str, content: &str, source: &str) -> ContextFragment {
        ContextFragment {
            id: id.to_string(),
            content: content.to_string(),
            source: source.to_string(),
            tokens: TokenCount::new((content.len() / 4) as u32),
        }
    }

    #[test]
    fn test_defragment_empty() {
        let defragmenter = ContextDefragmenter::new();
        let result = defragmenter.defragment(vec![], 1000).unwrap();
        assert!(result.main_narrative.is_empty());
        assert_eq!(result.total_tokens, TokenCount::zero());
    }

    #[test]
    fn test_defragment_single_fragment() {
        let defragmenter = ContextDefragmenter::new();
        let fragments = vec![create_test_fragment(
            "1",
            "fn main() { println!(\"Hello\"); }",
            "main.rs",
        )];

        let result = defragmenter.defragment(fragments, 1000).unwrap();
        assert!(!result.main_narrative.is_empty());
        assert!(result.main_narrative.contains("main.rs"));
    }

    #[test]
    fn test_defragment_multiple_fragments() {
        let defragmenter = ContextDefragmenter::new();
        let fragments = vec![
            create_test_fragment("1", "struct Point { x: i32, y: i32 }", "types.rs"),
            create_test_fragment("2", "fn add(a: i32, b: i32) -> i32 { a + b }", "math.rs"),
            create_test_fragment("3", "impl Point { fn new() -> Self { } }", "types.rs"),
        ];

        let result = defragmenter.defragment(fragments, 1000).unwrap();
        assert!(result.main_narrative.contains("types.rs"));
        assert!(result.main_narrative.contains("math.rs"));
    }

    #[test]
    fn test_cluster_by_semantics() {
        let defragmenter = ContextDefragmenter::new();
        let fragments = vec![
            create_test_fragment("1", "struct Point { x: i32 }", "types.rs"),
            create_test_fragment("2", "impl Point { fn new() }", "types.rs"),
            create_test_fragment("3", "fn add(a: i32) -> i32", "math.rs"),
        ];

        let clusters = defragmenter.cluster_by_semantics(&fragments);
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_extract_keywords() {
        let defragmenter = ContextDefragmenter::new();
        let text = "fn main() {}\nstruct Point {}\nimpl Point {}";
        let keywords = defragmenter.extract_keywords(text);

        assert!(keywords.contains(&"main".to_string()));
        assert!(keywords.contains(&"Point".to_string()));
    }

    #[test]
    fn test_infer_cluster_type() {
        let defragmenter = ContextDefragmenter::new();

        let trait_cluster = Cluster {
            id: 1,
            fragments: vec![],
            centroid: "trait MyTrait {}".to_string(),
        };
        assert_eq!(defragmenter.infer_cluster_type(&trait_cluster), "interface");

        let impl_cluster = Cluster {
            id: 2,
            fragments: vec![],
            centroid: "impl MyStruct {}".to_string(),
        };
        assert_eq!(
            defragmenter.infer_cluster_type(&impl_cluster),
            "implementation"
        );
    }

    #[test]
    fn test_create_semantic_bridges() {
        let defragmenter = ContextDefragmenter::new();
        let clusters = vec![
            Cluster {
                id: 1,
                fragments: vec![],
                centroid: "struct Point { x: i32 }".to_string(),
            },
            Cluster {
                id: 2,
                fragments: vec![],
                centroid: "impl Point { fn new() }".to_string(),
            },
        ];

        let bridges = defragmenter.create_semantic_bridges(&clusters);
        assert_eq!(bridges.len(), 1);
        assert!(!bridges[0].transition_text.is_empty());
    }

    #[test]
    fn test_truncate_to_tokens() {
        let defragmenter = ContextDefragmenter::new();
        let long_text = "a".repeat(1000);
        let truncated = defragmenter.truncate_to_tokens(&long_text, 10);

        assert!(truncated.len() < long_text.len());
        assert!(truncated.contains("Additional content"));
    }

    #[test]
    fn test_split_main_and_support() {
        let defragmenter = ContextDefragmenter::new();
        let fragments = vec![
            create_test_fragment("1", &"x".repeat(100), "file1.rs"),
            create_test_fragment("2", &"y".repeat(100), "file2.rs"),
        ];

        let narrative = "x".repeat(200);
        let (main, _support) = defragmenter.split_main_and_support(&narrative, &fragments, 20);

        assert!(main.len() < narrative.len());
    }
}
