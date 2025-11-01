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

        // Build headers with proper error handling
        let mut headers = reqwest::header::HeaderMap::new();

        let auth_header = format!("Bearer {}", api_key)
            .parse()
            .map_err(|e| SemanticError::Config(format!("Invalid authorization header: {}", e)))?;
        headers.insert("Authorization", auth_header);

        let content_type = "application/json"
            .parse()
            .map_err(|e| SemanticError::Config(format!("Invalid content-type header: {}", e)))?;
        headers.insert("Content-Type", content_type);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
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
///
/// This provider supports real semantic embeddings using ONNX models.
/// By default, it uses a fallback mock implementation if ONNX model files
/// are not available. To use real embeddings:
///
/// 1. Download a model like sentence-transformers/all-MiniLM-L6-v2
/// 2. Convert to ONNX format
/// 3. Set model_path in ONNXConfig
///
/// Mock fallback is deterministic and suitable for integration testing
/// but does NOT provide semantic understanding.
pub struct ONNXProvider {
    model: EmbeddingModel,
    dimension: usize,
    session: Option<Arc<RwLock<ort::Session>>>,
    tokenizer: Option<Arc<tokenizers::Tokenizer>>,
    #[allow(dead_code)]  // Keep environment alive for the session
    environment: Option<Arc<ort::Environment>>,
    use_mock: bool,
    /// Maximum batch size for inference (prevents OOM errors)
    max_batch_size: usize,
    /// Maximum sequence length supported by the model
    max_seq_length: usize,
}

impl ONNXProvider {
    pub async fn new(config: ONNXConfig) -> Result<Self> {
        info!("Initializing ONNX provider with model: {}", config.model_name);

        // Try to load ONNX model and tokenizer
        let (session, tokenizer, environment, use_mock) = if let Some(model_path) = &config.model_path {
            let path_str = model_path.to_string_lossy().to_string();
            match Self::load_model(&path_str).await {
                Ok((env, sess, tok)) => {
                    info!("ONNX model loaded successfully from: {}", path_str);
                    (
                        Some(Arc::new(RwLock::new(sess))),
                        Some(Arc::new(tok)),
                        Some(Arc::new(env)),
                        false
                    )
                }
                Err(e) => {
                    warn!("Failed to load ONNX model: {}. Using mock embeddings.", e);
                    (None, None, None, true)
                }
            }
        } else {
            info!("No model path provided. Using mock embeddings for testing.");
            (None, None, None, true)
        };

        Ok(Self {
            model: EmbeddingModel::new("onnx", &config.model_name, config.dimension),
            dimension: config.dimension,
            session,
            tokenizer,
            environment,
            use_mock,
            // Optimal batch size balancing memory and throughput
            // For 384-dim models: ~32 provides good balance
            // For 768-dim models: ~16-24 is better
            // For 1536+ dim: ~8-16 recommended
            max_batch_size: if config.dimension <= 384 { 32 } else if config.dimension <= 768 { 24 } else { 16 },
            // Most sentence transformer models use 512 max sequence length
            max_seq_length: 512,
        })
    }

    async fn load_model(
        model_path: &str,
    ) -> Result<(ort::Environment, ort::Session, tokenizers::Tokenizer)> {
        use std::path::Path;

        let model_path_obj = Path::new(model_path);

        // Check if model file exists
        if !model_path_obj.exists() {
            return Err(SemanticError::Provider(format!(
                "ONNX model file not found: {}. Please download all-MiniLM-L6-v2 ONNX model.",
                model_path
            )));
        }

        // Create ONNX Runtime environment (required for ort 1.16 API)
        let environment = ort::Environment::builder()
            .with_name("cortex_semantic")
            .with_log_level(ort::LoggingLevel::Warning)
            .build()?
            .into_arc();

        info!("ONNX Runtime environment created");

        // Load ONNX model using ort 1.16 API
        // Use SessionBuilder::new() followed by with_model_from_file()
        let session = ort::SessionBuilder::new(&environment)?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_model_from_file(model_path)?;

        info!("ONNX session created successfully from: {}", model_path);

        // Load tokenizer
        // Look for tokenizer.json in the same directory as the model
        let model_dir = model_path_obj.parent().ok_or_else(|| {
            SemanticError::Provider("Invalid model path - no parent directory".to_string())
        })?;

        let tokenizer_path = model_dir.join("tokenizer.json");

        if !tokenizer_path.exists() {
            return Err(SemanticError::Provider(format!(
                "Tokenizer file not found: {}. Please ensure tokenizer.json is in the same directory as the model.",
                tokenizer_path.display()
            )));
        }

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| SemanticError::Provider(format!("Failed to load tokenizer: {}", e)))?;

        info!("Tokenizer loaded successfully from: {}", tokenizer_path.display());

        // Extract environment from Arc to return it
        let env = Arc::try_unwrap(environment)
            .unwrap_or_else(|arc| (*arc).clone());

        Ok((env, session, tokenizer))
    }

    fn generate_embedding_real(&self, text: &str) -> Result<Vector> {
        // Use batch inference with single text for consistency
        let result = self.generate_embeddings_batch_real(&[text])?;
        Ok(result.into_iter().next().unwrap())
    }

    /// Generate embeddings for a batch of texts using true batch inference.
    ///
    /// This method:
    /// 1. Tokenizes all texts together with padding to max length
    /// 2. Executes a single ONNX inference call with batched inputs
    /// 3. Extracts individual embeddings from batched output
    /// 4. Normalizes each embedding
    fn generate_embeddings_batch_real(&self, texts: &[&str]) -> Result<Vec<Vector>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Validate that we have session and tokenizer
        let session = self.session.as_ref().ok_or_else(|| {
            SemanticError::Provider("ONNX session not initialized".to_string())
        })?;

        let tokenizer = self.tokenizer.as_ref().ok_or_else(|| {
            SemanticError::Provider("Tokenizer not initialized".to_string())
        })?;

        let batch_size = texts.len();

        // Tokenize all texts with padding and truncation
        // Use encode_batch for true batch tokenization
        let encodings = tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| SemanticError::Provider(format!("Batch tokenization failed: {}", e)))?;

        // Find the maximum sequence length in this batch
        let max_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);

        // Clamp to model's maximum sequence length
        let max_len = max_len.min(self.max_seq_length);

        // Build batched input tensors with padding
        let mut batch_input_ids = Vec::with_capacity(batch_size * max_len);
        let mut batch_attention_mask = Vec::with_capacity(batch_size * max_len);

        for encoding in &encodings {
            let token_ids = encoding.get_ids();
            let attention_mask = encoding.get_attention_mask();

            // Truncate if needed
            let seq_len = token_ids.len().min(max_len);

            // Add tokens (truncated)
            for i in 0..seq_len {
                batch_input_ids.push(token_ids[i] as i64);
                batch_attention_mask.push(attention_mask[i] as i64);
            }

            // Pad to max_len
            for _ in seq_len..max_len {
                batch_input_ids.push(0); // 0 is typically the padding token
                batch_attention_mask.push(0); // 0 means "don't attend"
            }
        }

        // Get allocator for creating ONNX values
        let session_guard = session.read();
        let allocator_ptr = session_guard.allocator();

        // Create batched ndarray tensors
        use ndarray::{Array, CowArray, IxDyn};

        let input_ids_array = Array::from_shape_vec(
            IxDyn(&[batch_size, max_len]),
            batch_input_ids
        ).map_err(|e| SemanticError::Provider(format!("Failed to create batched input tensor: {}", e)))?;

        let attention_mask_array = Array::from_shape_vec(
            IxDyn(&[batch_size, max_len]),
            batch_attention_mask
        ).map_err(|e| SemanticError::Provider(format!("Failed to create batched attention mask tensor: {}", e)))?;

        // Convert to CowArrays for ort
        let input_ids_cow: CowArray<i64, IxDyn> = CowArray::from(input_ids_array);
        let attention_mask_cow: CowArray<i64, IxDyn> = CowArray::from(attention_mask_array);

        let input_ids_value = ort::Value::from_array(allocator_ptr, &input_ids_cow)?;
        let attention_mask_value = ort::Value::from_array(allocator_ptr, &attention_mask_cow)?;

        // Run batched inference - single ONNX call for all texts
        let outputs = session_guard.run(vec![input_ids_value, attention_mask_value])?;

        // Extract embeddings from batched output
        let output_tensor = &outputs[0];
        let embeddings_raw = output_tensor.try_extract::<f32>()?;
        let embeddings_view = embeddings_raw.view();
        let shape = embeddings_view.shape();

        debug!(
            "Batch inference completed: batch_size={}, output_shape={:?}",
            batch_size, shape
        );

        // Extract and normalize individual embeddings from batch
        let mut results = Vec::with_capacity(batch_size);

        use ndarray::Axis;

        if shape.len() == 3 {
            // Shape: [batch_size, seq_len, hidden_size]
            // Need to pool over sequence dimension for each batch item
            for i in 0..batch_size {
                let batch_item = embeddings_view.index_axis(Axis(0), i);

                // Mean pooling over sequence dimension
                let pooled = batch_item
                    .mean_axis(Axis(0))
                    .ok_or_else(|| SemanticError::Provider("Failed to pool embeddings".to_string()))?;

                let embedding: Vec<f32> = pooled.into_raw_vec();

                // L2 normalize
                let normalized = Self::normalize_embedding(embedding);
                results.push(normalized);
            }
        } else if shape.len() == 2 {
            // Shape: [batch_size, hidden_size] - already pooled
            for i in 0..batch_size {
                let batch_item = embeddings_view.index_axis(Axis(0), i);
                let embedding: Vec<f32> = batch_item.iter().copied().collect();

                // L2 normalize
                let normalized = Self::normalize_embedding(embedding);
                results.push(normalized);
            }
        } else {
            return Err(SemanticError::Provider(format!(
                "Unexpected batched output shape: {:?}",
                shape
            )));
        }

        debug!(
            "Generated {} embeddings with dimension {} via batch inference",
            results.len(),
            results.first().map(|v| v.len()).unwrap_or(0)
        );

        Ok(results)
    }

    /// Normalize an embedding vector using L2 normalization.
    fn normalize_embedding(embedding: Vec<f32>) -> Vec<f32> {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-12 {
            embedding.iter().map(|x| x / norm).collect()
        } else {
            embedding
        }
    }

    fn generate_mock_embedding(&self, text: &str) -> Vector {
        // Deterministic mock embedding for testing
        // Uses text hash to create reproducible vectors
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
        if self.use_mock {
            // Use deterministic mock embeddings for testing
            Ok(self.generate_mock_embedding(text))
        } else {
            // Use real ONNX embeddings
            self.generate_embedding_real(text)
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        if self.use_mock {
            // Mock batch processing
            Ok(texts.iter().map(|text| self.generate_mock_embedding(text)).collect())
        } else {
            // Real ONNX batch processing with automatic batch splitting
            debug!("Processing {} texts with batch inference", texts.len());

            let mut all_embeddings = Vec::with_capacity(texts.len());

            // Split into optimal-sized batches to prevent OOM errors
            for chunk in texts.chunks(self.max_batch_size) {
                // Convert String slice to &str slice for batch processing
                let text_refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();

                // Execute true batch inference for this chunk
                let chunk_embeddings = self.generate_embeddings_batch_real(&text_refs)?;

                all_embeddings.extend(chunk_embeddings);
            }

            debug!(
                "Batch inference completed: {} texts processed in {} batches",
                texts.len(),
                (texts.len() + self.max_batch_size - 1) / self.max_batch_size
            );

            Ok(all_embeddings)
        }
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
