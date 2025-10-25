//! Output verification and correctness checks

use super::*;

pub struct Verifier {
    checks: Vec<VerificationCheck>,
}

impl Verifier {
    pub fn new() -> Self {
        Self {
            checks: vec![
                VerificationCheck {
                    name: "output_format".to_string(),
                    description: "Output has correct format".to_string(),
                },
                VerificationCheck {
                    name: "completeness".to_string(),
                    description: "Output is complete".to_string(),
                },
            ],
        }
    }

    pub fn verify(&self, output: &str) -> Result<VerificationReport> {
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        for check in &self.checks {
            let result = self.apply_check(check, output);
            if result {
                passed.push(check.name.clone());
            } else {
                failed.push(check.name.clone());
            }
        }

        Ok(VerificationReport {
            passed,
            failed,
            success: failed.is_empty(),
        })
    }

    fn apply_check(&self, check: &VerificationCheck, output: &str) -> bool {
        match check.name.as_str() {
            "output_format" => !output.is_empty(),
            "completeness" => output.len() > 10, // Placeholder
            _ => true,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub success: bool,
}
