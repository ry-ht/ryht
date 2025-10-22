//! Benchmarks for message serialization and deserialization.
//!
//! This benchmark suite measures the performance of:
//! - Message serialization to JSON
//! - Message deserialization from JSON
//! - Different message types (User, Assistant, System, Result)
//! - Content block serialization
//! - Batch operations

use cc_sdk::messages::{
    AssistantMessage, ContentBlock, ContentValue, Message, TextContent, ThinkingContent,
    ToolResultContent, ToolUseContent, UserMessage,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;

/// Create a simple user message.
fn create_user_message(content: &str) -> Message {
    Message::User {
        message: UserMessage {
            content: content.to_string(),
        },
    }
}

/// Create an assistant message with various content blocks.
fn create_assistant_message(blocks: usize) -> Message {
    let content: Vec<ContentBlock> = (0..blocks)
        .map(|i| {
            ContentBlock::Text(TextContent {
                text: format!("This is text block number {}", i),
            })
        })
        .collect();

    Message::Assistant {
        message: AssistantMessage { content },
    }
}

/// Create a complex assistant message with mixed content types.
fn create_complex_assistant_message() -> Message {
    Message::Assistant {
        message: AssistantMessage {
            content: vec![
                ContentBlock::Text(TextContent {
                    text: "Here's my analysis...".to_string(),
                }),
                ContentBlock::Thinking(ThinkingContent {
                    thinking: "Let me think about this problem step by step...".to_string(),
                    signature: "sig123".to_string(),
                }),
                ContentBlock::ToolUse(ToolUseContent {
                    id: "tool_abc123".to_string(),
                    name: "Bash".to_string(),
                    input: json!({"command": "ls -la", "cwd": "/tmp"}),
                }),
                ContentBlock::ToolResult(ToolResultContent {
                    tool_use_id: "tool_abc123".to_string(),
                    content: Some(ContentValue::Text("total 48\ndrwxr-xr-x 12 user staff 384 Jan 1 12:00 .".to_string())),
                    is_error: Some(false),
                }),
            ],
        },
    }
}

/// Create a result message.
fn create_result_message() -> Message {
    Message::Result {
        subtype: "success".to_string(),
        duration_ms: 1234,
        duration_api_ms: 1000,
        is_error: false,
        num_turns: 3,
        session_id: "session-abc123".to_string(),
        total_cost_usd: Some(0.0123),
        usage: Some(json!({
            "input_tokens": 1000,
            "output_tokens": 500,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0
        })),
        result: Some("Task completed successfully".to_string()),
    }
}

/// Benchmark user message serialization.
fn bench_user_message_serialization(c: &mut Criterion) {
    let message = create_user_message("Hello, Claude! This is a test message.");

    c.bench_function("serialize_user_message", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&message).unwrap());
            black_box(json)
        });
    });

    c.bench_function("serialize_user_message_pretty", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string_pretty(&message).unwrap());
            black_box(json)
        });
    });

    c.bench_function("serialize_user_message_value", |b| {
        b.iter(|| {
            let value = black_box(serde_json::to_value(&message).unwrap());
            black_box(value)
        });
    });
}

/// Benchmark user message deserialization.
fn bench_user_message_deserialization(c: &mut Criterion) {
    let message = create_user_message("Hello, Claude! This is a test message.");
    let json = serde_json::to_string(&message).unwrap();

    c.bench_function("deserialize_user_message", |b| {
        b.iter(|| {
            let msg: Message = black_box(serde_json::from_str(&json).unwrap());
            black_box(msg)
        });
    });

    let json_bytes = json.as_bytes();
    c.bench_function("deserialize_user_message_bytes", |b| {
        b.iter(|| {
            let msg: Message = black_box(serde_json::from_slice(json_bytes).unwrap());
            black_box(msg)
        });
    });
}

/// Benchmark assistant message serialization with varying block counts.
fn bench_assistant_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_assistant_message");

    for blocks in [1, 5, 10, 20, 50].iter() {
        let message = create_assistant_message(*blocks);
        group.bench_with_input(BenchmarkId::from_parameter(blocks), blocks, |b, _| {
            b.iter(|| {
                let json = black_box(serde_json::to_string(&message).unwrap());
                black_box(json)
            });
        });
    }

    group.finish();
}

/// Benchmark assistant message deserialization with varying block counts.
fn bench_assistant_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_assistant_message");

    for blocks in [1, 5, 10, 20, 50].iter() {
        let message = create_assistant_message(*blocks);
        let json = serde_json::to_string(&message).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(blocks), blocks, |b, _| {
            b.iter(|| {
                let msg: Message = black_box(serde_json::from_str(&json).unwrap());
                black_box(msg)
            });
        });
    }

    group.finish();
}

/// Benchmark complex assistant message with mixed content types.
fn bench_complex_message_serialization(c: &mut Criterion) {
    let message = create_complex_assistant_message();

    c.bench_function("serialize_complex_assistant", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&message).unwrap());
            black_box(json)
        });
    });

    let json = serde_json::to_string(&message).unwrap();
    c.bench_function("deserialize_complex_assistant", |b| {
        b.iter(|| {
            let msg: Message = black_box(serde_json::from_str(&json).unwrap());
            black_box(msg)
        });
    });
}

/// Benchmark result message serialization.
fn bench_result_message_serialization(c: &mut Criterion) {
    let message = create_result_message();

    c.bench_function("serialize_result_message", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&message).unwrap());
            black_box(json)
        });
    });

    let json = serde_json::to_string(&message).unwrap();
    c.bench_function("deserialize_result_message", |b| {
        b.iter(|| {
            let msg: Message = black_box(serde_json::from_str(&json).unwrap());
            black_box(msg)
        });
    });
}

/// Benchmark content block serialization.
fn bench_content_block_serialization(c: &mut Criterion) {
    let text_block = ContentBlock::Text(TextContent {
        text: "This is a text content block".to_string(),
    });

    let thinking_block = ContentBlock::Thinking(ThinkingContent {
        thinking: "Let me analyze this...".to_string(),
        signature: "sig123".to_string(),
    });

    let tool_use_block = ContentBlock::ToolUse(ToolUseContent {
        id: "tool_123".to_string(),
        name: "Bash".to_string(),
        input: json!({"command": "echo 'hello'"}),
    });

    let tool_result_block = ContentBlock::ToolResult(ToolResultContent {
        tool_use_id: "tool_123".to_string(),
        content: Some(ContentValue::Text("hello\n".to_string())),
        is_error: Some(false),
    });

    c.bench_function("serialize_text_block", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&text_block).unwrap());
            black_box(json)
        });
    });

    c.bench_function("serialize_thinking_block", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&thinking_block).unwrap());
            black_box(json)
        });
    });

    c.bench_function("serialize_tool_use_block", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&tool_use_block).unwrap());
            black_box(json)
        });
    });

    c.bench_function("serialize_tool_result_block", |b| {
        b.iter(|| {
            let json = black_box(serde_json::to_string(&tool_result_block).unwrap());
            black_box(json)
        });
    });
}

/// Benchmark batch message serialization.
fn bench_batch_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_message_batch");

    for count in [10, 50, 100, 500].iter() {
        let messages: Vec<Message> = (0..*count)
            .map(|i| {
                if i % 2 == 0 {
                    create_user_message(&format!("User message {}", i))
                } else {
                    create_assistant_message(5)
                }
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let json = black_box(serde_json::to_string(&messages).unwrap());
                black_box(json)
            });
        });
    }

    group.finish();
}

/// Benchmark batch message deserialization.
fn bench_batch_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_message_batch");

    for count in [10, 50, 100, 500].iter() {
        let messages: Vec<Message> = (0..*count)
            .map(|i| {
                if i % 2 == 0 {
                    create_user_message(&format!("User message {}", i))
                } else {
                    create_assistant_message(5)
                }
            })
            .collect();

        let json = serde_json::to_string(&messages).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let msgs: Vec<Message> = black_box(serde_json::from_str(&json).unwrap());
                black_box(msgs)
            });
        });
    }

    group.finish();
}

/// Benchmark message construction (non-serialization).
fn bench_message_construction(c: &mut Criterion) {
    c.bench_function("construct_user_message", |b| {
        b.iter(|| {
            let msg = black_box(UserMessage::new("Hello, Claude!"));
            black_box(msg)
        });
    });

    c.bench_function("construct_assistant_message", |b| {
        b.iter(|| {
            let msg = black_box(AssistantMessage::with_text("Hello back!"));
            black_box(msg)
        });
    });

    c.bench_function("construct_text_content", |b| {
        b.iter(|| {
            let content = black_box(TextContent::new("Sample text"));
            black_box(content)
        });
    });

    c.bench_function("construct_thinking_content", |b| {
        b.iter(|| {
            let content = black_box(ThinkingContent::new("Let me think...", "sig123"));
            black_box(content)
        });
    });
}

/// Benchmark message cloning.
fn bench_message_cloning(c: &mut Criterion) {
    let user_msg = create_user_message("Hello, Claude!");
    let assistant_msg = create_assistant_message(10);
    let complex_msg = create_complex_assistant_message();

    c.bench_function("clone_user_message", |b| {
        b.iter(|| {
            let cloned = black_box(user_msg.clone());
            black_box(cloned)
        });
    });

    c.bench_function("clone_assistant_message", |b| {
        b.iter(|| {
            let cloned = black_box(assistant_msg.clone());
            black_box(cloned)
        });
    });

    c.bench_function("clone_complex_message", |b| {
        b.iter(|| {
            let cloned = black_box(complex_msg.clone());
            black_box(cloned)
        });
    });
}

criterion_group!(
    benches,
    bench_user_message_serialization,
    bench_user_message_deserialization,
    bench_assistant_message_serialization,
    bench_assistant_message_deserialization,
    bench_complex_message_serialization,
    bench_result_message_serialization,
    bench_content_block_serialization,
    bench_batch_serialization,
    bench_batch_deserialization,
    bench_message_construction,
    bench_message_cloning,
);

criterion_main!(benches);
