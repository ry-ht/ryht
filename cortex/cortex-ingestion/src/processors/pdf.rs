//! PDF document processor.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::{CortexError, Result};
use pdf_extract::extract_text_from_mem;
use lopdf::Document as PdfDocument;
use std::collections::HashMap;

/// Processor for PDF documents
pub struct PdfProcessor {
    chunk_by_page: bool,
}

impl PdfProcessor {
    /// Create a new PDF processor
    pub fn new() -> Self {
        Self {
            chunk_by_page: true,
        }
    }

    /// Create a PDF processor that doesn't chunk by page
    pub fn without_page_chunks() -> Self {
        Self {
            chunk_by_page: false,
        }
    }

    /// Extract metadata from PDF
    fn extract_metadata(pdf: &PdfDocument) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        // Get document info dictionary
        if let Ok(info_obj) = pdf.trailer.get(b"Info").and_then(|obj| pdf.dereference(obj)) {
            if let Ok(dict) = info_obj.1.as_dict() {
                // Extract common metadata fields
                for (key, value) in dict.iter() {
                    let key_str = String::from_utf8_lossy(key);
                    if let Ok(value_bytes) = value.as_str() {
                        let value_decoded = String::from_utf8_lossy(value_bytes);
                        metadata.insert(
                            key_str.to_string(),
                            serde_json::Value::String(value_decoded.to_string()),
                        );
                    }
                }
            }
        }

        // Add page count
        let pages = pdf.get_pages();
        metadata.insert(
            "page_count".to_string(),
            serde_json::Value::Number(pages.len().into()),
        );

        // Extract PDF version (stored as string in lopdf)
        let version_str = format!("{:?}", pdf.version);
        metadata.insert(
            "pdf_version".to_string(),
            serde_json::Value::String(version_str),
        );

        // Check if PDF is encrypted
        metadata.insert(
            "encrypted".to_string(),
            serde_json::Value::Bool(pdf.is_encrypted()),
        );

        metadata
    }

    /// Detect document structure (headings, sections)
    fn detect_structure(text: &str) -> Vec<String> {
        let mut structure = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        for line in lines {
            let trimmed = line.trim();
            // Detect potential headings (short lines, uppercase, or followed by blank line)
            if !trimmed.is_empty()
                && trimmed.len() < 100
                && (trimmed.chars().filter(|c| c.is_uppercase()).count() as f32 / trimmed.len() as f32) > 0.5
            {
                structure.push(trimmed.to_string());
            }
        }

        structure
    }

    /// Extract images metadata from PDF
    fn extract_images_metadata(pdf: &PdfDocument) -> Vec<HashMap<String, serde_json::Value>> {
        let mut images = Vec::new();
        let pages = pdf.get_pages();

        for (page_num, page_id) in pages.iter() {
            if let Ok(page_dict) = pdf.get_object(*page_id).and_then(|obj| obj.as_dict()) {
                if let Ok(resources) = page_dict.get(b"Resources").and_then(|r| pdf.dereference(r)) {
                    if let Ok(resources_dict) = resources.1.as_dict() {
                        if let Ok(xobject) = resources_dict.get(b"XObject").and_then(|x| pdf.dereference(x)) {
                            if let Ok(xobject_dict) = xobject.1.as_dict() {
                                for (name, _obj_ref) in xobject_dict.iter() {
                                    let mut img_meta = HashMap::new();
                                    img_meta.insert(
                                        "page".to_string(),
                                        serde_json::Value::Number((*page_num).into()),
                                    );
                                    img_meta.insert(
                                        "name".to_string(),
                                        serde_json::Value::String(String::from_utf8_lossy(name).to_string()),
                                    );
                                    images.push(img_meta);
                                }
                            }
                        }
                    }
                }
            }
        }

        images
    }

    /// Extract text per page using lopdf
    fn extract_pages(pdf: &PdfDocument) -> Result<Vec<(u32, String)>> {
        let pages = pdf.get_pages();

        let mut page_texts = Vec::new();

        for (page_num, _page_id) in pages.iter() {
            // Try to extract text from the page
            // Note: lopdf's text extraction is basic; for production consider using pdf-extract
            // or a more sophisticated library
            let text = pdf
                .extract_text(&[*page_num])
                .unwrap_or_else(|_| String::new());

            page_texts.push((*page_num, text));
        }

        Ok(page_texts)
    }
}

impl Default for PdfProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for PdfProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing PDF document ({} bytes)", input.len());

        // Extract full text using pdf-extract (more robust)
        let text_content = extract_text_from_mem(input)
            .map_err(|e| CortexError::ingestion(format!("Failed to extract PDF text: {}", e)))?;

        // Parse PDF for metadata using lopdf
        let pdf = PdfDocument::load_mem(input)
            .map_err(|e| CortexError::ingestion(format!("Failed to load PDF: {}", e)))?;

        let mut metadata = Self::extract_metadata(&pdf);

        // Extract image metadata
        let images = Self::extract_images_metadata(&pdf);
        if !images.is_empty() {
            metadata.insert(
                "images".to_string(),
                serde_json::Value::Array(
                    images.iter().map(|img| serde_json::to_value(img).unwrap_or(serde_json::Value::Null)).collect()
                ),
            );
            metadata.insert(
                "image_count".to_string(),
                serde_json::Value::Number(images.len().into()),
            );
        }

        // Detect structure
        let structure = Self::detect_structure(&text_content);
        if !structure.is_empty() {
            metadata.insert(
                "detected_headings".to_string(),
                serde_json::Value::Array(
                    structure.iter().map(|s| serde_json::Value::String(s.clone())).collect()
                ),
            );
        }

        let mut chunks = Vec::new();

        if self.chunk_by_page {
            // Extract text per page for chunking
            match Self::extract_pages(&pdf) {
                Ok(pages) => {
                    for (page_num, page_text) in pages {
                        if !page_text.trim().is_empty() {
                            let chunk = ContentChunk::new(page_text, ChunkType::Page)
                                .with_metadata(
                                    "page_number".to_string(),
                                    serde_json::Value::Number(page_num.into()),
                                );
                            chunks.push(chunk);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to extract pages, using full text: {}", e);
                    // Fall back to full document as single chunk
                    chunks.push(ContentChunk::new(text_content.clone(), ChunkType::Document));
                }
            }
        } else {
            // Single chunk for entire document
            chunks.push(ContentChunk::new(text_content.clone(), ChunkType::Document));
        }

        let mut result = ProcessedContent::new(ContentType::Pdf, text_content).with_chunks(chunks);
        metadata.insert("format".to_string(), serde_json::Value::String("pdf".to_string()));
        result.metadata = metadata;
        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["pdf"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["application/pdf"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Pdf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pdf_processor_empty() {
        let processor = PdfProcessor::new();

        // Empty PDF should fail gracefully
        let result = processor.process(b"").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pdf_processor_extensions() {
        let processor = PdfProcessor::new();
        assert_eq!(processor.supported_extensions(), vec!["pdf"]);
        assert_eq!(processor.supported_mime_types(), vec!["application/pdf"]);
        assert_eq!(processor.content_type(), ContentType::Pdf);
    }
}
