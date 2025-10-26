//! Orchestrator-Worker Pattern Demo
//!
//! This example demonstrates the complete Orchestrator-Worker (hive-mind) pattern
//! implementation based on Anthropic's best practices.
//!
//! # Features Demonstrated
//!
//! 1. Query complexity analysis (Simple/Medium/Complex)
//! 2. Dynamic worker spawning based on complexity
//! 3. Parallel execution of independent workers
//! 4. Task delegation with explicit boundaries
//! 5. Result synthesis from multiple workers
//! 6. Parallel tool execution within workers
//! 7. Resource allocation rules
//! 8. Episodic memory integration
//!
//! # Expected Performance
//!
//! - 90% time reduction for complex queries through parallelization
//! - Intelligent resource allocation based on query complexity
//! - Worker pool management with capability matching

use std::sync::Arc;
use tokio::sync::RwLock;

use axon::orchestration::{
    LeadAgent,
    LeadAgentConfig,
    StrategyLibrary,
    WorkerRegistry,
    WorkerRegistryConfig,
    ResultSynthesizer,
    SynthesizerConfig,
    ParallelToolExecutor,
    QueryComplexity,
};

use axon::coordination::{
    UnifiedMessageBus,
    MessageBusConfig,
    MessageCoordinator,
};

use axon::cortex_bridge::{
    CortexBridge,
    CortexConfig,
    SessionId,
    WorkspaceId,
};

use axon::agents::{AgentId, AgentType, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=".repeat(80));
    println!("Orchestrator-Worker Pattern Demo");
    println!("Based on Anthropic's Multi-Agent Research System Architecture");
    println!("=".repeat(80));
    println!();

    // Step 1: Initialize Cortex Bridge for cognitive memory
    println!("Step 1: Initializing Cortex Bridge for cognitive memory...");
    let cortex_config = CortexConfig::default();
    let cortex = Arc::new(CortexBridge::new(cortex_config).await?);
    println!("  ✓ Cortex Bridge initialized");
    println!();

    // Step 2: Initialize Unified Message Bus
    println!("Step 2: Initializing Unified Message Bus...");
    let message_bus_config = MessageBusConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), message_bus_config));
    println!("  ✓ Message Bus initialized");
    println!();

    // Step 3: Initialize Message Coordinator
    println!("Step 3: Initializing Message Coordinator...");
    let coordinator = Arc::new(MessageCoordinator::new(message_bus.clone(), cortex.clone()));
    println!("  ✓ Message Coordinator initialized");
    println!();

    // Step 4: Initialize Strategy Library
    println!("Step 4: Initializing Strategy Library...");
    let strategy_config = axon::orchestration::strategy_library::StrategyLibraryConfig::default();
    let strategy_library = Arc::new(StrategyLibrary::new(cortex.clone(), strategy_config).await?);
    println!("  ✓ Strategy Library initialized with default strategies");
    println!();

    // Step 5: Initialize Worker Registry
    println!("Step 5: Initializing Worker Registry...");
    let registry_config = WorkerRegistryConfig::default();
    let worker_registry = Arc::new(RwLock::new(WorkerRegistry::new(registry_config)));

    // Register some worker agents
    {
        let mut registry = worker_registry.write().await;

        // Register Developer agents (code generation)
        for i in 1..=3 {
            registry.register_worker(
                AgentId::from_string(format!("developer-{}", i)),
                AgentType::Developer,
                vec![
                    "CodeGeneration".to_string(),
                    "CodeRefactoring".to_string(),
                ],
            )?;
        }

        // Register Reviewer agents (code review)
        for i in 1..=2 {
            registry.register_worker(
                AgentId::from_string(format!("reviewer-{}", i)),
                AgentType::Reviewer,
                vec![
                    "CodeReview".to_string(),
                    "CodeAnalysis".to_string(),
                ],
            )?;
        }

        // Register Researcher agents (information retrieval)
        for i in 1..=5 {
            registry.register_worker(
                AgentId::from_string(format!("researcher-{}", i)),
                AgentType::Researcher,
                vec!["InformationRetrieval".to_string()],
            )?;
        }
    }

    let stats = worker_registry.read().await.get_statistics();
    println!("  ✓ Worker Registry initialized");
    println!("    - Total workers: {}", stats.total_workers);
    println!("    - Available workers: {}", stats.idle_workers);
    println!();

    // Step 6: Initialize Result Synthesizer
    println!("Step 6: Initializing Result Synthesizer...");
    let synthesizer_config = SynthesizerConfig::default();
    let result_synthesizer = Arc::new(ResultSynthesizer::new(synthesizer_config));
    println!("  ✓ Result Synthesizer initialized");
    println!();

    // Step 7: Initialize Lead Agent (Orchestrator)
    println!("Step 7: Initializing Lead Agent (Orchestrator)...");
    let lead_agent_config = LeadAgentConfig {
        adaptive_allocation: true,
        early_termination: true,
        dynamic_spawning: true,
        max_concurrent_executions: 5,
        default_timeout: std::time::Duration::from_secs(300),
        enable_progress_tracking: true,
    };

    let lead_agent = LeadAgent::new(
        "OrchestratorAgent".to_string(),
        cortex.clone(),
        strategy_library.clone(),
        worker_registry.clone(),
        result_synthesizer.clone(),
        message_bus.clone(),
        coordinator.clone(),
        lead_agent_config,
    );
    println!("  ✓ Lead Agent initialized");
    println!("    - ID: {}", lead_agent.id());
    println!("    - Name: {}", lead_agent.name());
    println!();

    // Step 8: Initialize Parallel Tool Executor
    println!("Step 8: Initializing Parallel Tool Executor...");
    let tool_executor = ParallelToolExecutor::new(
        10, // max concurrent tools
        std::time::Duration::from_secs(60), // tool timeout
    );
    println!("  ✓ Parallel Tool Executor initialized");
    println!();

    // Demo: Execute queries of different complexities
    println!("=".repeat(80));
    println!("Demo: Executing Queries with Different Complexity Levels");
    println!("=".repeat(80));
    println!();

    let workspace_id = WorkspaceId::from("demo-workspace".to_string());
    let session_id = SessionId::from("demo-session".to_string());

    // Demo 1: Simple Query
    println!("Demo 1: Simple Query (1 worker, 3-10 tool calls)");
    println!("-".repeat(80));
    let simple_query = "What is the current version of Rust?";
    println!("Query: {}", simple_query);
    println!();

    match lead_agent.handle_query(simple_query, workspace_id.clone(), session_id.clone()).await {
        Ok(result) => {
            println!("Result:");
            println!("  - Workers used: {}", result.worker_count);
            println!("  - Success: {}", result.success);
            println!("  - Confidence: {:.1}%", result.confidence * 100.0);
            println!("  - Total tokens: {}", result.total_tokens_used);
            println!("  - Total cost: ${:.3}", result.total_cost_cents as f64 / 100.0);
            println!("  - Parallel efficiency: {:.1}%", result.parallel_efficiency * 100.0);
            println!("  - Time reduction: {:.1}%", result.time_reduction_percent);
            println!("  - Summary: {}", truncate_string(&result.summary, 200));
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    println!();

    // Demo 2: Medium Query
    println!("Demo 2: Medium Query (2-4 workers, 10-15 calls each)");
    println!("-".repeat(80));
    let medium_query = "Compare async/await in Rust vs JavaScript, analyze performance implications";
    println!("Query: {}", medium_query);
    println!();

    match lead_agent.handle_query(medium_query, workspace_id.clone(), session_id.clone()).await {
        Ok(result) => {
            println!("Result:");
            println!("  - Workers used: {}", result.worker_count);
            println!("  - Success: {}", result.success);
            println!("  - Confidence: {:.1}%", result.confidence * 100.0);
            println!("  - Total tokens: {}", result.total_tokens_used);
            println!("  - Total cost: ${:.3}", result.total_cost_cents as f64 / 100.0);
            println!("  - Parallel efficiency: {:.1}%", result.parallel_efficiency * 100.0);
            println!("  - Time reduction: {:.1}%", result.time_reduction_percent);
            println!("  - Findings: {} aspects covered", result.findings.len());
            println!("  - Recommendations: {} total", result.recommendations.len());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    println!();

    // Demo 3: Complex Query
    println!("Demo 3: Complex Query (10+ workers with delegation)");
    println!("-".repeat(80));
    let complex_query = "\
        Research and analyze multi-agent orchestration patterns in distributed systems. \
        Compare Anthropic's approach with alternatives like AutoGPT, LangChain agents, \
        and CrewAI. Investigate performance implications, cost optimization strategies, \
        and best practices for production deployment. Provide comprehensive recommendations \
        for implementing a scalable multi-agent system with cognitive memory integration.";
    println!("Query: {}", truncate_string(complex_query, 150));
    println!();

    match lead_agent.handle_query(complex_query, workspace_id.clone(), session_id.clone()).await {
        Ok(result) => {
            println!("Result:");
            println!("  - Workers used: {}", result.worker_count);
            println!("  - Success: {}", result.success);
            println!("  - Confidence: {:.1}%", result.confidence * 100.0);
            println!("  - Total tokens: {}", result.total_tokens_used);
            println!("  - Total cost: ${:.3}", result.total_cost_cents as f64 / 100.0);
            println!("  - Parallel efficiency: {:.1}%", result.parallel_efficiency * 100.0);
            println!("  - Time reduction: {:.1}%", result.time_reduction_percent);
            println!("  - Quality Metrics:");
            println!("    - Completeness: {:.1}%", result.quality_metrics.completeness * 100.0);
            println!("    - Consistency: {:.1}%", result.quality_metrics.consistency * 100.0);
            println!("    - Coverage: {:.1}%", result.quality_metrics.coverage * 100.0);
            println!("    - Redundancy: {:.1}%", result.quality_metrics.redundancy * 100.0);
            println!("  - Findings: {} aspects covered", result.findings.len());
            println!("  - Recommendations: {} total", result.recommendations.len());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    println!();

    // Demo 4: Parallel Tool Execution
    println!("Demo 4: Parallel Tool Execution (90% time reduction)");
    println!("-".repeat(80));

    use axon::orchestration::ToolCall;

    let tools = vec![
        ToolCall {
            tool_id: "search-1".to_string(),
            tool_name: "semantic_search".to_string(),
            params: serde_json::json!({"query": "multi-agent systems"}),
            outputs: vec!["search_results_1".to_string()],
            inputs: vec![],
            priority: 8,
        },
        ToolCall {
            tool_id: "search-2".to_string(),
            tool_name: "semantic_search".to_string(),
            params: serde_json::json!({"query": "distributed coordination"}),
            outputs: vec!["search_results_2".to_string()],
            inputs: vec![],
            priority: 8,
        },
        ToolCall {
            tool_id: "search-3".to_string(),
            tool_name: "semantic_search".to_string(),
            params: serde_json::json!({"query": "cognitive memory"}),
            outputs: vec!["search_results_3".to_string()],
            inputs: vec![],
            priority: 8,
        },
        ToolCall {
            tool_id: "analyze-1".to_string(),
            tool_name: "analyze_results".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["analysis".to_string()],
            inputs: vec!["search_results_1".to_string(), "search_results_2".to_string(), "search_results_3".to_string()],
            priority: 5,
        },
    ];

    println!("Executing {} tools with dependency analysis...", tools.len());

    match tool_executor.execute_tools(tools).await {
        Ok((results, stats)) => {
            println!();
            println!("Tool Execution Results:");
            println!("  - Total tools: {}", stats.total_tools);
            println!("  - Successful: {}", stats.successful);
            println!("  - Failed: {}", stats.failed);
            println!("  - Total duration: {:?}", stats.total_duration);
            println!("  - Sequential duration: {:?}", stats.sequential_duration);
            println!("  - Time saved: {:.1}%", stats.time_saved_percent);
            println!("  - Parallel efficiency: {:.1}%", stats.parallel_efficiency * 100.0);
            println!();
            println!("Individual Tool Results:");
            for result in results {
                println!("  - {} ({}): {} in {:?}",
                    result.tool_name,
                    result.tool_id,
                    if result.success { "✓ Success" } else { "✗ Failed" },
                    result.duration
                );
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    println!();

    // Summary
    println!("=".repeat(80));
    println!("Demo Complete");
    println!("=".repeat(80));
    println!();
    println!("Key Takeaways:");
    println!("  1. Query complexity determines resource allocation automatically");
    println!("  2. Simple queries use 1 worker, complex queries use 10+ workers");
    println!("  3. Parallel execution achieves 70-90% time reduction");
    println!("  4. Workers are selected based on capability matching");
    println!("  5. Results are synthesized from multiple workers coherently");
    println!("  6. All communication is tracked in episodic memory for learning");
    println!("  7. Circuit breakers and rate limiting ensure resilience");
    println!("  8. Tool dependencies are analyzed for optimal parallelization");
    println!();
    println!("Performance Metrics:");
    let final_stats = worker_registry.read().await.get_statistics();
    println!("  - Workers available: {}/{}", final_stats.idle_workers, final_stats.total_workers);
    println!("  - Tasks completed: {}", final_stats.total_tasks_completed);
    println!("  - Success rate: {:.1}%", final_stats.average_success_rate * 100.0);
    println!();

    Ok(())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
