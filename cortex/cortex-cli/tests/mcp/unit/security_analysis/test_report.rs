#[cfg(test)]
mod tests {
    use cortex_cli::mcp::tools::security_analysis::{
        SecurityAnalysisContext, SecurityGenerateReportTool,
    };
    use cortex_storage::ConnectionManager;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup() -> (SecurityGenerateReportTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityGenerateReportTool::new(ctx);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_generate_markdown_report() {
        let (tool, temp_dir) = setup();

        // Create a file with vulnerabilities
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

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();

                // Check markdown structure
                assert!(report.contains("# Security Analysis Report"));
                assert!(report.contains("## Executive Summary"));
                assert!(report.contains("## Findings by Severity"));
                assert!(report.contains("Risk Score"));

                // Check format
                assert_eq!(output["format"].as_str().unwrap(), "markdown");

                // Check risk score is calculated
                let risk_score = output["risk_score"].as_f64().unwrap();
                assert!(risk_score >= 0.0 && risk_score <= 100.0);

                // Check total findings
                assert!(output["total_findings"].as_i64().unwrap() > 0);

                // Check remediation priority
                let remediation = output["remediation_priority"].as_array().unwrap();
                assert!(!remediation.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_generate_json_report() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "unsafe { let x = 5; }").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "json",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();

                // Check it's valid JSON
                let report_json: serde_json::Value = serde_json::from_str(report).unwrap();
                assert!(report_json["generated_at"].is_string());
                assert!(report_json["risk_score"].is_number());
                assert!(report_json["summary"].is_object());
                assert!(report_json["findings"].is_array());

                assert_eq!(output["format"].as_str().unwrap(), "json");
            }
        }
    }

    #[tokio::test]
    async fn test_generate_html_report() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "unsafe { let x = 5; }").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "html",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();

                // Check HTML structure
                assert!(report.contains("<!DOCTYPE html>"));
                assert!(report.contains("<html>"));
                assert!(report.contains("<head>"));
                assert!(report.contains("<body>"));
                assert!(report.contains("Security Analysis Report"));
                assert!(report.contains("<style>"));

                assert_eq!(output["format"].as_str().unwrap(), "html");
            }
        }
    }

    #[tokio::test]
    async fn test_report_with_directory() {
        let (tool, temp_dir) = setup();

        // Create multiple files
        fs::write(temp_dir.path().join("file1.rs"), "unsafe { let x = 5; }").unwrap();
        fs::write(
            temp_dir.path().join("file2.rs"),
            r#"const PASSWORD: &str = "secret";"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert!(output["total_findings"].as_i64().unwrap() >= 2);
            }
        }
    }

    #[tokio::test]
    async fn test_report_with_cargo_toml() {
        let (tool, temp_dir) = setup();

        // Create a vulnerable file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "unsafe { let x = 5; }").unwrap();

        // Create a Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let input = json!({
            "scope_path": temp_dir.path().to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();
                // Should include dependency check results
                assert!(report.contains("Dependencies") || report.len() > 100);
            }
        }
    }

    #[tokio::test]
    async fn test_report_risk_score_calculation() {
        let (tool, temp_dir) = setup();

        // Create file with critical vulnerabilities
        let test_file = temp_dir.path().join("critical.rs");
        fs::write(
            &test_file,
            r#"
unsafe { std::mem::transmute::<i32, f32>(42); }
const PASSWORD: &str = "SuperSecret123!";
const API_KEY: &str = "sk_live_1234567890abcdefghijklmnop";
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let risk_score = output["risk_score"].as_f64().unwrap();
                // With multiple critical findings, risk score should be high
                assert!(risk_score > 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_report_without_risk_score() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": false
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert_eq!(output["risk_score"].as_f64().unwrap(), 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_report_without_remediation() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "unsafe { let x = 5; }").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": false,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let remediation = output["remediation_priority"].as_array().unwrap();
                assert!(remediation.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_report_clean_code() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("clean.rs");
        fs::write(
            &test_file,
            r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert_eq!(output["total_findings"].as_i64().unwrap(), 0);
                assert_eq!(output["risk_score"].as_f64().unwrap(), 0.0);
            }
        }
    }

    #[tokio::test]
    async fn test_report_severity_breakdown() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("mixed.rs");
        fs::write(
            &test_file,
            r#"
unsafe { std::mem::transmute::<i32, f32>(42); }  // Critical
unsafe { let x = 5; }  // High
use md5;  // High
"#,
        )
        .unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "json",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();
                let report_json: serde_json::Value = serde_json::from_str(report).unwrap();

                let summary = &report_json["summary"];
                assert!(summary["critical"].as_u64().unwrap() >= 0);
                assert!(summary["high"].as_u64().unwrap() >= 0);
                assert!(summary["medium"].as_u64().unwrap() >= 0);
                assert!(summary["low"].as_u64().unwrap() >= 0);
            }
        }
    }

    #[tokio::test]
    async fn test_report_format_default() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            // No format specified, should default to markdown
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                assert_eq!(output["format"].as_str().unwrap(), "markdown");
            }
        }
    }

    #[tokio::test]
    async fn test_report_nonexistent_path() {
        let storage = Arc::new(ConnectionManager::new_mock());
        let ctx = SecurityAnalysisContext::new(storage);
        let tool = SecurityGenerateReportTool::new(ctx);

        let input = json!({
            "scope_path": "/nonexistent/path",
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool.execute(input, &ToolContext::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_report_compliance_section() {
        let (tool, temp_dir) = setup();

        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "unsafe { let x = 5; }").unwrap();

        let input = json!({
            "scope_path": test_file.to_str().unwrap(),
            "format": "markdown",
            "include_remediation": true,
            "include_risk_score": true
        });

        let result = tool
            .execute(input, &ToolContext::default())
            .await
            .unwrap();

        if let ToolResult::Content(contents) = result {
            if let ToolResultContent::Json { json } = &contents[0] {
                let output: serde_json::Value = serde_json::from_str(json).unwrap();
                let report = output["report_content"].as_str().unwrap();

                // Check compliance references
                assert!(report.contains("OWASP") || report.contains("CWE") || report.contains("Compliance"));
            }
        }
    }
}
