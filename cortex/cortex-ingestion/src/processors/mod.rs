//! Document processors for various file formats.
//!
//! This module provides processors for different document types including
//! PDF, Markdown, HTML, JSON, YAML, CSV, and plain text files.

use async_trait::async_trait;
use cortex_core::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod pdf;
pub mod markdown;
pub mod txt;
pub mod html;
pub mod json;
pub mod yaml;
pub mod csv;

pub use pdf::PdfProcessor;
pub use markdown::MarkdownProcessor;
pub use txt::TxtProcessor;
pub use html::HtmlProcessor;
pub use json::JsonProcessor;
pub use yaml::YamlProcessor;
pub use csv::CsvProcessor;

/// Content type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Pdf,
    Markdown,
    Text,
    Html,
    Json,
    Yaml,
    Csv,
    Code,
    Unknown,
}

impl ContentType {
    /// Detect content type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => Self::Pdf,
            "md" | "markdown" | "mdown" | "mkdn" | "mkd" => Self::Markdown,
            "txt" | "text" => Self::Text,
            "html" | "htm" => Self::Html,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "csv" => Self::Csv,
            "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "h" => Self::Code,
            _ => Self::Unknown,
        }
    }

    /// Get supported file extensions for this content type
    pub fn extensions(&self) -> Vec<&'static str> {
        match self {
            Self::Pdf => vec!["pdf"],
            Self::Markdown => vec!["md", "markdown", "mdown", "mkdn", "mkd"],
            Self::Text => vec!["txt", "text"],
            Self::Html => vec!["html", "htm"],
            Self::Json => vec!["json"],
            Self::Yaml => vec!["yaml", "yml"],
            Self::Csv => vec!["csv"],
            Self::Code => vec!["rs", "py", "js", "ts", "go", "java", "cpp", "c", "h"],
            Self::Unknown => vec![],
        }
    }

    /// Get MIME types for this content type
    pub fn mime_types(&self) -> Vec<&'static str> {
        match self {
            Self::Pdf => vec!["application/pdf"],
            Self::Markdown => vec!["text/markdown", "text/x-markdown"],
            Self::Text => vec!["text/plain"],
            Self::Html => vec!["text/html"],
            Self::Json => vec!["application/json"],
            Self::Yaml => vec!["application/yaml", "text/yaml"],
            Self::Csv => vec!["text/csv"],
            Self::Code => vec!["text/plain"],
            Self::Unknown => vec!["application/octet-stream"],
        }
    }
}

/// Type of content chunk
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// Full document
    Document,
    /// Page in a document
    Page,
    /// Section with heading
    Section,
    /// Paragraph
    Paragraph,
    /// Code block
    CodeBlock,
    /// Table
    Table,
    /// List
    List,
    /// Sentence
    Sentence,
    /// Custom chunk type
    Custom(String),
}

/// A chunk of processed content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    /// The text content of this chunk
    pub content: String,
    /// Type of chunk
    pub chunk_type: ChunkType,
    /// Start offset in the original document (bytes)
    pub start_offset: usize,
    /// End offset in the original document (bytes)
    pub end_offset: usize,
    /// Metadata specific to this chunk
    pub metadata: HashMap<String, serde_json::Value>,
    /// Optional embedding vector (generated later)
    pub embedding: Option<Vec<f32>>,
}

impl ContentChunk {
    /// Create a new content chunk
    pub fn new(content: String, chunk_type: ChunkType) -> Self {
        Self {
            content,
            chunk_type,
            start_offset: 0,
            end_offset: 0,
            metadata: HashMap::new(),
            embedding: None,
        }
    }

    /// Create a chunk with offsets
    pub fn with_offsets(
        content: String,
        chunk_type: ChunkType,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            content,
            chunk_type,
            start_offset,
            end_offset,
            metadata: HashMap::new(),
            embedding: None,
        }
    }

    /// Add metadata to this chunk
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get character count
    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    /// Get word count
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }
}

/// Processed content result from a document processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedContent {
    /// The type of content
    pub content_type: ContentType,
    /// Full text content (extracted)
    pub text_content: String,
    /// Structured data (if applicable, e.g., JSON parsed)
    pub structured_data: Option<serde_json::Value>,
    /// Document-level metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Content chunks for semantic processing
    pub chunks: Vec<ContentChunk>,
}

impl ProcessedContent {
    /// Create a new processed content result
    pub fn new(content_type: ContentType, text_content: String) -> Self {
        Self {
            content_type,
            text_content,
            structured_data: None,
            metadata: HashMap::new(),
            chunks: Vec::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add structured data
    pub fn with_structured_data(mut self, data: serde_json::Value) -> Self {
        self.structured_data = Some(data);
        self
    }

    /// Add chunks
    pub fn with_chunks(mut self, chunks: Vec<ContentChunk>) -> Self {
        self.chunks = chunks;
        self
    }

    /// Get total character count
    pub fn char_count(&self) -> usize {
        self.text_content.chars().count()
    }

    /// Get total word count
    pub fn word_count(&self) -> usize {
        self.text_content.split_whitespace().count()
    }
}

/// Trait for document content processors
#[async_trait]
pub trait ContentProcessor: Send + Sync {
    /// Process a document and extract content
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent>;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&str>;

    /// Get supported MIME types
    fn supported_mime_types(&self) -> Vec<&str>;

    /// Get the content type this processor handles
    fn content_type(&self) -> ContentType;

    /// Check if this processor can handle the given file
    fn can_process(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return self.supported_extensions().contains(&ext);
        }
        false
    }

    /// Check if this processor can handle the given MIME type
    fn can_process_mime(&self, mime_type: &str) -> bool {
        self.supported_mime_types()
            .iter()
            .any(|&mt| mt == mime_type)
    }
}

/// Factory for creating content processors
pub struct ProcessorFactory {
    processors: HashMap<ContentType, Box<dyn ContentProcessor>>,
}

impl ProcessorFactory {
    /// Create a new processor factory with default processors
    pub fn new() -> Self {
        let mut factory = Self {
            processors: HashMap::new(),
        };

        // Register default processors
        factory.register(ContentType::Pdf, Box::new(PdfProcessor::new()));
        factory.register(ContentType::Markdown, Box::new(MarkdownProcessor::new()));
        factory.register(ContentType::Text, Box::new(TxtProcessor::new()));
        factory.register(ContentType::Html, Box::new(HtmlProcessor::new()));
        factory.register(ContentType::Json, Box::new(JsonProcessor::new()));
        factory.register(ContentType::Yaml, Box::new(YamlProcessor::new()));
        factory.register(ContentType::Csv, Box::new(CsvProcessor::new()));

        factory
    }

    /// Register a processor for a content type
    pub fn register(&mut self, content_type: ContentType, processor: Box<dyn ContentProcessor>) {
        self.processors.insert(content_type, processor);
    }

    /// Get a processor for a file path
    pub fn get_for_path(&self, path: &Path) -> Option<&dyn ContentProcessor> {
        // Try to detect content type from extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let content_type = ContentType::from_extension(ext);
            if let Some(processor) = self.processors.get(&content_type) {
                return Some(processor.as_ref());
            }
        }

        // Try each processor to see if it can handle the file
        for processor in self.processors.values() {
            if processor.can_process(path) {
                return Some(processor.as_ref());
            }
        }

        None
    }

    /// Get a processor for a MIME type
    pub fn get_for_mime(&self, mime_type: &str) -> Option<&dyn ContentProcessor> {
        for processor in self.processors.values() {
            if processor.can_process_mime(mime_type) {
                return Some(processor.as_ref());
            }
        }
        None
    }

    /// Get a processor for a content type
    pub fn get(&self, content_type: ContentType) -> Option<&dyn ContentProcessor> {
        self.processors.get(&content_type).map(|p| p.as_ref())
    }
}

impl Default for ProcessorFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to detect content type from file path
pub fn detect_content_type(path: &Path) -> ContentType {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        ContentType::from_extension(ext)
    } else {
        ContentType::Unknown
    }
}

/// Helper function to detect MIME type from file path
pub fn detect_mime_type(path: &Path) -> String {
    mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}
