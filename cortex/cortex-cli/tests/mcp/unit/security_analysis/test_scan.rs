#[cfg(test)]
mod tests {
    use cortex_cli::mcp::tools::security_analysis::{SecurityAnalysisContext, SecurityScanTool};
    use cortex_storage::ConnectionManager;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup() -> (SecurityScanTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityScanTool::new(ctx);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_scan_unsafe_rust_code() {
        let (tool, temp_dir) = setup();

        // Create a test file with unsafe code
        let test_file = temp_dir.path().join("test.rs");
        fs::write(
            &test_file,
            r#"
fn dangerous_function() {
    unsafe {
        let ptr = std::ptr::null_mut::<u32>();
        *ptr = 42;
    }
}
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": ["unsafe_code"],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            assert_eq!(contents.len(), 1);
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
                assert!(output["findings"].as_array().unwrap().len() > 0);
            } else {
                panic!("Expected JSON content");
            }
        } else {
            panic!("Expected Content result");
        }
    }

    #[tokio::test]
    async fn test_scan_hardcoded_secrets() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("config.rs");
        fs::write(
            &test_file,
            r#"
const API_KEY: &str = "sk_live_abc123def456ghi789jkl012mno";
const PASSWORD: &str = "SuperSecret123!";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": ["hardcoded_secrets"],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() >= 1);
                let findings = output["findings"].as_array().unwrap();
                assert!(findings.iter().any(|f| f["category"] == "hardcoded_secrets"));
            }
        }
    }

    #[tokio::test]
    async fn test_scan_sql_injection() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("database.rs");
        fs::write(
            &test_file,
            r#"
fn query_user(id: &str) -> String {
    let query = format!("SELECT * FROM users WHERE id = {}", id);
    query
}
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": ["injection"],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let findings = output["findings"].as_array().unwrap();
                assert!(findings.iter().any(|f| f["category"] == "injection"));
            }
        }
    }

    #[tokio::test]
    async fn test_scan_weak_crypto() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("crypto.rs");
        fs::write(
            &test_file,
            r#"
use md5;

fn hash_password(password: &str) -> String {
    format!("{:x}", md5::compute(password))
}
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": ["insecure_crypto"],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
                let findings = output["findings"].as_array().unwrap();
                assert!(findings.iter().any(|f| f["title"].as_str().unwrap().contains("MD5")));
            }
        }
    }

    #[tokio::test]
    async fn test_scan_directory() {
        let (tool, temp_dir) = setup();

        // Create multiple files with vulnerabilities
        fs::write(
            temp_dir.path().join("file1.rs"),
            "unsafe { let x = 5; }",
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("file2.rs"),
            r#"const PASSWORD: &str = "secret123";"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "scan_categories": [],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() >= 2);
            }
        }
    }

    #[tokio::test]
    async fn test_scan_severity_filtering() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(
            &test_file,
            r#"
unsafe { let x = 5; }
const PASSWORD: &str = "secret";
"#,
        )
        .unwrap();

        // Test with high severity threshold
        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": [],
            "min_severity": "high",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let findings = output["findings"].as_array().unwrap();
                // Should only include high and critical findings
                for finding in findings {
                    let severity = finding["severity"].as_str().unwrap();
                    assert!(severity == "high" || severity == "critical");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_scan_nonexistent_path() {
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityScanTool::new(ctx);

        let input = json!({
            "scope_path": "/nonexistent/path/to/file.rs",
            "scan_categories": [],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool.execute(input, &ToolContext::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_empty_file() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("empty.rs");
        fs::write(&test_file, "").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": [],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert_eq!(output["total_count"].as_i64().unwrap(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_scan_with_category_filter() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(
            &test_file,
            r#"
unsafe { let x = 5; }
const PASSWORD: &str = "secret123";
use md5;
"#,
        )
        .unwrap();

        // Only scan for unsafe_code
        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "scan_categories": ["unsafe_code"],
            "min_severity": "low",
            "include_dependencies": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let findings = output["findings"].as_array().unwrap();
                // All findings should be unsafe_code category
                for finding in findings {
                    assert_eq!(finding["category"].as_str().unwrap(), "unsafe_code");
                }
            }
        }
    }
}
