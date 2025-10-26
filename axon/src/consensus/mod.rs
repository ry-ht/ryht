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
use tokio::time::{timeout, Duration};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use futures::future::join_all;

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
    fn supports_multiple_rounds(&self) -> bool {
        false
    }
}

/// Main consensus protocol implementation
pub struct ConsensusProtocol {
    strategies: HashMap<String, Box<dyn ConsensusStrategy>>,
    vote_collector: Arc<VoteCollector>,
}

impl Default for ConsensusProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsensusProtocol {
    pub fn new() -> Self {
        let mut strategies: HashMap<String, Box<dyn ConsensusStrategy>> = HashMap::new();
        strategies.insert("simple_majority".to_string(), Box::new(SimpleMajority::default()));
        strategies.insert("sangha".to_string(), Box::new(SanghaConsensus::default()));
        strategies.insert("weighted".to_string(), Box::new(WeightedVoting::default()));
        strategies.insert("supermajority".to_string(), Box::new(SuperMajority::default()));

        Self {
            strategies,
            vote_collector: Arc::new(VoteCollector::new()),
        }
    }

    pub async fn initiate_consensus(
        &self,
        proposal: Proposal,
        strategy_name: &str,
        participants: Vec<AgentId>,
    ) -> Result<ConsensusResult> {
        let strategy = self
            .strategies
            .get(strategy_name)
            .ok_or_else(|| ConsensusError::StrategyNotFound(strategy_name.to_string()))?;

        // Check quorum
        let required_quorum = (participants.len() as f32 * strategy.required_quorum()).ceil() as usize;

        if participants.len() < required_quorum {
            return Err(ConsensusError::InsufficientQuorum {
                required: required_quorum as f32,
                available: participants.len(),
            });
        }

        // Collect votes with timeout
        let vote_timeout = Duration::from_secs(30);
        let votes = match timeout(
            vote_timeout,
            self.vote_collector.collect_votes(&proposal, &participants)
        ).await {
            Ok(votes) => votes,
            Err(_) => {
                // On timeout, use whatever votes we have collected
                self.vote_collector.get_partial_votes(&proposal.id).await
            }
        };

        // For strategies that support multiple rounds (like Sangha), handle iteratively
        if strategy.supports_multiple_rounds() && strategy_name == "sangha" {
            return self.handle_iterative_consensus(proposal, strategy.as_ref(), participants, votes).await;
        }

        strategy.evaluate_votes(votes)
    }

    async fn handle_iterative_consensus(
        &self,
        proposal: Proposal,
        strategy: &dyn ConsensusStrategy,
        participants: Vec<AgentId>,
        initial_votes: Vec<Vote>,
    ) -> Result<ConsensusResult> {
        let mut current_votes = initial_votes;
        let max_rounds = 5;

        for round in 1..=max_rounds {
            let result = strategy.evaluate_votes(current_votes.clone())?;

            // Check if harmony is achieved
            if let ConsensusResult::Harmonious { harmony_level, .. } = &result
                && *harmony_level >= 0.85 {
                    return Ok(ConsensusResult::Harmonious {
                        harmony_level: *harmony_level,
                        rounds: round,
                        votes: current_votes,
                    });
                }

            // If not harmonious and we have more rounds, collect revised votes
            if round < max_rounds {
                let vote_timeout = Duration::from_secs(20);
                let revised_votes = timeout(
                    vote_timeout,
                    self.vote_collector.collect_revised_votes(&proposal, &participants, &current_votes)
                ).await.unwrap_or_else(|_| current_votes.clone());

                current_votes = revised_votes;
            }
        }

        // After all rounds, return the final result
        Ok(ConsensusResult::Failed {
            reason: format!("Could not reach harmony after {} rounds", max_rounds),
            votes: current_votes,
        })
    }
}

/// Vote collector for gathering agent votes
struct VoteCollector {
    pending_votes: Arc<RwLock<HashMap<String, Vec<Vote>>>>,
}

impl VoteCollector {
    fn new() -> Self {
        Self {
            pending_votes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn collect_votes(
        &self,
        proposal: &Proposal,
        participants: &[AgentId],
    ) -> Vec<Vote> {
        let mut votes = Vec::new();
        let mut vote_futures = Vec::new();

        for participant in participants {
            let participant_id = participant.clone();
            let proposal_id = proposal.id.clone();
            let proposal_content = proposal.content.clone();

            // Simulate async vote collection from each agent
            let vote_future = async move {
                self.simulate_agent_vote(participant_id, proposal_id, proposal_content).await
            };

            vote_futures.push(vote_future);
        }

        // Collect all votes concurrently
        let collected_votes = join_all(vote_futures).await;

        for vote in collected_votes.into_iter().flatten() {
            votes.push(vote);
        }

        // Store votes for potential partial retrieval
        self.pending_votes.write().await.insert(proposal.id.clone(), votes.clone());

        votes
    }

    async fn collect_revised_votes(
        &self,
        proposal: &Proposal,
        participants: &[AgentId],
        previous_votes: &[Vote],
    ) -> Vec<Vote> {
        // In a real implementation, this would send previous votes to agents
        // and request revised opinions based on the group's feedback
        let mut revised_votes = Vec::new();

        for participant in participants {
            let vote = self.simulate_revised_vote(
                participant.clone(),
                proposal.id.clone(),
                proposal.content.clone(),
                previous_votes,
            ).await;

            if let Some(v) = vote {
                revised_votes.push(v);
            }
        }

        revised_votes
    }

    async fn get_partial_votes(&self, proposal_id: &str) -> Vec<Vote> {
        self.pending_votes
            .read()
            .await
            .get(proposal_id)
            .cloned()
            .unwrap_or_default()
    }

    async fn simulate_agent_vote(
        &self,
        voter: AgentId,
        proposal_id: String,
        _content: String,
    ) -> Option<Vote> {
        // Simulate agent decision-making
        // In a real implementation, this would communicate with actual agents
        use rand::Rng;
        let mut rng = rand::rng();

        let decision = if rng.random_bool(0.7) {
            Decision::Accept
        } else if rng.random_bool(0.5) {
            Decision::Reject
        } else {
            Decision::Abstain
        };

        let confidence = rng.random_range(0.5..1.0);

        Some(Vote {
            voter,
            proposal_id,
            decision,
            confidence,
            rationale: Some("Simulated vote rationale".to_string()),
            timestamp: Utc::now(),
        })
    }

    async fn simulate_revised_vote(
        &self,
        voter: AgentId,
        proposal_id: String,
        _content: String,
        previous_votes: &[Vote],
    ) -> Option<Vote> {
        // Simulate agent revising their vote based on others' opinions
        use rand::Rng;
        let mut rng = rand::rng();

        // Calculate group tendency
        let accept_ratio = previous_votes
            .iter()
            .filter(|v| matches!(v.decision, Decision::Accept))
            .count() as f32 / previous_votes.len().max(1) as f32;

        // Agents tend to move towards consensus in revision
        let decision = if accept_ratio > 0.6 && rng.random_bool(0.8) {
            Decision::Accept
        } else if accept_ratio < 0.4 && rng.random_bool(0.8) {
            Decision::Reject
        } else {
            Decision::Abstain
        };

        let confidence = rng.random_range(0.6..1.0); // Higher confidence after discussion

        Some(Vote {
            voter,
            proposal_id,
            decision,
            confidence,
            rationale: Some("Revised after group discussion".to_string()),
            timestamp: Utc::now(),
        })
    }
}

/// Weighted voting strategy
#[derive(Debug, Clone)]
pub struct WeightedVoting {
    threshold: f32,
    weights: HashMap<String, f32>, // Agent expertise weights
}

impl Default for WeightedVoting {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            weights: HashMap::new(),
        }
    }
}

impl ConsensusStrategy for WeightedVoting {
    fn name(&self) -> &str {
        "Weighted Voting"
    }

    fn required_quorum(&self) -> f32 {
        0.5
    }

    fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let mut weighted_accept = 0.0;
        let mut total_weight = 0.0;

        for vote in &votes {
            let weight = self.weights
                .get(&vote.voter.to_string())
                .copied()
                .unwrap_or(1.0);

            total_weight += weight;

            if matches!(vote.decision, Decision::Accept) {
                weighted_accept += weight * vote.confidence;
            }
        }

        let support = if total_weight > 0.0 {
            weighted_accept / total_weight
        } else {
            0.0
        };

        if support > self.threshold {
            Ok(ConsensusResult::Accepted {
                support,
                votes,
                unanimous: support >= 0.99,
            })
        } else {
            Ok(ConsensusResult::Rejected { support, votes })
        }
    }
}

/// Supermajority voting (2/3 threshold)
#[derive(Debug, Clone)]
pub struct SuperMajority {
    threshold: f32,
    min_participation: f32,
}

impl Default for SuperMajority {
    fn default() -> Self {
        Self {
            threshold: 0.67, // 2/3 majority
            min_participation: 0.6,
        }
    }
}

impl ConsensusStrategy for SuperMajority {
    fn name(&self) -> &str {
        "Super Majority"
    }

    fn required_quorum(&self) -> f32 {
        self.min_participation
    }

    fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let total = votes.len() as f32;
        let accepts = votes
            .iter()
            .filter(|v| matches!(v.decision, Decision::Accept))
            .count() as f32;

        let support = if total > 0.0 { accepts / total } else { 0.0 };

        if support >= self.threshold {
            Ok(ConsensusResult::Accepted {
                support,
                votes,
                unanimous: support >= 0.99,
            })
        } else {
            Ok(ConsensusResult::Rejected { support, votes })
        }
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
