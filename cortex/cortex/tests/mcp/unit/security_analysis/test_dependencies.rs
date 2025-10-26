#[cfg(test)]
mod tests {
    use cortex::mcp::tools::security_analysis::{
        SecurityAnalysisContext, SecurityCheckDependenciesTool,
    };
    use cortex_storage::ConnectionManager;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup() -> (SecurityCheckDependenciesTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityCheckDependenciesTool::new(ctx);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_check_dependencies_basic() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
regex = "1.5"
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": true,
            "check_outdated": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_vulnerabilities"].as_i64().unwrap() >= 0);
                assert!(output["total_license_issues"].as_i64().unwrap() >= 0);
                assert!(output["vulnerable_dependencies"].is_array());
                assert!(output["license_issues"].is_array());
                assert!(output["outdated_dependencies"].is_array());
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_with_dev_deps() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"

[dev-dependencies]
tempfile = "3.0"
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": cargo_file.to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": false,
            "check_outdated": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["vulnerable_dependencies"].is_array());
                assert!(output["license_issues"].as_array().unwrap().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_no_cargo_toml() {
        let (tool, temp_dir) = setup();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": true,
            "check_outdated": false
        });

        let result = tool.execute(input, &ToolContext::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_dependencies_version_parsing() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
simple = "1.0"
caret = "^1.2.3"
tilde = "~1.2"
exact = "=1.0.0"
inline = { version = "1.0", features = ["default"] }
workspace = { workspace = true }
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": true,
            "check_outdated": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                // Should parse multiple version formats
                assert!(output["outdated_dependencies"].is_array());
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_license_issues() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
gpl-crate = "1.0"
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": false,
            "check_licenses": true,
            "check_outdated": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let license_issues = output["license_issues"].as_array().unwrap();
                // Should detect GPL license
                if !license_issues.is_empty() {
                    assert!(license_issues[0]["license"].as_str().unwrap().contains("GPL"));
                }
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_selective_checks() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        // Only check vulnerabilities
        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": false,
            "check_outdated": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["vulnerable_dependencies"].is_array());
                assert!(output["license_issues"].as_array().unwrap().is_empty());
                assert!(output["outdated_dependencies"].as_array().unwrap().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_empty_cargo_toml() {
        let (tool, temp_dir) = setup();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
"#;

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": true,
            "check_outdated": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert_eq!(output["total_vulnerabilities"].as_i64().unwrap(), 0);
                assert_eq!(output["total_license_issues"].as_i64().unwrap(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_check_dependencies_malformed_cargo_toml() {
        let (tool, temp_dir) = setup();

        let cargo_toml = "this is not valid toml{[}]";

        let cargo_file = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_file, cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "check_vulnerabilities": true,
            "check_licenses": true,
            "check_outdated": true
        });

        // Should handle gracefully
        let result = tool.execute(input, &ToolContext::default()).await;
        // Malformed TOML should still parse (we use simple line parsing)
        assert!(result.is_ok());
    }
}
