//! Metadata extraction and management utilities.

use std::collections::HashMap;

/// Builder for creating metadata maps.
#[derive(Debug, Default)]
pub struct MetadataBuilder {
    metadata: HashMap<String, String>,
}

impl MetadataBuilder {
    /// Create a new metadata builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key-value pair
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a key-value pair if the value is Some
    pub fn add_option(mut self, key: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        if let Some(v) = value {
            self.metadata.insert(key.into(), v.into());
        }
        self
    }

    /// Build the metadata map
    pub fn build(self) -> HashMap<String, String> {
        self.metadata
    }
}

/// Extract common metadata from various sources
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Extract metadata from file path
    pub fn from_path(path: &std::path::Path) -> HashMap<String, String> {
        let mut builder = MetadataBuilder::new();

        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                builder = builder.add("extension", ext_str);
            }
        }

        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                builder = builder.add("filename", name_str);
            }
        }

        builder.build()
    }

    /// Extract metadata from content
    pub fn from_content(content: &str) -> HashMap<String, String> {
        MetadataBuilder::new()
            .add("length", content.len().to_string())
            .add("lines", content.lines().count().to_string())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_builder() {
        let metadata = MetadataBuilder::new()
            .add("key1", "value1")
            .add("key2", "value2")
            .build();

        assert_eq!(metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_metadata_from_path() {
        let path = std::path::Path::new("/test/file.rs");
        let metadata = MetadataExtractor::from_path(path);

        assert_eq!(metadata.get("extension"), Some(&"rs".to_string()));
        assert_eq!(metadata.get("filename"), Some(&"file.rs".to_string()));
    }
}
