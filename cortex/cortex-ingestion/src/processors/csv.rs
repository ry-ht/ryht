//! CSV processor for tabular data extraction.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::{CortexError, Result};
use csv::ReaderBuilder;
use std::collections::HashMap;

/// Processor for CSV documents
pub struct CsvProcessor {
    has_headers: bool,
    chunk_by_row: bool,
}

impl CsvProcessor {
    /// Create a new CSV processor
    pub fn new() -> Self {
        Self {
            has_headers: true,
            chunk_by_row: false,
        }
    }

    /// Create CSV processor that chunks by row
    pub fn with_row_chunks() -> Self {
        Self {
            has_headers: true,
            chunk_by_row: true,
        }
    }

    /// Parse CSV and extract data
    fn parse_csv(&self, input: &[u8]) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let mut reader = ReaderBuilder::new()
            .has_headers(self.has_headers)
            .from_reader(input);

        let headers = if self.has_headers {
            reader
                .headers()
                .map_err(|e| CortexError::ingestion(format!("Failed to read CSV headers: {}", e)))?
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result
                .map_err(|e| CortexError::ingestion(format!("Failed to read CSV record: {}", e)))?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            rows.push(row);
        }

        Ok((headers, rows))
    }

    /// Convert CSV to text representation
    fn csv_to_text(&self, headers: &[String], rows: &[Vec<String>]) -> String {
        let mut text = String::new();

        // Add headers
        if !headers.is_empty() {
            text.push_str(&headers.join(", "));
            text.push('\n');
        }

        // Add rows
        for row in rows {
            text.push_str(&row.join(", "));
            text.push('\n');
        }

        text
    }

    /// Convert CSV to JSON structure
    fn csv_to_json(&self, headers: &[String], rows: &[Vec<String>]) -> serde_json::Value {
        let mut json_rows = Vec::new();

        for row in rows {
            let mut json_row = serde_json::Map::new();

            if headers.is_empty() {
                // No headers, use column indices
                for (i, value) in row.iter().enumerate() {
                    json_row.insert(format!("col_{}", i), serde_json::Value::String(value.clone()));
                }
            } else {
                // Use headers as keys
                for (i, value) in row.iter().enumerate() {
                    if let Some(header) = headers.get(i) {
                        json_row.insert(header.clone(), serde_json::Value::String(value.clone()));
                    }
                }
            }

            json_rows.push(serde_json::Value::Object(json_row));
        }

        serde_json::Value::Array(json_rows)
    }

    /// Extract chunks from CSV data
    fn extract_chunks(&self, headers: &[String], rows: &[Vec<String>]) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        if self.chunk_by_row {
            // Create a chunk per row
            for (i, row) in rows.iter().enumerate() {
                let row_text = if headers.is_empty() {
                    row.join(", ")
                } else {
                    // Create key-value pairs
                    headers
                        .iter()
                        .zip(row.iter())
                        .map(|(h, v)| format!("{}: {}", h, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let mut chunk = ContentChunk::new(row_text, ChunkType::Paragraph);
                chunk.metadata.insert(
                    "row_number".to_string(),
                    serde_json::Value::Number(i.into()),
                );
                chunks.push(chunk);
            }
        } else {
            // Single chunk for entire table
            let table_text = self.csv_to_text(headers, rows);
            chunks.push(ContentChunk::new(table_text, ChunkType::Table));
        }

        chunks
    }
}

impl Default for CsvProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for CsvProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing CSV document ({} bytes)", input.len());

        // Parse CSV
        let (headers, rows) = self.parse_csv(input)?;

        // Convert to text
        let text_content = self.csv_to_text(&headers, &rows);

        // Convert to JSON structure
        let structured_data = self.csv_to_json(&headers, &rows);

        // Extract metadata
        let mut metadata = HashMap::new();
        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("csv".to_string()),
        );
        metadata.insert(
            "row_count".to_string(),
            serde_json::Value::Number(rows.len().into()),
        );
        metadata.insert(
            "column_count".to_string(),
            serde_json::Value::Number(headers.len().into()),
        );
        if !headers.is_empty() {
            metadata.insert(
                "headers".to_string(),
                serde_json::Value::Array(
                    headers
                        .iter()
                        .map(|h| serde_json::Value::String(h.clone()))
                        .collect(),
                ),
            );
        }

        // Extract chunks
        let chunks = self.extract_chunks(&headers, &rows);

        let mut result = ProcessedContent::new(ContentType::Csv, text_content)
            .with_structured_data(structured_data)
            .with_chunks(chunks);
        result.metadata = metadata;

        Ok(result)
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["csv"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["text/csv"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_csv_processor() {
        let processor = CsvProcessor::new();
        let content = b"name,age,city\nJohn,30,NYC\nJane,25,LA";

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Csv);
        assert!(result.structured_data.is_some());
        assert!(result.text_content.contains("John"));
    }

    #[tokio::test]
    async fn test_csv_no_headers() {
        let mut processor = CsvProcessor::new();
        processor.has_headers = false;
        let content = b"John,30,NYC\nJane,25,LA";

        let result = processor.process(content).await.unwrap();
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_csv_row_chunks() {
        let processor = CsvProcessor::with_row_chunks();
        let content = b"name,age\nJohn,30\nJane,25";

        let result = processor.process(content).await.unwrap();
        assert!(result.chunks.len() >= 2);
    }

    #[test]
    fn test_csv_extensions() {
        let processor = CsvProcessor::new();
        assert!(processor.supported_extensions().contains(&"csv"));
    }
}
