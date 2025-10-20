/// Code-specific embedder for semantic search
///
/// This module provides code embedding functionality using either
/// lightweight models or delegating to the main embedding engine.

use crate::embeddings::EmbeddingEngine;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for code embedding models
#[async_trait]
pub trait CodeEmbedder: Send + Sync {
    /// Embed a code snippet
    async fn embed_code(&self, code: &str) -> Result<Vec<f32>>;

    /// Embed multiple code snippets in batch
    async fn embed_batch(&self, codes: Vec<&str>) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Local code embedder using the embedding engine
pub struct LocalCodeEmbedder {
    engine: EmbeddingEngine,
}

impl LocalCodeEmbedder {
    /// Create a new local code embedder
    pub fn new() -> Result<Self> {
        let engine = EmbeddingEngine::new()?;
        Ok(Self { engine })
    }

    /// Preprocess code for embedding
    fn preprocess_code(&self, code: &str) -> String {
        // Remove excessive whitespace and normalize
        let normalized = code
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        // Truncate if too long (model-dependent, typically 512 tokens)
        if normalized.len() > 2000 {
            normalized[..2000].to_string()
        } else {
            normalized
        }
    }

    /// Extract key features from code for better embedding
    fn extract_code_features(&self, code: &str) -> String {
        // Extract function signatures, class names, important keywords
        // This is a simplified version - could be enhanced with AST parsing
        let mut features = Vec::new();

        // Extract function definitions
        for line in code.lines() {
            if line.contains("fn ") || line.contains("function ") || line.contains("def ") {
                features.push(line.trim());
            }
            if line.contains("class ") || line.contains("struct ") || line.contains("interface ") {
                features.push(line.trim());
            }
        }

        if features.is_empty() {
            self.preprocess_code(code)
        } else {
            features.join(" ")
        }
    }
}

#[async_trait]
impl CodeEmbedder for LocalCodeEmbedder {
    async fn embed_code(&self, code: &str) -> Result<Vec<f32>> {
        let processed = self.extract_code_features(code);
        self.engine.generate_embedding(&processed)
    }

    async fn embed_batch(&self, codes: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let processed: Vec<String> = codes
            .iter()
            .map(|code| self.extract_code_features(code))
            .collect();

        let refs: Vec<&str> = processed.iter().map(|s| s.as_str()).collect();
        self.engine.batch_generate(refs)
    }

    fn dimension(&self) -> usize {
        self.engine.dimension()
    }

    fn model_name(&self) -> &str {
        self.engine.model_name()
    }
}

impl Default for LocalCodeEmbedder {
    fn default() -> Self {
        Self::new().expect("Failed to create default code embedder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_code() {
        let embedder = LocalCodeEmbedder::new().unwrap();

        let code = r#"
        fn test_function() {
            let x = 42;
            println!("Hello, world!");
        }
        "#;

        let processed = embedder.preprocess_code(code);
        assert!(!processed.contains("\n"));
        assert!(processed.contains("fn test_function"));
    }

    #[test]
    fn test_extract_features() {
        let embedder = LocalCodeEmbedder::new().unwrap();

        let code = r#"
        fn process_data(input: &str) -> Result<String> {
            let result = input.to_uppercase();
            Ok(result)
        }

        struct DataProcessor {
            config: Config,
        }
        "#;

        let features = embedder.extract_code_features(code);
        assert!(features.contains("fn process_data"));
        assert!(features.contains("struct DataProcessor"));
    }

    #[tokio::test]
    async fn test_embed_code() {
        let embedder = LocalCodeEmbedder::new().unwrap();

        let code = "fn hello() { println!(\"Hello\"); }";
        let embedding = embedder.embed_code(code).await;

        assert!(embedding.is_ok());
        let vec = embedding.unwrap();
        assert_eq!(vec.len(), embedder.dimension());
    }

    #[tokio::test]
    async fn test_batch_embed() {
        let embedder = LocalCodeEmbedder::new().unwrap();

        let codes = vec![
            "fn add(a: i32, b: i32) -> i32 { a + b }",
            "fn subtract(a: i32, b: i32) -> i32 { a - b }",
        ];

        let embeddings = embedder.embed_batch(codes).await;
        assert!(embeddings.is_ok());

        let vecs = embeddings.unwrap();
        assert_eq!(vecs.len(), 2);
        assert_eq!(vecs[0].len(), embedder.dimension());
        assert_eq!(vecs[1].len(), embedder.dimension());
    }
}
