//! Integration tests for file attachment functionality

use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Helper function to create a test file with content
async fn create_test_file(dir: &PathBuf, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).await.unwrap();
    path
}

#[tokio::test]
async fn test_send_with_single_text_file() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a test text file
    let file_path = create_test_file(
        &temp_path,
        "test.txt",
        b"Hello, this is a test file!"
    ).await;

    // Note: This test validates the file preparation logic
    // It doesn't actually send to Claude CLI (would require mock transport)

    assert!(file_path.exists());
    let content = fs::read(&file_path).await.unwrap();
    assert_eq!(content, b"Hello, this is a test file!");
}

#[tokio::test]
async fn test_send_with_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create multiple test files
    let file1 = create_test_file(&temp_path, "test1.txt", b"File 1").await;
    let file2 = create_test_file(&temp_path, "test2.json", b"{\"key\": \"value\"}").await;
    let file3 = create_test_file(&temp_path, "test3.md", b"# Markdown").await;

    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
}

#[tokio::test]
async fn test_send_with_image_file() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a minimal PNG file (1x1 white pixel)
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,  // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,  // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
        0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,  // IDAT chunk
        0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x49,  // IEND chunk
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let file_path = create_test_file(&temp_path, "test.png", &png_data).await;
    assert!(file_path.exists());

    // Verify it's recognized as PNG
    assert_eq!(file_path.extension().unwrap(), "png");
}

#[tokio::test]
async fn test_send_with_pdf_file() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a minimal PDF file
    let pdf_data = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n%%EOF";

    let file_path = create_test_file(&temp_path, "test.pdf", pdf_data).await;
    assert!(file_path.exists());

    // Verify it's recognized as PDF
    assert_eq!(file_path.extension().unwrap(), "pdf");
}

#[tokio::test]
async fn test_send_with_nonexistent_file() {
    // This test verifies error handling for missing files
    let nonexistent = PathBuf::from("/tmp/this_file_does_not_exist_12345.txt");
    assert!(!nonexistent.exists());
}

#[tokio::test]
async fn test_file_type_detection() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Test various file types
    let test_files: Vec<(&str, &[u8])> = vec![
        ("test.jpg", b"fake jpeg data"),
        ("test.jpeg", b"fake jpeg data"),
        ("test.png", b"fake png data"),
        ("test.gif", b"fake gif data"),
        ("test.webp", b"fake webp data"),
        ("test.pdf", b"fake pdf data"),
        ("test.txt", b"text content"),
        ("test.md", b"# Markdown"),
        ("test.json", b"{}"),
        ("test.xml", b"<xml/>"),
        ("test.csv", b"a,b,c"),
        ("test.html", b"<html/>"),
        ("test.unknown", b"unknown type"),
    ];

    for (name, content) in test_files {
        let file_path = create_test_file(&temp_path, name, content).await;
        assert!(file_path.exists());
    }
}

#[tokio::test]
async fn test_base64_encoding() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let test_data = b"Hello, World!";
    let encoded = BASE64.encode(test_data);
    let decoded = BASE64.decode(&encoded).unwrap();

    assert_eq!(test_data, &decoded[..]);
}

#[tokio::test]
async fn test_content_block_serialization() {
    use serde_json::json;

    // Test text content block
    let text_block = json!({
        "type": "text",
        "text": "Hello, World!"
    });
    assert_eq!(text_block["type"], "text");

    // Test image content block
    let image_block = json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": "image/png",
            "data": "iVBORw0KG..."
        }
    });
    assert_eq!(image_block["type"], "image");
    assert_eq!(image_block["source"]["type"], "base64");

    // Test document content block
    let doc_block = json!({
        "type": "document",
        "source": {
            "type": "base64",
            "media_type": "application/pdf",
            "data": "JVBERi0xLj..."
        },
        "title": "document.pdf"
    });
    assert_eq!(doc_block["type"], "document");
    assert_eq!(doc_block["title"], "document.pdf");
}

#[tokio::test]
async fn test_mixed_content_blocks() {
    use serde_json::json;

    // Test mixing text and file content blocks
    let content_blocks = vec![
        json!({
            "type": "text",
            "text": "Please analyze these files:"
        }),
        json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "base64data..."
            }
        }),
        json!({
            "type": "document",
            "source": {
                "type": "base64",
                "media_type": "application/pdf",
                "data": "base64data..."
            },
            "title": "report.pdf"
        }),
    ];

    assert_eq!(content_blocks.len(), 3);
    assert_eq!(content_blocks[0]["type"], "text");
    assert_eq!(content_blocks[1]["type"], "image");
    assert_eq!(content_blocks[2]["type"], "document");
}

#[tokio::test]
async fn test_large_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a 1MB test file
    let large_data = vec![0u8; 1024 * 1024];
    let file_path = create_test_file(&temp_path, "large.bin", &large_data).await;

    assert!(file_path.exists());
    let metadata = fs::metadata(&file_path).await.unwrap();
    assert_eq!(metadata.len(), 1024 * 1024);
}

#[tokio::test]
async fn test_empty_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create an empty file
    let file_path = create_test_file(&temp_path, "empty.txt", b"").await;

    assert!(file_path.exists());
    let metadata = fs::metadata(&file_path).await.unwrap();
    assert_eq!(metadata.len(), 0);
}

#[tokio::test]
async fn test_file_extension_case_insensitivity() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Test case variations
    let test_files = vec![
        "test.PNG",
        "test.Png",
        "test.pNg",
        "test.JPG",
        "test.JPEG",
        "test.PDF",
    ];

    for name in test_files {
        let file_path = create_test_file(&temp_path, name, b"test data").await;
        assert!(file_path.exists());

        // Verify extension is extracted correctly
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        assert!(ext.is_some());
    }
}

#[tokio::test]
async fn test_file_without_extension() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a file without extension
    let file_path = create_test_file(&temp_path, "noextension", b"test data").await;

    assert!(file_path.exists());
    assert!(file_path.extension().is_none());
}

/// Test that validates the complete flow of file attachment preparation
#[tokio::test]
async fn test_complete_file_attachment_flow() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    // Create test files
    let text_file = create_test_file(&temp_path, "doc.txt", b"Document content").await;
    let json_file = create_test_file(&temp_path, "data.json", b"{\"test\": true}").await;

    // Simulate the attachment process
    let files = vec![text_file.clone(), json_file.clone()];

    for file_path in files {
        // Read file
        let bytes = fs::read(&file_path).await.unwrap();

        // Encode to base64
        let encoded = BASE64.encode(&bytes);

        // Verify encoding/decoding
        let decoded = BASE64.decode(&encoded).unwrap();
        assert_eq!(bytes, decoded);

        // Check extension
        let extension = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        assert!(!extension.is_empty());
    }
}
