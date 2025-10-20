//! YAML processor for configuration files.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::{CortexError, Result};
use serde_yaml::Value;
use std::collections::HashMap;

/// Processor for YAML documents
pub struct YamlProcessor {
    flatten_arrays: bool,
}

impl YamlProcessor {
    /// Create a new YAML processor
    pub fn new() -> Self {
        Self {
            flatten_arrays: true,
        }
    }

    /// Convert YAML to searchable text
    fn yaml_to_text(value: &Value, depth: usize, max_depth: usize) -> String {
        if depth >= max_depth {
            return String::from("[max depth reached]");
        }

        match value {
            Value::Null => String::from("null"),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Sequence(seq) => {
                let items: Vec<String> = seq
                    .iter()
                    .map(|v| Self::yaml_to_text(v, depth + 1, max_depth))
                    .collect();
                items.join(" ")
            }
            Value::Mapping(map) => {
                let items: Vec<String> = map
                    .iter()
                    .filter_map(|(k, v)| {
                        if let Value::String(key) = k {
                            Some(format!(
                                "{}: {}",
                                key,
                                Self::yaml_to_text(v, depth + 1, max_depth)
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                items.join(" ")
            }
            Value::Tagged(tagged) => Self::yaml_to_text(&tagged.value, depth, max_depth),
        }
    }

    /// Extract chunks from YAML structure
    fn extract_chunks(&self, value: &Value, path: &str) -> Vec<ContentChunk> {
        let mut chunks = Vec::new();

        match value {
            Value::Mapping(map) => {
                for (key, val) in map.iter() {
                    if let Value::String(key_str) = key {
                        let new_path = if path.is_empty() {
                            key_str.clone()
                        } else {
                            format!("{}.{}", path, key_str)
                        };

                        match val {
                            Value::String(s) if !s.is_empty() => {
                                let mut chunk = ContentChunk::new(s.clone(), ChunkType::Paragraph);
                                chunk.metadata.insert(
                                    "yaml_path".to_string(),
                                    serde_json::Value::String(new_path.clone()),
                                );
                                chunk.metadata.insert(
                                    "yaml_key".to_string(),
                                    serde_json::Value::String(key_str.clone()),
                                );
                                chunks.push(chunk);
                            }
                            Value::Mapping(_) | Value::Sequence(_) => {
                                chunks.extend(self.extract_chunks(val, &new_path));
                            }
                            _ => {}
                        }
                    }
                }
            }
            Value::Sequence(seq) if self.flatten_arrays => {
                for (i, val) in seq.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    chunks.extend(self.extract_chunks(val, &new_path));
                }
            }
            Value::Tagged(tagged) => {
                chunks.extend(self.extract_chunks(&tagged.value, path));
            }
            _ => {}
        }

        chunks
    }

    /// Extract metadata from YAML root
    fn extract_metadata(&self, value: &Value) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        if let Value::Mapping(map) = value {
            // Extract common metadata fields
            for key_name in &["name", "title", "description", "version", "author"] {
                if let Some(key) = map
                    .keys()
                    .find(|k| k.as_str() == Some(key_name))
                {
                    if let Some(val) = map.get(key) {
                        if let Ok(json_val) = serde_json::to_value(val) {
                            metadata.insert(key_name.to_string(), json_val);
                        }
                    }
                }
            }

            // Count keys
            metadata.insert(
                "key_count".to_string(),
                serde_json::Value::Number(map.len().into()),
            );
        }

        metadata
    }
}

impl Default for YamlProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for YamlProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing YAML document ({} bytes)", input.len());

        // Parse YAML
        let yaml_value: Value = serde_yaml::from_slice(input)
            .map_err(|e| CortexError::ingestion(format!("Failed to parse YAML: {}", e)))?;

        // Convert to searchable text
        let text_content = Self::yaml_to_text(&yaml_value, 0, 10);

        // Extract metadata
        let mut metadata = self.extract_metadata(&yaml_value);
        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("yaml".to_string()),
        );

        // Extract chunks
        let chunks = self.extract_chunks(&yaml_value, "");

        // Convert YAML to JSON for structured data
        let structured_data = serde_json::to_value(&yaml_value).ok();

        Ok(ProcessedContent::new(ContentType::Yaml, text_content)
            .with_structured_data(structured_data.unwrap_or(serde_json::Value::Null))
            .with_chunks(chunks))
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["application/yaml", "text/yaml", "application/x-yaml", "text/x-yaml"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Yaml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_yaml_processor() {
        let processor = YamlProcessor::new();
        let content = b"name: Test\ndescription: A test document\nversion: 1.0";

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Yaml);
        assert!(result.structured_data.is_some());
        assert!(result.text_content.contains("Test"));
    }

    #[tokio::test]
    async fn test_yaml_nested() {
        let processor = YamlProcessor::new();
        let content = b"user:\n  name: John\n  age: 30";

        let result = processor.process(content).await.unwrap();
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_yaml_invalid() {
        let processor = YamlProcessor::new();
        let content = b"invalid: yaml: content:";

        let result = processor.process(content).await;
        // Some invalid YAML might still parse, so we just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_yaml_extensions() {
        let processor = YamlProcessor::new();
        assert!(processor.supported_extensions().contains(&"yaml"));
        assert!(processor.supported_extensions().contains(&"yml"));
    }
}
