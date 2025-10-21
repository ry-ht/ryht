//! Tool result types.
//!
//! This module provides the result types returned by tool execution, including
//! content types and helper methods for constructing results.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Result of a tool execution.
///
/// Contains the output content and an optional error flag. Tools should return
/// this type from their `execute` method.
///
/// # Examples
///
/// ```
/// use mcp_server::tool::{ToolResult, ToolContent};
///
/// // Success result with text content
/// let result = ToolResult::success_text("Operation completed");
/// assert!(!result.is_error());
///
/// // Error result
/// let result = ToolResult::error("Something went wrong");
/// assert!(result.is_error());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// The content returned by the tool
    pub content: Vec<ToolContent>,

    /// Whether this result represents an error
    #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Creates a new successful result with the given content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{ToolResult, ToolContent};
    ///
    /// let content = vec![ToolContent::text("Success!")];
    /// let result = ToolResult::success(content);
    /// assert!(!result.is_error());
    /// ```
    pub fn success(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }

    /// Creates a successful result with a single text content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolResult;
    ///
    /// let result = ToolResult::success_text("Operation completed");
    /// assert_eq!(result.content.len(), 1);
    /// assert!(!result.is_error());
    /// ```
    pub fn success_text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(text)],
            is_error: None,
        }
    }

    /// Creates a successful result with a JSON value.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolResult;
    /// use serde_json::json;
    ///
    /// let result = ToolResult::success_json(json!({"status": "ok"}));
    /// assert!(!result.is_error());
    /// ```
    pub fn success_json(value: Value) -> Self {
        Self {
            content: vec![ToolContent::text(value.to_string())],
            is_error: None,
        }
    }

    /// Creates an error result with the given content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{ToolResult, ToolContent};
    ///
    /// let content = vec![ToolContent::text("Error occurred")];
    /// let result = ToolResult::error_with_content(content);
    /// assert!(result.is_error());
    /// ```
    pub fn error_with_content(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: Some(true),
        }
    }

    /// Creates an error result with a single text message.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolResult;
    ///
    /// let result = ToolResult::error("Something went wrong");
    /// assert!(result.is_error());
    /// ```
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: Some(true),
        }
    }

    /// Returns whether this result represents an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolResult;
    ///
    /// let success = ToolResult::success_text("OK");
    /// assert!(!success.is_error());
    ///
    /// let error = ToolResult::error("Failed");
    /// assert!(error.is_error());
    /// ```
    pub fn is_error(&self) -> bool {
        self.is_error.unwrap_or(false)
    }

    /// Returns whether this result represents success.
    pub fn is_success(&self) -> bool {
        !self.is_error()
    }
}

/// Content returned by a tool.
///
/// Tools can return different types of content including text, images, and resource references.
///
/// # Examples
///
/// ```
/// use mcp_server::tool::ToolContent;
///
/// // Text content
/// let text = ToolContent::text("Hello, world!");
///
/// // Image content
/// let image = ToolContent::image("base64data", "image/png");
///
/// // Resource reference
/// let resource = ToolContent::resource("file:///path/to/file.txt");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolContent {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Image content
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource reference
    Resource {
        /// URI of the resource
        uri: String,
    },
}

impl ToolContent {
    /// Creates text content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContent;
    ///
    /// let content = ToolContent::text("Hello");
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Creates image content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContent;
    ///
    /// let content = ToolContent::image("base64data", "image/png");
    /// ```
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    /// Creates resource content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContent;
    ///
    /// let content = ToolContent::resource("file:///path/to/file.txt");
    /// ```
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource { uri: uri.into() }
    }

    /// Returns the text if this is text content.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContent;
    ///
    /// let content = ToolContent::text("Hello");
    /// assert_eq!(content.as_text(), Some("Hello"));
    ///
    /// let image = ToolContent::image("data", "image/png");
    /// assert_eq!(image.as_text(), None);
    /// ```
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Returns the image data and MIME type if this is image content.
    pub fn as_image(&self) -> Option<(&str, &str)> {
        match self {
            Self::Image { data, mime_type } => Some((data, mime_type)),
            _ => None,
        }
    }

    /// Returns the URI if this is resource content.
    pub fn as_resource(&self) -> Option<&str> {
        match self {
            Self::Resource { uri } => Some(uri),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_result_success() {
        let content = vec![ToolContent::text("Success")];
        let result = ToolResult::success(content);
        assert!(!result.is_error());
        assert!(result.is_success());
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_success_text() {
        let result = ToolResult::success_text("Operation completed");
        assert!(!result.is_error());
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].as_text(), Some("Operation completed"));
    }

    #[test]
    fn test_tool_result_success_json() {
        let result = ToolResult::success_json(json!({"status": "ok"}));
        assert!(!result.is_error());
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong");
        assert!(result.is_error());
        assert!(!result.is_success());
        assert_eq!(result.content[0].as_text(), Some("Something went wrong"));
    }

    #[test]
    fn test_tool_result_error_with_content() {
        let content = vec![
            ToolContent::text("Error:"),
            ToolContent::text("Details here"),
        ];
        let result = ToolResult::error_with_content(content);
        assert!(result.is_error());
        assert_eq!(result.content.len(), 2);
    }

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::text("Hello, world!");
        assert_eq!(content.as_text(), Some("Hello, world!"));
        assert!(content.as_image().is_none());
        assert!(content.as_resource().is_none());
    }

    #[test]
    fn test_tool_content_image() {
        let content = ToolContent::image("base64data", "image/png");
        assert_eq!(content.as_image(), Some(("base64data", "image/png")));
        assert!(content.as_text().is_none());
        assert!(content.as_resource().is_none());
    }

    #[test]
    fn test_tool_content_resource() {
        let content = ToolContent::resource("file:///path/to/file.txt");
        assert_eq!(content.as_resource(), Some("file:///path/to/file.txt"));
        assert!(content.as_text().is_none());
        assert!(content.as_image().is_none());
    }

    #[test]
    fn test_tool_content_serialization() {
        let content = ToolContent::text("test");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "test");

        let image = ToolContent::image("data", "image/png");
        let json = serde_json::to_value(&image).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["data"], "data");
        assert_eq!(json["mimeType"], "image/png");
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult::success_text("Success");
        let json = serde_json::to_value(&result).unwrap();
        assert!(json["content"].is_array());
        assert!(json["isError"].is_null());

        let result = ToolResult::error("Failed");
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["isError"], true);
    }

    #[test]
    fn test_tool_result_equality() {
        let result1 = ToolResult::success_text("test");
        let result2 = ToolResult::success_text("test");
        assert_eq!(result1, result2);

        let result3 = ToolResult::error("test");
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_tool_content_equality() {
        let content1 = ToolContent::text("test");
        let content2 = ToolContent::text("test");
        assert_eq!(content1, content2);

        let content3 = ToolContent::text("other");
        assert_ne!(content1, content3);
    }
}
