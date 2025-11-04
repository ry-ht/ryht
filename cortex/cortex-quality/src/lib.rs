//! Quality Assurance
//!
//! Validation, verification, and quality checks for multi-agent workflows.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub mod validation;
pub mod verification;
pub mod testing;

pub use validation::*;
pub use verification::*;
pub use testing::*;

/// Quality coordinator
pub struct QualityCoordinator {
    validator: Validator,
    verifier: Verifier,
}

impl QualityCoordinator {
    pub fn new() -> Self {
        Self {
            validator: Validator::new(),
            verifier: Verifier::new(),
        }
    }

    pub fn validator(&self) -> &Validator {
        &self.validator
    }

    pub fn verifier(&self) -> &Verifier {
        &self.verifier
    }
}

impl Default for QualityCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for quality operations
pub type Result<T> = std::result::Result<T, QualityError>;

/// Quality errors
#[derive(Debug, thiserror::Error)]
pub enum QualityError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Quality check failed: {check_name}")]
    QualityCheckFailed { check_name: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
