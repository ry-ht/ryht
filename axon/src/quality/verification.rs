//! Output verification and correctness checks

use super::*;
use serde_json::Value;
use regex::Regex;
use std::collections::HashSet;

pub struct Verifier {
    checks: Vec<VerificationCheck>,
    format_verifiers: HashMap<String, Box<dyn FormatVerifier>>,
}

impl Verifier {
    pub fn new() -> Self {
        let mut format_verifiers: HashMap<String, Box<dyn FormatVerifier>> = HashMap::new();
        format_verifiers.insert("json".to_string(), Box::new(JsonFormatVerifier));
        format_verifiers.insert("code".to_string(), Box::new(CodeFormatVerifier));
        format_verifiers.insert("markdown".to_string(), Box::new(MarkdownFormatVerifier));
        format_verifiers.insert("test_results".to_string(), Box::new(TestResultsVerifier));

        Self {
            checks: Self::default_checks(),
            format_verifiers,
        }
    }

    fn default_checks() -> Vec<VerificationCheck> {
        vec![
            VerificationCheck {
                name: "output_format".to_string(),
                description: "Output has correct format".to_string(),
                check_type: CheckType::Format,
                severity: Severity::Error,
            },
            VerificationCheck {
                name: "completeness".to_string(),
                description: "Output is complete".to_string(),
                check_type: CheckType::Completeness,
                severity: Severity::Warning,
            },
            VerificationCheck {
                name: "consistency".to_string(),
                description: "Output is internally consistent".to_string(),
                check_type: CheckType::Consistency,
                severity: Severity::Warning,
            },
            VerificationCheck {
                name: "correctness".to_string(),
                description: "Output appears correct".to_string(),
                check_type: CheckType::Correctness,
                severity: Severity::Error,
            },
        ]
    }

    pub fn verify(&self, output: &str) -> Result<VerificationReport> {
        self.verify_with_context(output, &VerificationContext::default())
    }

    pub fn verify_with_context(&self, output: &str, context: &VerificationContext) -> Result<VerificationReport> {
        let mut passed = Vec::new();
        let mut failed = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        for check in &self.checks {
            let result = self.apply_check(check, output, context);
            match result {
                CheckResult::Pass => passed.push(check.name.clone()),
                CheckResult::Fail(reason) => {
                    match check.severity {
                        Severity::Error => failed.push(format!("{}: {}", check.name, reason)),
                        Severity::Warning => warnings.push(format!("{}: {}", check.name, reason)),
                        Severity::Info => suggestions.push(format!("{}: {}", check.name, reason)),
                    }
                }
                CheckResult::Skip => {} // Skip this check
            }
        }

        // Apply format-specific verification if specified
        if let Some(format) = &context.expected_format {
            if let Some(verifier) = self.format_verifiers.get(format) {
                let format_result = verifier.verify(output);
                if !format_result.valid {
                    failed.extend(format_result.errors.iter().map(|e| format!("format: {}", e)));
                }
                warnings.extend(format_result.warnings);
                suggestions.extend(format_result.suggestions);
            }
        }

        let success = failed.is_empty();
        Ok(VerificationReport {
            passed,
            failed,
            warnings,
            suggestions,
            success,
        })
    }

    fn apply_check(&self, check: &VerificationCheck, output: &str, context: &VerificationContext) -> CheckResult {
        match check.name.as_str() {
            "output_format" => {
                if output.trim().is_empty() {
                    return CheckResult::Fail("Output is empty".to_string());
                }

                // Check for expected format
                if let Some(expected) = &context.expected_format {
                    match expected.as_str() {
                        "json" => {
                            if serde_json::from_str::<Value>(output).is_err() {
                                return CheckResult::Fail("Invalid JSON format".to_string());
                            }
                        }
                        "markdown" => {
                            if !output.contains('#') && !output.contains('-') && !output.contains('*') {
                                return CheckResult::Fail("Does not appear to be Markdown".to_string());
                            }
                        }
                        _ => {}
                    }
                }

                CheckResult::Pass
            }
            "completeness" => {
                // Check minimum length
                if output.len() < context.min_output_length.unwrap_or(10) {
                    return CheckResult::Fail(format!("Output too short: {} characters", output.len()));
                }

                // Check for required keywords
                if !context.required_keywords.is_empty() {
                    let missing_keywords: Vec<_> = context.required_keywords.iter()
                        .filter(|kw| !output.contains(*kw))
                        .collect();

                    if !missing_keywords.is_empty() {
                        return CheckResult::Fail(format!("Missing required content: {:?}", missing_keywords));
                    }
                }

                // Check for required sections (for structured output)
                if let Ok(json) = serde_json::from_str::<Value>(output) {
                    for field in &context.required_fields {
                        if json.get(field).is_none() {
                            return CheckResult::Fail(format!("Missing required field: {}", field));
                        }
                    }
                }

                CheckResult::Pass
            }
            "consistency" => {
                // Check for internal consistency
                if let Ok(json) = serde_json::from_str::<Value>(output) {
                    // Example: Check if referenced IDs exist
                    if let Some(tasks) = json.get("tasks").and_then(|v| v.as_array()) {
                        let task_ids: HashSet<_> = tasks.iter()
                            .filter_map(|t| t.get("id").and_then(|v| v.as_str()))
                            .collect();

                        // Check if dependencies reference valid task IDs
                        for task in tasks {
                            if let Some(deps) = task.get("dependencies").and_then(|v| v.as_array()) {
                                for dep in deps {
                                    if let Some(dep_id) = dep.as_str() {
                                        if !task_ids.contains(dep_id) {
                                            return CheckResult::Fail(format!("Invalid dependency reference: {}", dep_id));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                CheckResult::Pass
            }
            "correctness" => {
                // Basic correctness checks
                // Check for common error patterns
                let error_patterns = [
                    "error:",
                    "ERROR:",
                    "failed:",
                    "FAILED:",
                    "exception:",
                    "panic:",
                ];

                for pattern in &error_patterns {
                    if output.contains(pattern) {
                        // Check if it's an actual error or just part of the content
                        let lines_with_error: Vec<_> = output.lines()
                            .filter(|line| line.contains(pattern))
                            .collect();

                        if !lines_with_error.is_empty() && !context.allow_error_messages {
                            return CheckResult::Fail(format!("Output contains error indicators: {:?}", lines_with_error[0]));
                        }
                    }
                }

                CheckResult::Pass
            }
            _ => CheckResult::Pass,
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VerificationCheck {
    pub name: String,
    pub description: String,
    pub check_type: CheckType,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub enum CheckType {
    Format,
    Completeness,
    Consistency,
    Correctness,
    Custom,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub enum CheckResult {
    Pass,
    Fail(String),
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationContext {
    pub expected_format: Option<String>,
    pub min_output_length: Option<usize>,
    pub required_keywords: Vec<String>,
    pub required_fields: Vec<String>,
    pub allow_error_messages: bool,
}

/// Trait for format-specific verifiers
trait FormatVerifier: Send + Sync {
    fn verify(&self, output: &str) -> FormatVerificationResult;
}

struct FormatVerificationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    suggestions: Vec<String>,
}

// Format verifiers

struct JsonFormatVerifier;

impl FormatVerifier for JsonFormatVerifier {
    fn verify(&self, output: &str) -> FormatVerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        match serde_json::from_str::<Value>(output) {
            Ok(json) => {
                // Check for common JSON issues
                if json.is_null() {
                    warnings.push("JSON is null".to_string());
                }

                if let Some(obj) = json.as_object() {
                    if obj.is_empty() {
                        warnings.push("JSON object is empty".to_string());
                    }

                    // Check for null values
                    let null_fields: Vec<_> = obj.iter()
                        .filter(|(_, v)| v.is_null())
                        .map(|(k, _)| k.clone())
                        .collect();

                    if !null_fields.is_empty() {
                        warnings.push(format!("Fields with null values: {:?}", null_fields));
                    }
                }

                FormatVerificationResult {
                    valid: true,
                    errors,
                    warnings,
                    suggestions,
                }
            }
            Err(e) => {
                errors.push(format!("Invalid JSON: {}", e));

                // Try to provide helpful suggestions
                if output.contains("'") {
                    suggestions.push("JSON requires double quotes, not single quotes".to_string());
                }
                if output.ends_with(',') {
                    suggestions.push("Remove trailing comma".to_string());
                }

                FormatVerificationResult {
                    valid: false,
                    errors,
                    warnings,
                    suggestions,
                }
            }
        }
    }
}

struct CodeFormatVerifier;

impl FormatVerifier for CodeFormatVerifier {
    fn verify(&self, output: &str) -> FormatVerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for basic code patterns
        if output.trim().is_empty() {
            errors.push("No code found".to_string());
        }

        // Check for unmatched brackets
        let open_braces = output.matches('{').count();
        let close_braces = output.matches('}').count();
        if open_braces != close_braces {
            errors.push(format!("Unmatched braces: {} open, {} close", open_braces, close_braces));
        }

        let open_parens = output.matches('(').count();
        let close_parens = output.matches(')').count();
        if open_parens != close_parens {
            errors.push(format!("Unmatched parentheses: {} open, {} close", open_parens, close_parens));
        }

        // Check for common code issues
        if output.contains("TODO") || output.contains("FIXME") {
            warnings.push("Code contains TODO/FIXME comments".to_string());
        }

        if output.contains("println!") || output.contains("console.log") || output.contains("print(") {
            warnings.push("Code contains debug print statements".to_string());
        }

        FormatVerificationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }
}

struct MarkdownFormatVerifier;

impl FormatVerifier for MarkdownFormatVerifier {
    fn verify(&self, output: &str) -> FormatVerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for markdown patterns
        let has_headers = output.contains('#');
        let has_lists = output.contains('-') || output.contains('*') || output.contains("1.");
        let has_code_blocks = output.contains("```");

        if !has_headers && !has_lists && !has_code_blocks {
            warnings.push("Document lacks structure (no headers, lists, or code blocks)".to_string());
            suggestions.push("Consider adding headers with # for better organization".to_string());
        }

        // Check for unclosed code blocks
        let code_block_count = output.matches("```").count();
        if code_block_count % 2 != 0 {
            errors.push("Unclosed code block detected".to_string());
        }

        // Check for broken links
        let link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        for cap in link_regex.captures_iter(output) {
            let link_text = &cap[1];
            let link_url = &cap[2];

            if link_text.is_empty() {
                warnings.push(format!("Empty link text for URL: {}", link_url));
            }
            if link_url.is_empty() {
                warnings.push(format!("Empty URL for link: {}", link_text));
            }
        }

        FormatVerificationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }
}

struct TestResultsVerifier;

impl FormatVerifier for TestResultsVerifier {
    fn verify(&self, output: &str) -> FormatVerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Try to parse as JSON test results
        if let Ok(json) = serde_json::from_str::<Value>(output) {
            // Check for required test result fields
            let required_fields = ["total", "passed", "failed"];
            for field in &required_fields {
                if json.get(field).is_none() {
                    errors.push(format!("Missing required field in test results: {}", field));
                }
            }

            // Check test counts
            if let (Some(total), Some(passed), Some(failed)) = (
                json.get("total").and_then(|v| v.as_u64()),
                json.get("passed").and_then(|v| v.as_u64()),
                json.get("failed").and_then(|v| v.as_u64()),
            ) {
                if passed + failed > total {
                    errors.push(format!("Test count mismatch: passed({}) + failed({}) > total({})",
                        passed, failed, total));
                }

                if failed > 0 {
                    warnings.push(format!("{} test(s) failed", failed));
                }

                let success_rate = if total > 0 {
                    (passed as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                if success_rate < 80.0 {
                    warnings.push(format!("Low test success rate: {:.1}%", success_rate));
                    suggestions.push("Consider improving test coverage and fixing failing tests".to_string());
                }
            }
        } else {
            // Try to parse as text output
            if output.contains("PASS") || output.contains("FAIL") {
                let pass_count = output.matches("PASS").count();
                let fail_count = output.matches("FAIL").count();

                if fail_count > 0 {
                    warnings.push(format!("{} test(s) failed", fail_count));
                }

                if pass_count == 0 && fail_count == 0 {
                    warnings.push("No test results found".to_string());
                }
            } else {
                errors.push("Unable to parse test results".to_string());
            }
        }

        FormatVerificationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }
}
