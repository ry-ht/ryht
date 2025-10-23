#[cfg(test)]
mod tests {
    use cortex_cli::mcp::tools::security_analysis::{
        SecurityAnalysisContext, SecurityAnalyzeSecretsTool,
    };
    use cortex_storage::ConnectionManager;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup() -> (SecurityAnalyzeSecretsTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityAnalyzeSecretsTool::new(ctx);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_detect_aws_access_key() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("config.rs");
        fs::write(
            &test_file,
            r#"
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": ["api_key"]
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
                let secrets = output["secrets_found"].as_array().unwrap();
                assert!(secrets
                    .iter()
                    .any(|s| s["secret_type"].as_str().unwrap().contains("AWS")));
            }
        }
    }

    #[tokio::test]
    async fn test_detect_github_token() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("auth.rs");
        fs::write(
            &test_file,
            r#"
const GITHUB_TOKEN: &str = "ghp_1234567890abcdefghijklmnopqrstuvwx";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": ["token"]
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
                let secrets = output["secrets_found"].as_array().unwrap();
                assert!(secrets
                    .iter()
                    .any(|s| s["secret_type"].as_str().unwrap().contains("GitHub")));
            }
        }
    }

    #[tokio::test]
    async fn test_detect_hardcoded_password() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("creds.rs");
        fs::write(
            &test_file,
            r#"
const PASSWORD: &str = "MySecretPassword123!";
let pwd = "AnotherPassword";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": ["password"]
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() >= 1);
                let secrets = output["secrets_found"].as_array().unwrap();
                assert!(secrets
                    .iter()
                    .any(|s| s["secret_type"].as_str().unwrap().contains("Password")));
            }
        }
    }

    #[tokio::test]
    async fn test_detect_private_key() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("keys.pem");
        fs::write(
            &test_file,
            r#"
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA1234567890abcdefg
-----END RSA PRIVATE KEY-----
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
                let secrets = output["secrets_found"].as_array().unwrap();
                assert!(secrets
                    .iter()
                    .any(|s| s["secret_type"].as_str().unwrap().contains("Private Key")));
            }
        }
    }

    #[tokio::test]
    async fn test_detect_jwt_token() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("token.rs");
        fs::write(
            &test_file,
            r#"
const JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
            }
        }
    }

    #[tokio::test]
    async fn test_detect_high_entropy_strings() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("config.rs");
        fs::write(
            &test_file,
            r#"
const SECRET: &str = "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC9dE1fG3hI5jK7lM9nO";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                // High entropy string should be detected
                assert!(output["total_count"].as_i64().unwrap() > 0);
            }
        }
    }

    #[tokio::test]
    async fn test_detect_base64_secrets() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("data.rs");
        fs::write(
            &test_file,
            r#"
const ENCODED: &str = "dGhpc2lzYXNlY3JldGtleQ==";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                // Base64 encoded data should be detected
                assert!(output["total_count"].as_i64().unwrap() > 0);
            }
        }
    }

    #[tokio::test]
    async fn test_scan_directory_for_secrets() {
        let (tool, temp_dir) = setup();

        // Create multiple files with secrets
        fs::write(
            temp_dir.path().join("config.rs"),
            r#"const API_KEY: &str = "AKIAIOSFODNN7EXAMPLE";"#,
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("auth.rs"),
            r#"const PASSWORD: &str = "SuperSecret123!";"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
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
    async fn test_secret_masking() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("secret.rs");
        fs::write(
            &test_file,
            r#"const KEY: &str = "AKIAIOSFODNN7EXAMPLE";"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let secrets = output["secrets_found"].as_array().unwrap();
                if !secrets.is_empty() {
                    let masked = secrets[0]["masked_value"].as_str().unwrap();
                    // Should be masked, not showing full value
                    assert!(masked.contains("...") || masked == "***");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_scan_env_file() {
        let (tool, temp_dir) = setup();

        let env_file = temp_dir.path().join(".env");
        fs::write(
            &env_file,
            r#"
DATABASE_URL=postgres://user:password@localhost/db
API_KEY=sk_live_1234567890abcdefg
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_count"].as_i64().unwrap() > 0);
            }
        }
    }

    #[tokio::test]
    async fn test_no_secrets_found() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("safe.rs");
        fs::write(
            &test_file,
            r#"
fn hello_world() {
    println!("Hello, world!");
}
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
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
    async fn test_confidence_scores() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("mixed.rs");
        fs::write(
            &test_file,
            r#"
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const MAYBE_SECRET: &str = "abc123def456";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "check_git_history": false,
            "secret_patterns": []
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let secrets = output["secrets_found"].as_array().unwrap();
                // Check that confidence scores are present and valid
                for secret in secrets {
                    let confidence = secret["confidence"].as_f64().unwrap();
                    assert!(confidence >= 0.0 && confidence <= 1.0);
                }
            }
        }
    }
}
