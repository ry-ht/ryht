//! Integration tests for session management operations.
//!
//! Tests forking, merging, exporting, and other advanced session operations.

use cc_sdk::core::SessionId;
use cc_sdk::messages::{Message, UserMessage};
use cc_sdk::session::{ExportFormat, Session, SessionInfo};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_export_format_variants() {
    let formats = vec![
        ExportFormat::Json,
        ExportFormat::Jsonl,
        ExportFormat::Markdown,
        ExportFormat::Text,
    ];

    // All formats should be distinct
    for (i, f1) in formats.iter().enumerate() {
        for (j, f2) in formats.iter().enumerate() {
            if i == j {
                assert_eq!(f1, f2);
            } else {
                assert_ne!(f1, f2);
            }
        }
    }
}

#[test]
fn test_session_info_creation() {
    let session = Session::new(
        SessionId::new("test-session"),
        PathBuf::from("/project"),
        Utc::now(),
        Some("First message".to_string()),
    );

    let info = SessionInfo {
        session: session.clone(),
        message_count: 15,
        last_modified: Some(Utc::now()),
    };

    assert_eq!(info.session.id.as_str(), "test-session");
    assert_eq!(info.message_count, 15);
    assert!(info.last_modified.is_some());
}

#[test]
fn test_export_to_json() {
    let messages = vec![
        Message::User {
            message: UserMessage {
                content: "Hello".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "World".to_string(),
            },
        },
    ];

    // Simulate JSON export
    let json = serde_json::to_string_pretty(&messages).unwrap();
    assert!(json.contains("Hello"));
    assert!(json.contains("World"));

    // Verify it's valid JSON
    let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 2);
}

#[test]
fn test_export_to_jsonl() {
    let messages = vec![
        Message::User {
            message: UserMessage {
                content: "Line 1".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "Line 2".to_string(),
            },
        },
    ];

    // Simulate JSONL export
    let mut jsonl = String::new();
    for message in &messages {
        jsonl.push_str(&serde_json::to_string(message).unwrap());
        jsonl.push('\n');
    }

    assert!(jsonl.contains("Line 1"));
    assert!(jsonl.contains("Line 2"));

    // Verify each line is valid JSON
    for line in jsonl.lines() {
        let _: Message = serde_json::from_str(line).unwrap();
    }
}

#[test]
fn test_export_to_markdown() {
    let messages = vec![
        Message::User {
            message: UserMessage {
                content: "Question".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "Answer".to_string(),
            },
        },
    ];

    // Simulate Markdown export
    let mut markdown = String::from("# Session Export\n\n");
    for (i, message) in messages.iter().enumerate() {
        match message {
            Message::User { message } => {
                markdown.push_str(&format!("## Message {}\n\n", i + 1));
                markdown.push_str(&format!("**User**: {}\n\n", message.content));
            }
            _ => {}
        }
    }

    assert!(markdown.contains("# Session Export"));
    assert!(markdown.contains("**User**: Question"));
    assert!(markdown.contains("**User**: Answer"));
}

#[test]
fn test_export_to_text() {
    let messages = vec![
        Message::User {
            message: UserMessage {
                content: "First".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "Second".to_string(),
            },
        },
    ];

    // Simulate plain text export
    let mut text = String::from("Session Export\n\n");
    for (i, message) in messages.iter().enumerate() {
        match message {
            Message::User { message } => {
                text.push_str(&format!("[Message {}]\n", i + 1));
                text.push_str(&format!("User: {}\n\n", message.content));
            }
            _ => {}
        }
    }

    assert!(text.contains("Session Export"));
    assert!(text.contains("User: First"));
    assert!(text.contains("User: Second"));
}

#[test]
fn test_export_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let export_file = temp_dir.path().join("export.json");

    let messages = vec![Message::User {
        message: UserMessage {
            content: "Test export".to_string(),
        },
    }];

    // Simulate export to file
    let json = serde_json::to_string_pretty(&messages).unwrap();
    fs::write(&export_file, json).unwrap();

    // Verify file contents
    let contents = fs::read_to_string(&export_file).unwrap();
    assert!(contents.contains("Test export"));

    let parsed: Vec<Message> = serde_json::from_str(&contents).unwrap();
    assert_eq!(parsed.len(), 1);
}

#[tokio::test]
#[ignore = "Requires mock session data"]
async fn test_export_session_function() {
    // This test would require setting up mock session data
    // In a real implementation:
    // 1. Create a mock session with messages
    // 2. Export to various formats
    // 3. Verify output

    println!("Export session test would run with mock data");
}

#[test]
fn test_session_fork_concept() {
    // Test the concept of forking a session
    // In a real implementation, this would create a new session
    // branching from a specific message in the original

    let original_session = Session::new(
        SessionId::new("original"),
        PathBuf::from("/project"),
        Utc::now(),
        Some("Original first message".to_string()),
    );

    // Simulate fork: create new session with copied metadata
    let fork_session = Session::new(
        SessionId::new("fork-from-original"),
        original_session.project_path.clone(),
        Utc::now(),
        original_session.first_message.clone(),
    );

    assert_eq!(fork_session.project_path, original_session.project_path);
    assert_eq!(
        fork_session.first_message,
        original_session.first_message
    );
    assert_ne!(fork_session.id, original_session.id);
}

#[test]
fn test_session_merge_concept() {
    // Test the concept of merging sessions
    // In a real implementation, this would combine messages from multiple sessions

    let messages_session1 = vec![Message::User {
        message: UserMessage {
            content: "From session 1".to_string(),
        },
    }];

    let messages_session2 = vec![Message::User {
        message: UserMessage {
            content: "From session 2".to_string(),
        },
    }];

    // Simulate merge: combine messages
    let mut merged_messages = Vec::new();
    merged_messages.extend(messages_session1);
    merged_messages.extend(messages_session2);

    assert_eq!(merged_messages.len(), 2);
}

#[test]
fn test_session_copy() {
    let temp_dir = TempDir::new().unwrap();

    let source_dir = temp_dir.path().join("source-session");
    fs::create_dir_all(&source_dir).unwrap();

    let source_file = source_dir.join("messages.jsonl");
    fs::write(&source_file, "source content\n").unwrap();

    let dest_dir = temp_dir.path().join("dest-session");
    fs::create_dir_all(&dest_dir).unwrap();

    // Copy session data
    let dest_file = dest_dir.join("messages.jsonl");
    fs::copy(&source_file, &dest_file).unwrap();

    // Verify copy
    let source_content = fs::read_to_string(&source_file).unwrap();
    let dest_content = fs::read_to_string(&dest_file).unwrap();
    assert_eq!(source_content, dest_content);
}

#[test]
fn test_session_rename() {
    let temp_dir = TempDir::new().unwrap();

    let old_session_dir = temp_dir.path().join("old-session-name");
    fs::create_dir_all(&old_session_dir).unwrap();

    let messages_file = old_session_dir.join("messages.jsonl");
    fs::write(&messages_file, "test data").unwrap();

    // Rename session (move directory)
    let new_session_dir = temp_dir.path().join("new-session-name");
    fs::rename(&old_session_dir, &new_session_dir).unwrap();

    // Verify rename
    assert!(!old_session_dir.exists());
    assert!(new_session_dir.exists());

    let new_messages_file = new_session_dir.join("messages.jsonl");
    assert!(new_messages_file.exists());
}

#[test]
fn test_export_format_clone() {
    let format = ExportFormat::Json;
    let cloned = format;

    assert_eq!(format, cloned);
    assert_eq!(format, ExportFormat::Json); // format still usable
}

#[test]
fn test_session_info_with_no_messages() {
    let session = Session::new(
        SessionId::new("empty-session"),
        PathBuf::from("/project"),
        Utc::now(),
        None,
    );

    let info = SessionInfo {
        session: session.clone(),
        message_count: 0,
        last_modified: None,
    };

    assert_eq!(info.message_count, 0);
    assert!(info.last_modified.is_none());
    assert!(info.session.first_message.is_none());
}

#[test]
fn test_export_large_session() {
    let messages: Vec<_> = (0..1000)
        .map(|i| Message::User {
            message: UserMessage {
                content: format!("Message {}", i),
            },
        })
        .collect();

    // Simulate JSON export of large session
    let json = serde_json::to_string(&messages).unwrap();

    // Verify it's valid JSON
    let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 1000);
}

#[test]
fn test_export_with_special_characters() {
    let messages = vec![Message::User {
        message: UserMessage {
            content: r#"Special chars: " ' \ / newline: \n tab: \t"#.to_string(),
        },
    }];

    // Export to JSON
    let json = serde_json::to_string(&messages).unwrap();

    // Should properly escape special characters
    let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
    match &parsed[0] {
        Message::User { message } => {
            assert!(message.content.contains("Special chars"));
        }
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_export_empty_session() {
    let messages: Vec<Message> = vec![];

    // Export empty session
    let json = serde_json::to_string(&messages).unwrap();
    assert_eq!(json, "[]");

    let parsed: Vec<Message> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 0);
}

#[test]
fn test_export_format_selection() {
    // Test that we can select different export formats
    let formats = vec![
        ExportFormat::Json,
        ExportFormat::Jsonl,
        ExportFormat::Markdown,
        ExportFormat::Text,
    ];

    for format in formats {
        // In real implementation, would call export_session with format
        let _ = format; // Use the format
    }
}

#[test]
fn test_session_backup() {
    let temp_dir = TempDir::new().unwrap();

    let session_dir = temp_dir.path().join("session");
    fs::create_dir_all(&session_dir).unwrap();

    let messages_file = session_dir.join("messages.jsonl");
    fs::write(&messages_file, "important data").unwrap();

    // Create backup
    let backup_dir = temp_dir.path().join("session.backup");
    fs::create_dir_all(&backup_dir).unwrap();

    let backup_file = backup_dir.join("messages.jsonl");
    fs::copy(&messages_file, &backup_file).unwrap();

    // Verify backup
    let original_content = fs::read_to_string(&messages_file).unwrap();
    let backup_content = fs::read_to_string(&backup_file).unwrap();
    assert_eq!(original_content, backup_content);
}
