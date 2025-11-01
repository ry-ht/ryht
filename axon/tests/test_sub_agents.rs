//! Test sub-agent functionality
//!
//! This test verifies that sub-agents can be launched and managed correctly.

use std::sync::Arc;
use axon::runtime::{RuntimeConfig, SubAgentManager};
use axon::agents::AgentType;

#[tokio::test]
async fn test_launch_sub_agent() {
    // Create runtime config
    let config = RuntimeConfig::default();

    // Create sub-agent manager
    let manager = Arc::new(SubAgentManager::new(config));

    // Launch a developer agent
    let result = manager.launch_agent(
        AgentType::Developer,
        "Generate a hello world function".to_string(),
        serde_json::json!({
            "language": "rust",
            "requirements": ["simple", "documented"]
        }),
    ).await;

    // Verify agent was launched
    assert!(result.is_ok(), "Failed to launch agent: {:?}", result.err());
    let agent_id = result.unwrap();
    assert!(!agent_id.is_empty(), "Agent ID should not be empty");

    // Check agent status
    let status = manager.get_agent_status(&agent_id).await;
    assert!(status.is_ok(), "Failed to get agent status");

    // List agents
    let agents = manager.list_agents().await;
    assert_eq!(agents.len(), 1, "Should have one agent");
    assert_eq!(agents[0].id, agent_id, "Agent ID should match");

    println!("✓ Sub-agent launched successfully: {}", agent_id);
}

#[tokio::test]
async fn test_multiple_sub_agents() {
    let config = RuntimeConfig::default();
    let manager = Arc::new(SubAgentManager::new(config));

    // Launch multiple agents
    let developer_id = manager.launch_agent(
        AgentType::Developer,
        "Task 1".to_string(),
        serde_json::json!({}),
    ).await.unwrap();

    let reviewer_id = manager.launch_agent(
        AgentType::Reviewer,
        "Task 2".to_string(),
        serde_json::json!({}),
    ).await.unwrap();

    // Verify both agents exist
    let agents = manager.list_agents().await;
    assert_eq!(agents.len(), 2, "Should have two agents");

    // Cancel one agent
    let cancel_result = manager.cancel_agent(&developer_id).await;
    assert!(cancel_result.is_ok(), "Failed to cancel agent");

    println!("✓ Multiple sub-agents managed successfully");
}

#[tokio::test]
async fn test_sub_agent_tools() {
    use axon::runtime::{LaunchSubAgentTool, ListSubAgentsTool, GetSubAgentStatusTool};

    let config = RuntimeConfig::default();
    let manager = Arc::new(SubAgentManager::new(config));

    // Test LaunchSubAgentTool
    let launch_tool = LaunchSubAgentTool::new(manager.clone());
    let launch_params = serde_json::json!({
        "agent_type": "Developer",
        "task": "Test task",
        "params": {}
    });

    let launch_result = launch_tool.execute(launch_params).await;
    assert!(launch_result.is_ok(), "Launch tool failed");
    let result = launch_result.unwrap();
    assert!(result.success, "Launch should succeed");

    // Test ListSubAgentsTool
    let list_tool = ListSubAgentsTool::new(manager.clone());
    let list_result = list_tool.execute(serde_json::json!({})).await;
    assert!(list_result.is_ok(), "List tool failed");
    let result = list_result.unwrap();
    assert!(result.success, "List should succeed");

    println!("✓ Sub-agent MCP tools working correctly");
}

#[tokio::test]
async fn test_agent_coordination() {
    let config = RuntimeConfig::default();
    let manager = Arc::new(SubAgentManager::new(config));

    // Launch orchestrator agent
    let orchestrator_id = manager.launch_agent(
        AgentType::Orchestrator,
        "Coordinate development task".to_string(),
        serde_json::json!({
            "project": "test_project",
            "requirements": ["feature_a", "feature_b"]
        }),
    ).await.unwrap();

    // Launch worker agents
    let developer_id = manager.launch_agent(
        AgentType::Developer,
        "Implement feature".to_string(),
        serde_json::json!({"feature": "feature_a"}),
    ).await.unwrap();

    let tester_id = manager.launch_agent(
        AgentType::Tester,
        "Test feature".to_string(),
        serde_json::json!({"feature": "feature_a"}),
    ).await.unwrap();

    // Send coordination message
    let message_result = manager.send_message(
        orchestrator_id.clone(),
        developer_id.clone(),
        axon::runtime::MessageType::TaskAssignment,
        serde_json::json!({
            "task": "implement_feature",
            "priority": "high"
        }),
    ).await;

    assert!(message_result.is_ok(), "Failed to send message");

    // Get messages for developer agent
    let messages = manager.get_messages(&developer_id).await;
    assert!(messages.is_ok(), "Failed to get messages");
    let msgs = messages.unwrap();
    assert_eq!(msgs.len(), 1, "Should have one message");

    println!("✓ Agent coordination working correctly");
}