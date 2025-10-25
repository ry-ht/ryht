//! Streaming utilities for JSONL output parsing and buffering.
//!
//! This module provides utilities for reading and parsing JSONL (JSON Lines) output
//! from the Claude CLI, extracting session information, and buffering output for
//! live access.
//!
//! # Features
//!
//! - **JSONL Line Reader**: Async line-by-line reading of JSONL streams
//! - **Session ID Extraction**: Parse session IDs from init messages
//! - **Output Buffering**: Buffer output for real-time access
//! - **Message Parsing**: Parse and validate JSONL messages
//!
//! # Examples
//!
//! ```rust,no_run
//! use cc_sdk::streaming::{JsonlReader, OutputBuffer};
//! use tokio::io::BufReader;
//! use futures::StreamExt;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Create a JSONL reader from any AsyncBufRead source
//! let reader = BufReader::new(tokio::io::stdin());
//! let mut jsonl_reader = JsonlReader::new(reader);
//!
//! // Read messages line by line
//! while let Some(result) = jsonl_reader.next().await {
//!     let message = result?;
//!     println!("Received: {:?}", message);
//! }
//! # Ok(())
//! # }
//! ```

use crate::{
    core::SessionId,
    error::{Error, TransportError},
    messages::Message,
    Result,
};
use futures::stream::Stream;
use pin_project_lite::pin_project;
use serde_json::Value;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

/// Maximum buffer size for output buffering (in lines)
const MAX_BUFFER_SIZE: usize = 10000;

/// A JSONL (JSON Lines) reader that parses messages line by line.
///
/// This reader takes any `AsyncBufRead` source and provides a `Stream` of
/// parsed `Message` objects. Each line should contain a valid JSON object
/// representing a Claude CLI message.
///
/// # Examples
///
/// ```rust,no_run
/// use cc_sdk::streaming::JsonlReader;
/// use tokio::io::BufReader;
/// use futures::StreamExt;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let reader = BufReader::new(tokio::io::stdin());
/// let mut jsonl_reader = JsonlReader::new(reader);
///
/// while let Some(result) = jsonl_reader.next().await {
///     let message = result?;
///     println!("Message: {:?}", message);
/// }
/// # Ok(())
/// # }
/// ```
pin_project! {
    pub struct JsonlReader<R: AsyncBufRead> {
        #[pin]
        reader: BufReader<R>,
        line_buffer: String,
    }
}

impl<R: AsyncBufRead> JsonlReader<R> {
    /// Create a new JSONL reader from an `AsyncBufRead` source.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use cc_sdk::streaming::JsonlReader;
    /// use tokio::io::BufReader;
    ///
    /// let reader = BufReader::new(tokio::io::stdin());
    /// let jsonl_reader = JsonlReader::new(reader);
    /// ```
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            line_buffer: String::with_capacity(4096),
        }
    }

    /// Read the next line and parse it as a message.
    ///
    /// Returns `None` when the stream ends, or an error if parsing fails.
    async fn read_next_message(self: Pin<&mut Self>) -> Result<Option<Message>> {
        let mut this = self.project();
        this.line_buffer.clear();

        match this.reader.read_line(this.line_buffer).await {
            Ok(0) => Ok(None), // EOF
            Ok(_) => {
                let line = this.line_buffer.trim();
                if line.is_empty() {
                    // Skip empty lines, try next
                    return Ok(None);
                }

                // Parse JSON
                let message: Message = serde_json::from_str(line).map_err(|e| {
                    Error::Transport(TransportError::InvalidMessage {
                        reason: format!("Failed to parse message: {}", e),
                        raw: line.to_string(),
                    })
                })?;

                Ok(Some(message))
            }
            Err(e) => Err(Error::Transport(TransportError::Io(e))),
        }
    }
}

impl<R: AsyncBufRead + Unpin> Stream for JsonlReader<R> {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Use a manual future to bridge async/await with poll
        let future = self.as_mut().read_next_message();
        tokio::pin!(future);

        match future.poll(cx) {
            Poll::Ready(Ok(Some(msg))) => Poll::Ready(Some(Ok(msg))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A thread-safe output buffer for storing JSONL output lines.
///
/// This buffer stores output lines in memory and provides methods to
/// retrieve them. It's useful for capturing Claude CLI output for
/// later inspection or replay.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::streaming::OutputBuffer;
///
/// let buffer = OutputBuffer::new();
///
/// // Add lines to the buffer
/// buffer.push("line 1");
/// buffer.push("line 2");
///
/// // Get all buffered lines
/// let lines = buffer.get_all();
/// assert_eq!(lines.len(), 2);
///
/// // Clear the buffer
/// buffer.clear();
/// assert_eq!(buffer.len(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct OutputBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
    max_size: usize,
}

impl OutputBuffer {
    /// Create a new output buffer with default maximum size.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(MAX_BUFFER_SIZE)
    }

    /// Create a new output buffer with a specified maximum size.
    ///
    /// When the buffer exceeds this size, oldest lines will be dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::with_capacity(1000);
    /// ```
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    /// Add a line to the buffer.
    ///
    /// If the buffer is full, the oldest line will be removed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// buffer.push("Hello, world!");
    /// ```
    pub fn push(&self, line: impl Into<String>) {
        let mut lines = self.lines.lock().unwrap();
        if lines.len() >= self.max_size {
            lines.pop_front();
        }
        lines.push_back(line.into());
    }

    /// Get all buffered lines.
    ///
    /// Returns a vector of all lines currently in the buffer, in order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// buffer.push("line 1");
    /// buffer.push("line 2");
    ///
    /// let lines = buffer.get_all();
    /// assert_eq!(lines.len(), 2);
    /// ```
    pub fn get_all(&self) -> Vec<String> {
        let lines = self.lines.lock().unwrap();
        lines.iter().cloned().collect()
    }

    /// Get the last N lines from the buffer.
    ///
    /// Returns up to `n` lines from the end of the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// buffer.push("line 1");
    /// buffer.push("line 2");
    /// buffer.push("line 3");
    ///
    /// let last_two = buffer.get_last(2);
    /// assert_eq!(last_two.len(), 2);
    /// assert_eq!(last_two[0], "line 2");
    /// assert_eq!(last_two[1], "line 3");
    /// ```
    pub fn get_last(&self, n: usize) -> Vec<String> {
        let lines = self.lines.lock().unwrap();
        let start = lines.len().saturating_sub(n);
        lines.iter().skip(start).cloned().collect()
    }

    /// Get lines matching a predicate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// buffer.push("error: something went wrong");
    /// buffer.push("info: all good");
    /// buffer.push("error: another issue");
    ///
    /// let errors = buffer.filter(|line| line.contains("error"));
    /// assert_eq!(errors.len(), 2);
    /// ```
    pub fn filter<F>(&self, predicate: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        let lines = self.lines.lock().unwrap();
        lines.iter().filter(|line| predicate(line)).cloned().collect()
    }

    /// Get the number of lines in the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// assert_eq!(buffer.len(), 0);
    ///
    /// buffer.push("line 1");
    /// assert_eq!(buffer.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        let lines = self.lines.lock().unwrap();
        lines.len()
    }

    /// Check if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// assert!(buffer.is_empty());
    ///
    /// buffer.push("line 1");
    /// assert!(!buffer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let lines = self.lines.lock().unwrap();
        lines.is_empty()
    }

    /// Clear all lines from the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::streaming::OutputBuffer;
    ///
    /// let buffer = OutputBuffer::new();
    /// buffer.push("line 1");
    /// buffer.push("line 2");
    ///
    /// buffer.clear();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn clear(&self) {
        let mut lines = self.lines.lock().unwrap();
        lines.clear();
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract session ID from an init message.
///
/// Init messages from Claude CLI typically contain a session ID in their data.
/// This function attempts to extract it from various message formats.
///
/// # Examples
///
/// ```rust,no_run
/// use cc_sdk::streaming::extract_session_id;
/// use cc_sdk::messages::Message;
///
/// # fn example(message: Message) -> Option<cc_sdk::core::SessionId> {
/// if let Some(session_id) = extract_session_id(&message) {
///     println!("Found session ID: {}", session_id);
///     Some(session_id)
/// } else {
///     None
/// }
/// # }
/// ```
pub fn extract_session_id(message: &Message) -> Option<SessionId> {
    match message {
        Message::System { subtype, data } if subtype == "init" => {
            // Try to extract session_id from data
            if let Some(session_id_val) = data.get("session_id") {
                if let Some(session_id_str) = session_id_val.as_str() {
                    return Some(SessionId::new(session_id_str));
                }
            }
            None
        }
        Message::Result { session_id, .. } => {
            Some(SessionId::new(session_id))
        }
        _ => None,
    }
}

/// Extract session ID from a raw JSONL line.
///
/// This is a convenience function that parses a JSON line and extracts
/// the session ID if present.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::streaming::extract_session_id_from_line;
///
/// let line = r#"{"type":"result","session_id":"abc123","subtype":"done","duration_ms":1000,"duration_api_ms":500,"is_error":false,"num_turns":1}"#;
/// if let Ok(Some(session_id)) = extract_session_id_from_line(line) {
///     assert_eq!(session_id.to_string(), "abc123");
/// }
/// ```
pub fn extract_session_id_from_line(line: &str) -> Result<Option<SessionId>> {
    let value: Value = serde_json::from_str(line).map_err(|e| {
        Error::Transport(TransportError::InvalidMessage {
            reason: format!("Failed to parse JSON: {}", e),
            raw: line.to_string(),
        })
    })?;

    // Check for session_id field
    if let Some(session_id_val) = value.get("session_id") {
        if let Some(session_id_str) = session_id_val.as_str() {
            return Ok(Some(SessionId::new(session_id_str)));
        }
    }

    // Check for init message with session_id in data
    if let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) {
        if msg_type == "system" {
            if let Some(subtype) = value.get("subtype").and_then(|v| v.as_str()) {
                if subtype == "init" {
                    if let Some(data) = value.get("data") {
                        if let Some(session_id_val) = data.get("session_id") {
                            if let Some(session_id_str) = session_id_val.as_str() {
                                return Ok(Some(SessionId::new(session_id_str)));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Parse a JSONL line into a Message.
///
/// This is a convenience function for parsing individual JSONL lines.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::streaming::parse_jsonl_line;
///
/// let line = r#"{"type":"user","message":{"content":"Hello"}}"#;
/// let message = parse_jsonl_line(line).unwrap();
/// ```
pub fn parse_jsonl_line(line: &str) -> Result<Message> {
    serde_json::from_str(line).map_err(|e| {
        Error::Transport(TransportError::InvalidMessage {
            reason: format!("Failed to parse message: {}", e),
            raw: line.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use tokio::io::BufReader;

    #[test]
    fn test_output_buffer_basic() {
        let buffer = OutputBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);

        buffer.push("line 1");
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        buffer.push("line 2");
        assert_eq!(buffer.len(), 2);

        let lines = buffer.get_all();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");
    }

    #[test]
    fn test_output_buffer_capacity() {
        let buffer = OutputBuffer::with_capacity(2);
        buffer.push("line 1");
        buffer.push("line 2");
        buffer.push("line 3"); // Should evict line 1

        let lines = buffer.get_all();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 2");
        assert_eq!(lines[1], "line 3");
    }

    #[test]
    fn test_output_buffer_get_last() {
        let buffer = OutputBuffer::new();
        buffer.push("line 1");
        buffer.push("line 2");
        buffer.push("line 3");
        buffer.push("line 4");

        let last_two = buffer.get_last(2);
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0], "line 3");
        assert_eq!(last_two[1], "line 4");

        let last_ten = buffer.get_last(10);
        assert_eq!(last_ten.len(), 4); // Only 4 lines available
    }

    #[test]
    fn test_output_buffer_filter() {
        let buffer = OutputBuffer::new();
        buffer.push("error: bad thing");
        buffer.push("info: good thing");
        buffer.push("error: another bad thing");
        buffer.push("debug: some detail");

        let errors = buffer.filter(|line| line.contains("error"));
        assert_eq!(errors.len(), 2);

        let infos = buffer.filter(|line| line.contains("info"));
        assert_eq!(infos.len(), 1);
    }

    #[test]
    fn test_output_buffer_clear() {
        let buffer = OutputBuffer::new();
        buffer.push("line 1");
        buffer.push("line 2");

        assert_eq!(buffer.len(), 2);
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    async fn test_jsonl_reader() {
        let input = r#"{"type":"user","message":{"content":"Hello"}}
{"type":"assistant","message":{"content":[]}}
"#;

        let reader = BufReader::new(input.as_bytes());
        let mut jsonl_reader = JsonlReader::new(reader);

        let mut messages = Vec::new();
        while let Some(result) = jsonl_reader.next().await {
            messages.push(result.unwrap());
        }

        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0], Message::User { .. }));
        assert!(matches!(messages[1], Message::Assistant { .. }));
    }

    #[test]
    fn test_extract_session_id_from_line() {
        let line = r#"{"type":"result","session_id":"test-123","subtype":"done","duration_ms":1000,"duration_api_ms":500,"is_error":false,"num_turns":1}"#;
        let session_id = extract_session_id_from_line(line).unwrap();
        assert!(session_id.is_some());
        assert_eq!(session_id.unwrap().to_string(), "test-123");
    }

    #[test]
    fn test_extract_session_id_from_init() {
        let line = r#"{"type":"system","subtype":"init","data":{"session_id":"init-456"}}"#;
        let session_id = extract_session_id_from_line(line).unwrap();
        assert!(session_id.is_some());
        assert_eq!(session_id.unwrap().to_string(), "init-456");
    }

    #[test]
    fn test_parse_jsonl_line() {
        let line = r#"{"type":"user","message":{"content":"Hello"}}"#;
        let message = parse_jsonl_line(line).unwrap();
        assert!(matches!(message, Message::User { .. }));
    }

    #[test]
    fn test_parse_invalid_jsonl() {
        let line = r#"{"invalid json"#;
        let result = parse_jsonl_line(line);
        assert!(result.is_err());
    }
}
