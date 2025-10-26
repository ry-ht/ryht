//! Common test utilities for Axon tests
//!
//! This module provides shared utilities, mocks, and helpers for testing
//! the multi-agent system components.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use axon::agents::*;
use axon::consensus::*;
use axon::cortex_bridge::*;

// Type alias to disambiguate AgentId
use axon::agents::AgentId as AxonAgentId;

/// Mock Cortex server for testing
pub struct MockCortexServer {
    sessions: Arc<RwLock<HashMap<String, MockSession>>>,
    episodes: Arc<RwLock<Vec<Episode>>>,
    patterns: Arc<RwLock<HashMap<String, Pattern>>>,
    locks: Arc<RwLock<HashMap<String, MockLock>>>,
}

impl MockCortexServer {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            episodes: Arc::new(RwLock::new(Vec::new())),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, session_id: String) -> String {
        let session = MockSession {
            id: session_id.clone(),
            files: HashMap::new(),
            created_at: Utc::now(),
        };
        self.sessions.write().await.insert(session_id.clone(), session);
        session_id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<MockSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn store_episode(&self, episode: Episode) -> String {
        let id = format!("episode_{}", self.episodes.read().await.len());
        self.episodes.write().await.push(episode);
        id
    }

    pub async fn store_pattern(&self, pattern: Pattern) -> String {
        let id = format!("pattern_{}", self.patterns.read().await.len());
        self.patterns.write().await.insert(id.clone(), pattern);
        id
    }
}

#[derive(Clone)]
pub struct MockSession {
    pub id: String,
    pub files: HashMap<String, String>,
    pub created_at: chrono::DateTime<Utc>,
}

pub struct MockLock {
    pub id: String,
    pub entity_id: String,
    pub lock_type: LockType,
    pub holder: AxonAgentId,
}

/// Create a test agent with specific capabilities
pub fn create_test_agent(
    name: &str,
    agent_type: AgentType,
    capabilities: Vec<Capability>,
) -> Box<dyn Agent> {
    let caps: HashSet<Capability> = capabilities.into_iter().collect();

    match agent_type {
        AgentType::Developer => Box::new(DeveloperAgent::new(name.to_string())),
        AgentType::Reviewer => Box::new(ReviewerAgent::new(name.to_string())),
        AgentType::Tester => Box::new(TesterAgent::new(name.to_string())),
        AgentType::Documenter => {
            // DocumenterAgent requires Arc<CortexBridge>, which we don't have in test helpers
            // For now, return a DeveloperAgent as a placeholder
            // In real usage, proper CortexBridge mock should be passed
            Box::new(DeveloperAgent::new(name.to_string()))
        }
        AgentType::Architect => Box::new(ArchitectAgent::new(name.to_string())),
        AgentType::Researcher => Box::new(ResearcherAgent::new(name.to_string())),
        AgentType::Optimizer => Box::new(OptimizerAgent::new(name.to_string())),
        AgentType::Orchestrator => Box::new(OrchestratorAgent::new(name.to_string())),
        _ => Box::new(DeveloperAgent::new(name.to_string())),
    }
}

/// Create a test vote
pub fn create_test_vote(
    voter: AxonAgentId,
    proposal_id: String,
    decision: Decision,
    confidence: f32,
) -> Vote {
    Vote {
        voter,
        proposal_id,
        decision,
        confidence,
        rationale: Some("Test vote rationale".to_string()),
        timestamp: Utc::now(),
    }
}

/// Create a test proposal
pub fn create_test_proposal(proposer: AxonAgentId, content: &str) -> Proposal {
    Proposal {
        id: format!("proposal_{}", uuid::Uuid::new_v4()),
        proposer,
        content: content.to_string(),
        description: "Test proposal".to_string(),
        priority: 1,
        created_at: Utc::now(),
    }
}

/// Create a test episode
pub fn create_test_episode(
    agent_id: AxonAgentId,
    task_type: &str,
    outcome: &str,
) -> Episode {
    Episode {
        id: format!("episode_{}", uuid::Uuid::new_v4()),
        episode_type: EpisodeType::Task,
        task_description: format!("{} task", task_type),
        agent_id: agent_id.to_string(),
        session_id: None,
        workspace_id: "test-workspace".to_string(),
        entities_created: vec![],
        entities_modified: vec![],
        entities_deleted: vec![],
        files_touched: vec![],
        queries_made: vec![],
        tools_used: vec![],
        solution_summary: outcome.to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({"test": true}),
        errors_encountered: vec![],
        lessons_learned: vec![],
        duration_seconds: 10,
        tokens_used: TokenUsage::default(),
        embedding: vec![],
        created_at: Utc::now(),
        completed_at: Some(Utc::now()),
    }
}

/// Create a test pattern
pub fn create_test_pattern(name: &str, context: &str) -> Pattern {
    Pattern {
        id: format!("pattern_{}", uuid::Uuid::new_v4()),
        pattern_type: PatternType::Code,
        name: name.to_string(),
        description: "Test pattern".to_string(),
        context: context.to_string(),
        before_state: serde_json::json!({}),
        after_state: serde_json::json!({}),
        transformation: serde_json::json!({"steps": ["test"]}),
        times_applied: 0,
        success_rate: 0.0,
        average_improvement: serde_json::json!({}),
        example_episodes: vec![],
        embedding: vec![],
    }
}

/// Assert that an agent has specific capabilities
pub fn assert_has_capabilities(agent: &dyn Agent, expected: Vec<Capability>) {
    let agent_caps = agent.capabilities();
    for cap in expected {
        assert!(
            agent_caps.contains(&cap),
            "Agent {} should have capability {:?}",
            agent.name(),
            cap
        );
    }
}

/// Assert that metrics are updated correctly
pub fn assert_metrics_updated(metrics: &AgentMetrics, expected_tasks: u64) {
    let snapshot = metrics.snapshot();
    assert_eq!(
        snapshot.tasks_completed, expected_tasks,
        "Expected {} completed tasks, got {}",
        expected_tasks, snapshot.tasks_completed
    );
}

/// Wait for async condition with timeout
pub async fn wait_for_condition<F>(mut check: F, timeout_ms: u64) -> bool
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        if check() {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    false
}

/// Create a test capability matcher with preset agents
pub fn create_test_capability_matcher() -> CapabilityMatcher {
    let mut matcher = CapabilityMatcher::new();

    // Register developer agent
    let dev_id = AxonAgentId::from_string("dev-1");
    let mut dev_caps = HashSet::new();
    dev_caps.insert(Capability::CodeGeneration);
    dev_caps.insert(Capability::CodeRefactoring);
    matcher.register_agent(dev_id, dev_caps);

    // Register reviewer agent
    let rev_id = AxonAgentId::from_string("rev-1");
    let mut rev_caps = HashSet::new();
    rev_caps.insert(Capability::CodeReview);
    rev_caps.insert(Capability::StaticAnalysis);
    matcher.register_agent(rev_id, rev_caps);

    // Register tester agent
    let test_id = AxonAgentId::from_string("test-1");
    let mut test_caps = HashSet::new();
    test_caps.insert(Capability::Testing);
    test_caps.insert(Capability::TestGeneration);
    matcher.register_agent(test_id, test_caps);

    matcher
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_agent() {
        let agent = create_test_agent(
            "test-dev",
            AgentType::Developer,
            vec![Capability::CodeGeneration],
        );
        assert_eq!(agent.agent_type(), AgentType::Developer);
        assert_eq!(agent.name(), "test-dev");
    }

    #[test]
    fn test_create_test_vote() {
        let voter = AxonAgentId::from_string("voter-1");
        let vote = create_test_vote(
            voter.clone(),
            "proposal-1".to_string(),
            Decision::Accept,
            0.9,
        );
        assert_eq!(vote.voter, voter);
        assert_eq!(vote.decision, Decision::Accept);
        assert_eq!(vote.confidence, 0.9);
    }

    #[tokio::test]
    async fn test_mock_cortex_server() {
        let server = MockCortexServer::new();
        let session_id = server.create_session("test-session".to_string()).await;
        assert_eq!(session_id, "test-session");

        let session = server.get_session(&session_id).await;
        assert!(session.is_some());
    }

    #[test]
    fn test_capability_matcher_creation() {
        let matcher = create_test_capability_matcher();

        let mut required = HashSet::new();
        required.insert(Capability::CodeGeneration);

        let agents = matcher.find_capable_agents(&required);
        assert_eq!(agents.len(), 1);
    }
}
