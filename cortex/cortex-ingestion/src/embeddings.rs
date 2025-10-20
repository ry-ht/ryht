//! Embedding generation interface for content ingestion.
//!
//! This module provides the interface for generating embeddings from text content.
//! The actual embedding models are implemented separately and can be plugged in.

use async_trait::async_trait;
use cortex_core::error::Result;
use std::sync::Arc;

/// Embedding provider interface
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch processing)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;

    /// Get maximum input length (in tokens)
    fn max_input_length(&self) -> usize;
}

/// Configuration for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Batch size for batch processing
    pub batch_size: usize,
    /// Whether to cache embeddings
    pub cache_enabled: bool,
    /// Maximum text length before truncation
    pub max_text_length: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            cache_enabled: true,
            max_text_length: 8000,
        }
    }
}

/// Progress callback for embedding generation
pub type ProgressCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// Embedding service that manages embedding generation
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    config: EmbeddingConfig,
    progress_callback: Option<ProgressCallback>,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(provider: Arc<dyn EmbeddingProvider>, config: EmbeddingConfig) -> Self {
        Self {
            provider,
            config,
            progress_callback: None,
        }
    }

    /// Create with default config
    pub fn with_provider(provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self::new(provider, EmbeddingConfig::default())
    }

    /// Set progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Generate embedding for a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Truncate if too long
        let truncated = if text.len() > self.config.max_text_length {
            &text[..self.config.max_text_length]
        } else {
            text
        };

        self.provider.embed(truncated).await
    }

    /// Generate embeddings in batches with progress tracking and retry logic
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::with_capacity(texts.len());
        let total_texts = texts.len();
        let mut processed = 0;

        // Process in batches
        for chunk in texts.chunks(self.config.batch_size) {
            // Truncate texts in batch
            let truncated: Vec<String> = chunk
                .iter()
                .map(|t| {
                    if t.len() > self.config.max_text_length {
                        t[..self.config.max_text_length].to_string()
                    } else {
                        t.clone()
                    }
                })
                .collect();

            // Retry logic with exponential backoff
            let mut retries = 0;
            let max_retries = 3;
            let batch_embeddings = loop {
                match self.provider.embed_batch(&truncated).await {
                    Ok(embeddings) => break embeddings,
                    Err(e) if retries < max_retries => {
                        retries += 1;
                        let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries as u32));
                        tracing::warn!(
                            "Embedding batch failed (attempt {}/{}), retrying in {:?}: {}",
                            retries,
                            max_retries,
                            delay,
                            e
                        );
                        tokio::time::sleep(delay).await;
                    }
                    Err(e) => return Err(e),
                }
            };

            all_embeddings.extend(batch_embeddings);
            processed += chunk.len();

            // Report progress
            if let Some(callback) = &self.progress_callback {
                callback(processed, total_texts);
            }
        }

        Ok(all_embeddings)
    }

    /// Generate embeddings with detailed progress information
    pub async fn embed_batch_with_progress(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, EmbeddingProgress)> {
        let start_time = std::time::Instant::now();
        let embeddings = self.embed_batch(texts).await?;
        let duration = start_time.elapsed();

        let progress = EmbeddingProgress {
            total: texts.len(),
            completed: embeddings.len(),
            failed: texts.len() - embeddings.len(),
            duration_secs: duration.as_secs_f64(),
            embeddings_per_sec: embeddings.len() as f64 / duration.as_secs_f64(),
        };

        Ok((embeddings, progress))
    }

    /// Get provider info
    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }
}

/// Mock embedding provider for testing
pub struct MockEmbeddingProvider {
    dimension: usize,
}

impl MockEmbeddingProvider {
    /// Create a new mock provider
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new(384)
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Generate a deterministic "fake" embedding based on text hash
        let hash = text.len() as f32;
        let mut embedding = vec![0.0; self.dimension];

        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash + i as f32) * 0.01).sin();
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn model_name(&self) -> &str {
        "mock-embedder"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn max_input_length(&self) -> usize {
        8192
    }
}

/// Progress information for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub duration_secs: f64,
    pub embeddings_per_sec: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = Arc::new(MockEmbeddingProvider::new(384));
        let service = EmbeddingService::with_provider(provider);

        let embedding = service.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let provider = Arc::new(MockEmbeddingProvider::new(384));
        let service = EmbeddingService::with_provider(provider);

        let texts = vec!["text1".to_string(), "text2".to_string(), "text3".to_string()];
        let embeddings = service.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 384);
    }

    #[tokio::test]
    async fn test_text_truncation() {
        let provider = Arc::new(MockEmbeddingProvider::new(384));
        let mut config = EmbeddingConfig::default();
        config.max_text_length = 100;
        let service = EmbeddingService::new(provider, config);

        let long_text = "a".repeat(1000);
        let embedding = service.embed(&long_text).await.unwrap();
        assert_eq!(embedding.len(), 384);
    }
}
