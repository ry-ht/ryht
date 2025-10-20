//! Plain text processor with intelligent chunking.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::Result;
use std::collections::HashMap;

/// Processor for plain text documents
pub struct TxtProcessor {
    chunk_size: usize,
    chunk_overlap: usize,
}

impl TxtProcessor {
    /// Create a new text processor with default settings
    pub fn new() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 100,
        }
    }

    /// Create with custom chunk size
    pub fn with_chunk_size(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap: overlap,
        }
    }

    /// Chunk text by paragraphs first, then by size
    fn chunk_text(&self, text: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        // Split by double newlines (paragraphs)
        let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();

        let mut current_chunk = String::new();
        let mut chunk_start = 0;

        for paragraph in paragraphs {
            let para_len = paragraph.chars().count();

            // If adding this paragraph exceeds chunk size, save current chunk
            if current_chunk.chars().count() + para_len > self.chunk_size
                && !current_chunk.is_empty()
            {
                let chunk_end = chunk_start + current_chunk.len();
                chunks.push(ContentChunk::with_offsets(
                    current_chunk.clone(),
                    ChunkType::Paragraph,
                    chunk_start,
                    chunk_end,
                ));

                // Start new chunk with overlap
                let words: Vec<&str> = current_chunk.split_whitespace().collect();
                let overlap_words: Vec<&str> = words
                    .iter()
                    .rev()
                    .take(self.chunk_overlap / 10)
                    .rev()
                    .copied()
                    .collect();

                chunk_start = chunk_end - overlap_words.join(" ").len();
                current_chunk = overlap_words.join(" ");
                if !current_chunk.is_empty() {
                    current_chunk.push_str(" ");
                }
            }

            current_chunk.push_str(paragraph);
            current_chunk.push_str("\n\n");
        }

        // Add final chunk
        if !current_chunk.trim().is_empty() {
            chunks.push(ContentChunk::with_offsets(
                current_chunk.trim().to_string(),
                ChunkType::Paragraph,
                chunk_start,
                chunk_start + current_chunk.len(),
            ));
        }

        chunks
    }
}

impl Default for TxtProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for TxtProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing text document ({} bytes)", input.len());

        // Try UTF-8 first, fall back to lossy conversion
        let text_content = String::from_utf8_lossy(input).to_string();

        let chunks = self.chunk_text(&text_content);

        let mut metadata = HashMap::new();
        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("text".to_string()),
        );
        metadata.insert(
            "lines".to_string(),
            serde_json::Value::Number(text_content.lines().count().into()),
        );
        metadata.insert(
            "words".to_string(),
            serde_json::Value::Number(text_content.split_whitespace().count().into()),
        );

        let mut result = ProcessedContent::new(ContentType::Text, text_content).with_chunks(chunks);
        result.metadata = metadata;

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["txt", "text"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["text/plain"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_txt_processor() {
        let processor = TxtProcessor::new();
        let content = b"This is a test.\n\nThis is another paragraph.";

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Text);
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_txt_chunking() {
        let processor = TxtProcessor::with_chunk_size(50, 10);
        let content = b"First paragraph with some text.\n\nSecond paragraph with more text.\n\nThird paragraph.";

        let result = processor.process(content).await.unwrap();
        assert!(result.chunks.len() > 1);
    }

    #[test]
    fn test_txt_extensions() {
        let processor = TxtProcessor::new();
        assert!(processor.supported_extensions().contains(&"txt"));
    }
}
