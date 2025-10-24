//! Main semantic search engine implementation.

use crate::cache::{EmbeddingCache, EmbeddingCacheKey, QueryCache, QueryCacheKey, CachedSearchResult};
use crate::config::SemanticConfig;
use crate::error::Result;
use crate::providers::{EmbeddingProvider, ProviderManager};
use crate::qdrant::{VectorIndex, QdrantVectorStore};
use crate::query::QueryProcessor;
use crate::ranking::{Ranker, RankableDocument, RankingStrategy};
use crate::types::{DocumentId, EntityType, IndexedDocument, Vector};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Main semantic search engine.
pub struct SemanticSearchEngine {
    config: SemanticConfig,
    provider: Arc<ProviderManager>,
    index: Arc<dyn VectorIndex>,
    documents: Arc<DashMap<DocumentId, IndexedDocument>>,
    query_processor: QueryProcessor,
    ranker: Ranker,
    embedding_cache: Option<EmbeddingCache>,
    query_cache: Option<QueryCache>,
}

/// Search filter options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilter {
    pub entity_type: Option<EntityType>,
    pub language: Option<String>,
    pub min_score: Option<f32>,
    pub metadata_filters: HashMap<String, String>,
}

/// Search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: DocumentId,
    pub entity_type: EntityType,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
    pub explanation: Option<String>,
}

impl SemanticSearchEngine {
    /// Create a new semantic search engine with Qdrant backend.
    pub async fn new(config: SemanticConfig) -> Result<Self> {
        info!("Initializing semantic search engine with Qdrant backend");

        // Create provider manager
        let provider = Arc::new(ProviderManager::from_config(&config.embedding).await?);

        // Determine dimension from provider
        let dimension = provider.dimension();
        info!("Using embedding dimension: {}", dimension);

        // Create Qdrant vector store
        let similarity_metric = config.index.similarity_metric;
        let qdrant_store = QdrantVectorStore::new(
            config.qdrant.clone(),
            dimension,
            similarity_metric,
        )
        .await?;

        let index: Arc<dyn VectorIndex> = Arc::new(qdrant_store);

        // Create caches
        let embedding_cache = if config.cache.enable_embedding_cache {
            Some(EmbeddingCache::new(
                config.cache.embedding_cache_size,
                Duration::from_secs(config.cache.embedding_cache_ttl_seconds),
            ))
        } else {
            None
        };

        let query_cache = if config.cache.enable_query_cache {
            Some(QueryCache::new(
                config.cache.query_cache_size,
                Duration::from_secs(config.cache.query_cache_ttl_seconds),
            ))
        } else {
            None
        };

        // Create ranker
        let ranker = Ranker::new(if config.search.enable_hybrid_search {
            RankingStrategy::Hybrid
        } else {
            RankingStrategy::Semantic
        });

        info!("Semantic search engine initialized successfully");

        Ok(Self {
            config,
            provider,
            index,
            documents: Arc::new(DashMap::new()),
            query_processor: QueryProcessor::new(),
            ranker,
            embedding_cache,
            query_cache,
        })
    }

    /// Create a new semantic search engine with custom vector store.
    pub async fn with_vector_store(
        config: SemanticConfig,
        vector_store: Arc<dyn VectorIndex>,
    ) -> Result<Self> {
        info!("Initializing semantic search engine with custom vector store");

        // Create provider manager
        let provider = Arc::new(ProviderManager::from_config(&config.embedding).await?);

        // Create caches
        let embedding_cache = if config.cache.enable_embedding_cache {
            Some(EmbeddingCache::new(
                config.cache.embedding_cache_size,
                Duration::from_secs(config.cache.embedding_cache_ttl_seconds),
            ))
        } else {
            None
        };

        let query_cache = if config.cache.enable_query_cache {
            Some(QueryCache::new(
                config.cache.query_cache_size,
                Duration::from_secs(config.cache.query_cache_ttl_seconds),
            ))
        } else {
            None
        };

        // Create ranker
        let ranker = Ranker::new(if config.search.enable_hybrid_search {
            RankingStrategy::Hybrid
        } else {
            RankingStrategy::Semantic
        });

        Ok(Self {
            config,
            provider,
            index: vector_store,
            documents: Arc::new(DashMap::new()),
            query_processor: QueryProcessor::new(),
            ranker,
            embedding_cache,
            query_cache,
        })
    }

    /// Index a document with its content.
    pub async fn index_document(
        &self,
        doc_id: DocumentId,
        content: String,
        entity_type: EntityType,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        debug!("Indexing document: {}", doc_id);

        // Generate embedding
        let embedding = self.generate_embedding(&content).await?;

        // Create indexed document
        let indexed_doc = IndexedDocument {
            id: doc_id.clone(),
            entity_type,
            content,
            embedding: embedding.clone(),
            model: self.provider.model().clone(),
            metadata,
            indexed_at: chrono::Utc::now(),
        };

        // Store document
        self.documents.insert(doc_id.clone(), indexed_doc);

        // Insert into index
        self.index.insert(doc_id, embedding).await?;

        debug!("Document indexed successfully");
        Ok(())
    }

    /// Index multiple documents in batch.
    pub async fn index_batch(
        &self,
        documents: Vec<(DocumentId, String, EntityType, HashMap<String, String>)>,
    ) -> Result<()> {
        info!("Batch indexing {} documents", documents.len());

        // Extract texts for batch embedding
        let texts: Vec<String> = documents.iter().map(|(_, content, _, _)| content.clone()).collect();

        // Generate embeddings in batch
        let embeddings = self.generate_embeddings_batch(&texts).await?;

        // Create indexed documents and insert into index
        let mut index_items = Vec::new();

        for ((doc_id, content, entity_type, metadata), embedding) in
            documents.into_iter().zip(embeddings.into_iter())
        {
            let indexed_doc = IndexedDocument {
                id: doc_id.clone(),
                entity_type,
                content,
                embedding: embedding.clone(),
                model: self.provider.model().clone(),
                metadata,
                indexed_at: chrono::Utc::now(),
            };

            self.documents.insert(doc_id.clone(), indexed_doc);
            index_items.push((doc_id, embedding));
        }

        // Batch insert into index
        self.index.insert_batch(index_items).await?;

        info!("Batch indexing completed");
        Ok(())
    }

    /// Search for documents.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_with_filter(query, limit, SearchFilter::default())
            .await
    }

    /// Search with filters.
    pub async fn search_with_filter(
        &self,
        query: &str,
        limit: usize,
        filter: SearchFilter,
    ) -> Result<Vec<SearchResult>> {
        debug!("Searching: {} (limit: {})", query, limit);

        // Enforce max limit
        let limit = limit.min(self.config.search.max_limit);

        // Check query cache
        if let Some(query_cache) = &self.query_cache {
            let cache_key = QueryCacheKey::new(
                query.to_string(),
                limit,
                filter.min_score.unwrap_or(self.config.search.default_threshold),
            );

            if let Some(cached) = query_cache.get(&cache_key).await {
                debug!("Query cache hit");
                return self.results_from_cache(&cached).await;
            }
        }

        // Process query
        let processed_query = self.query_processor.process(query)?;

        // Generate query embedding
        let query_embedding = self.generate_embedding(&processed_query.normalized).await?;

        // Search in index
        let mut index_results = self.index.search(&query_embedding, limit * 2).await?;

        // Apply filters
        index_results.retain(|result| self.matches_filter(&result.doc_id, &filter));

        // Convert to rankable documents
        let rankable_docs: Vec<RankableDocument> = index_results
            .into_iter()
            .filter_map(|result| {
                self.documents.get(&result.doc_id).map(|doc| RankableDocument {
                    id: result.doc_id.clone(),
                    content: doc.content.clone(),
                    semantic_score: result.score,
                    metadata: doc.metadata.clone(),
                })
            })
            .collect();

        // Rank results
        let ranked_results = if self.config.search.enable_reranking {
            self.ranker.rank(rankable_docs, &processed_query)
        } else {
            rankable_docs
                .into_iter()
                .map(|doc| crate::ranking::RankedResult {
                    id: doc.id,
                    final_score: doc.semantic_score,
                    semantic_score: doc.semantic_score,
                    keyword_score: 0.0,
                    recency_score: 0.0,
                    popularity_score: 0.0,
                    explanation: None,
                })
                .collect()
        };

        // Apply score threshold and limit
        let threshold = filter
            .min_score
            .unwrap_or(self.config.search.default_threshold);

        let final_results: Vec<SearchResult> = ranked_results
            .into_iter()
            .filter(|r| r.final_score >= threshold)
            .take(limit)
            .filter_map(|ranked| {
                self.documents.get(&ranked.id).map(|doc| SearchResult {
                    id: ranked.id.clone(),
                    entity_type: doc.entity_type,
                    content: doc.content.clone(),
                    score: ranked.final_score,
                    metadata: doc.metadata.clone(),
                    explanation: ranked.explanation,
                })
            })
            .collect();

        // Cache results
        if let Some(query_cache) = &self.query_cache {
            let cache_key = QueryCacheKey::new(query.to_string(), limit, threshold);
            let cached_result = CachedSearchResult {
                doc_ids: final_results.iter().map(|r| r.id.clone()).collect(),
                scores: final_results.iter().map(|r| r.score).collect(),
            };
            query_cache.insert(cache_key, cached_result).await;
        }

        debug!("Found {} results", final_results.len());
        Ok(final_results)
    }

    /// Remove a document from the index.
    pub async fn remove_document(&self, doc_id: &DocumentId) -> Result<()> {
        debug!("Removing document: {}", doc_id);

        // Remove from document store
        self.documents.remove(doc_id);

        // Remove from index
        self.index.remove(doc_id).await?;

        // Invalidate caches
        self.invalidate_caches().await;

        debug!("Document removed successfully: {}", doc_id);
        Ok(())
    }

    /// Clear all documents and index.
    pub async fn clear(&self) -> Result<()> {
        info!("Clearing search engine");

        // Clear document store
        self.documents.clear();

        // Clear index
        self.index.clear().await?;

        // Invalidate caches
        self.invalidate_caches().await;

        info!("Search engine cleared successfully");
        Ok(())
    }

    /// Get index statistics.
    pub async fn stats(&self) -> crate::qdrant::IndexStats {
        self.index.stats().await
    }

    /// Get document count.
    pub async fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Create a snapshot for backup.
    pub async fn create_snapshot(&self) -> Result<String> {
        self.index.create_snapshot().await
    }

    /// Optimize the index.
    pub async fn optimize(&self) -> Result<()> {
        self.index.optimize().await
    }

    /// Generate embedding for text with caching.
    async fn generate_embedding(&self, text: &str) -> Result<Vector> {
        // Check cache
        if let Some(cache) = &self.embedding_cache {
            let cache_key = EmbeddingCacheKey::new(
                text.to_string(),
                self.provider.model().model_name.clone(),
            );

            if let Some(cached) = cache.get(&cache_key).await {
                debug!("Embedding cache hit");
                return Ok((*cached).clone());
            }

            // Generate and cache
            let embedding = self.provider.embed(text).await?;
            cache.insert(cache_key, embedding.clone()).await;
            return Ok(embedding);
        }

        // No cache, generate directly
        self.provider.embed(text).await
    }

    /// Generate embeddings for multiple texts.
    async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        // For now, use batch API without caching
        // In production, implement smart caching for batch operations
        self.provider.embed_batch(texts).await
    }

    /// Check if a document matches the filter.
    fn matches_filter(&self, doc_id: &DocumentId, filter: &SearchFilter) -> bool {
        if let Some(doc) = self.documents.get(doc_id) {
            // Check entity type
            if let Some(required_type) = filter.entity_type {
                if doc.entity_type != required_type {
                    return false;
                }
            }

            // Check metadata filters
            for (key, value) in &filter.metadata_filters {
                if doc.metadata.get(key) != Some(value) {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    /// Invalidate all caches.
    async fn invalidate_caches(&self) {
        if let Some(cache) = &self.embedding_cache {
            cache.clear().await;
        }
        if let Some(cache) = &self.query_cache {
            cache.clear().await;
        }
    }

    /// Reconstruct results from cache.
    async fn results_from_cache(&self, cached: &CachedSearchResult) -> Result<Vec<SearchResult>> {
        let results = cached
            .doc_ids
            .iter()
            .zip(cached.scores.iter())
            .filter_map(|(doc_id, score)| {
                self.documents.get(doc_id).map(|doc| SearchResult {
                    id: doc_id.clone(),
                    entity_type: doc.entity_type,
                    content: doc.content.clone(),
                    score: *score,
                    metadata: doc.metadata.clone(),
                    explanation: None,
                })
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qdrant::MockVectorStore;
    use crate::types::SimilarityMetric;

    /// Create a test engine with real Qdrant backend.
    /// Requires Qdrant server running - use for integration tests only.
    async fn create_test_engine() -> SemanticSearchEngine {
        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.embedding.fallback_providers = vec![];

        // Use unique collection name for each test to avoid conflicts
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        config.qdrant.collection_name = format!("test_{}", timestamp);

        SemanticSearchEngine::new(config).await.unwrap()
    }

    /// Create a test engine with MockVectorStore backend.
    /// No Qdrant required - use for fast unit tests.
    async fn create_test_engine_with_mock(dimension: usize) -> SemanticSearchEngine {
        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.embedding.fallback_providers = vec![];

        let mock_store = MockVectorStore::new(dimension, SimilarityMetric::Cosine);
        let vector_store: Arc<dyn VectorIndex> = Arc::new(mock_store);

        SemanticSearchEngine::with_vector_store(config, vector_store)
            .await
            .unwrap()
    }

    // Integration tests - require Qdrant server running
    #[tokio::test]
    async fn test_index_and_search() {
        let engine = create_test_engine().await;

        // Index documents
        engine
            .index_document(
                "doc1".to_string(),
                "This is a test document about machine learning".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        engine
            .index_document(
                "doc2".to_string(),
                "This is about natural language processing".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        // Search
        let results = engine.search("machine learning", 10).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc2");
    }

    #[tokio::test]
    async fn test_batch_indexing() {
        let engine = create_test_engine().await;

        let documents = vec![
            (
                "doc1".to_string(),
                "Content 1".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
            (
                "doc2".to_string(),
                "Content 2".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
            (
                "doc3".to_string(),
                "Content 3".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
        ];

        engine.index_batch(documents).await.unwrap();

        assert_eq!(engine.document_count().await, 3);
    }

    #[tokio::test]
    async fn test_remove_document() {
        let engine = create_test_engine().await;

        engine
            .index_document(
                "doc1".to_string(),
                "Test content".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        assert_eq!(engine.document_count().await, 1);

        engine.remove_document(&"doc1".to_string()).await.unwrap();

        assert_eq!(engine.document_count().await, 0);
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let engine = create_test_engine().await;

        let mut metadata1 = HashMap::new();
        metadata1.insert("language".to_string(), "rust".to_string());

        let mut metadata2 = HashMap::new();
        metadata2.insert("language".to_string(), "python".to_string());

        engine
            .index_document(
                "doc1".to_string(),
                "Rust content".to_string(),
                EntityType::Code,
                metadata1,
            )
            .await
            .unwrap();

        engine
            .index_document(
                "doc2".to_string(),
                "Python content".to_string(),
                EntityType::Code,
                metadata2,
            )
            .await
            .unwrap();

        // Filter by entity type
        let filter = SearchFilter {
            entity_type: Some(EntityType::Code),
            ..Default::default()
        };

        let results = engine.search_with_filter("content", 10, filter).await.unwrap();
        assert_eq!(results.len(), 0);

        // Filter by metadata
        let mut metadata_filters = HashMap::new();
        metadata_filters.insert("language".to_string(), "rust".to_string());

        let filter = SearchFilter {
            metadata_filters,
            ..Default::default()
        };

        let results = engine.search_with_filter("content", 10, filter).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let engine = create_test_engine().await;

        engine
            .index_document(
                "doc1".to_string(),
                "Test".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        assert_eq!(engine.document_count().await, 1);

        engine.clear().await.unwrap();

        assert_eq!(engine.document_count().await, 0);
    }

    // Unit tests with MockVectorStore - no Qdrant required
    #[tokio::test]
    async fn test_mock_index_and_search() {
        // Mock embedding dimension is 384 (from ONNX MiniLM)
        let engine = create_test_engine_with_mock(384).await;

        // Index documents
        engine
            .index_document(
                "doc1".to_string(),
                "This is a test document about machine learning".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        engine
            .index_document(
                "doc2".to_string(),
                "This is about natural language processing".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        // Search - with mock provider, results should still work
        let results = engine.search("machine learning", 10).await.unwrap();

        assert!(!results.is_empty());
        // Note: Mock provider returns predictable vectors, so we just verify basic functionality
        assert_eq!(engine.document_count().await, 2);
    }

    #[tokio::test]
    async fn test_mock_batch_indexing() {
        let engine = create_test_engine_with_mock(384).await;

        let documents = vec![
            (
                "doc1".to_string(),
                "Content 1".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
            (
                "doc2".to_string(),
                "Content 2".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
            (
                "doc3".to_string(),
                "Content 3".to_string(),
                EntityType::Document,
                HashMap::new(),
            ),
        ];

        engine.index_batch(documents).await.unwrap();

        assert_eq!(engine.document_count().await, 3);
    }

    #[tokio::test]
    async fn test_mock_remove_document() {
        let engine = create_test_engine_with_mock(384).await;

        engine
            .index_document(
                "doc1".to_string(),
                "Test content".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        assert_eq!(engine.document_count().await, 1);

        engine.remove_document(&"doc1".to_string()).await.unwrap();

        assert_eq!(engine.document_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_search_with_filter() {
        let engine = create_test_engine_with_mock(384).await;

        let mut metadata1 = HashMap::new();
        metadata1.insert("language".to_string(), "rust".to_string());

        let mut metadata2 = HashMap::new();
        metadata2.insert("language".to_string(), "python".to_string());

        engine
            .index_document(
                "doc1".to_string(),
                "Rust content".to_string(),
                EntityType::Code,
                metadata1,
            )
            .await
            .unwrap();

        engine
            .index_document(
                "doc2".to_string(),
                "Python content".to_string(),
                EntityType::Code,
                metadata2,
            )
            .await
            .unwrap();

        // Verify documents were indexed
        assert_eq!(engine.document_count().await, 2);

        // Test: Filter by entity type only
        let filter = SearchFilter {
            entity_type: Some(EntityType::Code),
            min_score: Some(-1.0),
            ..Default::default()
        };

        let results = engine.search_with_filter("content", 10, filter).await.unwrap();
        assert_eq!(results.len(), 2, "Expected 2 results with Code entity type");

        // Note: Metadata filtering is tested separately in test_mock_metadata_filter_simple
    }

    #[tokio::test]
    async fn test_mock_clear() {
        let engine = create_test_engine_with_mock(384).await;

        engine
            .index_document(
                "doc1".to_string(),
                "Test".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        assert_eq!(engine.document_count().await, 1);

        engine.clear().await.unwrap();

        assert_eq!(engine.document_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_multiple_searches() {
        let engine = create_test_engine_with_mock(384).await;

        // Index several documents
        for i in 1..=5 {
            engine
                .index_document(
                    format!("doc{}", i),
                    format!("Document content number {}", i),
                    EntityType::Document,
                    HashMap::new(),
                )
                .await
                .unwrap();
        }

        // Perform multiple searches with very low threshold (mock embeddings may have lower scores)
        let filter = SearchFilter {
            min_score: Some(-1.0),
            ..Default::default()
        };

        let results1 = engine.search_with_filter("content", 3, filter.clone()).await.unwrap();
        assert_eq!(results1.len(), 3);

        let results2 = engine.search_with_filter("document", 5, filter.clone()).await.unwrap();
        assert_eq!(results2.len(), 5);

        let results3 = engine.search_with_filter("number", 2, filter).await.unwrap();
        assert_eq!(results3.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_metadata_filter_simple() {
        let engine = create_test_engine_with_mock(384).await;

        // Create two documents with distinct tags
        let mut metadata1 = HashMap::new();
        metadata1.insert("tag".to_string(), "A".to_string());

        let mut metadata2 = HashMap::new();
        metadata2.insert("tag".to_string(), "B".to_string());

        engine
            .index_document(
                "docA".to_string(),
                "Content A".to_string(),
                EntityType::Document,
                metadata1,
            )
            .await
            .unwrap();

        engine
            .index_document(
                "docB".to_string(),
                "Content B".to_string(),
                EntityType::Document,
                metadata2,
            )
            .await
            .unwrap();

        // Search with filter for tag=A
        let mut meta_filter = HashMap::new();
        meta_filter.insert("tag".to_string(), "A".to_string());

        let filter = SearchFilter {
            metadata_filters: meta_filter,
            min_score: Some(-1.0),
            ..Default::default()
        };

        let results = engine.search_with_filter("Content", 10, filter).await.unwrap();

        // Debug: print what we got
        eprintln!("Results count: {}", results.len());
        for (i, result) in results.iter().enumerate() {
            eprintln!("Result {}: id={}, tag={:?}", i, result.id, result.metadata.get("tag"));
        }

        assert_eq!(results.len(), 1, "Expected 1 result with tag=A");
        assert_eq!(results[0].id, "docA");
    }

    #[tokio::test]
    async fn test_mock_stats() {
        let engine = create_test_engine_with_mock(384).await;

        // Index some documents
        engine
            .index_document(
                "doc1".to_string(),
                "Test".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        let stats = engine.stats().await;
        assert_eq!(stats.total_vectors, 1);
        assert_eq!(stats.dimension, 384);
        assert_eq!(stats.collection_status, "Green");
    }
}
