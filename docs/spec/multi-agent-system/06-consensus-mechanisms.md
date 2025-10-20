# Axon: Consensus Mechanisms

## Overview

Consensus mechanisms in Axon provide democratic coordination between agents for decision making. This specification describes various consensus mechanisms, including Sangha consensus from CCSwarm, voting systems, conflict resolution and Byzantine fault tolerance. All consensus results are stored in Cortex via REST API for learning.

## Consensus Layer Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Consensus Layer (Axon)                     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │             Consensus Engine                           │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────────┐    │  │
│  │  │ Sangha   │  │  Voting  │  │   Byzantine     │    │  │
│  │  │Consensus │  │  System  │  │  Fault Tolerance│    │  │
│  │  └──────────┘  └──────────┘  └─────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│  ┌─────────────────────────┼──────────────────────────────┐  │
│  │                         │                              │  │
│  │  ┌──────────────┐  ┌───▼──────────┐  ┌────────────┐  │  │
│  │  │  Proposal    │  │   Conflict   │  │  Deadlock  │  │  │
│  │  │  Manager     │  │   Resolver   │  │  Detector  │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│                            ▼                                  │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │        Cortex Bridge (REST API)                         │ │
│  │  - Store consensus episodes                             │ │
│  │  - Learn from voting patterns                           │ │
│  │  - Query historical decisions                           │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## Core Consensus Framework

### Consensus Engine

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Consensus Engine manages all consensus mechanisms
pub struct ConsensusEngine {
    /// Available consensus strategies
    strategies: HashMap<String, Box<dyn ConsensusStrategy>>,

    /// Voting records storage
    voting_records: Arc<RwLock<VotingRecordStore>>,

    /// Timeout manager
    timeout_manager: TimeoutManager,

    /// Conflict resolver
    conflict_resolver: ConflictResolver,

    /// Deadlock detector
    deadlock_detector: DeadlockDetector,

    /// Bridge to Cortex for storing results
    cortex_bridge: Arc<CortexBridge>,

    /// Metrics
    metrics: ConsensusMetrics,
}

/// Base trait for all consensus strategies
pub trait ConsensusStrategy: Send + Sync {
    /// Minimum quorum for voting
    fn required_quorum(&self) -> f32;

    /// Voting mechanism
    fn voting_mechanism(&self) -> VotingMechanism;

    /// Evaluate votes and make decision
    async fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult>;

    /// Handle timeout
    fn handle_timeout(&self) -> TimeoutAction;

    /// Strategy name
    fn name(&self) -> &str;
}

/// Agent vote
#[derive(Debug, Clone)]
pub struct Vote {
    /// Voting agent identifier
    pub voter: AgentId,

    /// Proposal identifier
    pub proposal_id: ProposalId,

    /// Decision
    pub decision: Decision,

    /// Confidence in decision (0.0 to 1.0)
    pub confidence: f32,

    /// Decision rationale
    pub rationale: Option<String>,

    /// Voting timestamp
    pub timestamp: DateTime<Utc>,
}

/// Decision types
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Accept proposal
    Accept,

    /// Reject proposal
    Reject,

    /// Abstain
    Abstain,

    /// Conditional acceptance
    Conditional(Condition),
}

/// Condition for conditional acceptance
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub requirement: String,
    pub validator: ConditionValidator,
}

/// Consensus result
#[derive(Debug, Clone)]
pub enum ConsensusResult {
    /// Proposal accepted
    Accepted {
        support: f32,
        votes: Vec<Vote>,
        unanimous: bool,
    },

    /// Proposal rejected
    Rejected {
        support: f32,
        votes: Vec<Vote>,
    },

    /// Harmonious agreement (Sangha)
    Harmonious {
        proposal: Proposal,
        harmony_level: f32,
        rounds: usize,
        votes: Vec<Vote>,
    },

    /// No harmony (Sangha)
    NoHarmony {
        final_proposal: Proposal,
        rounds: usize,
        votes: Vec<Vote>,
    },

    /// Failed to reach consensus
    Failed {
        reason: String,
        votes: Vec<Vote>,
    },

    /// Timeout
    Timeout {
        partial_votes: Vec<Vote>,
    },
}

impl ConsensusEngine {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        let mut strategies: HashMap<String, Box<dyn ConsensusStrategy>> = HashMap::new();

        // Register all strategies
        strategies.insert("simple_majority".to_string(), Box::new(SimpleMajorityConsensus::default()));
        strategies.insert("supermajority".to_string(), Box::new(SupermajorityConsensus::two_thirds()));
        strategies.insert("weighted".to_string(), Box::new(WeightedConsensus::new()));
        strategies.insert("sangha".to_string(), Box::new(SanghaConsensus::default()));
        strategies.insert("byzantine".to_string(), Box::new(ByzantineConsensus::pbft()));
        strategies.insert("unanimous".to_string(), Box::new(UnanimousConsensus::default()));

        Self {
            strategies,
            voting_records: Arc::new(RwLock::new(VotingRecordStore::new())),
            timeout_manager: TimeoutManager::new(),
            conflict_resolver: ConflictResolver::new(),
            deadlock_detector: DeadlockDetector::new(),
            cortex_bridge,
            metrics: ConsensusMetrics::new(),
        }
    }

    /// Initiate consensus on proposal
    pub async fn initiate_consensus(
        &self,
        proposal: Proposal,
        strategy_name: &str,
        participants: Vec<AgentId>,
    ) -> Result<ConsensusResult> {
        let start = std::time::Instant::now();

        // Get strategy
        let strategy = self.strategies.get(strategy_name)
            .ok_or_else(|| Error::StrategyNotFound(strategy_name.to_string()))?;

        info!("Initiating {} consensus for proposal {}", strategy_name, proposal.id);

        // Check quorum
        if (participants.len() as f32) < strategy.required_quorum() {
            return Err(Error::InsufficientQuorum {
                required: strategy.required_quorum(),
                available: participants.len(),
            });
        }

        // Start voting
        let result = match strategy_name {
            "sangha" => {
                // Sangha requires special handling with rounds
                self.run_sangha_consensus(proposal, participants).await?
            }
            "byzantine" => {
                // Byzantine requires multi-phase protocol
                self.run_byzantine_consensus(proposal, participants).await?
            }
            _ => {
                // Standard voting
                self.run_standard_consensus(proposal, participants, strategy.as_ref()).await?
            }
        };

        let duration = start.elapsed();

        // Store result in Cortex for learning
        self.store_consensus_outcome(&result, duration).await?;

        // Update metrics
        self.metrics.record_consensus(&result, duration);

        Ok(result)
    }

    /// Standard voting
    async fn run_standard_consensus(
        &self,
        proposal: Proposal,
        participants: Vec<AgentId>,
        strategy: &dyn ConsensusStrategy,
    ) -> Result<ConsensusResult> {
        // Collect votes from participants
        let votes = self.collect_votes(&proposal, &participants).await?;

        // Evaluate votes through strategy
        let result = strategy.evaluate_votes(votes).await?;

        Ok(result)
    }

    /// Store consensus result in Cortex
    async fn store_consensus_outcome(
        &self,
        result: &ConsensusResult,
        duration: std::time::Duration,
    ) -> Result<()> {
        let episode = ConsensusEpisode {
            proposal_id: result.proposal_id(),
            strategy: result.strategy_used(),
            participants: result.participants(),
            votes: result.votes().clone(),
            outcome: result.outcome_type(),
            duration,
            timestamp: Utc::now(),
        };

        // POST /memory/episodes/consensus
        self.cortex_bridge.store_consensus_episode(episode).await?;

        Ok(())
    }

    /// Collect votes from participants
    async fn collect_votes(
        &self,
        proposal: &Proposal,
        participants: &[AgentId],
    ) -> Result<Vec<Vote>> {
        let mut votes = Vec::new();

        for agent_id in participants {
            // Send vote request to agent
            let vote = self.request_vote(agent_id, proposal).await?;
            votes.push(vote);
        }

        Ok(votes)
    }

    async fn request_vote(&self, agent_id: &AgentId, proposal: &Proposal) -> Result<Vote> {
        // Request vote from agent via message bus
        todo!("Implement vote request via message bus")
    }
}
```

## Sangha Consensus (CCSwarm)

Sangha consensus is based on Buddhist tradition of achieving harmony through discussion and reflection.

```rust
/// Sangha Consensus — Buddhist-inspired harmonious agreement
#[derive(Debug, Clone)]
pub struct SanghaConsensus {
    /// Minimum harmony level for decision making
    harmony_threshold: f32,

    /// Maximum number of discussion rounds
    max_rounds: usize,

    /// Convergence rate
    convergence_rate: f32,

    /// Minimum participation
    min_participation: f32,
}

impl Default for SanghaConsensus {
    fn default() -> Self {
        Self {
            harmony_threshold: 0.85,  // 85% harmony required
            max_rounds: 5,
            convergence_rate: 0.2,
            min_participation: 0.8,   // 80% must participate
        }
    }
}

impl SanghaConsensus {
    /// Seek harmonious solution through iterative discussion
    pub async fn seek_harmony(
        &self,
        mut proposal: Proposal,
        agents: Vec<AgentId>,
        message_bus: Arc<MessageBus>,
    ) -> Result<ConsensusResult> {
        info!("Starting Sangha consensus seeking harmony");

        for round in 0..self.max_rounds {
            info!("Sangha round {}/{}", round + 1, self.max_rounds);

            // Phase 1: Individual reflection
            self.reflection_phase(&agents, &proposal).await?;

            // Phase 2: Collect votes
            let votes = self.collect_votes_with_rationale(&proposal, &agents, &message_bus).await?;

            // Check participation
            let participation = votes.len() as f32 / agents.len() as f32;
            if participation < self.min_participation {
                warn!("Low participation in round {}: {:.1}%", round + 1, participation * 100.0);
                continue;
            }

            // Phase 3: Calculate harmony level
            let harmony = self.calculate_harmony(&votes);

            info!("Harmony level: {:.2}%", harmony * 100.0);

            // If harmony is achieved — return result
            if harmony >= self.harmony_threshold {
                return Ok(ConsensusResult::Harmonious {
                    proposal: proposal.clone(),
                    harmony_level: harmony,
                    rounds: round + 1,
                    votes,
                });
            }

            // Phase 4: Collective discussion
            let discussion = self.discussion_phase(&agents, &proposal, &votes, &message_bus).await?;

            // Phase 5: Refine proposal based on feedback
            proposal = self.refine_proposal(proposal, &votes, &discussion).await?;

            info!("Proposal refined based on community feedback");
        }

        // Failed to achieve harmony within allocated rounds
        let final_votes = self.collect_votes_with_rationale(&proposal, &agents, &message_bus).await?;

        Ok(ConsensusResult::NoHarmony {
            final_proposal: proposal,
            rounds: self.max_rounds,
            votes: final_votes,
        })
    }

    /// Individual reflection phase
    async fn reflection_phase(&self, agents: &[AgentId], proposal: &Proposal) -> Result<()> {
        let reflection_time = std::time::Duration::from_secs(30);

        info!("Reflection phase: agents contemplate proposal for {:?}", reflection_time);

        // Give agents time for reflection
        tokio::time::sleep(reflection_time).await;

        Ok(())
    }

    /// Collect votes with rationale
    async fn collect_votes_with_rationale(
        &self,
        proposal: &Proposal,
        agents: &[AgentId],
        message_bus: &MessageBus,
    ) -> Result<Vec<Vote>> {
        let mut votes = Vec::new();

        for agent_id in agents {
            // Request vote with mandatory rationale
            let vote = self.request_vote_with_rationale(agent_id, proposal, message_bus).await?;
            votes.push(vote);
        }

        Ok(votes)
    }

    async fn request_vote_with_rationale(
        &self,
        agent_id: &AgentId,
        proposal: &Proposal,
        message_bus: &MessageBus,
    ) -> Result<Vote> {
        let message = Message::VoteRequest {
            proposal: proposal.clone(),
            require_rationale: true,
            deadline: Utc::now() + chrono::Duration::seconds(60),
        };

        message_bus.send(agent_id.clone(), message).await?;

        // Wait for response
        // TODO: Implement vote reception
        todo!()
    }

    /// Calculate harmony level
    fn calculate_harmony(&self, votes: &[Vote]) -> f32 {
        if votes.is_empty() {
            return 0.0;
        }

        // Calculate variance in decisions
        let accept_count = votes.iter()
            .filter(|v| matches!(v.decision, Decision::Accept))
            .count() as f32;

        let reject_count = votes.iter()
            .filter(|v| matches!(v.decision, Decision::Reject))
            .count() as f32;

        let abstain_count = votes.iter()
            .filter(|v| matches!(v.decision, Decision::Abstain))
            .count() as f32;

        let total = votes.len() as f32;

        // Harmony is high when majority agrees
        let max_alignment = accept_count.max(reject_count).max(abstain_count);
        let alignment_ratio = max_alignment / total;

        // Also consider confidence
        let avg_confidence: f32 = votes.iter()
            .map(|v| v.confidence)
            .sum::<f32>() / total;

        // Combine alignment and confidence
        alignment_ratio * 0.7 + avg_confidence * 0.3
    }

    /// Discussion phase
    async fn discussion_phase(
        &self,
        agents: &[AgentId],
        proposal: &Proposal,
        votes: &[Vote],
        message_bus: &MessageBus,
    ) -> Result<Discussion> {
        info!("Discussion phase: agents share perspectives");

        // Publish voting results for everyone
        let summary = VotingSummary {
            proposal_id: proposal.id.clone(),
            votes: votes.to_vec(),
        };

        message_bus.publish(Topic::new("sangha_discussion"), Message::VotingSummary(summary)).await?;

        // Give time for discussion
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        // Collect comments and suggestions
        let discussion = Discussion {
            participants: agents.to_vec(),
            comments: vec![], // TODO: Collect from agents
            suggestions: vec![],
        };

        Ok(discussion)
    }

    /// Refine proposal based on feedback
    async fn refine_proposal(
        &self,
        mut proposal: Proposal,
        votes: &[Vote],
        discussion: &Discussion,
    ) -> Result<Proposal> {
        // Analyze rationale in votes
        for vote in votes {
            if let Some(rationale) = &vote.rationale {
                if matches!(vote.decision, Decision::Reject | Decision::Conditional(_)) {
                    // Consider rejection reasons
                    proposal = self.incorporate_feedback(proposal, rationale)?;
                }
            }
        }

        // Consider suggestions from discussion
        for suggestion in &discussion.suggestions {
            proposal = self.apply_suggestion(proposal, suggestion)?;
        }

        Ok(proposal)
    }

    fn incorporate_feedback(&self, mut proposal: Proposal, feedback: &str) -> Result<Proposal> {
        // TODO: Implement intelligent proposal refinement
        // Can use LLM for feedback analysis and proposal modification
        Ok(proposal)
    }

    fn apply_suggestion(&self, mut proposal: Proposal, suggestion: &Suggestion) -> Result<Proposal> {
        // TODO: Apply suggestion to proposal
        Ok(proposal)
    }
}

impl ConsensusStrategy for SanghaConsensus {
    fn required_quorum(&self) -> f32 {
        self.min_participation
    }

    fn voting_mechanism(&self) -> VotingMechanism {
        VotingMechanism::Iterative
    }

    async fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let harmony = self.calculate_harmony(&votes);

        if harmony >= self.harmony_threshold {
            Ok(ConsensusResult::Harmonious {
                proposal: Proposal::default(), // Would be provided in context
                harmony_level: harmony,
                rounds: 1,
                votes,
            })
        } else {
            Ok(ConsensusResult::NoHarmony {
                final_proposal: Proposal::default(),
                rounds: 1,
                votes,
            })
        }
    }

    fn handle_timeout(&self) -> TimeoutAction {
        TimeoutAction::ExtendDeadline {
            extension: std::time::Duration::from_secs(300),
        }
    }

    fn name(&self) -> &str {
        "Sangha Consensus"
    }
}

#[derive(Debug, Clone)]
pub struct Discussion {
    pub participants: Vec<AgentId>,
    pub comments: Vec<Comment>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub author: AgentId,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub author: AgentId,
    pub modification: String,
    pub rationale: String,
}
```

## Voting Systems

### 1. Simple Majority

```rust
/// Simple Majority — more than 50% votes
#[derive(Debug, Clone)]
pub struct SimpleMajorityConsensus {
    threshold: f32,
    min_participation: f32,
}

impl Default for SimpleMajorityConsensus {
    fn default() -> Self {
        Self {
            threshold: 0.51,
            min_participation: 0.5,
        }
    }
}

impl ConsensusStrategy for SimpleMajorityConsensus {
    fn required_quorum(&self) -> f32 {
        self.min_participation
    }

    fn voting_mechanism(&self) -> VotingMechanism {
        VotingMechanism::SingleRound
    }

    async fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let total = votes.len() as f32;
        let accepts = votes.iter()
            .filter(|v| matches!(v.decision, Decision::Accept))
            .count() as f32;

        let support = accepts / total;

        if support > self.threshold {
            Ok(ConsensusResult::Accepted {
                support,
                votes,
                unanimous: support >= 0.99,
            })
        } else {
            Ok(ConsensusResult::Rejected {
                support,
                votes,
            })
        }
    }

    fn handle_timeout(&self) -> TimeoutAction {
        TimeoutAction::UsePartialVotes
    }

    fn name(&self) -> &str {
        "Simple Majority"
    }
}
```

### 2. Weighted Voting

```rust
/// Weighted Voting — votes weighted by expertise
pub struct WeightedConsensus {
    weight_calculator: Arc<dyn WeightCalculator>,
    threshold: f32,
}

pub trait WeightCalculator: Send + Sync {
    fn calculate_weight(&self, agent: &AgentId, context: &VotingContext) -> f32;
}

/// Expertise-based weight calculator
pub struct ExpertiseWeightCalculator {
    /// Expertise scores for each agent by domain
    expertise_scores: HashMap<AgentId, HashMap<Domain, f32>>,

    /// Performance history from Cortex
    cortex_bridge: Arc<CortexBridge>,
}

impl WeightCalculator for ExpertiseWeightCalculator {
    fn calculate_weight(&self, agent: &AgentId, context: &VotingContext) -> f32 {
        // Get local scores
        let local_score = self.expertise_scores
            .get(agent)
            .and_then(|scores| scores.get(&context.domain))
            .copied()
            .unwrap_or(0.5);

        // TODO: Query Cortex for historical performance
        // let historical_score = self.cortex_bridge.get_agent_performance(agent).await;

        // Combine
        local_score
    }
}

impl WeightedConsensus {
    pub fn new() -> Self {
        Self {
            weight_calculator: Arc::new(ExpertiseWeightCalculator {
                expertise_scores: HashMap::new(),
                cortex_bridge: Arc::new(CortexBridge::new("http://localhost:8081").unwrap()),
            }),
            threshold: 0.6,
        }
    }
}

impl ConsensusStrategy for WeightedConsensus {
    fn required_quorum(&self) -> f32 {
        0.5
    }

    fn voting_mechanism(&self) -> VotingMechanism {
        VotingMechanism::Weighted
    }

    async fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let context = VotingContext::default(); // Would be provided

        let weighted_sum: f32 = votes.iter()
            .map(|v| {
                let weight = self.weight_calculator.calculate_weight(&v.voter, &context);
                match v.decision {
                    Decision::Accept => weight * v.confidence,
                    Decision::Reject => -weight * v.confidence,
                    Decision::Abstain => 0.0,
                    Decision::Conditional(_) => weight * v.confidence * 0.5,
                }
            })
            .sum();

        let total_weight: f32 = votes.iter()
            .map(|v| self.weight_calculator.calculate_weight(&v.voter, &context))
            .sum();

        let normalized_support = (weighted_sum + total_weight) / (2.0 * total_weight);

        if normalized_support > self.threshold {
            Ok(ConsensusResult::Accepted {
                support: normalized_support,
                votes,
                unanimous: false,
            })
        } else {
            Ok(ConsensusResult::Rejected {
                support: normalized_support,
                votes,
            })
        }
    }

    fn handle_timeout(&self) -> TimeoutAction {
        TimeoutAction::Fail
    }

    fn name(&self) -> &str {
        "Weighted Voting"
    }
}
```

### 3. Unanimous Consensus

```rust
/// Unanimous Consensus — requires agreement from all
#[derive(Debug, Clone, Default)]
pub struct UnanimousConsensus {
    allow_abstentions: bool,
}

impl ConsensusStrategy for UnanimousConsensus {
    fn required_quorum(&self) -> f32 {
        1.0 // All must participate
    }

    fn voting_mechanism(&self) -> VotingMechanism {
        VotingMechanism::Unanimous
    }

    async fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let all_accept = votes.iter().all(|v| {
            matches!(v.decision, Decision::Accept) ||
            (self.allow_abstentions && matches!(v.decision, Decision::Abstain))
        });

        if all_accept {
            Ok(ConsensusResult::Accepted {
                support: 1.0,
                votes,
                unanimous: true,
            })
        } else {
            let support = votes.iter()
                .filter(|v| matches!(v.decision, Decision::Accept))
                .count() as f32 / votes.len() as f32;

            Ok(ConsensusResult::Rejected {
                support,
                votes,
            })
        }
    }

    fn handle_timeout(&self) -> TimeoutAction {
        TimeoutAction::Fail
    }

    fn name(&self) -> &str {
        "Unanimous Consensus"
    }
}
```

## Byzantine Fault Tolerance

```rust
/// Byzantine Fault Tolerance Consensus
pub struct ByzantineConsensus {
    /// Maximum fraction of faulty nodes (usually 1/3)
    fault_tolerance: f32,

    /// Number of verification rounds
    verification_rounds: usize,

    /// Message authenticator
    message_authenticator: MessageAuthenticator,
}

impl ByzantineConsensus {
    /// Practical Byzantine Fault Tolerance
    pub fn pbft() -> Self {
        Self {
            fault_tolerance: 0.33,
            verification_rounds: 3,
            message_authenticator: MessageAuthenticator::new(),
        }
    }

    /// Execute Byzantine consensus protocol
    async fn execute_consensus(
        &self,
        proposal: Proposal,
        agents: Vec<AgentId>,
        message_bus: Arc<MessageBus>,
    ) -> Result<ConsensusResult> {
        let n = agents.len();
        let f = (n as f32 * self.fault_tolerance) as usize;
        let quorum = n - f;

        info!("Starting PBFT: n={}, f={}, quorum={}", n, f, quorum);

        // Phase 1: Pre-prepare
        let leader = self.select_leader(&agents);
        let pre_prepare = self.send_pre_prepare(&leader, &proposal, &message_bus).await?;

        // Phase 2: Prepare
        let prepare_votes = self.collect_prepare_votes(&agents, &pre_prepare, &message_bus).await?;

        if prepare_votes.len() < quorum {
            return Ok(ConsensusResult::Failed {
                reason: format!("Insufficient prepare votes: {}/{}", prepare_votes.len(), quorum),
                votes: vec![],
            });
        }

        // Phase 3: Commit
        let commit_votes = self.collect_commit_votes(&agents, &prepare_votes, &message_bus).await?;

        if commit_votes.len() < quorum {
            return Ok(ConsensusResult::Failed {
                reason: format!("Insufficient commit votes: {}/{}", commit_votes.len(), quorum),
                votes: vec![],
            });
        }

        // Consensus achieved
        Ok(ConsensusResult::Accepted {
            support: commit_votes.len() as f32 / n as f32,
            votes: commit_votes,
            unanimous: false,
        })
    }

    fn select_leader(&self, agents: &[AgentId]) -> AgentId {
        // Simple leader rotation
        agents[0].clone()
    }

    async fn send_pre_prepare(
        &self,
        leader: &AgentId,
        proposal: &Proposal,
        message_bus: &MessageBus,
    ) -> Result<PrePrepare> {
        let pre_prepare = PrePrepare {
            leader: leader.clone(),
            proposal: proposal.clone(),
            sequence: 0,
        };

        message_bus.publish(
            Topic::new("byzantine_pre_prepare"),
            Message::PrePrepare(pre_prepare.clone())
        ).await?;

        Ok(pre_prepare)
    }

    async fn collect_prepare_votes(
        &self,
        agents: &[AgentId],
        pre_prepare: &PrePrepare,
        message_bus: &MessageBus,
    ) -> Result<Vec<Vote>> {
        // Collect prepare messages from replicas
        // TODO: Implement via message bus subscription
        todo!()
    }

    async fn collect_commit_votes(
        &self,
        agents: &[AgentId],
        prepare_votes: &[Vote],
        message_bus: &MessageBus,
    ) -> Result<Vec<Vote>> {
        // Collect commit messages
        // TODO: Implement
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct PrePrepare {
    pub leader: AgentId,
    pub proposal: Proposal,
    pub sequence: u64,
}

pub struct MessageAuthenticator {
    // Cryptographic signature validation
}

impl MessageAuthenticator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn verify(&self, message: &Message, signature: &Signature) -> bool {
        // TODO: Implement signature verification
        true
    }
}
```

## Conflict Resolution

```rust
/// Conflict Resolver resolves conflicts between proposals
pub struct ConflictResolver {
    strategies: HashMap<ConflictType, ResolutionStrategy>,
    mediator: Option<AgentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConflictType {
    ResourceContention,
    MutualExclusion,
    CircularDependency,
    TemporalConflict,
}

#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    Priority,
    FirstComeFirstServed,
    Voting,
    Merge,
    Defer,
}

impl ConflictResolver {
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert(ConflictType::ResourceContention, ResolutionStrategy::Priority);
        strategies.insert(ConflictType::MutualExclusion, ResolutionStrategy::Voting);
        strategies.insert(ConflictType::CircularDependency, ResolutionStrategy::Defer);

        Self {
            strategies,
            mediator: None,
        }
    }

    pub async fn resolve(&self, conflict: Conflict) -> Result<Resolution> {
        let strategy = self.strategies.get(&conflict.conflict_type)
            .unwrap_or(&ResolutionStrategy::Voting);

        match strategy {
            ResolutionStrategy::Priority => {
                self.resolve_by_priority(conflict).await
            }
            ResolutionStrategy::Voting => {
                self.resolve_by_voting(conflict).await
            }
            ResolutionStrategy::Merge => {
                self.attempt_merge(conflict).await
            }
            ResolutionStrategy::FirstComeFirstServed => {
                self.resolve_fcfs(conflict).await
            }
            ResolutionStrategy::Defer => {
                Ok(Resolution::Deferred {
                    reason: "Circular dependency detected".to_string(),
                    retry_after: std::time::Duration::from_secs(300),
                })
            }
        }
    }

    async fn resolve_by_priority(&self, conflict: Conflict) -> Result<Resolution> {
        // Resolve based on proposal priority
        let winner = conflict.proposals
            .iter()
            .max_by_key(|p| p.priority)
            .cloned()
            .ok_or(Error::NoProposals)?;

        Ok(Resolution::Resolved {
            winner,
            rationale: "Higher priority proposal selected".to_string(),
        })
    }

    async fn resolve_by_voting(&self, conflict: Conflict) -> Result<Resolution> {
        // Let agents vote on conflicting proposals
        todo!("Implement voting-based conflict resolution")
    }

    async fn attempt_merge(&self, conflict: Conflict) -> Result<Resolution> {
        // Try to merge conflicting proposals
        todo!("Implement proposal merging")
    }

    async fn resolve_fcfs(&self, conflict: Conflict) -> Result<Resolution> {
        let winner = conflict.proposals
            .iter()
            .min_by_key(|p| p.created_at)
            .cloned()
            .ok_or(Error::NoProposals)?;

        Ok(Resolution::Resolved {
            winner,
            rationale: "First-come-first-served".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub conflict_type: ConflictType,
    pub proposals: Vec<Proposal>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum Resolution {
    Resolved {
        winner: Proposal,
        rationale: String,
    },
    Merged {
        merged_proposal: Proposal,
    },
    Deferred {
        reason: String,
        retry_after: std::time::Duration,
    },
}
```

## Deadlock Detection

```rust
use petgraph::Graph;
use petgraph::algo::is_cyclic_directed;

/// Deadlock Detector detects and prevents deadlocks
pub struct DeadlockDetector {
    wait_for_graph: Arc<RwLock<Graph<AgentId, ()>>>,
    detection_interval: std::time::Duration,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            wait_for_graph: Arc::new(RwLock::new(Graph::new())),
            detection_interval: std::time::Duration::from_secs(10),
        }
    }

    /// Detect cycles in wait-for graph
    pub async fn detect(&self) -> Vec<DeadlockCycle> {
        let graph = self.wait_for_graph.read().await;

        if is_cyclic_directed(&*graph) {
            // Find all cycles
            // TODO: Implement cycle detection
            vec![]
        } else {
            vec![]
        }
    }

    /// Check request safety (Banker's algorithm)
    pub async fn is_safe_state(&self, request: ResourceRequest) -> Result<bool> {
        // Banker's algorithm for deadlock prevention
        // TODO: Implement
        Ok(true)
    }

    pub async fn add_wait_edge(&self, waiter: AgentId, waited_for: AgentId) {
        let mut graph = self.wait_for_graph.write().await;
        // TODO: Add edge to graph
    }

    pub async fn remove_wait_edge(&self, waiter: AgentId, waited_for: AgentId) {
        let mut graph = self.wait_for_graph.write().await;
        // TODO: Remove edge from graph
    }
}

#[derive(Debug, Clone)]
pub struct DeadlockCycle {
    pub agents: Vec<AgentId>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub agent: AgentId,
    pub resource: ResourceId,
    pub lock_type: LockType,
}
```

## Integration with Cortex Episodes

```rust
impl ConsensusEngine {
    /// Store consensus episode in Cortex for learning
    async fn store_consensus_episode(
        &self,
        result: &ConsensusResult,
        duration: std::time::Duration,
    ) -> Result<()> {
        let episode = ConsensusEpisode {
            proposal_id: match result {
                ConsensusResult::Accepted { .. } => "accepted".to_string(),
                ConsensusResult::Rejected { .. } => "rejected".to_string(),
                ConsensusResult::Harmonious { proposal, .. } => proposal.id.clone(),
                ConsensusResult::NoHarmony { final_proposal, .. } => final_proposal.id.clone(),
                ConsensusResult::Failed { .. } => "failed".to_string(),
                ConsensusResult::Timeout { .. } => "timeout".to_string(),
            },
            strategy: result.strategy_used(),
            participants: result.participants(),
            votes: result.votes().clone(),
            outcome: result.outcome_type(),
            harmony_level: result.harmony_level(),
            duration,
            timestamp: Utc::now(),
        };

        // POST /memory/episodes
        self.cortex_bridge
            .store_episode(Episode::Consensus(episode))
            .await?;

        info!("Stored consensus episode in Cortex for learning");

        Ok(())
    }

    /// Get historical data about similar consensuses
    async fn query_similar_consensus(&self, proposal: &Proposal) -> Result<Vec<ConsensusEpisode>> {
        // POST /memory/search
        let results = self.cortex_bridge
            .search_episodes(&proposal.description, 10)
            .await?;

        Ok(results.into_iter()
            .filter_map(|ep| {
                if let Episode::Consensus(ce) = ep {
                    Some(ce)
                } else {
                    None
                }
            })
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEpisode {
    pub proposal_id: String,
    pub strategy: String,
    pub participants: Vec<AgentId>,
    pub votes: Vec<Vote>,
    pub outcome: OutcomeType,
    pub harmony_level: Option<f32>,
    pub duration: std::time::Duration,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeType {
    Accepted,
    Rejected,
    Harmonious,
    NoHarmony,
    Failed,
    Timeout,
}
```

## Metrics and Monitoring

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct ConsensusMetrics {
    pub proposals_submitted: AtomicU64,
    pub proposals_passed: AtomicU64,
    pub proposals_failed: AtomicU64,
    pub average_consensus_time_ms: AtomicU64,
    pub participation_rate: AtomicU64,
    pub conflict_count: AtomicU64,
    pub deadlock_incidents: AtomicU64,
}

impl ConsensusMetrics {
    pub fn new() -> Self {
        Self {
            proposals_submitted: AtomicU64::new(0),
            proposals_passed: AtomicU64::new(0),
            proposals_failed: AtomicU64::new(0),
            average_consensus_time_ms: AtomicU64::new(0),
            participation_rate: AtomicU64::new(0),
            conflict_count: AtomicU64::new(0),
            deadlock_incidents: AtomicU64::new(0),
        }
    }

    pub fn record_consensus(&self, result: &ConsensusResult, duration: std::time::Duration) {
        self.proposals_submitted.fetch_add(1, Ordering::Relaxed);

        match result {
            ConsensusResult::Accepted { .. } | ConsensusResult::Harmonious { .. } => {
                self.proposals_passed.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.proposals_failed.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.average_consensus_time_ms.store(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );
    }

    pub fn export_metrics(&self) -> MetricsSnapshot {
        let submitted = self.proposals_submitted.load(Ordering::Relaxed);
        let passed = self.proposals_passed.load(Ordering::Relaxed);

        MetricsSnapshot {
            timestamp: Utc::now(),
            proposals_total: submitted,
            success_rate: if submitted > 0 {
                passed as f64 / submitted as f64
            } else {
                0.0
            },
            avg_time_ms: self.average_consensus_time_ms.load(Ordering::Relaxed),
            conflicts: self.conflict_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub proposals_total: u64,
    pub success_rate: f64,
    pub avg_time_ms: u64,
    pub conflicts: u64,
}
```

## Supporting Types

```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProposalId(String);

impl ProposalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub proposer: AgentId,
    pub content: ProposalContent,
    pub description: String,
    pub priority: u32,
    pub created_at: DateTime<Utc>,
}

impl Default for Proposal {
    fn default() -> Self {
        Self {
            id: ProposalId::new(),
            proposer: AgentId("default".to_string()),
            content: ProposalContent::default(),
            description: String::new(),
            priority: 0,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProposalContent {
    pub action: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum VotingMechanism {
    SingleRound,
    Weighted,
    Iterative,
    Unanimous,
}

#[derive(Debug, Clone)]
pub enum TimeoutAction {
    UsePartialVotes,
    ExtendDeadline { extension: std::time::Duration },
    Fail,
}

#[derive(Debug, Clone, Default)]
pub struct VotingContext {
    pub domain: Domain,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Domain(String);

impl Default for Domain {
    fn default() -> Self {
        Self("general".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct VotingRecordStore {
    records: HashMap<ProposalId, Vec<Vote>>,
}

impl VotingRecordStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
}

pub struct TimeoutManager {
    timeouts: HashMap<ProposalId, DateTime<Utc>>,
}

impl TimeoutManager {
    pub fn new() -> Self {
        Self {
            timeouts: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConditionValidator {
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ResourceId(String);

#[derive(Debug, Clone)]
pub enum LockType {
    Shared,
    Exclusive,
}

pub struct Signature(Vec<u8>);

// Helper implementations for ConsensusResult
impl ConsensusResult {
    pub fn proposal_id(&self) -> String {
        match self {
            Self::Harmonious { proposal, .. } => proposal.id.0.clone(),
            Self::NoHarmony { final_proposal, .. } => final_proposal.id.0.clone(),
            _ => "unknown".to_string(),
        }
    }

    pub fn strategy_used(&self) -> String {
        "unknown".to_string() // Would be tracked in context
    }

    pub fn participants(&self) -> Vec<AgentId> {
        match self {
            Self::Accepted { votes, .. }
            | Self::Rejected { votes, .. }
            | Self::Harmonious { votes, .. }
            | Self::NoHarmony { votes, .. }
            | Self::Failed { votes, .. } => {
                votes.iter().map(|v| v.voter.clone()).collect()
            }
            Self::Timeout { partial_votes } => {
                partial_votes.iter().map(|v| v.voter.clone()).collect()
            }
        }
    }

    pub fn votes(&self) -> &[Vote] {
        match self {
            Self::Accepted { votes, .. }
            | Self::Rejected { votes, .. }
            | Self::Harmonious { votes, .. }
            | Self::NoHarmony { votes, .. }
            | Self::Failed { votes, .. } => votes,
            Self::Timeout { partial_votes } => partial_votes,
        }
    }

    pub fn outcome_type(&self) -> OutcomeType {
        match self {
            Self::Accepted { .. } => OutcomeType::Accepted,
            Self::Rejected { .. } => OutcomeType::Rejected,
            Self::Harmonious { .. } => OutcomeType::Harmonious,
            Self::NoHarmony { .. } => OutcomeType::NoHarmony,
            Self::Failed { .. } => OutcomeType::Failed,
            Self::Timeout { .. } => OutcomeType::Timeout,
        }
    }

    pub fn harmony_level(&self) -> Option<f32> {
        match self {
            Self::Harmonious { harmony_level, .. } => Some(*harmony_level),
            _ => None,
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Strategy not found: {0}")]
    StrategyNotFound(String),

    #[error("Insufficient quorum: required {required}, available {available}")]
    InsufficientQuorum { required: f32, available: usize },

    #[error("No proposals")]
    NoProposals,

    #[error("Other error: {0}")]
    Other(String),
}
```

---

These consensus mechanisms provide robust democratic decision making in the Axon multi-agent system, with full integration with Cortex for storing results and learning from historical data.
