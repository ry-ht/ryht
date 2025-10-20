//! HTML document processor with structure preservation.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::{CortexError, Result};
use html2text;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// Processor for HTML documents
pub struct HtmlProcessor {
    preserve_structure: bool,
    extract_metadata: bool,
}

impl HtmlProcessor {
    /// Create a new HTML processor
    pub fn new() -> Self {
        Self {
            preserve_structure: true,
            extract_metadata: true,
        }
    }

    /// Extract metadata from HTML head
    fn extract_metadata(&self, document: &Html) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        // Extract title
        if let Ok(title_selector) = Selector::parse("title") {
            if let Some(title) = document.select(&title_selector).next() {
                metadata.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.text().collect::<String>()),
                );
            }
        }

        // Extract meta tags
        if let Ok(meta_selector) = Selector::parse("meta") {
            for meta in document.select(&meta_selector) {
                if let Some(name) = meta.value().attr("name") {
                    if let Some(content) = meta.value().attr("content") {
                        metadata.insert(
                            format!("meta_{}", name),
                            serde_json::Value::String(content.to_string()),
                        );
                    }
                }
            }
        }

        metadata
    }

    /// Extract structured chunks from HTML
    fn extract_chunks(&self, document: &Html) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        // Extract headings with their sections
        for level in 1..=6 {
            let selector_str = format!("h{}", level);
            if let Ok(selector) = Selector::parse(&selector_str) {
                for heading in document.select(&selector) {
                    let heading_text = heading.text().collect::<String>();
                    if !heading_text.trim().is_empty() {
                        let mut chunk = ContentChunk::new(heading_text, ChunkType::Section);
                        chunk.metadata.insert(
                            "heading_level".to_string(),
                            serde_json::Value::Number(level.into()),
                        );
                        chunks.push(chunk);
                    }
                }
            };
        }

        // Extract paragraphs
        if let Ok(p_selector) = Selector::parse("p") {
            for paragraph in document.select(&p_selector) {
                let text = paragraph.text().collect::<String>();
                if !text.trim().is_empty() {
                    chunks.push(ContentChunk::new(text, ChunkType::Paragraph));
                }
            }
        }

        // Extract code blocks
        if let Ok(code_selector) = Selector::parse("pre code, code") {
            for code in document.select(&code_selector) {
                let text = code.text().collect::<String>();
                if !text.trim().is_empty() {
                    let mut chunk = ContentChunk::new(text, ChunkType::CodeBlock);
                    if let Some(class) = code.value().attr("class") {
                        chunk.metadata.insert(
                            "class".to_string(),
                            serde_json::Value::String(class.to_string()),
                        );
                    }
                    chunks.push(chunk);
                }
            }
        }

        // Extract tables
        if let Ok(table_selector) = Selector::parse("table") {
            for table in document.select(&table_selector) {
                let text = table.text().collect::<Vec<_>>().join(" ");
                if !text.trim().is_empty() {
                    chunks.push(ContentChunk::new(text, ChunkType::Table));
                }
            }
        }

        chunks
    }
}

impl Default for HtmlProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for HtmlProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing HTML document ({} bytes)", input.len());

        let html_content = String::from_utf8_lossy(input).to_string();

        // Parse HTML
        let document = Html::parse_document(&html_content);

        // Extract plain text
        let text_content = html2text::from_read(input, 80)
            .map_err(|e| CortexError::ingestion(format!("Failed to extract text from HTML: {}", e)))?;

        let mut metadata = HashMap::new();
        if self.extract_metadata {
            metadata = self.extract_metadata(&document);
        }

        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("html".to_string()),
        );

        let chunks = if self.preserve_structure {
            self.extract_chunks(&document)
        } else {
            vec![ContentChunk::new(text_content.clone(), ChunkType::Document)]
        };

        let mut result = ProcessedContent::new(ContentType::Html, text_content).with_chunks(chunks);
        result.metadata = metadata;

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["html", "htm"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["text/html"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Html
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_html_processor() {
        let processor = HtmlProcessor::new();
        let content = b"<html><head><title>Test</title></head><body><h1>Hello</h1><p>World</p></body></html>";

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Html);
        assert!(result.metadata.contains_key("title"));
    }

    #[tokio::test]
    async fn test_html_structure_extraction() {
        let processor = HtmlProcessor::new();
        let content = b"<html><body><h1>Title</h1><p>Para 1</p><p>Para 2</p></body></html>";

        let result = processor.process(content).await.unwrap();
        assert!(!result.chunks.is_empty());
    }

    #[test]
    fn test_html_extensions() {
        let processor = HtmlProcessor::new();
        assert!(processor.supported_extensions().contains(&"html"));
    }
}
