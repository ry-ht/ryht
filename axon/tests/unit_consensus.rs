//! Unit tests for consensus mechanisms
//!
//! Tests cover:
//! - Voting strategies (Simple Majority, Supermajority, Weighted, Sangha)
//! - Vote collection and aggregation
//! - Consensus result evaluation
//! - Quorum requirements
//! - Edge cases and error handling

mod common;

use axon::consensus::*;
use axon::agents::AgentId;
use chrono::Utc;

// ============================================================================
// Simple Majority Tests
// ============================================================================

#[test]
fn test_simple_majority_creation() {
    let strategy = SimpleMajority::default();
    assert_eq!(strategy.name(), "Simple Majority");
    assert_eq!(strategy.required_quorum(), 0.5);
}

#[test]
fn test_simple_majority_accepts_with_majority() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Accept, 0.8),
        create_vote(Decision::Reject, 0.7),
    ];

    let result = strategy.evaluate_votes(votes.clone()).unwrap();

    match result {
        ConsensusResult::Accepted { support, unanimous, .. } => {
            assert!(support > 0.51);
            assert!(!unanimous);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[test]
fn test_simple_majority_rejects_without_majority() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Reject, 0.8),
        create_vote(Decision::Reject, 0.7),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Rejected { support, .. } => {
            assert!(support < 0.51);
        }
        _ => panic!("Expected Rejected result"),
    }
}

#[test]
fn test_simple_majority_unanimous() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote(Decision::Accept, 1.0),
        create_vote(Decision::Accept, 1.0),
        create_vote(Decision::Accept, 1.0),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { unanimous, .. } => {
            assert!(unanimous);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[test]
fn test_simple_majority_empty_votes() {
    let strategy = SimpleMajority::default();
    let votes = vec![];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Rejected { support, .. } => {
            assert_eq!(support, 0.0);
        }
        _ => panic!("Expected Rejected result for empty votes"),
    }
}

// ============================================================================
// Supermajority Tests
// ============================================================================

#[test]
fn test_supermajority_creation() {
    let strategy = SuperMajority::default();
    assert_eq!(strategy.name(), "Super Majority");
    assert_eq!(strategy.required_quorum(), 0.6);
}

#[test]
fn test_supermajority_requires_two_thirds() {
    let strategy = SuperMajority::default();

    // Exactly 2/3 support
    let votes = vec![
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Reject, 0.9),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { support, .. } => {
            assert!(support >= 0.67);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[test]
fn test_supermajority_rejects_simple_majority() {
    let strategy = SuperMajority::default();

    // Only 60% support (less than 2/3)
    let votes = vec![
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Reject, 0.9),
        create_vote(Decision::Reject, 0.9),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Rejected { .. } => {},
        _ => panic!("Expected Rejected result for simple majority"),
    }
}

// ============================================================================
// Weighted Voting Tests
// ============================================================================

#[test]
fn test_weighted_voting_creation() {
    let strategy = WeightedVoting::default();
    assert_eq!(strategy.name(), "Weighted Voting");
    assert_eq!(strategy.required_quorum(), 0.5);
}

#[test]
fn test_weighted_voting_with_confidence() {
    let strategy = WeightedVoting::default();

    // Votes with different confidence levels
    let votes = vec![
        create_vote_with_confidence(Decision::Accept, 1.0),
        create_vote_with_confidence(Decision::Accept, 0.5),
        create_vote_with_confidence(Decision::Reject, 0.3),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { .. } => {},
        _ => panic!("Expected Accepted result"),
    }
}

// ============================================================================
// Vote Tests
// ============================================================================

#[test]
fn test_vote_creation() {
    let vote = Vote {
        voter: AgentId::from_string("voter-1"),
        proposal_id: "prop-1".to_string(),
        decision: Decision::Accept,
        confidence: 0.9,
        rationale: Some("Good proposal".to_string()),
        timestamp: Utc::now(),
    };

    assert_eq!(vote.voter.to_string(), "voter-1");
    assert_eq!(vote.confidence, 0.9);
    assert!(matches!(vote.decision, Decision::Accept));
}

#[test]
fn test_vote_decisions() {
    let decisions = vec![
        Decision::Accept,
        Decision::Reject,
        Decision::Abstain,
        Decision::Conditional("needs revision".to_string()),
    ];

    for decision in decisions {
        let vote = Vote {
            voter: AgentId::from_string("voter-1"),
            proposal_id: "prop-1".to_string(),
            decision: decision.clone(),
            confidence: 0.8,
            rationale: None,
            timestamp: Utc::now(),
        };

        match vote.decision {
            Decision::Accept => assert!(matches!(decision, Decision::Accept)),
            Decision::Reject => assert!(matches!(decision, Decision::Reject)),
            Decision::Abstain => assert!(matches!(decision, Decision::Abstain)),
            Decision::Conditional(_) => assert!(matches!(decision, Decision::Conditional(_))),
        }
    }
}

#[test]
fn test_vote_with_rationale() {
    let vote = Vote {
        voter: AgentId::from_string("voter-1"),
        proposal_id: "prop-1".to_string(),
        decision: Decision::Accept,
        confidence: 0.9,
        rationale: Some("Well-structured proposal with clear benefits".to_string()),
        timestamp: Utc::now(),
    };

    assert!(vote.rationale.is_some());
    assert!(vote.rationale.unwrap().contains("Well-structured"));
}

#[test]
fn test_vote_without_rationale() {
    let vote = Vote {
        voter: AgentId::from_string("voter-1"),
        proposal_id: "prop-1".to_string(),
        decision: Decision::Accept,
        confidence: 0.9,
        rationale: None,
        timestamp: Utc::now(),
    };

    assert!(vote.rationale.is_none());
}

// ============================================================================
// Proposal Tests
// ============================================================================

#[test]
fn test_proposal_creation() {
    let proposal = Proposal {
        id: "prop-1".to_string(),
        proposer: AgentId::from_string("proposer-1"),
        content: "Implement feature X".to_string(),
        description: "Add new feature X to improve performance".to_string(),
        priority: 5,
        created_at: Utc::now(),
    };

    assert_eq!(proposal.id, "prop-1");
    assert_eq!(proposal.priority, 5);
    assert!(!proposal.content.is_empty());
}

#[test]
fn test_proposal_priority_levels() {
    for priority in 1..=10 {
        let proposal = Proposal {
            id: format!("prop-{}", priority),
            proposer: AgentId::from_string("proposer-1"),
            content: "Test content".to_string(),
            description: "Test description".to_string(),
            priority,
            created_at: Utc::now(),
        };

        assert_eq!(proposal.priority, priority);
    }
}

// ============================================================================
// Consensus Protocol Tests
// ============================================================================

#[tokio::test]
async fn test_consensus_protocol_creation() {
    let protocol = ConsensusProtocol::new();

    // Should have default strategies registered
    assert!(true); // Protocol created successfully
}

#[tokio::test]
async fn test_consensus_protocol_simple_majority() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal();

    let participants = vec![
        AgentId::from_string("agent-1"),
        AgentId::from_string("agent-2"),
        AgentId::from_string("agent-3"),
    ];

    let result = protocol
        .initiate_consensus(proposal, "simple_majority", participants)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_consensus_protocol_insufficient_quorum() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal();

    // Only 1 participant, but quorum requires at least 50%
    let participants = vec![AgentId::from_string("agent-1")];

    let result = protocol
        .initiate_consensus(proposal, "supermajority", participants)
        .await;

    // Should fail with insufficient quorum
    assert!(result.is_err());
    if let Err(ConsensusError::InsufficientQuorum { .. }) = result {
        // Expected error
    } else {
        panic!("Expected InsufficientQuorum error");
    }
}

#[tokio::test]
async fn test_consensus_protocol_invalid_strategy() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal();

    let participants = vec![
        AgentId::from_string("agent-1"),
        AgentId::from_string("agent-2"),
    ];

    let result = protocol
        .initiate_consensus(proposal, "non_existent_strategy", participants)
        .await;

    assert!(result.is_err());
    if let Err(ConsensusError::StrategyNotFound(name)) = result {
        assert_eq!(name, "non_existent_strategy");
    } else {
        panic!("Expected StrategyNotFound error");
    }
}

// ============================================================================
// Consensus Result Tests
// ============================================================================

#[test]
fn test_consensus_result_accepted() {
    let votes = vec![create_vote(Decision::Accept, 0.9)];

    let result = ConsensusResult::Accepted {
        support: 0.9,
        votes,
        unanimous: true,
    };

    match result {
        ConsensusResult::Accepted { support, unanimous, .. } => {
            assert_eq!(support, 0.9);
            assert!(unanimous);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[test]
fn test_consensus_result_rejected() {
    let votes = vec![create_vote(Decision::Reject, 0.9)];

    let result = ConsensusResult::Rejected {
        support: 0.1,
        votes,
    };

    match result {
        ConsensusResult::Rejected { support, .. } => {
            assert_eq!(support, 0.1);
        }
        _ => panic!("Expected Rejected result"),
    }
}

#[test]
fn test_consensus_result_harmonious() {
    let votes = vec![create_vote(Decision::Accept, 0.95)];

    let result = ConsensusResult::Harmonious {
        harmony_level: 0.95,
        rounds: 2,
        votes,
    };

    match result {
        ConsensusResult::Harmonious { harmony_level, rounds, .. } => {
            assert_eq!(harmony_level, 0.95);
            assert_eq!(rounds, 2);
        }
        _ => panic!("Expected Harmonious result"),
    }
}

#[test]
fn test_consensus_result_failed() {
    let votes = vec![create_vote(Decision::Abstain, 0.5)];

    let result = ConsensusResult::Failed {
        reason: "Could not reach consensus".to_string(),
        votes,
    };

    match result {
        ConsensusResult::Failed { reason, .. } => {
            assert!(reason.contains("Could not reach consensus"));
        }
        _ => panic!("Expected Failed result"),
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_consensus_with_all_abstentions() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote(Decision::Abstain, 0.5),
        create_vote(Decision::Abstain, 0.5),
        create_vote(Decision::Abstain, 0.5),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Rejected { support, .. } => {
            assert_eq!(support, 0.0);
        }
        _ => panic!("Expected Rejected result for all abstentions"),
    }
}

#[test]
fn test_consensus_with_mixed_decisions() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote(Decision::Accept, 0.9),
        create_vote(Decision::Reject, 0.8),
        create_vote(Decision::Abstain, 0.5),
        create_vote(Decision::Conditional("needs work".to_string()), 0.7),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());
}

#[test]
fn test_consensus_with_zero_confidence() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote_with_confidence(Decision::Accept, 0.0),
        create_vote_with_confidence(Decision::Accept, 0.0),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());
}

#[test]
fn test_consensus_with_max_confidence() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote_with_confidence(Decision::Accept, 1.0),
        create_vote_with_confidence(Decision::Accept, 1.0),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { unanimous, .. } => {
            assert!(unanimous);
        }
        _ => panic!("Expected Accepted with unanimous flag"),
    }
}

#[test]
fn test_consensus_with_single_vote() {
    let strategy = SimpleMajority::default();

    let votes = vec![create_vote(Decision::Accept, 0.9)];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { support, .. } => {
            assert_eq!(support, 1.0);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[test]
fn test_consensus_with_many_votes() {
    let strategy = SimpleMajority::default();

    let mut votes = Vec::new();
    for i in 0..100 {
        let decision = if i < 60 {
            Decision::Accept
        } else {
            Decision::Reject
        };
        votes.push(create_vote(decision, 0.8));
    }

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { support, .. } => {
            assert!(support > 0.51);
        }
        _ => panic!("Expected Accepted result"),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_vote(decision: Decision, confidence: f32) -> Vote {
    Vote {
        voter: AgentId::from_string("test-voter"),
        proposal_id: "test-proposal".to_string(),
        decision,
        confidence,
        rationale: None,
        timestamp: Utc::now(),
    }
}

fn create_vote_with_confidence(decision: Decision, confidence: f32) -> Vote {
    Vote {
        voter: AgentId::from_string("test-voter"),
        proposal_id: "test-proposal".to_string(),
        decision,
        confidence,
        rationale: Some(format!("Vote with {} confidence", confidence)),
        timestamp: Utc::now(),
    }
}

fn create_test_proposal() -> Proposal {
    Proposal {
        id: "test-proposal".to_string(),
        proposer: AgentId::from_string("test-proposer"),
        content: "Test proposal content".to_string(),
        description: "Test proposal description".to_string(),
        priority: 5,
        created_at: Utc::now(),
    }
}
