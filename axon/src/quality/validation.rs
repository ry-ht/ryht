//! Input and configuration validation

use super::*;

pub struct Validator {
    rules: Vec<ValidationRule>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            rules: vec![
                ValidationRule {
                    name: "non_empty_input".to_string(),
                    description: "Input must not be empty".to_string(),
                },
                ValidationRule {
                    name: "valid_task_type".to_string(),
                    description: "Task type must be valid".to_string(),
                },
            ],
        }
    }

    pub fn validate(&self, input: &str) -> Result<ValidationReport> {
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        for rule in &self.rules {
            let result = self.apply_rule(rule, input);
            if result {
                passed.push(rule.name.clone());
            } else {
                failed.push(rule.name.clone());
            }
        }

        Ok(ValidationReport {
            passed,
            failed,
            success: failed.is_empty(),
        })
    }

    fn apply_rule(&self, rule: &ValidationRule, input: &str) -> bool {
        match rule.name.as_str() {
            "non_empty_input" => !input.is_empty(),
            "valid_task_type" => true, // Placeholder
            _ => true,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub success: bool,
}
