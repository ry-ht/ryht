//! Document ingestion and processing for Cortex.
//!
//! This crate provides comprehensive document ingestion capabilities including:
//! - Multiple document format processors (PDF, Markdown, HTML, JSON, YAML, CSV, Text)
//! - Intelligent chunking strategies (semantic, hierarchical, code-aware)
//! - Metadata extraction (language detection, keywords, document properties)
//! - Embedding generation interface
//! - External project import functionality

pub mod ingester;
pub mod chunker;
pub mod extractor;
pub mod filters;
pub mod processors;
pub mod embeddings;
pub mod project_loader;

pub use ingester::DocumentIngester;
pub use chunker::{Chunker, SemanticChunker, CodeChunker, HierarchicalChunker, ChunkStrategy};
pub use processors::{
    ContentProcessor, ContentType, ProcessedContent, ContentChunk, ChunkType,
    ProcessorFactory, detect_content_type, detect_mime_type,
};
pub use embeddings::{EmbeddingProvider, EmbeddingService, EmbeddingConfig};
pub use project_loader::{ProjectLoader, ProjectImportOptions, ImportReport, ImportedFile};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::ingester::DocumentIngester;
    pub use crate::chunker::{Chunker, SemanticChunker, CodeChunker, ChunkStrategy};
    pub use crate::processors::{
        ContentProcessor, ContentType, ProcessedContent, ContentChunk,
        ProcessorFactory,
    };
    pub use crate::embeddings::{EmbeddingProvider, EmbeddingService};
    pub use crate::project_loader::{ProjectLoader, ProjectImportOptions};
    pub use crate::extractor::{
        extract_comprehensive_metadata, detect_language, detect_programming_language,
    };
}
