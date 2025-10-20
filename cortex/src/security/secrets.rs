// Secrets detection patterns and logic

use super::types::{SecretPattern, SecurityLevel, VulnerabilityType};
use regex::Regex;
use once_cell::sync::Lazy;

/// Get all secret detection patterns
pub fn get_secret_patterns() -> Vec<SecretPattern> {
    vec![
        // AWS Access Key
        SecretPattern {
            name: "AWS Access Key ID".to_string(),
            pattern: r"(?i)(?:AKIA|ASIA|AIDA|AROA|AIPA|ANPA|ANVA|AGPA)[0-9A-Z]{16}".to_string(),
            vuln_type: VulnerabilityType::AwsAccessKey,
            level: SecurityLevel::Critical,
            description: "AWS Access Key ID detected in code".to_string(),
            remediation: "Remove hardcoded AWS credentials. Use environment variables or AWS IAM roles.".to_string(),
        },

        // AWS Secret Key
        SecretPattern {
            name: "AWS Secret Access Key".to_string(),
            pattern: r"(?i)aws(.{0,20})?(?:secret)(.{0,20})?['\"][0-9a-zA-Z/+=]{40}['\"]".to_string(),
            vuln_type: VulnerabilityType::AwsSecretKey,
            level: SecurityLevel::Critical,
            description: "AWS Secret Access Key detected in code".to_string(),
            remediation: "Remove hardcoded AWS credentials. Use environment variables or AWS IAM roles.".to_string(),
        },

        // GitHub Token
        SecretPattern {
            name: "GitHub Token".to_string(),
            pattern: r"(?i)github(.{0,20})?['\"]?[0-9a-zA-Z]{35,40}['\"]?".to_string(),
            vuln_type: VulnerabilityType::GithubToken,
            level: SecurityLevel::Critical,
            description: "GitHub personal access token detected in code".to_string(),
            remediation: "Remove hardcoded GitHub token. Use GitHub secrets or environment variables.".to_string(),
        },

        // Private SSH Key
        SecretPattern {
            name: "SSH Private Key".to_string(),
            pattern: r"-----BEGIN (?:RSA|DSA|EC|OPENSSH) PRIVATE KEY-----".to_string(),
            vuln_type: VulnerabilityType::PrivateKey,
            level: SecurityLevel::Critical,
            description: "SSH private key detected in code".to_string(),
            remediation: "Remove private key from code. Store securely in ~/.ssh or use secret management.".to_string(),
        },

        // JWT Secret
        SecretPattern {
            name: "JWT Secret".to_string(),
            pattern: r"(?i)jwt(.{0,20})?secret(.{0,20})?['\"][0-9a-zA-Z_\-]{32,}['\"]".to_string(),
            vuln_type: VulnerabilityType::JwtSecret,
            level: SecurityLevel::High,
            description: "JWT secret key detected in code".to_string(),
            remediation: "Move JWT secret to environment variables or secure configuration.".to_string(),
        },

        // Generic API Key
        SecretPattern {
            name: "API Key".to_string(),
            pattern: r"(?i)api[_-]?key['\"]?\s*[:=]\s*['\"]?[0-9a-zA-Z_\-]{32,}['\"]?".to_string(),
            vuln_type: VulnerabilityType::ApiKey,
            level: SecurityLevel::High,
            description: "API key detected in code".to_string(),
            remediation: "Remove hardcoded API key. Use environment variables or secret management.".to_string(),
        },

        // Hardcoded Password
        SecretPattern {
            name: "Hardcoded Password".to_string(),
            pattern: r"(?i)password['\"]?\s*[:=]\s*['\"][^\s]{8,}['\"]".to_string(),
            vuln_type: VulnerabilityType::Password,
            level: SecurityLevel::High,
            description: "Hardcoded password detected in code".to_string(),
            remediation: "Remove hardcoded password. Use environment variables or authentication service.".to_string(),
        },

        // Database Connection String
        SecretPattern {
            name: "Database Connection String".to_string(),
            pattern: r"(?i)(postgres|mysql|mongodb)://[a-zA-Z0-9_-]+:[a-zA-Z0-9_@.-]+@[a-zA-Z0-9.-]+".to_string(),
            vuln_type: VulnerabilityType::Password,
            level: SecurityLevel::Critical,
            description: "Database connection string with credentials detected".to_string(),
            remediation: "Remove connection string with credentials. Use environment variables.".to_string(),
        },

        // Slack Webhook
        SecretPattern {
            name: "Slack Webhook".to_string(),
            pattern: r"https://hooks\.slack\.com/services/T[0-9A-Z]{8}/B[0-9A-Z]{8}/[0-9a-zA-Z]{24}".to_string(),
            vuln_type: VulnerabilityType::ApiKey,
            level: SecurityLevel::High,
            description: "Slack webhook URL detected in code".to_string(),
            remediation: "Remove Slack webhook URL. Use environment variables.".to_string(),
        },

        // Generic Secret Pattern
        SecretPattern {
            name: "Generic Secret".to_string(),
            pattern: r"(?i)(secret|token|key|password|passwd|pwd|api[_-]?key)['\"]?\s*[:=]\s*['\"][^\s]{16,}['\"]".to_string(),
            vuln_type: VulnerabilityType::ApiKey,
            level: SecurityLevel::Medium,
            description: "Potential secret or credential detected in code".to_string(),
            remediation: "Review if this is sensitive data. If so, move to environment variables.".to_string(),
        },
    ]
}

/// Compiled regex cache
static COMPILED_PATTERNS: Lazy<Vec<(SecretPattern, Regex)>> = Lazy::new(|| {
    get_secret_patterns()
        .into_iter()
        .filter_map(|pattern| {
            match Regex::new(&pattern.pattern) {
                Ok(regex) => Some((pattern, regex)),
                Err(e) => {
                    eprintln!("Failed to compile pattern {}: {}", pattern.name, e);
                    None
                }
            }
        })
        .collect()
});

/// Scan content for secrets
pub fn scan_for_secrets(content: &str, file_path: &str) -> Vec<(SecretPattern, usize, String)> {
    let mut findings = Vec::new();

    for (pattern, regex) in COMPILED_PATTERNS.iter() {
        for (line_num, line) in content.lines().enumerate() {
            // Skip comments (simple heuristic)
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
                continue;
            }

            // Check for matches
            if let Some(mat) = regex.find(line) {
                // Calculate confidence based on context
                let confidence = calculate_confidence(line, &mat.as_str(), &pattern.name);

                if confidence >= 0.6 {
                    findings.push((
                        SecretPattern {
                            metadata: std::collections::HashMap::from([
                                ("confidence".to_string(), confidence.to_string()),
                            ]),
                            ..pattern.clone()
                        },
                        line_num + 1,
                        line.to_string(),
                    ));
                }
            }
        }
    }

    findings
}

/// Calculate confidence score based on context
fn calculate_confidence(line: &str, matched_text: &str, pattern_name: &str) -> f32 {
    let mut confidence = 0.8; // Base confidence

    let line_lower = line.to_lowercase();

    // Reduce confidence if it looks like an example or placeholder
    if line_lower.contains("example") || line_lower.contains("placeholder")
        || line_lower.contains("dummy") || line_lower.contains("test") {
        confidence -= 0.3;
    }

    // Reduce confidence for obvious placeholders
    if matched_text.contains("xxx") || matched_text.contains("***")
        || matched_text == "your_api_key_here" || matched_text == "changeme" {
        confidence -= 0.5;
    }

    // Increase confidence if in config files
    if line_lower.contains("config") || line_lower.contains("env") {
        confidence += 0.1;
    }

    // Increase confidence for AWS patterns (very specific)
    if pattern_name.contains("AWS") {
        confidence += 0.1;
    }

    confidence.max(0.0).min(1.0)
}

impl SecretPattern {
    pub fn metadata(&mut self) -> &mut std::collections::HashMap<String, String> {
        &mut std::collections::HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_access_key_detection() {
        let content = r#"
            const AWS_KEY = "AKIAIOSFODNN7EXAMPLE";
        "#;
        let findings = scan_for_secrets(content, "test.ts");
        assert!(findings.len() > 0);
        assert_eq!(findings[0].0.vuln_type, VulnerabilityType::AwsAccessKey);
    }

    #[test]
    fn test_hardcoded_password_detection() {
        let content = r#"
            const password = "mySecretPassword123";
        "#;
        let findings = scan_for_secrets(content, "test.ts");
        assert!(findings.len() > 0);
    }

    #[test]
    fn test_no_false_positive_on_comments() {
        let content = r#"
            // Example: const password = "dontDetectThis";
        "#;
        let findings = scan_for_secrets(content, "test.ts");
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn test_confidence_calculation() {
        let line_example = "const API_KEY = 'example_key_here';";
        let confidence = calculate_confidence(line_example, "example_key_here", "API Key");
        assert!(confidence < 0.6); // Should be low due to "example"

        let line_real = "const API_KEY = 'sk-abc123def456ghi789';";
        let confidence_real = calculate_confidence(line_real, "sk-abc123def456ghi789", "API Key");
        assert!(confidence_real >= 0.6); // Should be high
    }
}
