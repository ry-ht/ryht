//! Multi-Agent Runtime Example
//!
//! This example demonstrates how to use the Axon runtime system to spawn
//! and execute tasks on multiple agents in parallel.

use std::sync::Arc;
use std::time::Duration;

use axon::{
    agents::{AgentId, AgentType},
    coordination::UnifiedMessageBus,
    cortex_bridge::{CortexBridge, WorkspaceId, SessionId},
    orchestration::{
        LeadAgent, LeadAgentConfig, StrategyLibrary, WorkerRegistry,
        ResultSynthesizer, LeadAgentWithRuntime, TaskDelegation,
    },
    runtime::{AgentRuntime, RuntimeConfig, ProcessConfig, ResourceLimits, McpConfig},
    coordination::MessageCoordinator,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Axon Multi-Agent Runtime Example ===\n");

    // 1. Create message bus
    println!("1. Creating message bus...");
    let message_bus = Arc::new(UnifiedMessageBus::new());

    // 2. Create runtime configuration
    println!("2. Configuring runtime...");
    let mut runtime_config = RuntimeConfig::default();

    // Customize process config
    runtime_config.process = ProcessConfig {
        max_concurrent_processes: 5,
        spawn_timeout: Duration::from_secs(30),
        shutdown_grace_period: Duration::from_secs(10),
        enable_isolation: true,
        working_directory: None,
        environment: vec![],
    };

    // Customize resource limits
    runtime_config.resources = ResourceLimits {
        max_memory_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
        cpu_limit_percent: Some(80.0),
        max_file_descriptors: Some(1024),
        max_task_duration: Duration::from_secs(300),
        max_tool_calls_per_task: 50,
        max_output_size_bytes: 10 * 1024 * 1024, // 10MB
        enable_resource_tracking: true,
    };

    // 3. Create and start the runtime
    println!("3. Initializing agent runtime...");
    let runtime = Arc::new(AgentRuntime::new(
        runtime_config,
        message_bus.clone(),
    ));

    runtime.start().await?;
    println!("   Runtime started successfully\n");

    // 4. Spawn worker agents
    println!("4. Spawning worker agents...");

    let developer_id = runtime.spawn_agent(
        "Developer-1".to_string(),
        AgentType::Developer,
        "cortex",
        &["mcp".to_string(), "stdio".to_string()],
    ).await?;
    println!("   ✓ Spawned Developer agent: {}", developer_id);

    let reviewer_id = runtime.spawn_agent(
        "Reviewer-1".to_string(),
        AgentType::Reviewer,
        "cortex",
        &["mcp".to_string(), "stdio".to_string()],
    ).await?;
    println!("   ✓ Spawned Reviewer agent: {}", reviewer_id);

    let tester_id = runtime.spawn_agent(
        "Tester-1".to_string(),
        AgentType::Tester,
        "cortex",
        &["mcp".to_string(), "stdio".to_string()],
    ).await?;
    println!("   ✓ Spawned Tester agent: {}\n", tester_id);

    // 5. Create task delegations
    println!("5. Creating task delegations...");

    let dev_task = TaskDelegation::builder()
        .objective("Implement a new feature for user authentication".to_string())
        .add_scope("auth module".to_string())
        .add_constraint("Don't modify database schema".to_string())
        .max_tool_calls(20)
        .timeout(Duration::from_secs(120))
        .priority(8)
        .required_capabilities(vec!["CodeGeneration".to_string()])
        .build()?;

    let review_task = TaskDelegation::builder()
        .objective("Review authentication code for security issues".to_string())
        .add_scope("auth module".to_string())
        .max_tool_calls(15)
        .timeout(Duration::from_secs(90))
        .priority(7)
        .required_capabilities(vec!["CodeReview".to_string()])
        .build()?;

    let test_task = TaskDelegation::builder()
        .objective("Generate comprehensive tests for authentication".to_string())
        .add_scope("auth module".to_string())
        .max_tool_calls(25)
        .timeout(Duration::from_secs(150))
        .priority(6)
        .required_capabilities(vec!["Testing".to_string()])
        .build()?;

    println!("   ✓ Created 3 task delegations\n");

    // 6. Execute tasks in parallel
    println!("6. Executing tasks in parallel...");

    let tasks = vec![
        (developer_id.clone(), dev_task),
        (reviewer_id.clone(), review_task),
        (tester_id.clone(), test_task),
    ];

    let start = std::time::Instant::now();
    let results = runtime.execute_tasks_parallel(tasks).await;
    let duration = start.elapsed();

    println!("   ✓ All tasks completed in {:.2}s\n", duration.as_secs_f64());

    // 7. Display results
    println!("7. Task Results:");
    for (idx, result) in results.iter().enumerate() {
        match result {
            Ok(worker_result) => {
                println!("\n   Task {} ({}):", idx + 1, worker_result.task.objective);
                println!("   - Worker: {}", worker_result.worker_id);
                println!("   - Success: {}", worker_result.success);
                println!("   - Duration: {:.2}s", worker_result.duration.as_secs_f64());
                println!("   - Tokens: {}", worker_result.tokens_used);
                println!("   - Cost: {}¢", worker_result.cost_cents);
            }
            Err(e) => {
                println!("\n   Task {} failed: {}", idx + 1, e);
            }
        }
    }

    // 8. Get runtime statistics
    println!("\n8. Runtime Statistics:");
    let stats = runtime.get_statistics().await;
    println!("   - Total agents spawned: {}", stats.total_agents_spawned);
    println!("   - Active agents: {}", stats.active_agents);
    println!("   - Total tasks executed: {}", stats.total_tasks_executed);
    println!("   - Total tasks failed: {}", stats.total_tasks_failed);

    if let Some(executor_stats) = stats.executor_stats {
        println!("   - Total tool calls: {}", executor_stats.total_tool_calls);
        println!("   - Avg execution time: {}ms", executor_stats.avg_execution_time_ms);
    }

    // 9. Cleanup
    println!("\n9. Shutting down...");
    runtime.shutdown().await?;
    println!("   ✓ Runtime shutdown complete\n");

    println!("=== Example Complete ===");

    Ok(())
}
