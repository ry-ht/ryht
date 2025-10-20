//! JSON processor for structured data extraction.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::{CortexError, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Processor for JSON documents
pub struct JsonProcessor {
    flatten_arrays: bool,
    max_depth: usize,
}

impl JsonProcessor {
    /// Create a new JSON processor
    pub fn new() -> Self {
        Self {
            flatten_arrays: true,
            max_depth: 10,
        }
    }

    /// Convert JSON to searchable text
    fn json_to_text(value: &Value, depth: usize, max_depth: usize) -> String {
        if depth >= max_depth {
            return String::from("[max depth reached]");
        }

        match value {
            Value::Null => String::from("null"),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| Self::json_to_text(v, depth + 1, max_depth))
                    .collect();
                items.join(" ")
            }
            Value::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        format!("{}: {}", k, Self::json_to_text(v, depth + 1, max_depth))
                    })
                    .collect();
                items.join(" ")
            }
        }
    }

    /// Extract chunks from JSON structure
    fn extract_chunks(&self, value: &Value, path: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        match value {
            Value::Object(obj) => {
                for (key, val) in obj.iter() {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    match val {
                        Value::String(s) if !s.is_empty() => {
                            let mut chunk = ContentChunk::new(s.clone(), ChunkType::Paragraph);
                            chunk.metadata.insert(
                                "json_path".to_string(),
                                serde_json::Value::String(new_path.clone()),
                            );
                            chunk.metadata.insert(
                                "json_key".to_string(),
                                serde_json::Value::String(key.clone()),
                            );
                            chunks.push(chunk);
                        }
                        Value::Object(_) | Value::Array(_) => {
                            chunks.extend(self.extract_chunks(val, &new_path));
                        }
                        _ => {}
                    }
                }
            }
            Value::Array(arr) if self.flatten_arrays => {
                for (i, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    chunks.extend(self.extract_chunks(val, &new_path));
                }
            }
            _ => {}
        }

        chunks
    }

    /// Extract metadata from JSON root
    fn extract_metadata(&self, value: &Value) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        if let Value::Object(obj) = value {
            // Extract common metadata fields if they exist at root level
            for key in &["title", "name", "description", "version", "author", "date"] {
                if let Some(val) = obj.get(*key) {
                    metadata.insert(key.to_string(), val.clone());
                }
            }

            // Count keys
            metadata.insert(
                "key_count".to_string(),
                serde_json::Value::Number(obj.len().into()),
            );
        }

        metadata
    }
}

impl Default for JsonProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for JsonProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing JSON document ({} bytes)", input.len());

        // Parse JSON
        let json_value: Value = serde_json::from_slice(input)
            .map_err(|e| CortexError::ingestion(format!("Failed to parse JSON: {}", e)))?;

        // Convert to searchable text
        let text_content = Self::json_to_text(&json_value, 0, self.max_depth);

        // Extract metadata
        let mut metadata = self.extract_metadata(&json_value);
        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("json".to_string()),
        );

        // Extract chunks
        let chunks = self.extract_chunks(&json_value, "");

        Ok(ProcessedContent::new(ContentType::Json, text_content)
            .with_structured_data(json_value)
            .with_chunks(chunks))
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["json"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["application/json"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_processor() {
        let processor = JsonProcessor::new();
        let content = br#"{"title": "Test", "description": "A test document"}"#;

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Json);
        assert!(result.structured_data.is_some());
        assert!(result.text_content.contains("Test"));
    }

    #[tokio::test]
    async fn test_json_nested() {
        let processor = JsonProcessor::new();
        let content = br#"{"user": {"name": "John", "age": 30}}"#;

        let result = processor.process(content).await.unwrap();
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_json_invalid() {
        let processor = JsonProcessor::new();
        let content = b"invalid json";

        let result = processor.process(content).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_json_extensions() {
        let processor = JsonProcessor::new();
        assert!(processor.supported_extensions().contains(&"json"));
    }
}
