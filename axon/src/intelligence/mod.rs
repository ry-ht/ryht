//! Intelligence Layer
//!
//! Optimization layer for task execution through integration with Cortex.
//! Axon handles model routing and context optimization, while Cortex provides
//! learning, knowledge graphs, and pattern recognition.
//!
//! # Components
//!
//! - Model Router: Selects optimal LLM provider based on task requirements
//! - Context Optimizer: Optimizes token usage through Cortex Context 3.0
//! - Pattern Analyzer: Extracts and applies patterns from Cortex

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub mod router;
pub mod optimizer;
pub mod patterns;

pub use router::*;
pub use optimizer::*;
pub use patterns::*;

/// Result type for intelligence operations
pub type Result<T> = std::result::Result<T, IntelligenceError>;

/// Intelligence errors
#[derive(Debug, thiserror::Error)]
pub enum IntelligenceError {
    #[error("No suitable model found for task")]
    NoSuitableModel,

    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),

    #[error("Pattern analysis failed: {0}")]
    PatternAnalysisFailed(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Intelligence coordinator
pub struct IntelligenceCoordinator {
    model_router: Arc<ModelRouter>,
    context_optimizer: Arc<ContextOptimizer>,
    pattern_analyzer: Arc<PatternAnalyzer>,
}

impl IntelligenceCoordinator {
    pub fn new(
        model_router: Arc<ModelRouter>,
        context_optimizer: Arc<ContextOptimizer>,
        pattern_analyzer: Arc<PatternAnalyzer>,
    ) -> Self {
        Self {
            model_router,
            context_optimizer,
            pattern_analyzer,
        }
    }

    pub fn model_router(&self) -> &ModelRouter {
        &self.model_router
    }

    pub fn context_optimizer(&self) -> &ContextOptimizer {
        &self.context_optimizer
    }

    pub fn pattern_analyzer(&self) -> &PatternAnalyzer {
        &self.pattern_analyzer
    }
}
