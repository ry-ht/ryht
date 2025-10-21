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

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// Content blocks
    pub content: Vec<ContentBlock>,
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

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingContent {
    /// Thinking content
    pub thinking: String,
    /// Signature
    pub signature: String,
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
    /// Message content
    pub content: String,
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
}
