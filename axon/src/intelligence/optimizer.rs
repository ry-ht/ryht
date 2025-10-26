//! Context optimization for token efficiency
//!
//! This module provides intelligent context optimization through:
//! - Accurate token counting via tiktoken-rs
//! - Semantic compression of context
//! - Intelligent chunking for large contexts
//! - Priority-based information retention
//! - Integration with Cortex for learned optimization patterns

use super::*;
use crate::cortex_bridge::{CortexBridge, Pattern};
use tiktoken_rs::{get_bpe_from_model, CoreBPE};

/// Optimized context with detailed metrics
#[derive(Debug, Clone)]
pub struct OptimizedContext {
    pub content: String,
    pub token_count: usize,
    pub optimization_ratio: f32,
    /// Parts of context ranked by importance
    pub ranked_parts: Vec<ContextPart>,
    /// Applied optimizations
    pub applied_optimizations: Vec<String>,
}

/// A part of context with importance score
#[derive(Debug, Clone)]
pub struct ContextPart {
    pub content: String,
    pub importance: f32,
    pub token_count: usize,
    pub part_type: PartType,
}

/// Type of context part
#[derive(Debug, Clone, PartialEq)]
pub enum PartType {
    Code,
    Documentation,
    Comment,
    ImportStatement,
    TypeDefinition,
    FunctionSignature,
    TestCode,
    Metadata,
    Unknown,
}

/// Optimization strategy
#[derive(Debug, Clone)]
pub enum OptimizationStrategy {
    /// Preserve all content
    None,
    /// Remove whitespace and comments
    Basic,
    /// Semantic compression - keep important parts
    Semantic { min_importance: f32 },
    /// Aggressive - maximum compression
    Aggressive,
    /// Custom strategy with specific rules
    Custom {
        preserve_types: Vec<PartType>,
        min_importance: f32,
    },
}

/// Context optimizer with Cortex integration
pub struct ContextOptimizer {
    /// Optimization cache
    cache: Arc<RwLock<HashMap<u64, OptimizedContext>>>,
    /// Token counter for accurate token counting
    token_counter: Arc<RwLock<Option<CoreBPE>>>,
    /// Cortex bridge for learned patterns
    cortex: Option<Arc<CortexBridge>>,
    /// Default model for token counting
    default_model: String,
}

impl ContextOptimizer {
    /// Create a new ContextOptimizer without Cortex integration
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            token_counter: Arc::new(RwLock::new(None)),
            cortex: None,
            default_model: "gpt-4".to_string(),
        }
    }

    /// Create a new ContextOptimizer with Cortex integration
    pub fn with_cortex(cortex: Arc<CortexBridge>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            token_counter: Arc::new(RwLock::new(None)),
            cortex: Some(cortex),
            default_model: "gpt-4".to_string(),
        }
    }

    /// Set the model for token counting
    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    /// Initialize token counter for a specific model
    async fn ensure_token_counter(&self, model: &str) -> Result<()> {
        let mut counter = self.token_counter.write().await;
        if counter.is_none() {
            *counter = Some(
                get_bpe_from_model(model).map_err(|e| {
                    IntelligenceError::OptimizationFailed(format!(
                        "Failed to initialize tokenizer: {}",
                        e
                    ))
                })?,
            );
        }
        Ok(())
    }

    /// Count tokens accurately using tiktoken
    pub async fn count_tokens(&self, content: &str) -> Result<usize> {
        self.ensure_token_counter(&self.default_model).await?;

        let counter = self.token_counter.read().await;
        if let Some(bpe) = counter.as_ref() {
            let tokens = bpe.encode_with_special_tokens(content);
            Ok(tokens.len())
        } else {
            // Fallback to rough estimate
            Ok(content.len() / 4)
        }
    }

    /// Optimize content to fit within target token count
    pub async fn optimize(
        &self,
        content: String,
        target_tokens: usize,
        strategy: OptimizationStrategy,
    ) -> Result<OptimizedContext> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Calculate hash for caching
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        target_tokens.hash(&mut hasher);
        let hash = hasher.finish();

        // Check cache
        if let Some(cached) = self.cache.read().await.get(&hash) {
            return Ok(cached.clone());
        }

        // Count original tokens
        let original_tokens = self.count_tokens(&content).await?;

        // If already within target, return as-is
        if original_tokens <= target_tokens {
            let result = OptimizedContext {
                content: content.clone(),
                token_count: original_tokens,
                optimization_ratio: 1.0,
                ranked_parts: vec![ContextPart {
                    content: content.clone(),
                    importance: 1.0,
                    token_count: original_tokens,
                    part_type: PartType::Unknown,
                }],
                applied_optimizations: vec!["none - within target".to_string()],
            };
            self.cache.write().await.insert(hash, result.clone());
            return Ok(result);
        }

        // Parse and rank content parts
        let ranked_parts = self.parse_and_rank_content(&content).await?;

        // Apply optimization strategy
        let result = match strategy {
            OptimizationStrategy::None => {
                self.optimize_none(&content, original_tokens, ranked_parts)
                    .await?
            }
            OptimizationStrategy::Basic => {
                self.optimize_basic(&content, original_tokens, ranked_parts)
                    .await?
            }
            OptimizationStrategy::Semantic { min_importance } => {
                self.optimize_semantic(
                    &content,
                    original_tokens,
                    target_tokens,
                    ranked_parts,
                    min_importance,
                )
                .await?
            }
            OptimizationStrategy::Aggressive => {
                self.optimize_aggressive(&content, original_tokens, target_tokens, ranked_parts)
                    .await?
            }
            OptimizationStrategy::Custom {
                preserve_types,
                min_importance,
            } => {
                self.optimize_custom(
                    &content,
                    original_tokens,
                    target_tokens,
                    ranked_parts,
                    preserve_types,
                    min_importance,
                )
                .await?
            }
        };

        // Cache result
        self.cache.write().await.insert(hash, result.clone());

        Ok(result)
    }

    /// Optimize with simple semantic strategy
    pub async fn optimize_simple(
        &self,
        content: String,
        target_tokens: usize,
    ) -> Result<OptimizedContext> {
        self.optimize(content, target_tokens, OptimizationStrategy::Semantic {
            min_importance: 0.3,
        })
        .await
    }

    /// Parse content into parts and rank by importance
    async fn parse_and_rank_content(&self, content: &str) -> Result<Vec<ContextPart>> {
        let mut parts = Vec::new();

        // Split by lines and analyze each
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let part_type = self.classify_line(trimmed);
            let importance = self.calculate_importance(trimmed, &part_type).await;
            let token_count = self.count_tokens(line).await?;

            parts.push(ContextPart {
                content: line.to_string(),
                importance,
                token_count,
                part_type,
            });
        }

        // Sort by importance (descending)
        parts.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(parts)
    }

    /// Classify a line of text
    fn classify_line(&self, line: &str) -> PartType {
        let trimmed = line.trim();

        // Code patterns
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ")
        {
            return PartType::FunctionSignature;
        }

        if trimmed.starts_with("struct ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("pub type ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("pub trait ")
        {
            return PartType::TypeDefinition;
        }

        if trimmed.starts_with("use ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
        {
            return PartType::ImportStatement;
        }

        if trimmed.starts_with("//") || trimmed.starts_with("#") {
            return PartType::Comment;
        }

        if trimmed.starts_with("///") || trimmed.starts_with("/**") {
            return PartType::Documentation;
        }

        if trimmed.starts_with("#[test]") || trimmed.contains("test_") {
            return PartType::TestCode;
        }

        // Check for code vs documentation
        if trimmed.contains('{') || trimmed.contains('}') || trimmed.contains(';') {
            PartType::Code
        } else if trimmed.starts_with("@") || trimmed.starts_with("---") {
            PartType::Metadata
        } else {
            PartType::Unknown
        }
    }

    /// Calculate importance of a line based on its content and type
    async fn calculate_importance(&self, line: &str, part_type: &PartType) -> f32 {
        // Base importance by type
        let mut importance = match part_type {
            PartType::FunctionSignature => 0.9,
            PartType::TypeDefinition => 0.85,
            PartType::Code => 0.7,
            PartType::Documentation => 0.5,
            PartType::ImportStatement => 0.3,
            PartType::Comment => 0.2,
            PartType::TestCode => 0.4,
            PartType::Metadata => 0.1,
            PartType::Unknown => 0.5,
        };

        // Boost importance for certain keywords
        let important_keywords = [
            "error",
            "panic",
            "unsafe",
            "critical",
            "important",
            "todo",
            "fixme",
            "bug",
            "security",
        ];

        for keyword in &important_keywords {
            if line.to_lowercase().contains(keyword) {
                importance = (importance + 0.2_f32).min(1.0);
                break;
            }
        }

        // Check Cortex patterns if available
        if let Some(cortex) = &self.cortex
            && let Ok(patterns) = cortex.get_patterns().await {
                importance = self.adjust_importance_from_patterns(line, importance, patterns);
            }

        importance
    }

    /// Adjust importance based on learned patterns
    fn adjust_importance_from_patterns(
        &self,
        line: &str,
        base_importance: f32,
        patterns: Vec<Pattern>,
    ) -> f32 {
        let mut importance = base_importance;

        for pattern in patterns {
            // Check if pattern context matches
            if let Some(keywords) = pattern.context.split_whitespace().next()
                && line.contains(keywords) && pattern.success_rate > 0.8 {
                    importance = (importance + 0.1).min(1.0);
                }
        }

        importance
    }

    /// No optimization - return as-is
    async fn optimize_none(
        &self,
        content: &str,
        token_count: usize,
        ranked_parts: Vec<ContextPart>,
    ) -> Result<OptimizedContext> {
        Ok(OptimizedContext {
            content: content.to_string(),
            token_count,
            optimization_ratio: 1.0,
            ranked_parts,
            applied_optimizations: vec!["none".to_string()],
        })
    }

    /// Basic optimization - remove comments and extra whitespace
    async fn optimize_basic(
        &self,
        _content: &str,
        original_tokens: usize,
        ranked_parts: Vec<ContextPart>,
    ) -> Result<OptimizedContext> {
        let optimizations = vec!["remove_comments".to_string(), "trim_whitespace".to_string()];

        // Filter out comments and trim whitespace
        let optimized_parts: Vec<_> = ranked_parts
            .into_iter()
            .filter(|p| p.part_type != PartType::Comment)
            .map(|mut p| {
                p.content = p.content.trim().to_string();
                p
            })
            .collect();

        let optimized_content = optimized_parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let new_token_count = self.count_tokens(&optimized_content).await?;

        Ok(OptimizedContext {
            content: optimized_content,
            token_count: new_token_count,
            optimization_ratio: new_token_count as f32 / original_tokens as f32,
            ranked_parts: optimized_parts,
            applied_optimizations: optimizations,
        })
    }

    /// Semantic optimization - keep important parts based on ranking
    async fn optimize_semantic(
        &self,
        _content: &str,
        original_tokens: usize,
        target_tokens: usize,
        mut ranked_parts: Vec<ContextPart>,
        min_importance: f32,
    ) -> Result<OptimizedContext> {
        let mut optimizations = vec![
            "semantic_ranking".to_string(),
            format!("min_importance_{:.2}", min_importance),
        ];

        // Filter by minimum importance
        ranked_parts.retain(|p| p.importance >= min_importance);

        // Select parts until we reach target
        let mut selected_parts = Vec::new();
        let mut current_tokens = 0;

        for part in ranked_parts {
            if current_tokens + part.token_count <= target_tokens {
                current_tokens += part.token_count;
                selected_parts.push(part);
            } else if selected_parts.is_empty() {
                // At least include one part, even if it exceeds target
                selected_parts.push(part);
                optimizations.push("exceeded_target_for_one_part".to_string());
                break;
            }
        }

        let optimized_content = selected_parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let new_token_count = self.count_tokens(&optimized_content).await?;

        Ok(OptimizedContext {
            content: optimized_content,
            token_count: new_token_count,
            optimization_ratio: new_token_count as f32 / original_tokens as f32,
            ranked_parts: selected_parts,
            applied_optimizations: optimizations,
        })
    }

    /// Aggressive optimization - maximum compression
    async fn optimize_aggressive(
        &self,
        _content: &str,
        original_tokens: usize,
        target_tokens: usize,
        ranked_parts: Vec<ContextPart>,
    ) -> Result<OptimizedContext> {
        let optimizations = vec![
            "aggressive_compression".to_string(),
            "remove_all_comments".to_string(),
            "remove_imports".to_string(),
            "keep_only_critical".to_string(),
        ];

        // Keep only the most important parts
        let critical_parts: Vec<_> = ranked_parts
            .into_iter()
            .filter(|p| {
                matches!(
                    p.part_type,
                    PartType::FunctionSignature | PartType::TypeDefinition | PartType::Code
                ) && p.importance >= 0.7
            })
            .collect();

        let mut selected_parts = Vec::new();
        let mut current_tokens = 0;

        for part in critical_parts {
            if current_tokens + part.token_count <= target_tokens {
                current_tokens += part.token_count;
                selected_parts.push(part);
            }
        }

        if selected_parts.is_empty() {
            return Err(IntelligenceError::OptimizationFailed(
                "Cannot compress to target tokens while preserving critical information"
                    .to_string(),
            ));
        }

        let optimized_content = selected_parts
            .iter()
            .map(|p| p.content.trim())
            .collect::<Vec<_>>()
            .join("\n");

        let new_token_count = self.count_tokens(&optimized_content).await?;

        Ok(OptimizedContext {
            content: optimized_content,
            token_count: new_token_count,
            optimization_ratio: new_token_count as f32 / original_tokens as f32,
            ranked_parts: selected_parts,
            applied_optimizations: optimizations,
        })
    }

    /// Custom optimization with specific rules
    async fn optimize_custom(
        &self,
        _content: &str,
        original_tokens: usize,
        target_tokens: usize,
        ranked_parts: Vec<ContextPart>,
        preserve_types: Vec<PartType>,
        min_importance: f32,
    ) -> Result<OptimizedContext> {
        let optimizations = vec![
            "custom_strategy".to_string(),
            format!("preserve_types_{}", preserve_types.len()),
            format!("min_importance_{:.2}", min_importance),
        ];

        // Filter by type and importance
        let filtered_parts: Vec<_> = ranked_parts
            .into_iter()
            .filter(|p| {
                (preserve_types.contains(&p.part_type) || p.importance >= min_importance)
                    && p.part_type != PartType::Comment
            })
            .collect();

        let mut selected_parts = Vec::new();
        let mut current_tokens = 0;

        for part in filtered_parts {
            if current_tokens + part.token_count <= target_tokens {
                current_tokens += part.token_count;
                selected_parts.push(part);
            }
        }

        let optimized_content = selected_parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let new_token_count = self.count_tokens(&optimized_content).await?;

        Ok(OptimizedContext {
            content: optimized_content,
            token_count: new_token_count,
            optimization_ratio: new_token_count as f32 / original_tokens as f32,
            ranked_parts: selected_parts,
            applied_optimizations: optimizations,
        })
    }

    /// Chunk large content into manageable pieces
    pub async fn chunk_content(
        &self,
        content: String,
        max_tokens_per_chunk: usize,
        overlap_tokens: usize,
    ) -> Result<Vec<OptimizedContext>> {
        let parts = self.parse_and_rank_content(&content).await?;

        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;

        for part in parts {
            if current_tokens + part.token_count > max_tokens_per_chunk && !current_chunk.is_empty()
            {
                // Create chunk
                let chunk_content = current_chunk
                    .iter()
                    .map(|p: &ContextPart| p.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let chunk_tokens = self.count_tokens(&chunk_content).await?;

                chunks.push(OptimizedContext {
                    content: chunk_content,
                    token_count: chunk_tokens,
                    optimization_ratio: 1.0,
                    ranked_parts: current_chunk.clone(),
                    applied_optimizations: vec!["chunked".to_string()],
                });

                // Keep overlap
                let overlap_count = overlap_tokens.min(current_tokens);
                let mut overlap_accumulated = 0;
                current_chunk.retain(|p| {
                    if overlap_accumulated < overlap_count {
                        overlap_accumulated += p.token_count;
                        true
                    } else {
                        false
                    }
                });
                current_tokens = overlap_accumulated;
            }

            current_tokens += part.token_count;
            current_chunk.push(part);
        }

        // Add final chunk
        if !current_chunk.is_empty() {
            let chunk_content = current_chunk
                .iter()
                .map(|p| p.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let chunk_tokens = self.count_tokens(&chunk_content).await?;

            chunks.push(OptimizedContext {
                content: chunk_content,
                token_count: chunk_tokens,
                optimization_ratio: 1.0,
                ranked_parts: current_chunk,
                applied_optimizations: vec!["chunked".to_string()],
            });
        }

        Ok(chunks)
    }

    /// Clear optimization cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }
}

impl Default for ContextOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CODE: &str = r#"
// This is a comment
use std::collections::HashMap;

/// This is documentation
pub struct MyStruct {
    field1: String,
    field2: i32,
}

impl MyStruct {
    pub fn new(field1: String, field2: i32) -> Self {
        Self { field1, field2 }
    }

    // Get field1
    pub fn get_field1(&self) -> &str {
        &self.field1
    }
}

#[test]
fn test_my_struct() {
    let s = MyStruct::new("test".to_string(), 42);
    assert_eq!(s.get_field1(), "test");
}
"#;

    #[tokio::test]
    async fn test_optimizer_creation() {
        let optimizer = ContextOptimizer::new();
        assert!(optimizer.cortex.is_none());
    }

    #[tokio::test]
    async fn test_count_tokens() {
        let optimizer = ContextOptimizer::new();
        let count = optimizer.count_tokens("Hello, world!").await.unwrap();
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_classify_line() {
        let optimizer = ContextOptimizer::new();

        assert_eq!(
            optimizer.classify_line("pub fn test() {}"),
            PartType::FunctionSignature
        );
        assert_eq!(
            optimizer.classify_line("struct MyStruct {}"),
            PartType::TypeDefinition
        );
        assert_eq!(
            optimizer.classify_line("use std::collections::HashMap;"),
            PartType::ImportStatement
        );
        assert_eq!(
            optimizer.classify_line("// This is a comment"),
            PartType::Comment
        );
        assert_eq!(
            optimizer.classify_line("/// Documentation"),
            PartType::Documentation
        );
    }

    #[tokio::test]
    async fn test_parse_and_rank() {
        let optimizer = ContextOptimizer::new();
        let parts = optimizer.parse_and_rank_content(SAMPLE_CODE).await.unwrap();

        assert!(!parts.is_empty());

        // Check that parts are sorted by importance
        for i in 0..parts.len().saturating_sub(1) {
            assert!(parts[i].importance >= parts[i + 1].importance);
        }
    }

    #[tokio::test]
    async fn test_optimize_none() {
        let optimizer = ContextOptimizer::new();
        let result = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                1000,
                OptimizationStrategy::None,
            )
            .await
            .unwrap();

        assert_eq!(result.optimization_ratio, 1.0);
        assert!(result.content.contains("MyStruct"));
    }

    #[tokio::test]
    async fn test_optimize_basic() {
        let optimizer = ContextOptimizer::new();
        let result = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                1000,
                OptimizationStrategy::Basic,
            )
            .await
            .unwrap();

        // Should remove comments
        assert!(!result.content.contains("// This is a comment"));
        assert!(result.optimization_ratio < 1.0);
    }

    #[tokio::test]
    async fn test_optimize_semantic() {
        let optimizer = ContextOptimizer::new();
        let original_tokens = optimizer.count_tokens(SAMPLE_CODE).await.unwrap();

        let result = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                original_tokens / 2,
                OptimizationStrategy::Semantic {
                    min_importance: 0.5,
                },
            )
            .await
            .unwrap();

        assert!(result.token_count <= original_tokens);
        assert!(result.optimization_ratio < 1.0);

        // Should keep important parts
        assert!(result.content.contains("MyStruct") || result.content.contains("new"));
    }

    #[tokio::test]
    async fn test_optimize_aggressive() {
        let optimizer = ContextOptimizer::new();
        let original_tokens = optimizer.count_tokens(SAMPLE_CODE).await.unwrap();

        let result = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                original_tokens / 3,
                OptimizationStrategy::Aggressive,
            )
            .await
            .unwrap();

        assert!(result.token_count <= original_tokens / 2);
        // Should remove comments, imports, tests
        assert!(!result.content.contains("test_my_struct"));
    }

    #[tokio::test]
    async fn test_optimize_custom() {
        let optimizer = ContextOptimizer::new();
        let original_tokens = optimizer.count_tokens(SAMPLE_CODE).await.unwrap();

        let result = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                original_tokens / 2,
                OptimizationStrategy::Custom {
                    preserve_types: vec![PartType::TypeDefinition, PartType::FunctionSignature],
                    min_importance: 0.6,
                },
            )
            .await
            .unwrap();

        // Should preserve struct and function definitions
        assert!(
            result.content.contains("MyStruct") || result.content.contains("struct")
        );
    }

    #[tokio::test]
    async fn test_chunking() {
        let optimizer = ContextOptimizer::new();
        let long_content = SAMPLE_CODE.repeat(5);

        let chunks = optimizer
            .chunk_content(long_content.clone(), 50, 10)
            .await
            .unwrap();

        assert!(chunks.len() > 1);

        // Each chunk should be within limits
        for chunk in &chunks {
            assert!(chunk.token_count <= 60); // 50 + some tolerance
        }
    }

    #[tokio::test]
    async fn test_cache() {
        let optimizer = ContextOptimizer::new();

        let result1 = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                100,
                OptimizationStrategy::Basic,
            )
            .await
            .unwrap();

        let result2 = optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                100,
                OptimizationStrategy::Basic,
            )
            .await
            .unwrap();

        assert_eq!(result1.content, result2.content);
        assert_eq!(result1.token_count, result2.token_count);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let optimizer = ContextOptimizer::new();

        optimizer
            .optimize(
                SAMPLE_CODE.to_string(),
                100,
                OptimizationStrategy::Basic,
            )
            .await
            .unwrap();

        optimizer.clear_cache().await;

        let cache = optimizer.cache.read().await;
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_importance_calculation() {
        let optimizer = ContextOptimizer::new();

        // Function signatures should have high importance
        let importance1 = optimizer
            .calculate_importance("pub fn critical_function()", &PartType::FunctionSignature)
            .await;
        assert!(importance1 > 0.8);

        // Comments should have low importance
        let importance2 = optimizer
            .calculate_importance("// just a comment", &PartType::Comment)
            .await;
        assert!(importance2 < 0.5);

        // Lines with important keywords should get a boost
        let importance3 = optimizer
            .calculate_importance("// FIXME: security issue", &PartType::Comment)
            .await;
        assert!(importance3 > importance2);
    }

    #[tokio::test]
    async fn test_within_target() {
        let optimizer = ContextOptimizer::new();
        let short_content = "let x = 42;";
        let tokens = optimizer.count_tokens(short_content).await.unwrap();

        let result = optimizer
            .optimize(
                short_content.to_string(),
                tokens * 2,
                OptimizationStrategy::Semantic {
                    min_importance: 0.5,
                },
            )
            .await
            .unwrap();

        // Should not modify content if already within target
        assert_eq!(result.optimization_ratio, 1.0);
        assert!(result
            .applied_optimizations
            .iter()
            .any(|s| s.contains("within target")));
    }
}
