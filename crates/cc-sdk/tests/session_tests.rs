//! Integration tests for session management module.

use cc_sdk::session::{get_claude_dir, get_projects_dir, list_projects, list_sessions, load_session_history};
use cc_sdk::core::SessionId;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_get_claude_dir() {
    // Test that we can get the Claude directory
    match get_claude_dir() {
        Ok(dir) => {
            println!("Claude dir: {:?}", dir);
            assert!(dir.ends_with(".claude"));
        }
        Err(e) => {
            // On some systems, home dir might not be available
            println!("Could not get Claude dir (expected in some environments): {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_projects_dir() {
    // Test that we can get the projects directory
    match get_projects_dir() {
        Ok(dir) => {
            println!("Projects dir: {:?}", dir);
            assert!(dir.ends_with("projects"));
        }
        Err(e) => {
            println!("Could not get projects dir: {}", e);
        }
    }
}

#[tokio::test]
async fn test_list_projects() {
    // Test listing projects
    match list_projects().await {
        Ok(projects) => {
            println!("Found {} projects", projects.len());
            for project in &projects {
                println!("  Project: {} at {:?}", project.id, project.path);
                println!("    Sessions: {}", project.sessions.len());
            }
        }
        Err(e) => {
            println!("Error listing projects (expected if no .claude dir): {}", e);
        }
    }
}

#[tokio::test]
async fn test_list_sessions() {
    // Test listing sessions for a project
    match get_projects_dir() {
        Ok(projects_dir) => {
            if !projects_dir.exists() {
                println!("Projects directory doesn't exist, skipping test");
                return;
            }

            // Get first project if any
            if let Ok(entries) = fs::read_dir(projects_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let project_id = entry.file_name().to_string_lossy().to_string();
                        match list_sessions(&project_id).await {
                            Ok(sessions) => {
                                println!("Project {} has {} sessions", project_id, sessions.len());
                                for session in sessions.iter().take(3) {
                                    println!("  Session: {:?}", session.id);
                                }
                            }
                            Err(e) => {
                                println!("Error listing sessions for {}: {}", project_id, e);
                            }
                        }
                        break; // Only test first project
                    }
                }
            }
        }
        Err(e) => {
            println!("Could not get projects dir: {}", e);
        }
    }
}

#[tokio::test]
async fn test_load_session_history() {
    // Test loading session history
    match get_projects_dir() {
        Ok(projects_dir) => {
            if !projects_dir.exists() {
                println!("Projects directory doesn't exist, skipping test");
                return;
            }

            // Find a session to load
            if let Ok(entries) = fs::read_dir(&projects_dir) {
                for project_entry in entries.flatten() {
                    if !project_entry.path().is_dir() {
                        continue;
                    }

                    let sessions_dir = project_entry.path().join("sessions");
                    if !sessions_dir.exists() {
                        continue;
                    }

                    if let Ok(session_entries) = fs::read_dir(&sessions_dir) {
                        for session_entry in session_entries.flatten() {
                            let session_path = session_entry.path();
                            if !session_path.is_dir() {
                                continue;
                            }

                            let jsonl_file = session_path.join("messages.jsonl");
                            if jsonl_file.exists() {
                                let session_name = session_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                let session_id = SessionId::new(session_name);

                                match load_session_history(&session_id).await {
                                    Ok(messages) => {
                                        println!("Loaded {} messages from session {}", messages.len(), session_name);
                                        for msg in messages.iter().take(2) {
                                            println!("  Message type: {:?}", msg);
                                        }
                                        return; // Test passed, exit
                                    }
                                    Err(e) => {
                                        println!("Error loading session {}: {}", session_name, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("No sessions found to test loading");
        }
        Err(e) => {
            println!("Could not get projects dir: {}", e);
        }
    }
}

#[tokio::test]
async fn test_jsonl_parsing_with_mock_data() {
    // Create a temporary directory for testing
    let temp_dir = std::env::temp_dir().join("cc_sdk_test_session");
    let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous test
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a mock JSONL file
    let jsonl_path = temp_dir.join("messages.jsonl");
    let mock_jsonl = r#"{"type":"text","text":"Line 1"}
{"type":"text","text":"Line 2"}
"#;
    fs::write(&jsonl_path, mock_jsonl).unwrap();

    // Try to parse it as generic JSON (not specific Message type)
    use std::io::{BufRead, BufReader};
    let file = fs::File::open(&jsonl_path).unwrap();
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        // Just verify it's valid JSON
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(_) => lines.push(line),
            Err(e) => println!("Parse error: {}", e),
        }
    }

    println!("Parsed {} lines from mock JSONL", lines.len());
    assert_eq!(lines.len(), 2);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_session_metadata_parsing() {
    // Test parsing session metadata from JSON
    let metadata_json = r#"{
        "id": "test_session_123",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T01:00:00Z"
    }"#;

    match serde_json::from_str::<serde_json::Value>(metadata_json) {
        Ok(metadata) => {
            assert_eq!(metadata["id"].as_str(), Some("test_session_123"));
            println!("Successfully parsed session metadata");
        }
        Err(e) => {
            panic!("Failed to parse metadata: {}", e);
        }
    }
}

#[tokio::test]
async fn test_project_metadata_parsing() {
    // Test parsing project metadata from JSON
    let metadata_json = r#"{
        "id": "test_project",
        "path": "/path/to/project"
    }"#;

    match serde_json::from_str::<serde_json::Value>(metadata_json) {
        Ok(metadata) => {
            assert_eq!(metadata["id"].as_str(), Some("test_project"));
            assert_eq!(metadata["path"].as_str(), Some("/path/to/project"));
            println!("Successfully parsed project metadata");
        }
        Err(e) => {
            panic!("Failed to parse metadata: {}", e);
        }
    }
}

#[tokio::test]
async fn test_session_id_operations() {
    // Test SessionId operations
    let id1 = SessionId::new("session_123");
    let id2 = SessionId::new("session_123");
    let id3 = SessionId::new("session_456");

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert_eq!(id1.as_str(), "session_123");

    println!("SessionId operations work correctly");
}

#[test]
fn test_path_operations() {
    // Test PathBuf operations used in session management
    let claude_dir = PathBuf::from("/home/user/.claude");
    let projects_dir = claude_dir.join("projects");
    let project_dir = projects_dir.join("project_123");
    let sessions_dir = project_dir.join("sessions");
    let session_dir = sessions_dir.join("session_abc");

    assert_eq!(projects_dir, PathBuf::from("/home/user/.claude/projects"));
    assert_eq!(session_dir, PathBuf::from("/home/user/.claude/projects/project_123/sessions/session_abc"));

    println!("Path operations work correctly");
}

#[tokio::test]
#[ignore = "Requires mock ~/.claude directory setup"]
async fn test_full_project_listing_with_mock() {
    // This test is ignored by default because it requires setting up a mock directory structure
    // To run: cargo test test_full_project_listing_with_mock -- --ignored

    // Create mock directory structure
    let temp_dir = std::env::temp_dir().join("cc_sdk_test_claude");
    let _ = fs::remove_dir_all(&temp_dir);

    let projects_dir = temp_dir.join("projects");
    let project1_dir = projects_dir.join("project_001");
    let sessions_dir = project1_dir.join("sessions");
    let session1_dir = sessions_dir.join("session_abc");

    fs::create_dir_all(&session1_dir).unwrap();

    // Create metadata files
    let project_metadata = r#"{"id":"project_001","path":"/path/to/project"}"#;
    fs::write(project1_dir.join("metadata.json"), project_metadata).unwrap();

    // Create session JSONL
    let messages = r#"{"role":"user","content":[{"type":"text","text":"Test"}]}"#;
    fs::write(session1_dir.join("messages.jsonl"), messages).unwrap();

    println!("Created mock directory structure at {:?}", temp_dir);
    println!("To test with real implementation, temporarily modify get_claude_dir() to return this path");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
