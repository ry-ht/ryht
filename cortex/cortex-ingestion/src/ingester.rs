//! Enhanced document ingestion implementation with multi-format support.

use crate::embeddings::{EmbeddingService, MockEmbeddingProvider};
use crate::extractor::extract_comprehensive_metadata;
use crate::processors::ProcessorFactory;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::traits::{Ingester, Storage};
use cortex_core::types::{Chunk, Document};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Enhanced document ingester with multi-format support
pub struct DocumentIngester {
    storage: Arc<dyn Storage>,
    processor_factory: Arc<ProcessorFactory>,
    embedding_service: Option<Arc<EmbeddingService>>,
    auto_chunk: bool,
    generate_embeddings: bool,
}

impl DocumentIngester {
    /// Create a new document ingester
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            processor_factory: Arc::new(ProcessorFactory::new()),
            embedding_service: None,
            auto_chunk: true,
            generate_embeddings: false,
        }
    }

    /// Enable automatic chunking
    pub fn with_auto_chunk(mut self, enabled: bool) -> Self {
        self.auto_chunk = enabled;
        self
    }

    /// Set embedding service
    pub fn with_embedding_service(mut self, service: Arc<EmbeddingService>) -> Self {
        self.embedding_service = Some(service);
        self
    }

    /// Enable embedding generation
    pub fn with_embeddings(mut self, enabled: bool) -> Self {
        self.generate_embeddings = enabled;
        // Create default embedding service if needed
        if enabled && self.embedding_service.is_none() {
            let provider = Arc::new(MockEmbeddingProvider::default());
            self.embedding_service = Some(Arc::new(EmbeddingService::with_provider(provider)));
        }
        self
    }

    /// Calculate content hash
    fn hash_content(content: &[u8]) -> String {
        let hash = blake3::hash(content);
        hash.to_hex().to_string()
    }

    /// Detect MIME type from file extension
    fn detect_mime_type(path: &Path) -> String {
        mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string())
    }

    /// Read file content
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .map_err(|e| CortexError::ingestion(format!("Failed to read file: {}", e)))
    }

    /// Process file with appropriate processor
    async fn process_file(&self, path: &Path, content: &[u8]) -> Result<ProcessedFileInfo> {
        // Get processor for this file type
        let processor = self.processor_factory.get_for_path(path);

        let (text_content, chunks, metadata) = if let Some(processor) = processor {
            tracing::debug!(
                "Processing {} with {:?} processor",
                path.display(),
                processor.content_type()
            );

            match processor.process(content).await {
                Ok(processed) => {
                    let chunks = if self.auto_chunk {
                        processed.chunks
                    } else {
                        Vec::new()
                    };

                    (processed.text_content, chunks, processed.metadata)
                }
                Err(e) => {
                    tracing::warn!("Failed to process file: {}, using fallback", e);
                    // Fallback to basic processing
                    let text = String::from_utf8_lossy(content).to_string();
                    let meta = extract_comprehensive_metadata(path, &text);
                    (text, Vec::new(), meta)
                }
            }
        } else {
            // No processor available, try basic text extraction
            tracing::debug!("No processor for {}, using basic text extraction", path.display());
            let text = String::from_utf8_lossy(content).to_string();
            let meta = extract_comprehensive_metadata(path, &text);
            (text, Vec::new(), meta)
        };

        Ok(ProcessedFileInfo {
            text_content,
            chunks,
            metadata,
        })
    }

    /// Generate embeddings for chunks
    async fn generate_chunk_embeddings(
        &self,
        chunks: &[crate::processors::ContentChunk],
    ) -> Result<Vec<Vec<f32>>> {
        if let Some(embedding_service) = &self.embedding_service {
            let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
            embedding_service.embed_batch(&texts).await
        } else {
            Ok(Vec::new())
        }
    }
}

struct ProcessedFileInfo {
    text_content: String,
    chunks: Vec<crate::processors::ContentChunk>,
    metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[async_trait]
impl Ingester for DocumentIngester {
    async fn ingest_file(&self, project_id: CortexId, path: &Path) -> Result<Document> {
        tracing::info!("Ingesting file: {:?}", path);

        let content = self.read_file(path).await?;
        let content_hash = Self::hash_content(&content);
        let mime_type = Self::detect_mime_type(path);

        // Process file with appropriate processor
        let processed = self.process_file(path, &content).await?;

        // Convert metadata to HashMap<String, String> for Document
        let metadata: std::collections::HashMap<String, String> = processed
            .metadata
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();

        let document = Document {
            id: CortexId::new(),
            project_id,
            path: path.to_string_lossy().to_string(),
            content_hash: content_hash.clone(),
            size: content.len() as u64,
            mime_type,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata,
        };

        // Store document
        self.storage.store_document(&document).await?;

        // Process and store chunks if enabled
        if self.auto_chunk && !processed.chunks.is_empty() {
            tracing::debug!("Processing {} chunks for document", processed.chunks.len());

            // Generate embeddings if enabled
            let embeddings = if self.generate_embeddings {
                self.generate_chunk_embeddings(&processed.chunks).await?
            } else {
                Vec::new()
            };

            // Store chunks
            for (idx, chunk) in processed.chunks.iter().enumerate() {
                let chunk_metadata: std::collections::HashMap<String, String> = chunk
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect();

                let db_chunk = Chunk {
                    id: CortexId::new(),
                    document_id: document.id,
                    content: chunk.content.clone(),
                    start_offset: chunk.start_offset,
                    end_offset: chunk.end_offset,
                    chunk_index: idx,
                    metadata: chunk_metadata,
                };

                // Note: In a full implementation, you'd store the chunk
                // For now we just log it
                tracing::trace!("Would store chunk {}: {} bytes", idx, chunk.content.len());

                // Store embedding if available
                if let Some(embedding) = embeddings.get(idx) {
                    let emb = cortex_core::types::Embedding {
                        id: CortexId::new(),
                        entity_id: db_chunk.id,
                        entity_type: cortex_core::types::EntityType::Chunk,
                        vector: embedding.clone(),
                        model: self
                            .embedding_service
                            .as_ref()
                            .map(|s| s.model_name().to_string())
                            .unwrap_or_default(),
                        created_at: chrono::Utc::now(),
                    };
                    self.storage.store_embedding(&emb).await?;
                }
            }
        }

        Ok(document)
    }

    async fn ingest_directory(&self, project_id: CortexId, path: &Path) -> Result<Vec<Document>> {
        tracing::info!("Ingesting directory: {:?}", path);

        let walker = ignore::WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        let mut documents = Vec::new();

        for entry in walker {
            let entry = entry.map_err(|e| CortexError::ingestion(format!("Walk error: {}", e)))?;

            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                match self.ingest_file(project_id, entry.path()).await {
                    Ok(doc) => documents.push(doc),
                    Err(e) => {
                        tracing::warn!("Failed to ingest {:?}: {}", entry.path(), e);
                    }
                }
            }
        }

        Ok(documents)
    }

    async fn update_document(&self, document_id: CortexId, path: &Path) -> Result<Document> {
        tracing::info!("Updating document: {:?}", path);

        let content = self.read_file(path).await?;
        let content_hash = Self::hash_content(&content);

        // Get existing document
        let mut document = self
            .storage
            .get_document(document_id)
            .await?
            .ok_or_else(|| CortexError::not_found("document", document_id.to_string()))?;

        // Update fields
        document.content_hash = content_hash;
        document.size = content.len() as u64;
        document.updated_at = chrono::Utc::now();

        self.storage.store_document(&document).await?;

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::types::{Project, SystemStats};
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Mock storage for testing
    struct MockStorage;

    #[async_trait]
    impl Storage for MockStorage {
        async fn store_project(&self, _project: &Project) -> Result<()> {
            Ok(())
        }

        async fn get_project(&self, _id: CortexId) -> Result<Option<Project>> {
            Ok(None)
        }

        async fn list_projects(&self) -> Result<Vec<Project>> {
            Ok(Vec::new())
        }

        async fn delete_project(&self, _id: CortexId) -> Result<()> {
            Ok(())
        }

        async fn store_document(&self, _document: &Document) -> Result<()> {
            Ok(())
        }

        async fn get_document(&self, _id: CortexId) -> Result<Option<Document>> {
            Ok(Some(Document {
                id: _id,
                project_id: CortexId::new(),
                path: "test.txt".to_string(),
                content_hash: "hash".to_string(),
                size: 100,
                mime_type: "text/plain".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                metadata: HashMap::new(),
            }))
        }

        async fn list_documents(&self, _project_id: CortexId) -> Result<Vec<Document>> {
            Ok(Vec::new())
        }

        async fn delete_document(&self, _id: CortexId) -> Result<()> {
            Ok(())
        }

        async fn store_embedding(
            &self,
            _embedding: &cortex_core::types::Embedding,
        ) -> Result<()> {
            Ok(())
        }

        async fn get_embeddings(&self, _entity_id: CortexId) -> Result<Vec<cortex_core::types::Embedding>> {
            Ok(Vec::new())
        }

        async fn store_episode(&self, _episode: &cortex_core::types::Episode) -> Result<()> {
            Ok(())
        }

        async fn get_episode(&self, _id: CortexId) -> Result<Option<cortex_core::types::Episode>> {
            Ok(None)
        }

        async fn get_stats(&self) -> Result<SystemStats> {
            Ok(SystemStats {
                total_projects: 0,
                total_documents: 0,
                total_chunks: 0,
                total_embeddings: 0,
                total_episodes: 0,
                storage_size_bytes: 0,
                last_updated: chrono::Utc::now(),
            })
        }
    }

    #[test]
    fn test_mime_type_detection() {
        let path = Path::new("/test/file.rs");
        let mime = DocumentIngester::detect_mime_type(path);
        assert!(mime.contains("rust") || mime == "text/plain");

        let path = Path::new("/test/file.json");
        let mime = DocumentIngester::detect_mime_type(path);
        assert!(mime.contains("json"));
    }

    #[tokio::test]
    async fn test_ingester_creation() {
        let storage = Arc::new(MockStorage);
        let ingester = DocumentIngester::new(storage).with_auto_chunk(true).with_embeddings(true);

        assert!(ingester.auto_chunk);
        assert!(ingester.generate_embeddings);
    }
}
