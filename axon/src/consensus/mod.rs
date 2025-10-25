//! Consensus Mechanisms
//!
//! Democratic coordination for multi-agent decision making including:
//! - Sangha consensus (harmonious agreement)
//! - Simple majority voting
//! - Weighted voting
//! - Byzantine fault tolerance
//! - Conflict resolution

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod voting;
pub mod sangha;
pub mod conflict;

pub use voting::*;
pub use sangha::*;
pub use conflict::*;

use crate::agents::AgentId;

/// Base trait for consensus strategies
pub trait ConsensusStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn required_quorum(&self) -> f32;
    fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult>;
}

/// Main consensus protocol implementation
pub struct ConsensusProtocol {
    strategies: HashMap<String, Box<dyn ConsensusStrategy>>,
}

impl ConsensusProtocol {
    pub fn new() -> Self {
        let mut strategies: HashMap<String, Box<dyn ConsensusStrategy>> = HashMap::new();
        strategies.insert("simple_majority".to_string(), Box::new(SimpleMajority::default()));
        strategies.insert("sangha".to_string(), Box::new(SanghaConsensus::default()));

        Self { strategies }
    }

    pub fn initiate_consensus(
        &self,
        proposal: Proposal,
        strategy_name: &str,
        participants: Vec<AgentId>,
    ) -> Result<ConsensusResult> {
        let strategy = self
            .strategies
            .get(strategy_name)
            .ok_or_else(|| ConsensusError::StrategyNotFound(strategy_name.to_string()))?;

        // Collect votes (placeholder)
        let votes = vec![];

        strategy.evaluate_votes(votes)
    }
}

/// Vote from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: AgentId,
    pub proposal_id: String,
    pub decision: Decision,
    pub confidence: f32,
    pub rationale: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Decision types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    Accept,
    Reject,
    Abstain,
    Conditional(String),
}

/// Consensus result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusResult {
    Accepted {
        support: f32,
        votes: Vec<Vote>,
        unanimous: bool,
    },
    Rejected {
        support: f32,
        votes: Vec<Vote>,
    },
    Harmonious {
        harmony_level: f32,
        rounds: usize,
        votes: Vec<Vote>,
    },
    Failed {
        reason: String,
        votes: Vec<Vote>,
    },
}

/// Proposal for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposer: AgentId,
    pub content: String,
    pub description: String,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
}

/// Result type
pub type Result<T> = std::result::Result<T, ConsensusError>;

/// Consensus errors
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Strategy not found: {0}")]
    StrategyNotFound(String),

    #[error("Insufficient quorum: required {required}, available {available}")]
    InsufficientQuorum { required: f32, available: usize },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
