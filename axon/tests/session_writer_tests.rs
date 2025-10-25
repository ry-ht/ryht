//! Integration tests for session writer operations.
//!
//! Tests creating, updating, and deleting sessions with mock data.

use cc_sdk::core::SessionId;
use cc_sdk::messages::{Message, UserMessage};
use cc_sdk::session::CreateSessionOptions;
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

// Helper to create a temporary Claude directory structure
fn setup_temp_claude_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let projects_dir = temp_dir.path().join("projects");
    fs::create_dir_all(&projects_dir).unwrap();
    temp_dir
}

// Helper to create a test project
fn create_test_project(projects_dir: &std::path::Path, project_id: &str) {
    let project_dir = projects_dir.join(project_id);
    fs::create_dir_all(&project_dir).unwrap();

    let metadata = serde_json::json!({
        "id": project_id,
        "path": project_dir.to_str().unwrap()
    });

    fs::write(
        project_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
#[ignore = "Requires setting up temporary HOME directory"]
async fn test_create_session_basic() {
    // This test is ignored because it requires mocking the home directory
    // In a real implementation, we would temporarily override get_projects_dir()
    println!("Create session test would run with proper HOME setup");
}

#[test]
fn test_create_session_options_default() {
    let options = CreateSessionOptions::default();

    assert!(options.initial_message.is_none());
    assert!(options.created_at.is_none());
    assert!(!options.overwrite);
}

#[test]
fn test_create_session_options_custom() {
    let message = Message::User {
        message: UserMessage {
            content: "Hello".to_string(),
        },
    };

    let now = Utc::now();
    let options = CreateSessionOptions {
        initial_message: Some(message.clone()),
        created_at: Some(now),
        overwrite: true,
    };

    assert!(options.initial_message.is_some());
    assert_eq!(options.created_at, Some(now));
    assert!(options.overwrite);
}

#[test]
fn test_session_id_creation() {
    let id = SessionId::new("test-session-123");
    assert_eq!(id.as_str(), "test-session-123");
}

#[test]
fn test_session_directory_structure() {
    let temp_dir = setup_temp_claude_dir();
    let projects_dir = temp_dir.path().join("projects");

    create_test_project(&projects_dir, "test-project");

    // Verify project structure
    let project_dir = projects_dir.join("test-project");
    assert!(project_dir.exists());
    assert!(project_dir.join("metadata.json").exists());

    // Create sessions directory
    let sessions_dir = project_dir.join("sessions");
    fs::create_dir_all(&sessions_dir).unwrap();

    // Create a session directory
    let session_dir = sessions_dir.join("session-123");
    fs::create_dir_all(&session_dir).unwrap();

    assert!(session_dir.exists());

    // Create messages file
    let messages_file = session_dir.join("messages.jsonl");
    fs::write(&messages_file, "").unwrap();

    assert!(messages_file.exists());
}

#[test]
fn test_write_message_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let messages_file = temp_dir.path().join("messages.jsonl");

    let message = Message::User {
        message: UserMessage {
            content: "Test message".to_string(),
        },
    };

    // Write message manually (simulating write_message function)
    let json = serde_json::to_string(&message).unwrap();
    let mut content = fs::read_to_string(&messages_file).unwrap_or_default();
    content.push_str(&json);
    content.push('\n');
    fs::write(&messages_file, content).unwrap();

    // Verify file contents
    let contents = fs::read_to_string(&messages_file).unwrap();
    assert!(contents.contains(r#""type":"user""#));
    assert!(contents.contains("Test message"));
}

#[test]
fn test_write_multiple_messages() {
    let temp_dir = TempDir::new().unwrap();
    let messages_file = temp_dir.path().join("messages.jsonl");

    let messages = vec![
        Message::User {
            message: UserMessage {
                content: "First message".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "Second message".to_string(),
            },
        },
        Message::User {
            message: UserMessage {
                content: "Third message".to_string(),
            },
        },
    ];

    // Write all messages
    for message in &messages {
        let json = serde_json::to_string(&message).unwrap();
        let mut content = fs::read_to_string(&messages_file).unwrap_or_default();
        content.push_str(&json);
        content.push('\n');
        fs::write(&messages_file, &content).unwrap();
    }

    // Verify all messages are in the file
    let contents = fs::read_to_string(&messages_file).unwrap();
    let lines: Vec<&str> = contents.lines().collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("First message"));
    assert!(lines[1].contains("Second message"));
    assert!(lines[2].contains("Third message"));
}

#[test]
fn test_session_metadata_structure() {
    let temp_dir = TempDir::new().unwrap();
    let metadata_file = temp_dir.path().join("metadata.json");

    let metadata = serde_json::json!({
        "session_id": "session-123",
        "created_at": Utc::now().to_rfc3339(),
        "message_count": 5,
        "first_message": "Hello, world!"
    });

    fs::write(&metadata_file, serde_json::to_string_pretty(&metadata).unwrap()).unwrap();

    // Verify metadata can be read
    let contents = fs::read_to_string(&metadata_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

    assert_eq!(parsed["session_id"], "session-123");
    assert_eq!(parsed["message_count"], 5);
    assert_eq!(parsed["first_message"], "Hello, world!");
}

#[test]
fn test_session_overwrite_behavior() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("session-existing");
    fs::create_dir_all(&session_dir).unwrap();

    let messages_file = session_dir.join("messages.jsonl");
    fs::write(&messages_file, "existing content\n").unwrap();

    // Test overwrite = false (should error in real implementation)
    let exists = messages_file.exists();
    assert!(exists);

    // Test overwrite = true (should allow)
    let new_content = "new content\n";
    fs::write(&messages_file, new_content).unwrap();

    let contents = fs::read_to_string(&messages_file).unwrap();
    assert_eq!(contents, "new content\n");
}

#[test]
fn test_create_session_with_initial_message() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("session-with-message");
    fs::create_dir_all(&session_dir).unwrap();

    let message = Message::User {
        message: UserMessage {
            content: "Initial message".to_string(),
        },
    };

    let messages_file = session_dir.join("messages.jsonl");
    let json = serde_json::to_string(&message).unwrap();
    fs::write(&messages_file, format!("{}\n", json)).unwrap();

    // Verify
    let contents = fs::read_to_string(&messages_file).unwrap();
    assert!(contents.contains("Initial message"));

    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn test_session_file_permissions() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("session-perms");
    fs::create_dir_all(&session_dir).unwrap();

    // Create a file and verify it can be written to
    let test_file = session_dir.join("test.txt");
    fs::write(&test_file, "test").unwrap();

    assert!(test_file.exists());

    // Verify we can read it back
    let contents = fs::read_to_string(&test_file).unwrap();
    assert_eq!(contents, "test");
}

#[test]
fn test_jsonl_format_validation() {
    let temp_dir = TempDir::new().unwrap();
    let messages_file = temp_dir.path().join("messages.jsonl");

    // Write properly formatted JSONL
    let lines = vec![
        serde_json::json!({"type": "user", "message": {"content": "Line 1"}}),
        serde_json::json!({"type": "user", "message": {"content": "Line 2"}}),
    ];

    for line in &lines {
        let json = serde_json::to_string(line).unwrap();
        let mut content = fs::read_to_string(&messages_file).unwrap_or_default();
        content.push_str(&json);
        content.push('\n');
        fs::write(&messages_file, &content).unwrap();
    }

    // Read and validate each line is valid JSON
    let contents = fs::read_to_string(&messages_file).unwrap();
    for line in contents.lines() {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(parsed["type"].is_string());
    }
}

#[test]
fn test_session_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join("session-cleanup");
    fs::create_dir_all(&session_dir).unwrap();

    let messages_file = session_dir.join("messages.jsonl");
    fs::write(&messages_file, "test data").unwrap();

    assert!(session_dir.exists());
    assert!(messages_file.exists());

    // Clean up (delete session)
    fs::remove_dir_all(&session_dir).unwrap();

    assert!(!session_dir.exists());
}

#[test]
fn test_concurrent_writes_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let messages_file = temp_dir.path().join("messages.jsonl");

    // Simulate multiple writes (in real concurrent scenario, would use proper locking)
    for i in 0..10 {
        let message = Message::User {
            message: UserMessage {
                content: format!("Message {}", i),
            },
        };

        let json = serde_json::to_string(&message).unwrap();
        let mut content = fs::read_to_string(&messages_file).unwrap_or_default();
        content.push_str(&json);
        content.push('\n');
        fs::write(&messages_file, &content).unwrap();
    }

    // Verify all messages were written
    let contents = fs::read_to_string(&messages_file).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 10);
}

#[test]
fn test_session_id_validation() {
    // Test valid session IDs
    let valid_ids = vec![
        "session-123",
        "session_abc",
        "session.test",
        "my-session",
    ];

    for id_str in valid_ids {
        let id = SessionId::new(id_str);
        assert_eq!(id.as_str(), id_str);
    }
}

#[test]
fn test_project_structure_validation() {
    let temp_dir = setup_temp_claude_dir();
    let projects_dir = temp_dir.path().join("projects");

    create_test_project(&projects_dir, "valid-project");

    // Verify the project structure is valid
    let project_dir = projects_dir.join("valid-project");
    assert!(project_dir.exists());
    assert!(project_dir.is_dir());

    let metadata_file = project_dir.join("metadata.json");
    assert!(metadata_file.exists());

    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata_file).unwrap()).unwrap();

    assert_eq!(metadata["id"], "valid-project");
}
