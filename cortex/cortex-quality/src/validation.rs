//! Input and configuration validation

use super::*;
use serde_json::Value;
use std::collections::HashSet;

pub struct Validator {
    rules: Vec<ValidationRule>,
    valid_task_types: HashSet<String>,
    schema_validators: HashMap<String, Box<dyn SchemaValidator>>,
}

impl Validator {
    pub fn new() -> Self {
        let mut valid_task_types = HashSet::new();
        valid_task_types.insert("development".to_string());
        valid_task_types.insert("review".to_string());
        valid_task_types.insert("testing".to_string());
        valid_task_types.insert("documentation".to_string());
        valid_task_types.insert("optimization".to_string());
        valid_task_types.insert("security".to_string());
        valid_task_types.insert("architecture".to_string());
        valid_task_types.insert("deployment".to_string());
        valid_task_types.insert("monitoring".to_string());

        let mut schema_validators: HashMap<String, Box<dyn SchemaValidator>> = HashMap::new();
        schema_validators.insert("development".to_string(), Box::new(DevelopmentTaskValidator));
        schema_validators.insert("testing".to_string(), Box::new(TestingTaskValidator));
        schema_validators.insert("review".to_string(), Box::new(ReviewTaskValidator));

        Self {
            rules: Self::default_rules(),
            valid_task_types,
            schema_validators,
        }
    }

    fn default_rules() -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "non_empty_input".to_string(),
                description: "Input must not be empty".to_string(),
                rule_type: RuleType::Required,
            },
            ValidationRule {
                name: "valid_task_type".to_string(),
                description: "Task type must be valid".to_string(),
                rule_type: RuleType::Enum,
            },
            ValidationRule {
                name: "valid_json_format".to_string(),
                description: "Input must be valid JSON if JSON is expected".to_string(),
                rule_type: RuleType::Format,
            },
            ValidationRule {
                name: "required_fields".to_string(),
                description: "All required fields must be present".to_string(),
                rule_type: RuleType::Schema,
            },
        ]
    }

    pub fn validate(&self, input: &str) -> Result<ValidationReport> {
        let mut passed = Vec::new();
        let mut failed = Vec::new();
        let mut warnings = Vec::new();

        for rule in &self.rules {
            let result = self.apply_rule(rule, input);
            match result {
                ValidationResult::Pass => passed.push(rule.name.clone()),
                ValidationResult::Fail(reason) => {
                    failed.push(format!("{}: {}", rule.name, reason));
                }
                ValidationResult::Warning(msg) => {
                    warnings.push(format!("{}: {}", rule.name, msg));
                    passed.push(rule.name.clone());
                }
            }
        }

        let success = failed.is_empty();
        Ok(ValidationReport {
            passed,
            failed,
            warnings,
            success,
        })
    }

    pub fn validate_task(&self, task_json: &str) -> Result<TaskValidationReport> {
        // Parse JSON
        let task_data: Value = serde_json::from_str(task_json)
            .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

        // Extract task type
        let task_type = task_data.get("task_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing task_type field"))?;

        // Validate task type
        if !self.valid_task_types.contains(task_type) {
            return Ok(TaskValidationReport {
                valid: false,
                task_type: task_type.to_string(),
                errors: vec![format!("Invalid task type: {}", task_type)],
                warnings: vec![],
                suggestions: vec![format!("Valid task types are: {:?}", self.valid_task_types)],
            });
        }

        // Apply schema validation if available
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        if let Some(validator) = self.schema_validators.get(task_type) {
            let schema_result = validator.validate(&task_data);
            errors.extend(schema_result.errors);
            warnings.extend(schema_result.warnings);
            suggestions.extend(schema_result.suggestions);
        }

        Ok(TaskValidationReport {
            valid: errors.is_empty(),
            task_type: task_type.to_string(),
            errors,
            warnings,
            suggestions,
        })
    }

    fn apply_rule(&self, rule: &ValidationRule, input: &str) -> ValidationResult {
        match rule.name.as_str() {
            "non_empty_input" => {
                if input.trim().is_empty() {
                    ValidationResult::Fail("Input is empty".to_string())
                } else {
                    ValidationResult::Pass
                }
            }
            "valid_task_type" => {
                // Try to extract task type from input
                if let Ok(json_val) = serde_json::from_str::<Value>(input) {
                    if let Some(task_type) = json_val.get("task_type").and_then(|v| v.as_str()) {
                        if self.valid_task_types.contains(task_type) {
                            ValidationResult::Pass
                        } else {
                            ValidationResult::Fail(format!("Unknown task type: {}", task_type))
                        }
                    } else {
                        ValidationResult::Warning("No task_type field found in JSON".to_string())
                    }
                } else {
                    // Not JSON input, skip this validation
                    ValidationResult::Pass
                }
            }
            "valid_json_format" => {
                // Check if input looks like it should be JSON
                if input.trim().starts_with('{') || input.trim().starts_with('[') {
                    if serde_json::from_str::<Value>(input).is_ok() {
                        ValidationResult::Pass
                    } else {
                        ValidationResult::Fail("Invalid JSON format".to_string())
                    }
                } else {
                    ValidationResult::Pass // Not JSON, skip
                }
            }
            "required_fields" => {
                if let Ok(json_val) = serde_json::from_str::<Value>(input) {
                    let required = ["id", "name", "task_type"];
                    let missing: Vec<_> = required.iter()
                        .filter(|field| json_val.get(*field).is_none())
                        .collect();

                    if missing.is_empty() {
                        ValidationResult::Pass
                    } else {
                        ValidationResult::Warning(format!("Missing optional fields: {:?}", missing))
                    }
                } else {
                    ValidationResult::Pass // Not JSON, skip
                }
            }
            _ => ValidationResult::Pass,
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub description: String,
    pub rule_type: RuleType,
}

#[derive(Debug, Clone)]
pub enum RuleType {
    Required,
    Enum,
    Format,
    Schema,
    Custom,
}

#[derive(Debug, Clone)]
pub enum ValidationResult {
    Pass,
    Fail(String),
    Warning(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskValidationReport {
    pub valid: bool,
    pub task_type: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Trait for schema validators
trait SchemaValidator: Send + Sync {
    fn validate(&self, data: &Value) -> SchemaValidationResult;
}

struct SchemaValidationResult {
    errors: Vec<String>,
    warnings: Vec<String>,
    suggestions: Vec<String>,
}

// Schema validators for specific task types

struct DevelopmentTaskValidator;

impl SchemaValidator for DevelopmentTaskValidator {
    fn validate(&self, data: &Value) -> SchemaValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for required fields
        if data.get("description").is_none() {
            errors.push("Missing 'description' field for development task".to_string());
        }

        if data.get("language").is_none() {
            warnings.push("No 'language' specified, will use default".to_string());
            suggestions.push("Consider specifying the programming language".to_string());
        }

        if let Some(complexity) = data.get("complexity")
            && let Some(comp_str) = complexity.as_str() {
                let valid_complexities = ["simple", "moderate", "complex"];
                if !valid_complexities.contains(&comp_str) {
                    warnings.push(format!("Unknown complexity level: {}", comp_str));
                    suggestions.push(format!("Valid complexity levels: {:?}", valid_complexities));
                }
            }

        SchemaValidationResult {
            errors,
            warnings,
            suggestions,
        }
    }
}

struct TestingTaskValidator;

impl SchemaValidator for TestingTaskValidator {
    fn validate(&self, data: &Value) -> SchemaValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for testing-specific fields
        if data.get("test_type").is_none() {
            warnings.push("No 'test_type' specified (unit, integration, e2e)".to_string());
        }

        if let Some(coverage) = data.get("target_coverage")
            && let Some(cov_num) = coverage.as_f64()
                && (!(0.0..=100.0).contains(&cov_num)) {
                    errors.push(format!("Invalid coverage target: {}", cov_num));
                    suggestions.push("Coverage should be between 0 and 100".to_string());
                }

        if data.get("test_framework").is_none() {
            suggestions.push("Consider specifying a test framework".to_string());
        }

        SchemaValidationResult {
            errors,
            warnings,
            suggestions,
        }
    }
}

struct ReviewTaskValidator;

impl SchemaValidator for ReviewTaskValidator {
    fn validate(&self, data: &Value) -> SchemaValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for review-specific fields
        if data.get("review_type").is_none() {
            warnings.push("No 'review_type' specified (code, security, architecture)".to_string());
        }

        if data.get("files_to_review").is_none() && data.get("pr_number").is_none() {
            errors.push("Either 'files_to_review' or 'pr_number' must be specified".to_string());
        }

        if let Some(checklist) = data.get("checklist") {
            if !checklist.is_array() {
                warnings.push("'checklist' should be an array of review items".to_string());
            }
        } else {
            suggestions.push("Consider adding a review checklist".to_string());
        }

        SchemaValidationResult {
            errors,
            warnings,
            suggestions,
        }
    }
}
