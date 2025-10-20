//! Resource content representation.
//!
//! This module provides the [`ResourceContent`] enum for representing resource
//! content as either text or binary data (blob).

use serde::{Deserialize, Serialize};

/// Resource content returned by resource read operations.
///
/// Resources can return either text content (UTF-8 encoded strings) or
/// binary content (arbitrary byte arrays).
///
/// # Examples
///
/// ## Text Content
///
/// ```rust
/// use mcp_server::resource::ResourceContent;
///
/// let content = ResourceContent::text("Hello, World!", "text/plain");
///
/// assert!(content.is_text());
/// assert_eq!(content.as_text(), Some("Hello, World!"));
/// assert_eq!(content.mime_type(), "text/plain");
/// ```
///
/// ## Binary Content
///
/// ```rust
/// use mcp_server::resource::ResourceContent;
///
/// let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
/// let content = ResourceContent::blob(data.clone(), "image/png");
///
/// assert!(content.is_blob());
/// assert_eq!(content.as_blob(), Some(&data[..]));
/// assert_eq!(content.mime_type(), "image/png");
/// ```
///
/// ## JSON Content
///
/// ```rust
/// use mcp_server::resource::ResourceContent;
/// use serde_json::json;
///
/// let json_str = json!({"name": "Alice", "age": 30}).to_string();
/// let content = ResourceContent::text(json_str, "application/json");
///
/// assert_eq!(content.mime_type(), "application/json");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceContent {
    /// Text content with UTF-8 encoding.
    Text {
        /// The text content as a UTF-8 string
        text: String,
        /// MIME type (e.g., "text/plain", "text/markdown", "application/json")
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Binary content (arbitrary bytes).
    Blob {
        /// The binary data
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        /// MIME type (e.g., "image/png", "application/octet-stream")
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl ResourceContent {
    /// Creates a new text resource content.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content
    /// * `mime_type` - The MIME type (e.g., "text/plain", "application/json")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let content = ResourceContent::text("# Title\n\nContent", "text/markdown");
    /// assert!(content.is_text());
    /// ```
    pub fn text(text: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: mime_type.into(),
        }
    }

    /// Creates a new binary (blob) resource content.
    ///
    /// # Arguments
    ///
    /// * `data` - The binary data
    /// * `mime_type` - The MIME type (e.g., "image/png", "application/pdf")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG magic
    /// let content = ResourceContent::blob(data, "image/png");
    /// assert!(content.is_blob());
    /// ```
    pub fn blob(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self::Blob {
            data,
            mime_type: mime_type.into(),
        }
    }

    /// Returns `true` if this is text content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert!(text.is_text());
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert!(!blob.is_text());
    /// ```
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Returns `true` if this is binary (blob) content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert!(blob.is_blob());
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert!(!text.is_blob());
    /// ```
    pub fn is_blob(&self) -> bool {
        matches!(self, Self::Blob { .. })
    }

    /// Returns the MIME type of this content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert_eq!(text.mime_type(), "text/plain");
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "image/jpeg");
    /// assert_eq!(blob.mime_type(), "image/jpeg");
    /// ```
    pub fn mime_type(&self) -> &str {
        match self {
            Self::Text { mime_type, .. } => mime_type,
            Self::Blob { mime_type, .. } => mime_type,
        }
    }

    /// Returns the text content if this is a text variant.
    ///
    /// # Returns
    ///
    /// - `Some(&str)` if this is text content
    /// - `None` if this is binary content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert_eq!(text.as_text(), Some("hello"));
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert_eq!(blob.as_text(), None);
    /// ```
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            Self::Blob { .. } => None,
        }
    }

    /// Returns the binary data if this is a blob variant.
    ///
    /// # Returns
    ///
    /// - `Some(&[u8])` if this is binary content
    /// - `None` if this is text content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert_eq!(blob.as_blob(), Some(&[1, 2, 3][..]));
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert_eq!(text.as_blob(), None);
    /// ```
    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Self::Blob { data, .. } => Some(data),
            Self::Text { .. } => None,
        }
    }

    /// Converts this content into owned text if it's a text variant.
    ///
    /// # Returns
    ///
    /// - `Some(String)` if this is text content
    /// - `None` if this is binary content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert_eq!(text.into_text(), Some("hello".to_string()));
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert_eq!(blob.into_text(), None);
    /// ```
    pub fn into_text(self) -> Option<String> {
        match self {
            Self::Text { text, .. } => Some(text),
            Self::Blob { .. } => None,
        }
    }

    /// Converts this content into owned binary data if it's a blob variant.
    ///
    /// # Returns
    ///
    /// - `Some(Vec<u8>)` if this is binary content
    /// - `None` if this is text content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContent;
    ///
    /// let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
    /// assert_eq!(blob.into_blob(), Some(vec![1, 2, 3]));
    ///
    /// let text = ResourceContent::text("hello", "text/plain");
    /// assert_eq!(text.into_blob(), None);
    /// ```
    pub fn into_blob(self) -> Option<Vec<u8>> {
        match self {
            Self::Blob { data, .. } => Some(data),
            Self::Text { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_creation() {
        let content = ResourceContent::text("Hello", "text/plain");
        assert!(content.is_text());
        assert!(!content.is_blob());
    }

    #[test]
    fn test_blob_creation() {
        let content = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
        assert!(content.is_blob());
        assert!(!content.is_text());
    }

    #[test]
    fn test_text_mime_type() {
        let content = ResourceContent::text("Hello", "text/plain");
        assert_eq!(content.mime_type(), "text/plain");
    }

    #[test]
    fn test_blob_mime_type() {
        let content = ResourceContent::blob(vec![1, 2, 3], "image/png");
        assert_eq!(content.mime_type(), "image/png");
    }

    #[test]
    fn test_as_text() {
        let text = ResourceContent::text("Hello", "text/plain");
        assert_eq!(text.as_text(), Some("Hello"));

        let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
        assert_eq!(blob.as_text(), None);
    }

    #[test]
    fn test_as_blob() {
        let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
        assert_eq!(blob.as_blob(), Some(&[1, 2, 3][..]));

        let text = ResourceContent::text("Hello", "text/plain");
        assert_eq!(text.as_blob(), None);
    }

    #[test]
    fn test_into_text() {
        let text = ResourceContent::text("Hello", "text/plain");
        assert_eq!(text.into_text(), Some("Hello".to_string()));
    }

    #[test]
    fn test_into_blob() {
        let blob = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
        assert_eq!(blob.into_blob(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_json_content() {
        let json_str = r#"{"name":"Alice","age":30}"#;
        let content = ResourceContent::text(json_str, "application/json");

        assert_eq!(content.mime_type(), "application/json");
        assert_eq!(content.as_text(), Some(json_str));
    }

    #[test]
    fn test_markdown_content() {
        let markdown = "# Title\n\n## Subtitle\n\nContent here.";
        let content = ResourceContent::text(markdown, "text/markdown");

        assert_eq!(content.mime_type(), "text/markdown");
        assert_eq!(content.as_text(), Some(markdown));
    }

    #[test]
    fn test_png_header() {
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let content = ResourceContent::blob(png_header.clone(), "image/png");

        assert_eq!(content.mime_type(), "image/png");
        assert_eq!(content.as_blob(), Some(&png_header[..]));
    }

    #[test]
    fn test_serialization_text() {
        let content = ResourceContent::text("Hello", "text/plain");
        let json = serde_json::to_string(&content).unwrap();

        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
        assert!(json.contains("\"mimeType\":\"text/plain\""));
    }

    #[test]
    fn test_deserialization_text() {
        let json = r#"{"type":"text","text":"Hello","mimeType":"text/plain"}"#;
        let content: ResourceContent = serde_json::from_str(json).unwrap();

        assert!(content.is_text());
        assert_eq!(content.as_text(), Some("Hello"));
        assert_eq!(content.mime_type(), "text/plain");
    }

    #[test]
    fn test_serialization_blob() {
        let content = ResourceContent::blob(vec![1, 2, 3], "application/octet-stream");
        let json = serde_json::to_string(&content).unwrap();

        assert!(json.contains("\"type\":\"blob\""));
        assert!(json.contains("\"mimeType\":\"application/octet-stream\""));
    }

    #[test]
    fn test_clone() {
        let content = ResourceContent::text("Hello", "text/plain");
        let cloned = content.clone();

        assert_eq!(content.as_text(), cloned.as_text());
        assert_eq!(content.mime_type(), cloned.mime_type());
    }

    #[test]
    fn test_equality() {
        let content1 = ResourceContent::text("Hello", "text/plain");
        let content2 = ResourceContent::text("Hello", "text/plain");
        let content3 = ResourceContent::text("World", "text/plain");

        assert_eq!(content1, content2);
        assert_ne!(content1, content3);
    }

    #[test]
    fn test_empty_text() {
        let content = ResourceContent::text("", "text/plain");
        assert_eq!(content.as_text(), Some(""));
    }

    #[test]
    fn test_empty_blob() {
        let content = ResourceContent::blob(vec![], "application/octet-stream");
        assert_eq!(content.as_blob(), Some(&[][..]));
    }
}
