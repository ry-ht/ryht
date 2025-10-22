//! Integration tests for streaming and metrics modules.

use cc_sdk::{
    metrics::SessionMetrics,
    streaming::{extract_session_id_from_line, parse_jsonl_line, JsonlReader, OutputBuffer},
    messages::Message,
    Result,
};
use futures::StreamExt;
use tokio::io::BufReader;

#[tokio::test]
async fn test_jsonl_reader_basic() {
    let input = r#"{"type":"user","message":{"content":"Hello"}}
{"type":"assistant","message":{"content":[]}}
{"type":"result","session_id":"test-123","subtype":"done","duration_ms":1000,"duration_api_ms":500,"is_error":false,"num_turns":1}
"#;

    let reader = BufReader::new(input.as_bytes());
    let mut jsonl_reader = JsonlReader::new(reader);

    let mut messages = Vec::new();
    while let Some(result) = jsonl_reader.next().await {
        messages.push(result.unwrap());
    }

    assert_eq!(messages.len(), 3);
    assert!(matches!(messages[0], Message::User { .. }));
    assert!(matches!(messages[1], Message::Assistant { .. }));
    assert!(matches!(messages[2], Message::Result { .. }));
}

#[tokio::test]
async fn test_jsonl_reader_with_metrics() {
    let input = r#"{"type":"user","message":{"content":"Calculate 2+2"}}
{"type":"assistant","message":{"content":[{"text":"The answer is 4"}]}}
{"type":"result","session_id":"calc-456","subtype":"done","duration_ms":2500,"duration_api_ms":2000,"is_error":false,"num_turns":2,"usage":{"input_tokens":50,"output_tokens":25}}
"#;

    let reader = BufReader::new(input.as_bytes());
    let mut metrics_stream = SessionMetrics::from_jsonl_stream(reader);

    let mut final_metrics = None;
    while let Some(metrics) = metrics_stream.next().await {
        final_metrics = Some(metrics);
    }

    let metrics = final_metrics.unwrap();
    assert_eq!(metrics.message_count, 3);
    assert_eq!(metrics.user_message_count, 1);
    assert_eq!(metrics.assistant_message_count, 1);
    assert_eq!(metrics.prompt_tokens, Some(50));
    assert_eq!(metrics.completion_tokens, Some(25));
    assert_eq!(metrics.total_tokens, Some(75));
    assert_eq!(metrics.duration_ms, Some(2500));
    assert!(metrics.cost_usd.is_some());
}

#[test]
fn test_output_buffer_operations() {
    let buffer = OutputBuffer::new();

    // Test push and get
    buffer.push("line 1");
    buffer.push("line 2");
    buffer.push("line 3");

    assert_eq!(buffer.len(), 3);
    assert!(!buffer.is_empty());

    let all_lines = buffer.get_all();
    assert_eq!(all_lines.len(), 3);
    assert_eq!(all_lines[0], "line 1");

    // Test get_last
    let last_two = buffer.get_last(2);
    assert_eq!(last_two.len(), 2);
    assert_eq!(last_two[0], "line 2");
    assert_eq!(last_two[1], "line 3");

    // Test filter
    buffer.push("error: something bad");
    buffer.push("info: all good");

    let errors = buffer.filter(|line| line.contains("error"));
    assert_eq!(errors.len(), 1);

    // Test clear
    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_output_buffer_capacity() {
    let buffer = OutputBuffer::with_capacity(3);

    buffer.push("line 1");
    buffer.push("line 2");
    buffer.push("line 3");
    buffer.push("line 4"); // Should evict line 1

    let lines = buffer.get_all();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line 2");
    assert_eq!(lines[1], "line 3");
    assert_eq!(lines[2], "line 4");
}

#[test]
fn test_extract_session_id_from_result() {
    let line = r#"{"type":"result","session_id":"result-789","subtype":"done","duration_ms":1000,"duration_api_ms":500,"is_error":false,"num_turns":1}"#;
    let session_id = extract_session_id_from_line(line).unwrap();

    assert!(session_id.is_some());
    assert_eq!(session_id.unwrap().to_string(), "result-789");
}

#[test]
fn test_extract_session_id_from_init() {
    let line = r#"{"type":"system","subtype":"init","data":{"session_id":"init-101112"}}"#;
    let session_id = extract_session_id_from_line(line).unwrap();

    assert!(session_id.is_some());
    assert_eq!(session_id.unwrap().to_string(), "init-101112");
}

#[test]
fn test_extract_session_id_not_present() {
    let line = r#"{"type":"user","message":{"content":"Hello"}}"#;
    let session_id = extract_session_id_from_line(line).unwrap();

    assert!(session_id.is_none());
}

#[test]
fn test_parse_jsonl_line_user_message() {
    let line = r#"{"type":"user","message":{"content":"Test message"}}"#;
    let message = parse_jsonl_line(line).unwrap();

    match message {
        Message::User { message } => {
            assert_eq!(message.content, "Test message");
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_jsonl_line_invalid() {
    let line = r#"{"invalid": "json"#;
    let result = parse_jsonl_line(line);

    assert!(result.is_err());
}

#[test]
fn test_session_metrics_default() {
    let metrics = SessionMetrics::new();

    assert_eq!(metrics.message_count, 0);
    assert_eq!(metrics.user_message_count, 0);
    assert_eq!(metrics.assistant_message_count, 0);
    assert_eq!(metrics.tool_use_count, 0);
    assert_eq!(metrics.error_count, 0);
    assert_eq!(metrics.total_tokens, None);
    assert_eq!(metrics.cost_usd, None);
}

#[test]
fn test_session_metrics_custom_pricing() {
    let mut metrics = SessionMetrics::with_pricing(1.0, 5.0);
    metrics.prompt_tokens = Some(1_000_000); // 1M tokens
    metrics.completion_tokens = Some(500_000); // 500K tokens

    metrics.calculate_cost();

    // Cost = (1M * $1/1M) + (0.5M * $5/1M) = $1 + $2.50 = $3.50
    assert_eq!(metrics.cost_usd, Some(3.5));
}

#[test]
fn test_session_metrics_update_from_line() {
    let mut metrics = SessionMetrics::new();

    let line = r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":200,"output_tokens":150},"total_cost_usd":0.005}"#;
    metrics.update_from_line(line).unwrap();

    assert_eq!(metrics.duration_ms, Some(5000));
    assert_eq!(metrics.prompt_tokens, Some(200));
    assert_eq!(metrics.completion_tokens, Some(150));
    assert_eq!(metrics.total_tokens, Some(350));
}

#[test]
fn test_session_metrics_multiple_updates() {
    let mut metrics = SessionMetrics::new();

    // First message - user
    let line1 = r#"{"type":"user","message":{"content":"Hello"}}"#;
    metrics.update_from_line(line1).unwrap();

    // Second message - assistant
    let line2 = r#"{"type":"assistant","message":{"content":[{"text":"Hi there"}]}}"#;
    metrics.update_from_line(line2).unwrap();

    // Third message - result with usage
    let line3 = r#"{"type":"result","duration_ms":3000,"usage":{"input_tokens":100,"output_tokens":75},"is_error":false}"#;
    metrics.update_from_line(line3).unwrap();

    assert_eq!(metrics.message_count, 3);
    assert_eq!(metrics.user_message_count, 1);
    assert_eq!(metrics.assistant_message_count, 1);
    assert_eq!(metrics.total_tokens, Some(175));
    assert_eq!(metrics.error_count, 0);
}

#[test]
fn test_session_metrics_error_tracking() {
    let mut metrics = SessionMetrics::new();

    let line = r#"{"type":"result","is_error":true,"duration_ms":1000}"#;
    metrics.update_from_line(line).unwrap();

    assert_eq!(metrics.error_count, 1);
    assert_eq!(metrics.error_rate(), Some(1.0)); // 1 error / 1 message = 100%
}

#[test]
fn test_session_metrics_avg_tokens_per_message() {
    let mut metrics = SessionMetrics::new();
    metrics.total_tokens = Some(1500);
    metrics.message_count = 10;

    assert_eq!(metrics.avg_tokens_per_message(), Some(150.0));
}

#[test]
fn test_session_metrics_cost_per_message() {
    let mut metrics = SessionMetrics::new();
    metrics.cost_usd = Some(0.50);
    metrics.message_count = 5;

    assert_eq!(metrics.cost_per_message(), Some(0.1));
}

#[test]
fn test_session_metrics_reset() {
    let mut metrics = SessionMetrics::with_pricing(2.0, 10.0);
    metrics.message_count = 10;
    metrics.total_tokens = Some(1000);
    metrics.cost_usd = Some(0.5);

    metrics.reset();

    assert_eq!(metrics.message_count, 0);
    assert_eq!(metrics.total_tokens, None);
    assert_eq!(metrics.cost_usd, None);
    // Pricing should be preserved
    assert_eq!(metrics.input_token_cost, 2.0);
    assert_eq!(metrics.output_token_cost, 10.0);
}

#[test]
fn test_session_metrics_set_pricing() {
    let mut metrics = SessionMetrics::new();
    metrics.prompt_tokens = Some(1_000_000);
    metrics.completion_tokens = Some(1_000_000);

    // Default pricing
    let initial_cost = metrics.cost_usd;

    // Change pricing
    metrics.set_pricing(1.0, 5.0);

    // Cost should be recalculated
    assert_ne!(metrics.cost_usd, initial_cost);
    assert_eq!(metrics.cost_usd, Some(6.0)); // (1M * $1/1M) + (1M * $5/1M)
}

#[tokio::test]
async fn test_end_to_end_streaming_with_metrics() {
    // Simulate a complete session
    let session_data = r#"{"type":"system","subtype":"init","data":{"session_id":"e2e-session-001"}}
{"type":"user","message":{"content":"What is the capital of France?"}}
{"type":"assistant","message":{"content":[{"text":"The capital of France is Paris."}]}}
{"type":"result","session_id":"e2e-session-001","subtype":"done","duration_ms":1500,"duration_api_ms":1200,"is_error":false,"num_turns":1,"usage":{"input_tokens":80,"output_tokens":60},"total_cost_usd":0.0012}
"#;

    let reader = BufReader::new(session_data.as_bytes());
    let mut jsonl_reader = JsonlReader::new(reader);

    let mut metrics = SessionMetrics::new();
    let mut session_id = None;

    while let Some(result) = jsonl_reader.next().await {
        let message = result.unwrap();

        // Extract session ID
        if session_id.is_none() {
            session_id = cc_sdk::streaming::extract_session_id(&message);
        }

        // Update metrics
        metrics.update_from_message(&message);
    }

    // Verify session ID was extracted
    assert!(session_id.is_some());
    assert_eq!(session_id.unwrap().to_string(), "e2e-session-001");

    // Verify metrics
    assert_eq!(metrics.message_count, 4);
    assert_eq!(metrics.user_message_count, 1);
    assert_eq!(metrics.assistant_message_count, 1);
    assert_eq!(metrics.total_tokens, Some(140));
    assert_eq!(metrics.duration_ms, Some(1500));
    assert_eq!(metrics.cost_usd, Some(0.0012));
    assert_eq!(metrics.error_count, 0);
}

#[test]
fn test_output_buffer_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let buffer = Arc::new(OutputBuffer::new());
    let mut handles = vec![];

    // Spawn multiple threads writing to buffer
    for i in 0..10 {
        let buffer_clone = Arc::clone(&buffer);
        let handle = thread::spawn(move || {
            for j in 0..10 {
                buffer_clone.push(format!("thread-{}-line-{}", i, j));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all lines were added
    assert_eq!(buffer.len(), 100);
}

#[tokio::test]
async fn test_metrics_stream_partial_updates() {
    let input = r#"{"type":"user","message":{"content":"First message"}}
{"type":"assistant","message":{"content":[{"text":"Response 1"}]}}
{"type":"user","message":{"content":"Second message"}}
{"type":"assistant","message":{"content":[{"text":"Response 2"}]}}
"#;

    let reader = BufReader::new(input.as_bytes());
    let mut metrics_stream = SessionMetrics::from_jsonl_stream(reader);

    let mut metrics_snapshots = Vec::new();
    while let Some(metrics) = metrics_stream.next().await {
        metrics_snapshots.push(metrics);
    }

    // Should have 4 snapshots (one per message)
    assert_eq!(metrics_snapshots.len(), 4);

    // First snapshot - 1 user message
    assert_eq!(metrics_snapshots[0].user_message_count, 1);
    assert_eq!(metrics_snapshots[0].assistant_message_count, 0);

    // Second snapshot - 1 user, 1 assistant
    assert_eq!(metrics_snapshots[1].user_message_count, 1);
    assert_eq!(metrics_snapshots[1].assistant_message_count, 1);

    // Third snapshot - 2 users, 1 assistant
    assert_eq!(metrics_snapshots[2].user_message_count, 2);
    assert_eq!(metrics_snapshots[2].assistant_message_count, 1);

    // Fourth snapshot - 2 users, 2 assistants
    assert_eq!(metrics_snapshots[3].user_message_count, 2);
    assert_eq!(metrics_snapshots[3].assistant_message_count, 2);
}
