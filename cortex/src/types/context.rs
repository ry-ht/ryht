use super::{SymbolId, TokenCount};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimized context for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedContext {
    pub content: String,
    pub compression_ratio: f32,
    pub strategy: CompressionStrategy,
    pub quality: f32,
    pub token_count: TokenCount,
}

/// Compression strategy used
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionStrategy {
    None,
    RemoveComments,
    RemoveWhitespace,
    AbstractToSignatures,
    Summarize,
    ExtractKeyPoints,
    UltraCompact,
    /// Only signatures and interfaces
    Skeleton,
    /// Natural language summaries
    Summary,
    /// Remove unused code
    TreeShaking,
    /// Combine multiple strategies
    Hybrid,
}

/// Semantic bridge between context fragments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticBridge {
    pub from: String,
    pub to: String,
    pub connection: String,
    pub transition_text: String,
}

/// Context request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub files: Vec<String>,
    pub symbols: Vec<SymbolId>,
    pub max_tokens: Option<TokenCount>,
}

/// Attention pattern from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionPattern {
    pub focused_symbols: HashMap<SymbolId, f32>,
    pub predicted_next: Vec<SymbolId>,
}

/// Working context in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingContext {
    pub symbols: Vec<SymbolId>,
    pub attention_weights: HashMap<SymbolId, f32>,
    pub total_tokens: TokenCount,
}

/// Context fragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFragment {
    pub id: String,
    pub content: String,
    pub source: String,
    pub tokens: TokenCount,
}

/// Unified context after defragmentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedContext {
    pub main_narrative: String,
    pub support_fragments: Vec<ContextFragment>,
    pub total_tokens: TokenCount,
}

/// LLM adapter type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LLMAdapter {
    Claude3 {
        context_window: usize,
    },
    GPT4 {
        context_window: usize,
    },
    Gemini {
        context_window: usize,
    },
    Custom {
        name: String,
        context_window: usize,
    },
}

impl LLMAdapter {
    pub fn context_window(&self) -> usize {
        match self {
            LLMAdapter::Claude3 { context_window } => *context_window,
            LLMAdapter::GPT4 { context_window } => *context_window,
            LLMAdapter::Gemini { context_window } => *context_window,
            LLMAdapter::Custom { context_window, .. } => *context_window,
        }
    }

    pub fn claude3() -> Self {
        LLMAdapter::Claude3 {
            context_window: 200_000,
        }
    }

    pub fn gpt4() -> Self {
        LLMAdapter::GPT4 {
            context_window: 128_000,
        }
    }

    pub fn gemini() -> Self {
        LLMAdapter::Gemini {
            context_window: 1_000_000,
        }
    }

    pub fn custom(context_window: usize) -> Self {
        LLMAdapter::Custom {
            name: "custom".to_string(),
            context_window,
        }
    }
}

impl OptimizedContext {
    pub fn new(content: String, token_count: TokenCount) -> Self {
        Self {
            content,
            compression_ratio: 1.0,
            strategy: CompressionStrategy::None,
            quality: 1.0,
            token_count,
        }
    }

    pub fn quality_score(&self) -> f32 {
        self.quality
    }
}

/// Defragmented context result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefragmentedContext {
    pub content: String,
    pub bridges: Vec<SemanticBridge>,
    pub narrative: String,
    pub token_count: TokenCount,
}

/// Attention history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionHistoryEntry {
    pub timestamp: u64,
    pub pattern: AttentionPattern,
    pub query_context: String,
}

/// Predicted focus areas with probabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedFocus {
    pub high_probability: Vec<SymbolId>,
    pub medium_probability: Vec<SymbolId>,
    pub context: Vec<SymbolId>,
    pub confidence: f32,
}

/// Query with metadata for attention analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextQuery {
    pub text: String,
    pub symbols: Vec<SymbolId>,
    pub context_size: TokenCount,
    pub timestamp: u64,
}
