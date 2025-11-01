//! Tests for VFS file type restrictions
//!
//! These tests verify that VFS correctly rejects write operations on code files
//! and only allows operations on documents, reports, and configuration files.

use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy,
};
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn create_test_vfs() -> (Arc<VirtualFileSystem>, Arc<ConnectionManager>) {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 0,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(30)),
            max_lifetime: Some(Duration::from_secs(60)),
            retry_policy: RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: false,
            recycle_after_uses: Some(10000),
            shutdown_grace_period: Duration::from_secs(30),
        },
        namespace: format!("test_{}", Uuid::new_v4()),
        database: "test".to_string(),
    };

    let storage = Arc::new(ConnectionManager::new(config).await.unwrap());
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

    (vfs, storage)
}

#[tokio::test]
async fn test_reject_rust_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("src/main.rs").unwrap();
    let content = b"fn main() {}";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .rs file creation");

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("VFS write operations are not allowed for code files"),
        "Error message should explain restriction: {}",
        error_msg
    );
    assert!(
        error_msg.contains("AI agents should edit code files directly"),
        "Error message should provide guidance: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_reject_typescript_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Test .ts file
    let path = VirtualPath::new("src/index.ts").unwrap();
    let content = b"const x: string = 'hello';";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .ts file creation");

    // Test .tsx file
    let path = VirtualPath::new("src/App.tsx").unwrap();
    let content = b"export const App = () => <div>Hello</div>;";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .tsx file creation");
}

#[tokio::test]
async fn test_reject_javascript_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Test .js file
    let path = VirtualPath::new("src/index.js").unwrap();
    let content = b"console.log('hello');";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .js file creation");

    // Test .jsx file
    let path = VirtualPath::new("src/App.jsx").unwrap();
    let content = b"export const App = () => <div>Hello</div>;";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .jsx file creation");
}

#[tokio::test]
async fn test_reject_python_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("src/main.py").unwrap();
    let content = b"print('hello')";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .py file creation");
}

#[tokio::test]
async fn test_reject_various_code_files() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let code_files = vec![
        ("test.go", b"package main" as &[u8]),
        ("test.java", b"public class Test {}"),
        ("test.cpp", b"int main() {}"),
        ("test.c", b"int main() {}"),
        ("test.h", b"#ifndef TEST_H"),
        ("test.hpp", b"#ifndef TEST_HPP"),
        ("test.cs", b"class Program {}"),
        ("test.rb", b"puts 'hello'"),
        ("test.php", b"<?php echo 'hello'; ?>"),
        ("test.swift", b"print(\"hello\")"),
        ("test.kt", b"fun main() {}"),
        ("test.scala", b"object Main {}"),
        ("test.sh", b"#!/bin/bash"),
        ("test.bash", b"#!/bin/bash"),
    ];

    for (filename, content) in code_files {
        let path = VirtualPath::new(filename).unwrap();
        let result = vfs.create_file(&workspace_id, &path, content).await;
        assert!(
            result.is_err(),
            "Should reject {} file creation",
            filename
        );
    }
}

#[tokio::test]
async fn test_allow_markdown_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("README.md").unwrap();
    let content = b"# Hello World\n\nThis is a test document.";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_ok(), "Should allow .md file creation");

    // Verify we can read it back
    let read_content = vfs.read_file(&workspace_id, &path).await;
    assert!(read_content.is_ok());
    assert_eq!(read_content.unwrap(), content);
}

#[tokio::test]
async fn test_allow_text_file_creation() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("notes.txt").unwrap();
    let content = b"Some notes here";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_ok(), "Should allow .txt file creation");
}

#[tokio::test]
async fn test_allow_config_files() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let config_files = vec![
        ("config.json", b"{\"key\": \"value\"}" as &[u8]),
        ("config.yaml", b"key: value"),
        ("config.yml", b"key: value"),
        ("config.toml", b"[section]\nkey = \"value\""),
        ("config.xml", b"<config><key>value</key></config>"),
        (".env", b"KEY=value"),
    ];

    for (filename, content) in config_files {
        let path = VirtualPath::new(filename).unwrap();
        let result = vfs.create_file(&workspace_id, &path, content).await;
        assert!(
            result.is_ok(),
            "Should allow {} file creation",
            filename
        );
    }
}

#[tokio::test]
async fn test_reject_code_file_write() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("main.rs").unwrap();
    let content = b"fn main() {}";

    let result = vfs.write_file(&workspace_id, &path, content).await;
    assert!(result.is_err(), "Should reject .rs file write");

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("VFS write operations are not allowed for code files"),
        "Error message should explain restriction: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_reject_code_file_update() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // First, create a document file (allowed)
    let doc_path = VirtualPath::new("README.md").unwrap();
    let doc_content = b"# Original content";
    vfs.create_file(&workspace_id, &doc_path, doc_content).await.unwrap();

    // Now try to update it (should still work for documents)
    let new_content = b"# Updated content";
    let result = vfs.update_file(&workspace_id, &doc_path, new_content).await;
    assert!(result.is_ok(), "Should allow updating document files");

    // But trying to update a code file path should be rejected
    // even if we haven't created it yet
    let code_path = VirtualPath::new("test.rs").unwrap();
    let code_content = b"fn test() {}";

    let result = vfs.update_file(&workspace_id, &code_path, code_content).await;
    assert!(result.is_err(), "Should reject code file update");
}

#[tokio::test]
async fn test_allow_document_update() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Create a markdown file
    let path = VirtualPath::new("docs/guide.md").unwrap();
    let original_content = b"# Guide\n\nOriginal version.";
    vfs.create_file(&workspace_id, &path, original_content).await.unwrap();

    // Update it
    let new_content = b"# Guide\n\nUpdated version with more details.";
    let result = vfs.update_file(&workspace_id, &path, new_content).await;
    assert!(result.is_ok(), "Should allow updating markdown files");

    // Verify the update
    let read_content = vfs.read_file(&workspace_id, &path).await.unwrap();
    assert_eq!(read_content, new_content);
}

#[tokio::test]
async fn test_error_message_quality() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    let path = VirtualPath::new("src/lib.rs").unwrap();
    let content = b"pub fn test() {}";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    let error_msg = error.to_string();

    // Check that error message is helpful
    assert!(
        error_msg.contains("src/lib.rs"),
        "Error should mention the filename: {}",
        error_msg
    );
    assert!(
        error_msg.contains("VFS write operations are not allowed"),
        "Error should explain the restriction: {}",
        error_msg
    );
    assert!(
        error_msg.contains("directly in the filesystem"),
        "Error should suggest the correct approach: {}",
        error_msg
    );
    assert!(
        error_msg.contains("documents, reports, and configuration"),
        "Error should explain what VFS is for: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_allow_no_extension_files() {
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Files without extensions should be allowed (could be docs or configs)
    let path = VirtualPath::new("LICENSE").unwrap();
    let content = b"MIT License\n\nCopyright...";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_ok(), "Should allow files without extensions");
}
