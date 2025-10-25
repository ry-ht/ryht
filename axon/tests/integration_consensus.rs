//! Integration tests for multi-agent consensus
//!
//! Tests consensus mechanisms with multiple agents voting

mod common;

use axon::consensus::*;
use axon::agents::AgentId;
use chrono::Utc;

// ============================================================================
// Multi-Agent Consensus Tests
// ============================================================================

#[tokio::test]
async fn test_three_agent_consensus() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal("Implement feature X");

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
async fn test_five_agent_supermajority() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal("Major architecture change");

    let participants = vec![
        AgentId::from_string("agent-1"),
        AgentId::from_string("agent-2"),
        AgentId::from_string("agent-3"),
        AgentId::from_string("agent-4"),
        AgentId::from_string("agent-5"),
    ];

    let result = protocol
        .initiate_consensus(proposal, "supermajority", participants)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_consensus_with_different_strategies() {
    let protocol = ConsensusProtocol::new();

    let participants = vec![
        AgentId::from_string("agent-1"),
        AgentId::from_string("agent-2"),
        AgentId::from_string("agent-3"),
    ];

    // Test simple majority
    let result1 = protocol
        .initiate_consensus(
            create_test_proposal("Proposal 1"),
            "simple_majority",
            participants.clone(),
        )
        .await;
    assert!(result1.is_ok());

    // Test supermajority
    let result2 = protocol
        .initiate_consensus(
            create_test_proposal("Proposal 2"),
            "supermajority",
            participants.clone(),
        )
        .await;
    assert!(result2.is_ok());

    // Test weighted
    let result3 = protocol
        .initiate_consensus(
            create_test_proposal("Proposal 3"),
            "weighted",
            participants.clone(),
        )
        .await;
    assert!(result3.is_ok());
}

// ============================================================================
// Voting Pattern Tests
// ============================================================================

#[tokio::test]
async fn test_unanimous_consensus() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 1.0),
        create_vote("agent-2", Decision::Accept, 1.0),
        create_vote("agent-3", Decision::Accept, 1.0),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    match result {
        ConsensusResult::Accepted { unanimous, .. } => {
            assert!(unanimous);
        }
        _ => panic!("Expected Accepted result"),
    }
}

#[tokio::test]
async fn test_split_vote() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 0.9),
        create_vote("agent-2", Decision::Reject, 0.9),
    ];

    let result = strategy.evaluate_votes(votes).unwrap();

    // With 50-50 split, should not reach majority
    match result {
        ConsensusResult::Rejected { support, .. } => {
            assert!(support == 0.5);
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_abstention_handling() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 0.9),
        create_vote("agent-2", Decision::Abstain, 0.5),
        create_vote("agent-3", Decision::Accept, 0.8),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());
}

// ============================================================================
// Quorum Tests
// ============================================================================

#[tokio::test]
async fn test_insufficient_quorum() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal("Test proposal");

    // Only one participant when supermajority requires 60% participation
    let participants = vec![AgentId::from_string("agent-1")];

    let result = protocol
        .initiate_consensus(proposal, "supermajority", participants)
        .await;

    assert!(result.is_err());
    if let Err(ConsensusError::InsufficientQuorum { .. }) = result {
        // Expected error
    } else {
        panic!("Expected InsufficientQuorum error");
    }
}

#[tokio::test]
async fn test_minimum_quorum_met() {
    let protocol = ConsensusProtocol::new();
    let proposal = create_test_proposal("Test proposal");

    // Provide enough participants to meet quorum
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

// ============================================================================
// Decision Types Tests
// ============================================================================

#[tokio::test]
async fn test_conditional_votes() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 0.9),
        create_vote(
            "agent-2",
            Decision::Conditional("needs revision".to_string()),
            0.7,
        ),
        create_vote("agent-3", Decision::Accept, 0.8),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mixed_decisions() {
    let strategy = SimpleMajority::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 0.95),
        create_vote("agent-2", Decision::Reject, 0.90),
        create_vote("agent-3", Decision::Abstain, 0.50),
        create_vote("agent-4", Decision::Accept, 0.85),
        create_vote(
            "agent-5",
            Decision::Conditional("minor changes".to_string()),
            0.70,
        ),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());
}

// ============================================================================
// Confidence Level Tests
// ============================================================================

#[tokio::test]
async fn test_confidence_weighted_voting() {
    let strategy = WeightedVoting::default();

    let votes = vec![
        create_vote("agent-1", Decision::Accept, 1.0),
        create_vote("agent-2", Decision::Accept, 0.5),
        create_vote("agent-3", Decision::Reject, 0.3),
    ];

    let result = strategy.evaluate_votes(votes);
    assert!(result.is_ok());

    match result.unwrap() {
        ConsensusResult::Accepted { support, .. } => {
            // High confidence accepts should dominate
            assert!(support > 0.5);
        }
        _ => panic!("Expected Accepted result"),
    }
}

// ============================================================================
// Proposal Priority Tests
// ============================================================================

#[test]
fn test_proposal_priority_ordering() {
    let proposals = vec![
        create_test_proposal_with_priority("Low priority", 1),
        create_test_proposal_with_priority("High priority", 10),
        create_test_proposal_with_priority("Medium priority", 5),
    ];

    assert!(proposals[1].priority > proposals[0].priority);
    assert!(proposals[1].priority > proposals[2].priority);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_proposal(content: &str) -> Proposal {
    Proposal {
        id: format!("proposal-{}", uuid::Uuid::new_v4()),
        proposer: AgentId::from_string("proposer-1"),
        content: content.to_string(),
        description: format!("Description for {}", content),
        priority: 5,
        created_at: Utc::now(),
    }
}

fn create_test_proposal_with_priority(content: &str, priority: u32) -> Proposal {
    Proposal {
        id: format!("proposal-{}", uuid::Uuid::new_v4()),
        proposer: AgentId::from_string("proposer-1"),
        content: content.to_string(),
        description: format!("Description for {}", content),
        priority,
        created_at: Utc::now(),
    }
}

fn create_vote(voter_id: &str, decision: Decision, confidence: f32) -> Vote {
    Vote {
        voter: AgentId::from_string(voter_id),
        proposal_id: "test-proposal".to_string(),
        decision,
        confidence,
        rationale: Some(format!("Rationale for {}", voter_id)),
        timestamp: Utc::now(),
    }
}
