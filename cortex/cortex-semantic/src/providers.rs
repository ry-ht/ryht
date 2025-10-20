//! Embedding providers for generating vector embeddings.

use crate::config::{EmbeddingProviderConfig, OpenAIConfig, ONNXConfig, OllamaConfig};
use crate::error::{Result, SemanticError};
use crate::types::{EmbeddingModel, Vector};
use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Trait for embedding providers.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text.
    async fn embed(&self, text: &str) -> Result<Vector>;

    /// Generate embeddings for multiple texts (batched).
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>>;

    /// Get the embedding model information.
    fn model(&self) -> &EmbeddingModel;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize {
        self.model().dimension
    }
}

/// Provider manager that handles fallback chains.
pub struct ProviderManager {
    primary: Box<dyn EmbeddingProvider>,
    fallbacks: Vec<Box<dyn EmbeddingProvider>>,
}

impl ProviderManager {
    pub async fn from_config(config: &EmbeddingProviderConfig) -> Result<Self> {
        let primary = Self::create_provider(&config.primary_provider, config).await?;

        let mut fallbacks = Vec::new();
        for provider_name in &config.fallback_providers {
            match Self::create_provider(provider_name, config).await {
                Ok(provider) => fallbacks.push(provider),
                Err(e) => warn!("Failed to create fallback provider {}: {}", provider_name, e),
            }
        }

        Ok(Self { primary, fallbacks })
    }

    async fn create_provider(
        name: &str,
        config: &EmbeddingProviderConfig,
    ) -> Result<Box<dyn EmbeddingProvider>> {
        match name.to_lowercase().as_str() {
            "openai" => Ok(Box::new(OpenAIProvider::new(config.openai.clone()).await?)),
            "onnx" => Ok(Box::new(ONNXProvider::new(config.onnx.clone()).await?)),
            "ollama" => Ok(Box::new(OllamaProvider::new(config.ollama.clone()).await?)),
            "mock" => Ok(Box::new(MockProvider::new(384))),
            _ => Err(SemanticError::Provider(format!("Unknown provider: {}", name))),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for ProviderManager {
    async fn embed(&self, text: &str) -> Result<Vector> {
        // Try primary provider
        match self.primary.embed(text).await {
            Ok(embedding) => return Ok(embedding),
            Err(e) => warn!("Primary provider failed: {}", e),
        }

        // Try fallback providers
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.embed(text).await {
                Ok(embedding) => {
                    info!("Fallback provider {} succeeded", i);
                    return Ok(embedding);
                }
                Err(e) => warn!("Fallback provider {} failed: {}", i, e),
            }
        }

        Err(SemanticError::Provider(
            "All providers failed".to_string(),
        ))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        // Try primary provider
        match self.primary.embed_batch(texts).await {
            Ok(embeddings) => return Ok(embeddings),
            Err(e) => warn!("Primary provider batch failed: {}", e),
        }

        // Try fallback providers
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.embed_batch(texts).await {
                Ok(embeddings) => {
                    info!("Fallback provider {} succeeded for batch", i);
                    return Ok(embeddings);
                }
                Err(e) => warn!("Fallback provider {} batch failed: {}", i, e),
            }
        }

        Err(SemanticError::Provider(
            "All providers failed for batch".to_string(),
        ))
    }

    fn model(&self) -> &EmbeddingModel {
        self.primary.model()
    }
}

/// OpenAI embedding provider.
pub struct OpenAIProvider {
    client: Client,
    config: OpenAIConfig,
    model: EmbeddingModel,
}

#[derive(Serialize)]
struct OpenAIRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}

impl OpenAIProvider {
    pub async fn new(config: OpenAIConfig) -> Result<Self> {
        let api_key = config.api_key.clone().ok_or_else(|| {
            SemanticError::Config("OpenAI API key not configured".to_string())
        })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", api_key).parse().unwrap(),
                );
                headers.insert("Content-Type", "application/json".parse().unwrap());
                headers
            })
            .build()?;

        let dimension = config.dimension.unwrap_or_else(|| {
            match config.model.as_str() {
                "text-embedding-3-small" => 1536,
                "text-embedding-3-large" => 3072,
                "text-embedding-ada-002" => 1536,
                _ => 1536,
            }
        });

        let model = EmbeddingModel::new("openai", &config.model, dimension);

        info!("Initialized OpenAI provider with model: {}", config.model);

        Ok(Self {
            client,
            config,
            model,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn embed(&self, text: &str) -> Result<Vector> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        debug!("Generating {} embeddings with OpenAI", texts.len());

        let request = OpenAIRequest {
            input: texts.to_vec(),
            model: self.config.model.clone(),
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(SemanticError::Provider(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let response: OpenAIResponse = response.json().await?;

        let embeddings = response
            .data
            .into_iter()
            .map(|e| e.embedding)
            .collect::<Vec<_>>();

        if embeddings.len() != texts.len() {
            return Err(SemanticError::Provider(format!(
                "Expected {} embeddings, got {}",
                texts.len(),
                embeddings.len()
            )));
        }

        Ok(embeddings)
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}

/// ONNX Runtime embedding provider for local models.
pub struct ONNXProvider {
    model: EmbeddingModel,
    dimension: usize,
}

impl ONNXProvider {
    pub async fn new(config: ONNXConfig) -> Result<Self> {
        info!("Initialized ONNX provider with model: {}", config.model_name);

        Ok(Self {
            model: EmbeddingModel::new("onnx", &config.model_name, config.dimension),
            dimension: config.dimension,
        })
    }

    fn generate_mock_embedding(&self, text: &str) -> Vector {
        // Simple deterministic embedding for testing
        let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        let mut embedding = vec![0.0; self.dimension];
        for (i, val) in embedding.iter_mut().enumerate() {
            let seed = hash.wrapping_add(i as u64);
            *val = ((seed % 1000) as f32 / 1000.0) - 0.5;
        }

        // Normalize
        let norm = (embedding.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm > 0.0 {
            embedding.iter_mut().for_each(|x| *x /= norm);
        }

        embedding
    }
}

#[async_trait]
impl EmbeddingProvider for ONNXProvider {
    async fn embed(&self, text: &str) -> Result<Vector> {
        // For now, return mock embeddings
        // In production, this would use ONNX Runtime
        Ok(self.generate_mock_embedding(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        // For now, process sequentially with mock embeddings
        // In production, this would use ONNX Runtime batch processing
        Ok(texts.iter().map(|text| self.generate_mock_embedding(text)).collect())
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}

/// Ollama embedding provider for local LLMs.
pub struct OllamaProvider {
    client: Client,
    config: OllamaConfig,
    model: EmbeddingModel,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    embedding: Vec<f32>,
}

impl OllamaProvider {
    pub async fn new(config: OllamaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        let model = EmbeddingModel::ollama(&config.model, config.dimension);

        info!("Initialized Ollama provider with model: {}", config.model);

        Ok(Self {
            client,
            config,
            model,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vector> {
        debug!("Generating embedding with Ollama");

        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: text.to_string(),
        };

        let url = format!("{}/api/embeddings", self.config.endpoint);
        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(SemanticError::Provider(format!(
                "Ollama API error: {}",
                error_text
            )));
        }

        let response: OllamaResponse = response.json().await?;
        Ok(response.embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        // Ollama doesn't support batch embeddings, process sequentially
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}

/// Mock provider for testing.
pub struct MockProvider {
    model: EmbeddingModel,
    dimension: usize,
}

impl MockProvider {
    pub fn new(dimension: usize) -> Self {
        Self {
            model: EmbeddingModel::new("mock", "mock-model", dimension),
            dimension,
        }
    }

    fn generate_embedding(&self, text: &str) -> Vector {
        // Deterministic mock embedding based on text
        let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        let mut embedding = vec![0.0; self.dimension];
        for (i, val) in embedding.iter_mut().enumerate() {
            let seed = hash.wrapping_add(i as u64);
            *val = ((seed % 1000) as f32 / 1000.0) - 0.5;
        }

        // Normalize
        let norm = (embedding.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm > 0.0 {
            embedding.iter_mut().for_each(|x| *x /= norm);
        }

        embedding
    }
}

#[async_trait]
impl EmbeddingProvider for MockProvider {
    async fn embed(&self, text: &str) -> Result<Vector> {
        Ok(self.generate_embedding(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        Ok(texts.iter().map(|t| self.generate_embedding(t)).collect())
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider::new(128);
        assert_eq!(provider.dimension(), 128);

        let embedding = provider.embed("test").await.unwrap();
        assert_eq!(embedding.len(), 128);

        // Same text should produce same embedding
        let embedding2 = provider.embed("test").await.unwrap();
        assert_eq!(embedding, embedding2);

        // Different text should produce different embedding
        let embedding3 = provider.embed("different").await.unwrap();
        assert_ne!(embedding, embedding3);
    }

    #[tokio::test]
    async fn test_mock_provider_batch() {
        let provider = MockProvider::new(128);
        let texts = vec!["hello".to_string(), "world".to_string()];

        let embeddings = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 128);
        assert_eq!(embeddings[1].len(), 128);
        assert_ne!(embeddings[0], embeddings[1]);
    }
}
