//! Message parsing utilities
//!
//! This module handles parsing of JSON messages from the Claude CLI into
//! strongly typed Message enums.

use crate::{
    Result,
    error::Error,
    messages::{
        AssistantMessage, ContentBlock, ContentValue, Message, TextContent, ThinkingContent,
        ToolResultContent, ToolUseContent, UserMessage,
    },
};
use serde_json::Value;
use tracing::{debug, trace};

/// Parse a JSON value into a Message
pub fn parse_message(json: Value) -> Result<Option<Message>> {
    // Get message type
    let msg_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Transport(crate::error::TransportError::InvalidMessage {
            reason: "Missing 'type' field".to_string(),
            raw: json.to_string(),
        }))?;

    match msg_type {
        "user" => parse_user_message(json),
        "assistant" => parse_assistant_message(json),
        "system" => parse_system_message(json),
        "result" => parse_result_message(json),
        _ => {
            debug!("Ignoring message type: {}", msg_type);
            Ok(None)
        }
    }
}

/// Parse a user message
fn parse_user_message(json: Value) -> Result<Option<Message>> {
    let message = json
        .get("message")
        .ok_or_else(|| Error::Transport(crate::error::TransportError::InvalidMessage {
            reason: "Missing 'message' field".to_string(),
            raw: json.to_string(),
        }))?;

    // Handle different content formats
    let content = if let Some(content_str) = message.get("content").and_then(|v| v.as_str()) {
        // Simple string content
        content_str.to_string()
    } else if let Some(_content_array) = message.get("content").and_then(|v| v.as_array()) {
        // Array content (e.g., tool results) - we'll skip these for now
        // as they're not standard user messages but tool responses
        debug!("Skipping user message with array content (likely tool result)");
        return Ok(None);
    } else {
        return Err(Error::Transport(crate::error::TransportError::InvalidMessage {
            reason: "Missing or invalid 'content' field".to_string(),
            raw: json.to_string(),
        }));
    };

    Ok(Some(Message::User {
        message: UserMessage { content },
    }))
}

/// Parse an assistant message
fn parse_assistant_message(json: Value) -> Result<Option<Message>> {
    let message = json
        .get("message")
        .ok_or_else(|| Error::Transport(crate::error::TransportError::InvalidMessage {
            reason: "Missing 'message' field".to_string(),
            raw: json.to_string(),
        }))?;

    let content_array = message
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Transport(crate::error::TransportError::InvalidMessage {
            reason: "Missing or invalid 'content' array".to_string(),
            raw: json.to_string(),
        }))?;

    let mut content_blocks = Vec::new();

    for content_item in content_array {
        if let Some(block) = parse_content_block(content_item)? {
            content_blocks.push(block);
        }
    }

    Ok(Some(Message::Assistant {
        message: AssistantMessage {
            content: content_blocks,
        },
    }))
}

/// Parse a content block
fn parse_content_block(json: &Value) -> Result<Option<ContentBlock>> {
    // First check if it has a type field
    if let Some(block_type) = json.get("type").and_then(|v| v.as_str()) {
        match block_type {
            "text" => {
                let text = json.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::Transport(crate::error::TransportError::InvalidMessage {
                        reason: "Missing 'text' field in text block".to_string(),
                        raw: json.to_string(),
                    })
                })?;
                Ok(Some(ContentBlock::Text(TextContent {
                    text: text.to_string(),
                })))
            }
            "thinking" => {
                let thinking = json.get("thinking").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::Transport(crate::error::TransportError::InvalidMessage {
                        reason: "Missing 'thinking' field in thinking block".to_string(),
                        raw: json.to_string(),
                    })
                })?;
                let signature = json.get("signature").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::Transport(crate::error::TransportError::InvalidMessage {
                        reason: "Missing 'signature' field in thinking block".to_string(),
                        raw: json.to_string(),
                    })
                })?;
                Ok(Some(ContentBlock::Thinking(ThinkingContent {
                    thinking: thinking.to_string(),
                    signature: signature.to_string(),
                })))
            }
            "tool_use" => {
                let id = json.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::Transport(crate::error::TransportError::InvalidMessage {
                        reason: "Missing 'id' field in tool_use block".to_string(),
                        raw: json.to_string(),
                    })
                })?;
                let name = json.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::Transport(crate::error::TransportError::InvalidMessage {
                        reason: "Missing 'name' field in tool_use block".to_string(),
                        raw: json.to_string(),
                    })
                })?;
                let input = json
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

                Ok(Some(ContentBlock::ToolUse(ToolUseContent {
                    id: id.to_string(),
                    name: name.to_string(),
                    input,
                })))
            }
            "tool_result" => {
                let tool_use_id = json
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        Error::Transport(crate::error::TransportError::InvalidMessage {
                            reason: "Missing 'tool_use_id' field in tool_result block".to_string(),
                            raw: json.to_string(),
                        })
                    })?;

                let content = if let Some(content_val) = json.get("content") {
                    if let Some(text) = content_val.as_str() {
                        Some(ContentValue::Text(text.to_string()))
                    } else {
                        content_val.as_array().map(|array| ContentValue::Structured(array.clone()))
                    }
                } else {
                    None
                };

                let is_error = json.get("is_error").and_then(|v| v.as_bool());

                Ok(Some(ContentBlock::ToolResult(ToolResultContent {
                    tool_use_id: tool_use_id.to_string(),
                    content,
                    is_error,
                })))
            }
            _ => {
                debug!("Unknown content block type: {}", block_type);
                Ok(None)
            }
        }
    } else {
        // Try to parse as a simple text block (backward compatibility)
        if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
            Ok(Some(ContentBlock::Text(TextContent {
                text: text.to_string(),
            })))
        } else {
            trace!("Skipping non-text content block without type");
            Ok(None)
        }
    }
}

/// Parse a system message
fn parse_system_message(json: Value) -> Result<Option<Message>> {
    let subtype = json
        .get("subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let data = json
        .get("data")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    Ok(Some(Message::System { subtype, data }))
}

/// Parse a result message
fn parse_result_message(json: Value) -> Result<Option<Message>> {
    // Use serde to parse the full result message
    match serde_json::from_value::<Message>(json.clone()) {
        Ok(msg) => Ok(Some(msg)),
        Err(_e) => {
            // Fallback: create a minimal result message
            let subtype = json
                .get("subtype")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let duration_ms = json
                .get("duration_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let session_id = json
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(Some(Message::Result {
                subtype,
                duration_ms,
                duration_api_ms: json
                    .get("duration_api_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                is_error: json
                    .get("is_error")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                num_turns: json.get("num_turns").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                session_id,
                total_cost_usd: json.get("total_cost_usd").and_then(|v| v.as_f64()),
                usage: json.get("usage").cloned(),
                result: json
                    .get("result")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_user_message() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello, Claude!"
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::User { message }) = result {
            assert_eq!(message.content, "Hello, Claude!");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_parse_assistant_message_with_text() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello! How can I help you?"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 1);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "Hello! How can I help you?");
            } else {
                panic!("Expected Text content block");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_thinking_block() {
        let json = json!({
            "type": "thinking",
            "thinking": "Let me analyze this problem...",
            "signature": "thinking_sig_123"
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::Thinking(thinking)) = result {
            assert_eq!(thinking.thinking, "Let me analyze this problem...");
            assert_eq!(thinking.signature, "thinking_sig_123");
        } else {
            panic!("Expected Thinking content block");
        }
    }

    #[test]
    fn test_parse_tool_use_block() {
        let json = json!({
            "type": "tool_use",
            "id": "tool_123",
            "name": "read_file",
            "input": {
                "path": "/tmp/test.txt"
            }
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolUse(tool_use)) = result {
            assert_eq!(tool_use.id, "tool_123");
            assert_eq!(tool_use.name, "read_file");
            assert_eq!(tool_use.input["path"], "/tmp/test.txt");
        } else {
            panic!("Expected ToolUse content block");
        }
    }

    #[test]
    fn test_parse_system_message() {
        let json = json!({
            "type": "system",
            "subtype": "status",
            "data": {
                "status": "ready"
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::System { subtype, data }) = result {
            assert_eq!(subtype, "status");
            assert_eq!(data["status"], "ready");
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_parse_result_message() {
        let json = json!({
            "type": "result",
            "subtype": "conversation_turn",
            "duration_ms": 1234,
            "duration_api_ms": 1000,
            "is_error": false,
            "num_turns": 1,
            "session_id": "test_session",
            "total_cost_usd": 0.001
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result {
            subtype,
            duration_ms,
            session_id,
            total_cost_usd,
            ..
        }) = result
        {
            assert_eq!(subtype, "conversation_turn");
            assert_eq!(duration_ms, 1234);
            assert_eq!(session_id, "test_session");
            assert_eq!(total_cost_usd, Some(0.001));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_unknown_message_type() {
        let json = json!({
            "type": "unknown_type",
            "data": "some data"
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_none());
    }

    // ===== ERROR CASE TESTS =====

    #[test]
    fn test_parse_message_missing_type_field() {
        let json = json!({
            "message": {
                "content": "Hello"
            }
        });

        let result = parse_message(json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'type' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing type field"),
        }
    }

    #[test]
    fn test_parse_message_invalid_type_field() {
        let json = json!({
            "type": 123, // not a string
            "message": {}
        });

        let result = parse_message(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_user_message_missing_message_field() {
        let json = json!({
            "type": "user",
            "content": "Hello" // wrong structure
        });

        let result = parse_message(json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'message' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing message field"),
        }
    }

    #[test]
    fn test_parse_user_message_missing_content_field() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user"
                // no content field
            }
        });

        let result = parse_message(json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing or invalid 'content' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing content field"),
        }
    }

    #[test]
    fn test_parse_user_message_with_array_content_returns_none() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool_123",
                        "content": "result"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        // Should return None for array content in user messages
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_assistant_message_missing_message_field() {
        let json = json!({
            "type": "assistant",
            "content": [] // wrong structure
        });

        let result = parse_message(json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'message' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing message field"),
        }
    }

    #[test]
    fn test_parse_assistant_message_missing_content_array() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant"
                // no content array
            }
        });

        let result = parse_message(json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing or invalid 'content' array"));
            }
            _ => panic!("Expected InvalidMessage error for missing content array"),
        }
    }

    #[test]
    fn test_parse_assistant_message_content_not_array() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": "Just a string" // should be array
            }
        });

        let result = parse_message(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_text_block_missing_text_field() {
        let json = json!({
            "type": "text"
            // missing "text" field
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'text' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing text field"),
        }
    }

    #[test]
    fn test_parse_thinking_block_missing_thinking_field() {
        let json = json!({
            "type": "thinking",
            "signature": "sig_123"
            // missing "thinking" field
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'thinking' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing thinking field"),
        }
    }

    #[test]
    fn test_parse_thinking_block_missing_signature_field() {
        let json = json!({
            "type": "thinking",
            "thinking": "Some thought"
            // missing "signature" field
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'signature' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing signature field"),
        }
    }

    #[test]
    fn test_parse_tool_use_block_missing_id() {
        let json = json!({
            "type": "tool_use",
            "name": "read_file",
            "input": {}
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'id' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing id field"),
        }
    }

    #[test]
    fn test_parse_tool_use_block_missing_name() {
        let json = json!({
            "type": "tool_use",
            "id": "tool_123",
            "input": {}
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'name' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing name field"),
        }
    }

    #[test]
    fn test_parse_tool_use_block_missing_input_defaults_to_empty_object() {
        let json = json!({
            "type": "tool_use",
            "id": "tool_123",
            "name": "read_file"
            // no input field
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolUse(tool_use)) = result {
            assert_eq!(tool_use.id, "tool_123");
            assert_eq!(tool_use.name, "read_file");
            assert!(tool_use.input.is_object());
            assert_eq!(tool_use.input.as_object().unwrap().len(), 0);
        } else {
            panic!("Expected ToolUse content block");
        }
    }

    #[test]
    fn test_parse_tool_result_block_missing_tool_use_id() {
        let json = json!({
            "type": "tool_result",
            "content": "result"
        });

        let result = parse_content_block(&json);
        assert!(result.is_err());
        match result {
            Err(Error::Transport(crate::error::TransportError::InvalidMessage { reason, .. })) => {
                assert!(reason.contains("Missing 'tool_use_id' field"));
            }
            _ => panic!("Expected InvalidMessage error for missing tool_use_id field"),
        }
    }

    // ===== EDGE CASE TESTS =====

    #[test]
    fn test_parse_assistant_message_empty_content_array() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": []
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 0);
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_assistant_message_multiple_content_blocks() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "First part"
                    },
                    {
                        "type": "text",
                        "text": "Second part"
                    },
                    {
                        "type": "thinking",
                        "thinking": "Analysis",
                        "signature": "sig_123"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 3);
            assert!(matches!(message.content[0], ContentBlock::Text(_)));
            assert!(matches!(message.content[1], ContentBlock::Text(_)));
            assert!(matches!(message.content[2], ContentBlock::Thinking(_)));
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_assistant_message_with_tool_use_and_text() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Let me read that file for you."
                    },
                    {
                        "type": "tool_use",
                        "id": "tool_456",
                        "name": "read_file",
                        "input": {
                            "path": "/home/user/file.txt"
                        }
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 2);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "Let me read that file for you.");
            } else {
                panic!("Expected Text block");
            }
            if let ContentBlock::ToolUse(tool_use) = &message.content[1] {
                assert_eq!(tool_use.name, "read_file");
                assert_eq!(tool_use.id, "tool_456");
            } else {
                panic!("Expected ToolUse block");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_tool_result_with_text_content() {
        let json = json!({
            "type": "tool_result",
            "tool_use_id": "tool_123",
            "content": "File contents here"
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolResult(tool_result)) = result {
            assert_eq!(tool_result.tool_use_id, "tool_123");
            assert!(matches!(tool_result.content, Some(ContentValue::Text(_))));
            if let Some(ContentValue::Text(text)) = tool_result.content {
                assert_eq!(text, "File contents here");
            }
            assert_eq!(tool_result.is_error, None);
        } else {
            panic!("Expected ToolResult content block");
        }
    }

    #[test]
    fn test_parse_tool_result_with_structured_content() {
        let json = json!({
            "type": "tool_result",
            "tool_use_id": "tool_456",
            "content": [
                {
                    "type": "text",
                    "text": "Structured result"
                }
            ]
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolResult(tool_result)) = result {
            assert_eq!(tool_result.tool_use_id, "tool_456");
            assert!(matches!(tool_result.content, Some(ContentValue::Structured(_))));
        } else {
            panic!("Expected ToolResult content block");
        }
    }

    #[test]
    fn test_parse_tool_result_with_error_flag() {
        let json = json!({
            "type": "tool_result",
            "tool_use_id": "tool_789",
            "content": "Error occurred",
            "is_error": true
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolResult(tool_result)) = result {
            assert_eq!(tool_result.tool_use_id, "tool_789");
            assert_eq!(tool_result.is_error, Some(true));
        } else {
            panic!("Expected ToolResult content block");
        }
    }

    #[test]
    fn test_parse_tool_result_without_content() {
        let json = json!({
            "type": "tool_result",
            "tool_use_id": "tool_999"
            // no content field
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolResult(tool_result)) = result {
            assert_eq!(tool_result.tool_use_id, "tool_999");
            assert_eq!(tool_result.content, None);
        } else {
            panic!("Expected ToolResult content block");
        }
    }

    #[test]
    fn test_parse_system_message_with_minimal_fields() {
        let json = json!({
            "type": "system"
            // no subtype, no data
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::System { subtype, data }) = result {
            assert_eq!(subtype, "unknown");
            assert!(data.is_object());
            assert_eq!(data.as_object().unwrap().len(), 0);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_parse_system_message_with_complex_data() {
        let json = json!({
            "type": "system",
            "subtype": "session_start",
            "data": {
                "session_id": "sess_123",
                "model": "claude-3-opus",
                "config": {
                    "temperature": 0.7,
                    "max_tokens": 4096
                }
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::System { subtype, data }) = result {
            assert_eq!(subtype, "session_start");
            assert_eq!(data["session_id"], "sess_123");
            assert_eq!(data["model"], "claude-3-opus");
            assert_eq!(data["config"]["temperature"], 0.7);
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_parse_result_message_with_all_fields() {
        let json = json!({
            "type": "result",
            "subtype": "conversation_turn",
            "duration_ms": 5000,
            "duration_api_ms": 4500,
            "is_error": false,
            "num_turns": 3,
            "session_id": "test_session_456",
            "total_cost_usd": 0.0123,
            "usage": {
                "input_tokens": 100,
                "output_tokens": 200
            },
            "result": "Success"
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result {
            subtype,
            duration_ms,
            duration_api_ms,
            is_error,
            num_turns,
            session_id,
            total_cost_usd,
            usage,
            result: res,
        }) = result
        {
            assert_eq!(subtype, "conversation_turn");
            assert_eq!(duration_ms, 5000);
            assert_eq!(duration_api_ms, 4500);
            assert_eq!(is_error, false);
            assert_eq!(num_turns, 3);
            assert_eq!(session_id, "test_session_456");
            assert_eq!(total_cost_usd, Some(0.0123));
            assert!(usage.is_some());
            assert_eq!(res, Some("Success".to_string()));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_result_message_minimal_fields() {
        let json = json!({
            "type": "result",
            "subtype": "error"
            // minimal fields
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result { subtype, .. }) = result {
            assert_eq!(subtype, "error");
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_content_block_without_type_field_with_text() {
        let json = json!({
            "text": "Backward compatibility text"
            // no type field
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::Text(text)) = result {
            assert_eq!(text.text, "Backward compatibility text");
        } else {
            panic!("Expected Text content block");
        }
    }

    #[test]
    fn test_parse_content_block_without_type_and_without_text_returns_none() {
        let json = json!({
            "some_field": "some_value"
            // no type, no text
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_content_block_unknown_type_returns_none() {
        let json = json!({
            "type": "unknown_content_type",
            "data": "some data"
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_user_message_with_unicode() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello 世界! 🌍 Здравствуй мир!"
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::User { message }) = result {
            assert_eq!(message.content, "Hello 世界! 🌍 Здравствуй мир!");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_parse_assistant_message_with_unicode_in_blocks() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "こんにちは (Hello) 🎌"
                    },
                    {
                        "type": "thinking",
                        "thinking": "Analyzing 分析中...",
                        "signature": "sig_unicode_123"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 2);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "こんにちは (Hello) 🎌");
            }
            if let ContentBlock::Thinking(thinking) = &message.content[1] {
                assert_eq!(thinking.thinking, "Analyzing 分析中...");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_message_with_very_large_content() {
        let large_text = "A".repeat(100_000); // 100KB of text
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": large_text
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::User { message }) = result {
            assert_eq!(message.content.len(), 100_000);
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_parse_tool_use_with_complex_nested_input() {
        let json = json!({
            "type": "tool_use",
            "id": "tool_complex",
            "name": "complex_tool",
            "input": {
                "nested": {
                    "deep": {
                        "value": [1, 2, 3],
                        "metadata": {
                            "author": "test",
                            "timestamp": 1234567890
                        }
                    }
                },
                "array": ["a", "b", "c"]
            }
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolUse(tool_use)) = result {
            assert_eq!(tool_use.id, "tool_complex");
            assert_eq!(tool_use.name, "complex_tool");
            assert_eq!(tool_use.input["nested"]["deep"]["value"][0], 1);
            assert_eq!(tool_use.input["nested"]["deep"]["metadata"]["author"], "test");
            assert_eq!(tool_use.input["array"][0], "a");
        } else {
            panic!("Expected ToolUse content block");
        }
    }

    #[test]
    fn test_parse_empty_strings() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": ""
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::User { message }) = result {
            assert_eq!(message.content, "");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_parse_text_block_with_empty_text() {
        let json = json!({
            "type": "text",
            "text": ""
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::Text(text)) = result {
            assert_eq!(text.text, "");
        } else {
            panic!("Expected Text content block");
        }
    }

    #[test]
    fn test_parse_assistant_message_skips_invalid_blocks() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Valid block"
                    },
                    {
                        "type": "unknown_block_type",
                        "data": "invalid"
                    },
                    {
                        "type": "text",
                        "text": "Another valid block"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            // Should only have 2 valid blocks (unknown block is skipped)
            assert_eq!(message.content.len(), 2);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "Valid block");
            }
            if let ContentBlock::Text(text) = &message.content[1] {
                assert_eq!(text.text, "Another valid block");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_result_message_with_error_flag() {
        let json = json!({
            "type": "result",
            "subtype": "api_error",
            "is_error": true,
            "duration_ms": 100,
            "session_id": "error_session"
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result { is_error, subtype, .. }) = result {
            assert_eq!(is_error, true);
            assert_eq!(subtype, "api_error");
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_special_characters_in_text() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Special chars: \n\t\r\"'\\/@#$%^&*()"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            if let ContentBlock::Text(text) = &message.content[0] {
                assert!(text.text.contains("Special chars"));
                assert!(text.text.contains("\\/@#$%^&*()"));
            }
        } else {
            panic!("Expected Assistant message");
        }
    }
}
