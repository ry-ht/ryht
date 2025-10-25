//! Message and content types for Claude Code SDK
//!
//! This module contains all message-related types used throughout the SDK,
//! including user messages, assistant messages, content blocks, and streaming messages.
//!
//! # Message Types
//!
//! - [`Message`] - Main message enum for chat messages
//! - [`UserMessage`] - User message content
//! - [`AssistantMessage`] - Assistant message content with content blocks
//!
//! # Content Types
//!
//! - [`ContentBlock`] - Enum for different content block types
//! - [`TextContent`] - Plain text content
//! - [`ThinkingContent`] - Extended thinking content
//! - [`ToolUseContent`] - Tool use request
//! - [`ToolResultContent`] - Tool execution result
//!
//! # Example
//!
//! ```rust
//! use cc_sdk::messages::{Message, UserMessage};
//!
//! let message = Message::User {
//!     message: UserMessage {
//!         content: "Hello, Claude!".to_string(),
//!     },
//! };
//! ```

use serde::{Deserialize, Serialize};

/// Main message type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Message {
    /// User message
    User {
        /// Message content
        message: UserMessage,
    },
    /// Assistant message
    Assistant {
        /// Message content
        message: AssistantMessage,
    },
    /// System message
    System {
        /// Subtype of system message
        subtype: String,
        /// Additional data
        data: serde_json::Value,
    },
    /// Result message indicating end of turn
    Result {
        /// Result subtype
        subtype: String,
        /// Duration in milliseconds
        duration_ms: i64,
        /// API duration in milliseconds
        duration_api_ms: i64,
        /// Whether an error occurred
        is_error: bool,
        /// Number of turns
        num_turns: i32,
        /// Session ID
        session_id: String,
        /// Total cost in USD
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
        /// Usage statistics
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        /// Result message
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<String>,
    },
}

/// User message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    /// Message content
    pub content: String,
}

impl UserMessage {
    /// Create a new user message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::UserMessage;
    ///
    /// let msg = UserMessage::new("Hello, Claude!");
    /// assert_eq!(msg.content, "Hello, Claude!");
    /// ```
    #[inline]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl From<String> for UserMessage {
    #[inline]
    fn from(content: String) -> Self {
        Self { content }
    }
}

impl From<&str> for UserMessage {
    #[inline]
    fn from(content: &str) -> Self {
        Self {
            content: content.to_string(),
        }
    }
}

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// Content blocks
    pub content: Vec<ContentBlock>,
}

impl AssistantMessage {
    /// Create a new assistant message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::{AssistantMessage, ContentBlock, TextContent};
    ///
    /// let msg = AssistantMessage::new(vec![
    ///     ContentBlock::Text(TextContent { text: "Hello!".to_string() })
    /// ]);
    /// assert_eq!(msg.content.len(), 1);
    /// ```
    #[inline]
    pub fn new(content: Vec<ContentBlock>) -> Self {
        Self { content }
    }

    /// Create a new assistant message with a single text block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::AssistantMessage;
    ///
    /// let msg = AssistantMessage::with_text("Hello!");
    /// assert_eq!(msg.content.len(), 1);
    /// ```
    #[inline]
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text(TextContent {
                text: text.into(),
            })],
        }
    }
}

impl Default for AssistantMessage {
    fn default() -> Self {
        Self {
            content: Vec::new(),
        }
    }
}

/// Result message (re-export for convenience)
pub use Message::Result as ResultMessage;
/// System message (re-export for convenience)
pub use Message::System as SystemMessage;

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlock {
    /// Text content
    Text(TextContent),
    /// Thinking content
    Thinking(ThinkingContent),
    /// Tool use request
    ToolUse(ToolUseContent),
    /// Tool result
    ToolResult(ToolResultContent),
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContent {
    /// Text content
    pub text: String,
}

impl TextContent {
    /// Create a new text content block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::TextContent;
    ///
    /// let content = TextContent::new("Hello, world!");
    /// assert_eq!(content.text, "Hello, world!");
    /// ```
    #[inline]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl From<String> for TextContent {
    #[inline]
    fn from(text: String) -> Self {
        Self { text }
    }
}

impl From<&str> for TextContent {
    #[inline]
    fn from(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingContent {
    /// Thinking content
    pub thinking: String,
    /// Signature
    pub signature: String,
}

impl ThinkingContent {
    /// Create a new thinking content block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::ThinkingContent;
    ///
    /// let content = ThinkingContent::new("Let me think...", "sig123");
    /// assert_eq!(content.thinking, "Let me think...");
    /// assert_eq!(content.signature, "sig123");
    /// ```
    #[inline]
    pub fn new(thinking: impl Into<String>, signature: impl Into<String>) -> Self {
        Self {
            thinking: thinking.into(),
            signature: signature.into(),
        }
    }
}

/// Tool use content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseContent {
    /// Tool use ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input parameters
    pub input: serde_json::Value,
}

impl ToolUseContent {
    /// Create a new tool use content block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::ToolUseContent;
    /// use serde_json::json;
    ///
    /// let content = ToolUseContent::new("tool_123", "Bash", json!({"command": "ls"}));
    /// assert_eq!(content.id, "tool_123");
    /// assert_eq!(content.name, "Bash");
    /// ```
    #[inline]
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

/// Tool result content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultContent {
    /// Tool use ID this result corresponds to
    pub tool_use_id: String,
    /// Result content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentValue>,
    /// Whether this is an error result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResultContent {
    /// Create a new tool result content block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::{ToolResultContent, ContentValue};
    ///
    /// let content = ToolResultContent::new("tool_123", Some(ContentValue::Text("Success".to_string())), false);
    /// assert_eq!(content.tool_use_id, "tool_123");
    /// assert_eq!(content.is_error, Some(false));
    /// ```
    #[inline]
    pub fn new(tool_use_id: impl Into<String>, content: Option<ContentValue>, is_error: bool) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content,
            is_error: Some(is_error),
        }
    }

    /// Create a successful tool result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::ToolResultContent;
    ///
    /// let result = ToolResultContent::success("tool_123", "Command completed");
    /// assert_eq!(result.is_error, Some(false));
    /// ```
    #[inline]
    pub fn success(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: Some(ContentValue::Text(content.into())),
            is_error: Some(false),
        }
    }

    /// Create an error tool result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::messages::ToolResultContent;
    ///
    /// let result = ToolResultContent::error("tool_123", "Command failed");
    /// assert_eq!(result.is_error, Some(true));
    /// ```
    #[inline]
    pub fn error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: Some(ContentValue::Text(content.into())),
            is_error: Some(true),
        }
    }
}

/// Content value for tool results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentValue {
    /// Text content
    Text(String),
    /// Structured content
    Structured(Vec<serde_json::Value>),
}

/// User content structure for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    /// Role (always "user")
    pub role: String,
    /// Message content (either string or array of content blocks)
    #[serde(flatten)]
    pub content: UserContentData,
}

/// User content data - either text or content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContentData {
    /// Simple text content
    Text {
        /// Text content
        content: String,
    },
    /// Array of content blocks (text, images, documents)
    Blocks {
        /// Content blocks
        content: Vec<UserContentBlock>,
    },
}

/// Content block for user messages (text, image, document)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UserContentBlock {
    /// Text content block
    Text {
        /// Text content
        text: String,
    },
    /// Image content block
    Image {
        /// Image source
        source: ImageSource,
    },
    /// Document content block
    Document {
        /// Document source
        source: DocumentSource,
        /// Optional document title
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional context about the document
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
}

/// Image source for image content blocks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ImageSource {
    /// Base64-encoded image data
    Base64 {
        /// Media type (e.g., "image/jpeg", "image/png")
        media_type: String,
        /// Base64-encoded image data
        data: String,
    },
    /// URL to image
    Url {
        /// URL to the image
        url: String,
    },
}

/// Document source for document content blocks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DocumentSource {
    /// Base64-encoded document data
    Base64 {
        /// Media type (e.g., "application/pdf", "text/plain")
        media_type: String,
        /// Base64-encoded document data
        data: String,
    },
    /// File ID from Files API
    File {
        /// File ID from Claude Files API
        file_id: String,
    },
}

/// Assistant content structure for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContent {
    /// Role (always "assistant")
    pub role: String,
    /// Content blocks
    pub content: Vec<ContentBlock>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::User {
            message: UserMessage {
                content: "Hello".to_string(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""content":"Hello""#));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_thinking_content_serialization() {
        let thinking = ThinkingContent {
            thinking: "Let me think about this...".to_string(),
            signature: "sig123".to_string(),
        };

        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains(r#""thinking":"Let me think about this...""#));
        assert!(json.contains(r#""signature":"sig123""#));

        let deserialized: ThinkingContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.thinking, thinking.thinking);
        assert_eq!(deserialized.signature, thinking.signature);
    }

    // Property-based tests
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            // UserMessage property tests
            #[test]
            fn user_message_serialization_roundtrip(content in "\\PC*") {
                let msg = UserMessage::new(content);
                let json = serde_json::to_string(&msg).unwrap();
                let deserialized: UserMessage = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(msg, deserialized);
            }

            #[test]
            fn user_message_from_conversions(content in "\\PC*") {
                let msg1 = UserMessage::from(content.clone());
                let msg2 = UserMessage::from(content.as_str());
                let msg3 = UserMessage::new(content.clone());

                prop_assert_eq!(&msg1, &msg2);
                prop_assert_eq!(&msg2, &msg3);
                prop_assert_eq!(&msg1.content, &content);
            }

            // TextContent property tests
            #[test]
            fn text_content_serialization_roundtrip(text in "\\PC*") {
                let content = TextContent::new(text);
                let json = serde_json::to_string(&content).unwrap();
                let deserialized: TextContent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(content, deserialized);
            }

            #[test]
            fn text_content_from_conversions(text in "\\PC*") {
                let c1 = TextContent::from(text.clone());
                let c2 = TextContent::from(text.as_str());
                let c3 = TextContent::new(text.clone());

                prop_assert_eq!(&c1, &c2);
                prop_assert_eq!(&c2, &c3);
                prop_assert_eq!(&c1.text, &text);
            }

            // ThinkingContent property tests
            #[test]
            fn thinking_content_serialization_roundtrip(
                thinking in "\\PC{1,100}",
                signature in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let content = ThinkingContent::new(thinking, signature);
                let json = serde_json::to_string(&content).unwrap();
                let deserialized: ThinkingContent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(content, deserialized);
            }

            // ToolUseContent property tests
            #[test]
            fn tool_use_content_serialization_roundtrip(
                id in "[a-zA-Z0-9_-]{1,50}",
                name in "[a-zA-Z]{1,30}",
                value in prop::collection::vec(0i32..100, 0..5)
            ) {
                let input = serde_json::json!({"values": value});
                let content = ToolUseContent::new(id, name, input);
                let json = serde_json::to_string(&content).unwrap();
                let deserialized: ToolUseContent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(content, deserialized);
            }

            // ToolResultContent property tests
            #[test]
            fn tool_result_content_serialization_roundtrip(
                tool_use_id in "[a-zA-Z0-9_-]{1,50}",
                result_text in "\\PC{0,200}",
                is_error in prop::bool::ANY
            ) {
                let content = ToolResultContent::new(
                    tool_use_id,
                    Some(ContentValue::Text(result_text)),
                    is_error
                );
                let json = serde_json::to_string(&content).unwrap();
                let deserialized: ToolResultContent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(content, deserialized);
            }

            #[test]
            fn tool_result_success_always_not_error(
                tool_use_id in "[a-zA-Z0-9_-]{1,50}",
                content in "\\PC{1,100}"
            ) {
                let result = ToolResultContent::success(tool_use_id, content);
                prop_assert_eq!(result.is_error, Some(false));
            }

            #[test]
            fn tool_result_error_always_is_error(
                tool_use_id in "[a-zA-Z0-9_-]{1,50}",
                content in "\\PC{1,100}"
            ) {
                let result = ToolResultContent::error(tool_use_id, content);
                prop_assert_eq!(result.is_error, Some(true));
            }

            // AssistantMessage property tests
            #[test]
            fn assistant_message_serialization_roundtrip(
                texts in prop::collection::vec("\\PC{1,50}", 0..5)
            ) {
                let content_blocks: Vec<ContentBlock> = texts
                    .into_iter()
                    .map(|t| ContentBlock::Text(TextContent::new(t)))
                    .collect();

                let msg = AssistantMessage::new(content_blocks);
                let json = serde_json::to_string(&msg).unwrap();
                let deserialized: AssistantMessage = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(msg, deserialized);
            }

            #[test]
            fn assistant_message_with_text_creates_single_block(text in "\\PC+") {
                let msg = AssistantMessage::with_text(text.clone());
                prop_assert_eq!(msg.content.len(), 1);

                if let Some(ContentBlock::Text(text_content)) = msg.content.first() {
                    prop_assert_eq!(&text_content.text, &text);
                } else {
                    panic!("Expected text content block");
                }
            }

            // Message enum property tests
            #[test]
            fn message_user_serialization_roundtrip(content in "\\PC{1,200}") {
                let msg = Message::User {
                    message: UserMessage::new(content),
                };
                let json = serde_json::to_string(&msg).unwrap();
                let deserialized: Message = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(msg, deserialized);
            }

            #[test]
            fn message_assistant_serialization_roundtrip(
                texts in prop::collection::vec("\\PC{1,30}", 1..4)
            ) {
                let content: Vec<ContentBlock> = texts
                    .into_iter()
                    .map(|t| ContentBlock::Text(TextContent::new(t)))
                    .collect();

                let msg = Message::Assistant {
                    message: AssistantMessage::new(content),
                };
                let json = serde_json::to_string(&msg).unwrap();
                let deserialized: Message = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(msg, deserialized);
            }

            #[test]
            fn message_result_serialization_roundtrip(
                subtype in "[a-z]{1,20}",
                duration_ms in 0i64..10000,
                duration_api_ms in 0i64..10000,
                is_error in prop::bool::ANY,
                num_turns in 0i32..100,
                session_id in "[a-zA-Z0-9_-]{1,50}"
            ) {
                let msg = Message::Result {
                    subtype,
                    duration_ms,
                    duration_api_ms,
                    is_error,
                    num_turns,
                    session_id,
                    total_cost_usd: None,
                    usage: None,
                    result: None,
                };
                let json = serde_json::to_string(&msg).unwrap();
                let deserialized: Message = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(msg, deserialized);
            }

            // ContentValue property tests
            #[test]
            fn content_value_text_roundtrip(text in "\\PC*") {
                let value = ContentValue::Text(text);
                let json = serde_json::to_string(&value).unwrap();
                let deserialized: ContentValue = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(value, deserialized);
            }
        }
    }
}
