//! Workflow execution engine

use super::*;
use crate::agents::{
    Agent, AgentType, AgentId, Capability, CapabilityMatcher,
    developer::DeveloperAgent,
    reviewer::ReviewerAgent,
    tester::TesterAgent,
    orchestrator::OrchestratorAgent,
};
use std::sync::Arc;
use tokio::time::{timeout, Duration as TokioDuration};

pub struct WorkflowExecutor {
    agent_pool: Arc<RwLock<AgentPool>>,
    capability_matcher: Arc<RwLock<CapabilityMatcher>>,
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowExecutor {
    pub fn new() -> Self {
        let mut agent_pool = AgentPool::new();
        let mut capability_matcher = CapabilityMatcher::new();

        // Initialize default agents
        agent_pool.initialize_default_agents(&mut capability_matcher);

        Self {
            agent_pool: Arc::new(RwLock::new(agent_pool)),
            capability_matcher: Arc::new(RwLock::new(capability_matcher)),
        }
    }

    pub async fn execute(
        &self,
        workflow: Workflow,
        schedule: ExecutionSchedule,
    ) -> Result<WorkflowResult> {
        let start = std::time::Instant::now();
        let mut task_results = HashMap::new();

        // Execute tasks according to schedule
        for task_id in &schedule.sorted_tasks {
            if let Some(task) = workflow.tasks.iter().find(|t| t.id == *task_id) {
                // Check dependencies are complete
                if !self.check_dependencies(&task.id, &workflow.dependencies, &task_results).await {
                    task_results.insert(task_id.clone(), TaskResult {
                        task_id: task_id.clone(),
                        success: false,
                        output: None,
                        error: Some("Dependencies not met".to_string()),
                    });
                    continue;
                }

                // Execute with timeout
                let task_timeout = TokioDuration::from_secs(300); // 5 minutes default
                let result = timeout(task_timeout, self.execute_task(task)).await;

                let task_result = match result {
                    Ok(Ok(res)) => res,
                    Ok(Err(e)) => TaskResult {
                        task_id: task.id.clone(),
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    },
                    Err(_) => TaskResult {
                        task_id: task.id.clone(),
                        success: false,
                        output: None,
                        error: Some("Task execution timeout".to_string()),
                    },
                };

                task_results.insert(task_id.clone(), task_result);
            }
        }

        let success = task_results.values().all(|r| r.success);

        Ok(WorkflowResult {
            workflow_id: workflow.id,
            success,
            duration: start.elapsed(),
            task_results,
        })
    }

    async fn execute_task(&self, task: &Task) -> Result<TaskResult> {
        // Determine required capabilities based on task type
        let required_capabilities = self.get_required_capabilities(&task.task_type);

        // Find suitable agent
        let agent_id = {
            let matcher = self.capability_matcher.read().await;
            matcher.find_best_agent(&required_capabilities)
                .ok_or_else(|| OrchestrationError::NoSuitableAgent {
                    task_id: task.id.clone()
                })?
        };

        // Get agent from pool and execute
        let mut pool = self.agent_pool.write().await;
        let execution_result = pool.execute_with_agent(&agent_id, task).await;

        match execution_result {
            Ok(output) => Ok(TaskResult {
                task_id: task.id.clone(),
                success: true,
                output: Some(output),
                error: None,
            }),
            Err(e) => Ok(TaskResult {
                task_id: task.id.clone(),
                success: false,
                output: None,
                error: Some(e.to_string()),
            })
        }
    }

    async fn check_dependencies(
        &self,
        task_id: &str,
        dependencies: &HashMap<String, Vec<String>>,
        completed_tasks: &HashMap<String, TaskResult>,
    ) -> bool {
        if let Some(deps) = dependencies.get(task_id) {
            for dep in deps {
                if !completed_tasks.get(dep).is_some_and(|r| r.success) {
                    return false;
                }
            }
        }
        true
    }

    fn get_required_capabilities(&self, task_type: &TaskType) -> HashSet<Capability> {
        let mut caps = HashSet::new();

        match task_type {
            TaskType::Development => {
                caps.insert(Capability::CodeGeneration);
                caps.insert(Capability::CodeRefactoring);
            }
            TaskType::Review => {
                caps.insert(Capability::CodeReview);
                caps.insert(Capability::CodeAnalysis);
            }
            TaskType::Testing => {
                caps.insert(Capability::Testing);
                caps.insert(Capability::TestGeneration);
            }
            TaskType::Documentation => {
                caps.insert(Capability::Documentation);
                caps.insert(Capability::DocGeneration);
            }
            TaskType::Custom(custom_type) => {
                // Map custom types to capabilities
                match custom_type.as_str() {
                    "optimization" => {
                        caps.insert(Capability::CodeOptimization);
                        caps.insert(Capability::PerformanceOptimization);
                    }
                    "security" => {
                        caps.insert(Capability::SecurityAnalysis);
                        caps.insert(Capability::SecurityAudit);
                    }
                    "architecture" => {
                        caps.insert(Capability::SystemDesign);
                        caps.insert(Capability::ArchitectureAnalysis);
                    }
                    _ => {
                        // Default to code generation for unknown custom types
                        caps.insert(Capability::CodeGeneration);
                    }
                }
            }
        }

        caps
    }
}

/// Agent pool for managing available agents
struct AgentPool {
    agents: HashMap<AgentId, Box<dyn Agent>>,
    agent_states: HashMap<AgentId, AgentPoolState>,
}

#[derive(Debug, Clone)]
enum AgentPoolState {
    Idle,
    Busy,
}

impl AgentPool {
    fn new() -> Self {
        Self {
            agents: HashMap::new(),
            agent_states: HashMap::new(),
        }
    }

    fn initialize_default_agents(&mut self, matcher: &mut CapabilityMatcher) {
        // Create default developer agent
        let dev_agent = Box::new(DeveloperAgent::new("Developer-1".to_string()));
        let dev_id = dev_agent.id().clone();
        let dev_caps = dev_agent.capabilities().clone();
        self.agents.insert(dev_id.clone(), dev_agent);
        self.agent_states.insert(dev_id.clone(), AgentPoolState::Idle);
        matcher.register_agent(dev_id, dev_caps);

        // Create default reviewer agent
        let review_agent = Box::new(ReviewerAgent::new("Reviewer-1".to_string()));
        let review_id = review_agent.id().clone();
        let review_caps = review_agent.capabilities().clone();
        self.agents.insert(review_id.clone(), review_agent);
        self.agent_states.insert(review_id.clone(), AgentPoolState::Idle);
        matcher.register_agent(review_id, review_caps);

        // Create default tester agent
        let test_agent = Box::new(TesterAgent::new("Tester-1".to_string()));
        let test_id = test_agent.id().clone();
        let test_caps = test_agent.capabilities().clone();
        self.agents.insert(test_id.clone(), test_agent);
        self.agent_states.insert(test_id.clone(), AgentPoolState::Idle);
        matcher.register_agent(test_id, test_caps);

        // Create orchestrator agent
        let orch_agent = Box::new(OrchestratorAgent::new("Orchestrator-1".to_string()));
        let orch_id = orch_agent.id().clone();
        let orch_caps = orch_agent.capabilities().clone();
        self.agents.insert(orch_id.clone(), orch_agent);
        self.agent_states.insert(orch_id.clone(), AgentPoolState::Idle);
        matcher.register_agent(orch_id, orch_caps);
    }

    async fn execute_with_agent(
        &mut self,
        agent_id: &AgentId,
        task: &Task,
    ) -> std::result::Result<serde_json::Value, String> {
        // Check if agent is available
        let state = self.agent_states.get(agent_id)
            .ok_or_else(|| format!("Agent not found: {}", agent_id))?;

        if !matches!(state, AgentPoolState::Idle) {
            return Err("Agent is busy".to_string());
        }

        // Mark agent as busy
        self.agent_states.insert(agent_id.clone(), AgentPoolState::Busy);

        // Execute task based on agent type
        let agent = self.agents.get(agent_id)
            .ok_or_else(|| format!("Agent not found in pool: {}", agent_id))?;

        let start_time = std::time::Instant::now();

        // Simulate actual task execution based on task type and input
        let result = match agent.agent_type() {
            AgentType::Developer => self.execute_developer_task(task),
            AgentType::Reviewer => self.execute_reviewer_task(task),
            AgentType::Tester => self.execute_tester_task(task),
            AgentType::Orchestrator => self.execute_orchestrator_task(task),
            _ => self.execute_generic_task(task),
        };

        // Update agent metrics
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let tokens_used = 500; // Simulated token usage
        let cost_cents = 1; // Simulated cost

        if result.is_ok() {
            agent.metrics().record_success(duration_ms, tokens_used, cost_cents);
        } else {
            agent.metrics().record_failure();
        }

        // Mark agent as idle again
        self.agent_states.insert(agent_id.clone(), AgentPoolState::Idle);

        result
    }

    fn execute_developer_task(&self, task: &Task) -> std::result::Result<serde_json::Value, String> {
        // Parse task input and generate appropriate output
        let task_description = task.input.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Generate code");

        Ok(serde_json::json!({
            "task_type": "development",
            "task_id": task.id,
            "code_generated": true,
            "description": task_description,
            "files_modified": ["src/main.rs"],
            "lines_added": 42,
            "lines_removed": 10,
            "status": "completed"
        }))
    }

    fn execute_reviewer_task(&self, task: &Task) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "task_type": "review",
            "task_id": task.id,
            "review_completed": true,
            "issues_found": 3,
            "suggestions": ["Consider error handling", "Add documentation"],
            "approval_status": "approved_with_suggestions",
            "status": "completed"
        }))
    }

    fn execute_tester_task(&self, task: &Task) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "task_type": "testing",
            "task_id": task.id,
            "tests_generated": 10,
            "tests_passed": 9,
            "tests_failed": 1,
            "coverage_percentage": 85.5,
            "status": "completed"
        }))
    }

    fn execute_orchestrator_task(&self, task: &Task) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "task_type": "orchestration",
            "task_id": task.id,
            "subtasks_created": 5,
            "delegation_complete": true,
            "status": "completed"
        }))
    }

    fn execute_generic_task(&self, task: &Task) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "task_type": "generic",
            "task_id": task.id,
            "input_processed": task.input,
            "status": "completed"
        }))
    }
}
