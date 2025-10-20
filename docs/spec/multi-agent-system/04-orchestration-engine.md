# Axon: Orchestration Engine

## Overview

The Orchestration Engine is the heart of the Axon system, responsible for coordinating task execution between agents. The engine is based on DAG (Directed Acyclic Graph) for modeling task dependencies and provides efficient parallel execution with automatic error handling and integration with Cortex for state management.

## Orchestration Engine Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Orchestration Engine                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────┐  │
│  │   Workflow     │  │      Task      │  │    Agent     │  │
│  │   Manager      │  │   Scheduler    │  │  Assignment  │  │
│  └────────┬───────┘  └────────┬───────┘  └──────┬───────┘  │
│           │                   │                   │          │
│           └───────────────────┼───────────────────┘          │
│                               │                              │
│  ┌────────────────────────────▼──────────────────────────┐  │
│  │              DAG Execution Engine                      │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────────┐    │  │
│  │  │  DAG     │  │Parallel  │  │   Dependency    │    │  │
│  │  │Validator │  │Executor  │  │    Resolver     │    │  │
│  │  └──────────┘  └──────────┘  └─────────────────┘    │  │
│  └────────────────────────────────────────────────────┘  │
│                               │                           │
│  ┌────────────────────────────▼──────────────────────────┐  │
│  │           Error Handler & Retry Logic                 │  │
│  └────────────────────────────────────────────────────────┘  │
│                               │                           │
│  ┌────────────────────────────▼──────────────────────────┐  │
│  │         Cortex Integration Layer                      │  │
│  │         (Task Management via /tasks)               │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## DAG-Based Workflow Engine

### Структура Workflow

```rust
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Workflow represents a directed acyclic graph of tasks
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub description: String,
    pub tasks: Vec<Task>,
    pub dependencies: HashMap<TaskId, Vec<TaskId>>,
    pub metadata: WorkflowMetadata,
}

#[derive(Debug, Clone)]
pub struct WorkflowMetadata {
    pub created_at: DateTime<Utc>,
    pub created_by: AgentId,
    pub priority: Priority,
    pub timeout: Duration,
    pub max_retries: u32,
}

/// Task is a unit of work in a workflow
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub task_type: TaskType,
    pub requirements: TaskRequirements,
    pub input: TaskInput,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    /// Code development
    Development {
        language: String,
        scope: Vec<String>,
    },
    /// Code review
    Review {
        review_type: ReviewType,
    },
    /// Testing
    Testing {
        test_type: TestType,
        coverage_threshold: f32,
    },
    /// Documentation
    Documentation {
        format: DocFormat,
    },
    /// Refactoring
    Refactoring {
        optimization_level: OptimizationLevel,
    },
    /// Custom task
    Custom {
        handler: String,
        config: serde_json::Value,
    },
}

#[derive(Debug, Clone)]
pub struct TaskRequirements {
    pub capabilities: Vec<Capability>,
    pub estimated_duration: Duration,
    pub resources: ResourceRequirements,
}

#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub gpu_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

### DAG Validator

```rust
/// Validator checks DAG correctness before execution
pub struct DagValidator;

impl DagValidator {
    /// Checks that the graph is a DAG (no cycles)
    pub fn validate(workflow: &Workflow) -> Result<(), OrchestrationError> {
        // 1. Check for cycles (DFS)
        Self::check_cycles(&workflow.dependencies)?;

        // 2. Check that all dependencies exist
        Self::check_dependencies_exist(workflow)?;

        // 3. Check task requirements
        Self::check_task_requirements(workflow)?;

        // 4. Check time estimates
        Self::check_time_estimates(workflow)?;

        Ok(())
    }

    fn check_cycles(deps: &HashMap<TaskId, Vec<TaskId>>) -> Result<(), OrchestrationError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in deps.keys() {
            if !visited.contains(task_id) {
                if Self::has_cycle_util(task_id, deps, &mut visited, &mut rec_stack) {
                    return Err(OrchestrationError::CycleDetected {
                        task_id: task_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    fn has_cycle_util(
        task_id: &TaskId,
        deps: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
    ) -> bool {
        visited.insert(task_id.clone());
        rec_stack.insert(task_id.clone());

        if let Some(dependencies) = deps.get(task_id) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if Self::has_cycle_util(dep, deps, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        false
    }

    fn check_dependencies_exist(workflow: &Workflow) -> Result<(), OrchestrationError> {
        let task_ids: HashSet<_> = workflow.tasks.iter().map(|t| t.id.clone()).collect();

        for (task_id, deps) in &workflow.dependencies {
            if !task_ids.contains(task_id) {
                return Err(OrchestrationError::TaskNotFound {
                    task_id: task_id.clone(),
                });
            }

            for dep_id in deps {
                if !task_ids.contains(dep_id) {
                    return Err(OrchestrationError::DependencyNotFound {
                        task_id: task_id.clone(),
                        dependency_id: dep_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    fn check_task_requirements(workflow: &Workflow) -> Result<(), OrchestrationError> {
        for task in &workflow.tasks {
            // Проверка требуемых capabilities
            if task.requirements.capabilities.is_empty() {
                return Err(OrchestrationError::InvalidRequirements {
                    task_id: task.id.clone(),
                    reason: "No capabilities specified".to_string(),
                });
            }

            // Проверка ресурсов
            if task.requirements.resources.cpu_cores == 0 {
                return Err(OrchestrationError::InvalidRequirements {
                    task_id: task.id.clone(),
                    reason: "CPU cores must be > 0".to_string(),
                });
            }
        }

        Ok(())
    }

    fn check_time_estimates(workflow: &Workflow) -> Result<(), OrchestrationError> {
        let total_estimate: Duration = workflow.tasks
            .iter()
            .map(|t| t.requirements.estimated_duration)
            .sum();

        if total_estimate > workflow.metadata.timeout {
            return Err(OrchestrationError::TimeoutTooShort {
                estimated: total_estimate,
                timeout: workflow.metadata.timeout,
            });
        }

        Ok(())
    }
}
```

## Task Decomposition and Scheduling

### Task Scheduler

```rust
/// Scheduler determines task execution order
pub struct TaskScheduler {
    cortex_bridge: Arc<CortexBridge>,
    agent_pool: Arc<RwLock<AgentPool>>,
}

impl TaskScheduler {
    pub fn new(cortex_bridge: Arc<CortexBridge>, agent_pool: Arc<RwLock<AgentPool>>) -> Self {
        Self {
            cortex_bridge,
            agent_pool,
        }
    }

    /// Creates task execution schedule based on DAG
    pub async fn create_schedule(&self, workflow: &Workflow) -> Result<ExecutionSchedule> {
        // 1. Topological sorting to determine order
        let sorted_tasks = self.topological_sort(workflow)?;

        // 2. Determine parallelism levels
        let levels = self.compute_execution_levels(workflow, &sorted_tasks)?;

        // 3. Estimate execution time
        let critical_path = self.compute_critical_path(workflow, &sorted_tasks)?;

        // 4. Resource allocation
        let resource_allocation = self.allocate_resources(workflow, &levels).await?;

        Ok(ExecutionSchedule {
            workflow_id: workflow.id.clone(),
            levels,
            sorted_tasks,
            critical_path,
            resource_allocation,
            estimated_duration: critical_path.duration,
        })
    }

    /// Topological sorting of tasks
    fn topological_sort(&self, workflow: &Workflow) -> Result<Vec<TaskId>> {
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        let mut sorted = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize in-degree for each task
        for task in &workflow.tasks {
            in_degree.insert(task.id.clone(), 0);
        }

        // Count in-degree
        for deps in workflow.dependencies.values() {
            for dep in deps {
                *in_degree.get_mut(dep).unwrap() += 1;
            }
        }

        // Add tasks without dependencies to queue
        for (task_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(task_id.clone());
            }
        }

        // Kahn's algorithm
        while let Some(task_id) = queue.pop_front() {
            sorted.push(task_id.clone());

            if let Some(dependencies) = workflow.dependencies.get(&task_id) {
                for dep in dependencies {
                    let degree = in_degree.get_mut(dep).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        if sorted.len() != workflow.tasks.len() {
            return Err(OrchestrationError::InvalidDag {
                reason: "Not all tasks could be sorted - cycle detected".to_string(),
            });
        }

        Ok(sorted)
    }

    /// Computes levels for parallel execution
    fn compute_execution_levels(
        &self,
        workflow: &Workflow,
        sorted_tasks: &[TaskId],
    ) -> Result<Vec<ExecutionLevel>> {
        let mut task_to_level: HashMap<TaskId, usize> = HashMap::new();
        let mut max_level = 0;

        // Compute level for each task
        for task_id in sorted_tasks {
            let mut level = 0;

            // Find maximum level among dependencies
            if let Some(deps) = workflow.dependencies.get(task_id) {
                for dep in deps {
                    if let Some(&dep_level) = task_to_level.get(dep) {
                        level = level.max(dep_level + 1);
                    }
                }
            }

            task_to_level.insert(task_id.clone(), level);
            max_level = max_level.max(level);
        }

        // Group tasks by levels
        let mut levels = vec![Vec::new(); max_level + 1];
        for (task_id, level) in task_to_level {
            levels[level].push(task_id);
        }

        // Create ExecutionLevel for each level
        let execution_levels = levels
            .into_iter()
            .enumerate()
            .map(|(level_num, task_ids)| {
                let tasks: Vec<_> = task_ids
                    .into_iter()
                    .filter_map(|id| workflow.tasks.iter().find(|t| t.id == id).cloned())
                    .collect();

                ExecutionLevel {
                    level: level_num,
                    tasks,
                    parallel_execution: true,
                }
            })
            .collect();

        Ok(execution_levels)
    }

    /// Computes critical path (longest path)
    fn compute_critical_path(
        &self,
        workflow: &Workflow,
        sorted_tasks: &[TaskId],
    ) -> Result<CriticalPath> {
        let mut earliest_start: HashMap<TaskId, Duration> = HashMap::new();
        let mut latest_finish: HashMap<TaskId, Duration> = HashMap::new();

        // Forward pass - compute earliest start
        for task_id in sorted_tasks {
            let task = workflow.tasks.iter().find(|t| t.id == *task_id).unwrap();
            let mut earliest = Duration::ZERO;

            if let Some(deps) = workflow.dependencies.get(task_id) {
                for dep_id in deps {
                    let dep = workflow.tasks.iter().find(|t| t.id == *dep_id).unwrap();
                    let dep_finish = earliest_start[dep_id] + dep.requirements.estimated_duration;
                    earliest = earliest.max(dep_finish);
                }
            }

            earliest_start.insert(task_id.clone(), earliest);
        }

        // Backward pass - compute latest finish
        let total_duration = sorted_tasks
            .iter()
            .map(|id| {
                let task = workflow.tasks.iter().find(|t| t.id == *id).unwrap();
                earliest_start[id] + task.requirements.estimated_duration
            })
            .max()
            .unwrap_or(Duration::ZERO);

        for task_id in sorted_tasks.iter().rev() {
            let task = workflow.tasks.iter().find(|t| t.id == *task_id).unwrap();
            let mut latest = total_duration;

            // Find all tasks that depend on the current one
            for (dependent_id, deps) in &workflow.dependencies {
                if deps.contains(task_id) {
                    let dependent_latest = latest_finish
                        .get(dependent_id)
                        .copied()
                        .unwrap_or(total_duration);
                    let dependent = workflow.tasks.iter().find(|t| t.id == *dependent_id).unwrap();
                    latest = latest.min(dependent_latest - dependent.requirements.estimated_duration);
                }
            }

            latest_finish.insert(task_id.clone(), latest);
        }

        // Find critical path (tasks with zero slack)
        let critical_tasks: Vec<_> = sorted_tasks
            .iter()
            .filter(|&task_id| {
                let task = workflow.tasks.iter().find(|t| t.id == *task_id).unwrap();
                let slack = latest_finish[task_id] - earliest_start[task_id] - task.requirements.estimated_duration;
                slack.as_secs() == 0
            })
            .cloned()
            .collect();

        Ok(CriticalPath {
            tasks: critical_tasks,
            duration: total_duration,
        })
    }

    /// Allocates resources for execution
    async fn allocate_resources(
        &self,
        workflow: &Workflow,
        levels: &[ExecutionLevel],
    ) -> Result<ResourceAllocation> {
        let mut allocation = ResourceAllocation::default();
        let agent_pool = self.agent_pool.read().await;

        for level in levels {
            for task in &level.tasks {
                // Find suitable agent
                let agent = agent_pool
                    .find_suitable_agent(&task.requirements)
                    .ok_or_else(|| OrchestrationError::NoSuitableAgent {
                        task_id: task.id.clone(),
                        requirements: task.requirements.clone(),
                    })?;

                allocation.assignments.insert(task.id.clone(), agent.id.clone());
            }
        }

        Ok(allocation)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionSchedule {
    pub workflow_id: WorkflowId,
    pub levels: Vec<ExecutionLevel>,
    pub sorted_tasks: Vec<TaskId>,
    pub critical_path: CriticalPath,
    pub resource_allocation: ResourceAllocation,
    pub estimated_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ExecutionLevel {
    pub level: usize,
    pub tasks: Vec<Task>,
    pub parallel_execution: bool,
}

#[derive(Debug, Clone)]
pub struct CriticalPath {
    pub tasks: Vec<TaskId>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceAllocation {
    pub assignments: HashMap<TaskId, AgentId>,
}
```

## Agent Assignment Algorithms

### Agent Pool Management

```rust
/// Agent Pool manages available agents
pub struct AgentPool {
    agents: HashMap<AgentId, Agent>,
    availability: HashMap<AgentId, AgentAvailability>,
    capabilities_index: HashMap<Capability, Vec<AgentId>>,
}

#[derive(Debug, Clone)]
pub struct AgentAvailability {
    pub status: AgentStatus,
    pub current_load: f32,
    pub max_concurrent_tasks: usize,
    pub active_tasks: Vec<TaskId>,
}

impl AgentPool {
    /// Finds the most suitable agent for a task
    pub fn find_suitable_agent(&self, requirements: &TaskRequirements) -> Option<&Agent> {
        // 1. Filter agents by capabilities
        let candidates: Vec<_> = self.agents
            .values()
            .filter(|agent| {
                requirements.capabilities.iter().all(|cap| agent.capabilities.contains(cap))
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // 2. Filter by availability
        let available: Vec<_> = candidates
            .into_iter()
            .filter(|agent| {
                if let Some(avail) = self.availability.get(&agent.id) {
                    avail.status == AgentStatus::Idle ||
                    (avail.status == AgentStatus::Working &&
                     avail.active_tasks.len() < avail.max_concurrent_tasks)
                } else {
                    false
                }
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        // 3. Choose agent with minimum load
        available
            .into_iter()
            .min_by(|a, b| {
                let load_a = self.availability[&a.id].current_load;
                let load_b = self.availability[&b.id].current_load;
                load_a.partial_cmp(&load_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Assigns a task to an agent
    pub fn assign_task(&mut self, agent_id: &AgentId, task_id: TaskId) -> Result<()> {
        let avail = self.availability
            .get_mut(agent_id)
            .ok_or_else(|| OrchestrationError::AgentNotFound {
                agent_id: agent_id.clone(),
            })?;

        avail.active_tasks.push(task_id);
        avail.current_load = avail.active_tasks.len() as f32 / avail.max_concurrent_tasks as f32;

        if avail.status == AgentStatus::Idle {
            avail.status = AgentStatus::Working;
        }

        Ok(())
    }

    /// Releases agent after task completion
    pub fn release_task(&mut self, agent_id: &AgentId, task_id: &TaskId) -> Result<()> {
        let avail = self.availability
            .get_mut(agent_id)
            .ok_or_else(|| OrchestrationError::AgentNotFound {
                agent_id: agent_id.clone(),
            })?;

        avail.active_tasks.retain(|id| id != task_id);
        avail.current_load = avail.active_tasks.len() as f32 / avail.max_concurrent_tasks as f32;

        if avail.active_tasks.is_empty() {
            avail.status = AgentStatus::Idle;
        }

        Ok(())
    }

    /// Gets agent pool statistics
    pub fn get_statistics(&self) -> PoolStatistics {
        let total = self.agents.len();
        let idle = self.availability
            .values()
            .filter(|a| a.status == AgentStatus::Idle)
            .count();
        let working = self.availability
            .values()
            .filter(|a| a.status == AgentStatus::Working)
            .count();
        let avg_load = self.availability
            .values()
            .map(|a| a.current_load)
            .sum::<f32>() / total as f32;

        PoolStatistics {
            total_agents: total,
            idle_agents: idle,
            working_agents: working,
            average_load: avg_load,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_agents: usize,
    pub idle_agents: usize,
    pub working_agents: usize,
    pub average_load: f32,
}
```

## Parallel and Sequential Execution Patterns

### Workflow Executor

```rust
/// Executor runs workflow according to schedule
pub struct WorkflowExecutor {
    cortex_bridge: Arc<CortexBridge>,
    agent_pool: Arc<RwLock<AgentPool>>,
    message_bus: Arc<MessageBus>,
    metrics: Arc<ExecutorMetrics>,
}

impl WorkflowExecutor {
    pub async fn execute(
        &self,
        workflow: Workflow,
        schedule: ExecutionSchedule,
    ) -> Result<WorkflowResult> {
        let start_time = Instant::now();
        let workflow_id = workflow.id.clone();

        // Create task in Cortex for tracking
        let cortex_task_id = self.cortex_bridge.create_task(TaskDefinition {
            title: workflow.name.clone(),
            description: workflow.description.clone(),
            workspace_id: "default".to_string(),
            estimated_hours: schedule.estimated_duration.as_secs_f64() / 3600.0,
        }).await?;

        info!("Executing workflow {} with {} levels", workflow_id, schedule.levels.len());

        let mut results: HashMap<TaskId, TaskResult> = HashMap::new();
        let mut failed_tasks: Vec<TaskId> = Vec::new();

        // Execute levels sequentially, tasks within level - in parallel
        for level in &schedule.levels {
            info!("Executing level {} with {} tasks", level.level, level.tasks.len());

            if level.parallel_execution {
                // Parallel execution of level tasks
                let level_results = self.execute_level_parallel(&workflow, level, &schedule.resource_allocation).await?;

                for (task_id, result) in level_results {
                    if result.success {
                        results.insert(task_id, result);
                    } else {
                        failed_tasks.push(task_id);
                    }
                }
            } else {
                // Sequential execution
                for task in &level.tasks {
                    let result = self.execute_task(&workflow, task, &schedule.resource_allocation).await?;

                    if result.success {
                        results.insert(task.id.clone(), result);
                    } else {
                        failed_tasks.push(task.id.clone());
                        break; // Interrupt level execution on error
                    }
                }
            }

            // If there are failed tasks, interrupt workflow
            if !failed_tasks.is_empty() {
                break;
            }
        }

        let duration = start_time.elapsed();
        let success = failed_tasks.is_empty();

        // Update task status in Cortex
        self.cortex_bridge.update_task(
            &cortex_task_id,
            if success { TaskStatus::Completed } else { TaskStatus::Failed },
            TaskMetadata {
                duration,
                notes: if failed_tasks.is_empty() {
                    None
                } else {
                    Some(format!("Failed tasks: {:?}", failed_tasks))
                },
            },
        ).await?;

        // Record metrics
        self.metrics.record_workflow_execution(workflow_id.clone(), duration, success);

        Ok(WorkflowResult {
            workflow_id,
            success,
            task_results: results,
            failed_tasks,
            duration,
            cortex_task_id,
        })
    }

    /// Parallel execution of level tasks
    async fn execute_level_parallel(
        &self,
        workflow: &Workflow,
        level: &ExecutionLevel,
        allocation: &ResourceAllocation,
    ) -> Result<HashMap<TaskId, TaskResult>> {
        let mut handles = Vec::new();

        for task in &level.tasks {
            let workflow_clone = workflow.clone();
            let task_clone = task.clone();
            let allocation_clone = allocation.clone();
            let executor = self.clone();

            let handle = tokio::spawn(async move {
                let result = executor.execute_task(&workflow_clone, &task_clone, &allocation_clone).await;
                (task_clone.id.clone(), result)
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let results = futures::future::join_all(handles).await;

        let mut task_results = HashMap::new();
        for result in results {
            match result {
                Ok((task_id, Ok(task_result))) => {
                    task_results.insert(task_id, task_result);
                }
                Ok((task_id, Err(e))) => {
                    error!("Task {} failed: {}", task_id, e);
                    task_results.insert(task_id, TaskResult {
                        task_id: task_id.clone(),
                        success: false,
                        error: Some(e.to_string()),
                        output: None,
                        duration: Duration::ZERO,
                    });
                }
                Err(e) => {
                    error!("Task execution panicked: {}", e);
                }
            }
        }

        Ok(task_results)
    }

    /// Execution of a single task
    async fn execute_task(
        &self,
        workflow: &Workflow,
        task: &Task,
        allocation: &ResourceAllocation,
    ) -> Result<TaskResult> {
        let start_time = Instant::now();
        let agent_id = allocation.assignments
            .get(&task.id)
            .ok_or_else(|| OrchestrationError::NoAgentAssigned {
                task_id: task.id.clone(),
            })?;

        info!("Executing task {} with agent {}", task.id, agent_id);

        // Get agent from pool
        let mut agent_pool = self.agent_pool.write().await;
        let agent = agent_pool.agents
            .get(agent_id)
            .ok_or_else(|| OrchestrationError::AgentNotFound {
                agent_id: agent_id.clone(),
            })?
            .clone();

        // Assign task to agent
        agent_pool.assign_task(agent_id, task.id.clone())?;
        drop(agent_pool);

        // Create session in Cortex for isolation
        let session_id = self.cortex_bridge.create_session(
            agent_id.clone(),
            WorkspaceId::from("default"),
            SessionScope {
                paths: vec!["src/**".to_string()],
                read_only_paths: vec!["tests/**".to_string()],
            },
        ).await?;

        // Get context from Cortex
        let episodes = self.cortex_bridge.search_episodes(&task.name, 5).await?;

        // Execute task with retries on errors
        let result = self.execute_with_retry(
            &agent,
            task,
            &session_id,
            episodes,
            workflow.metadata.max_retries,
        ).await;

        // Merge changes in Cortex
        if result.is_ok() {
            let merge_report = self.cortex_bridge.merge_session(
                &session_id,
                MergeStrategy::Auto,
            ).await?;

            info!("Merged session {} with {} conflicts", session_id, merge_report.conflicts_resolved);
        }

        // Close session
        self.cortex_bridge.close_session(&session_id, agent_id).await?;

        // Release agent
        let mut agent_pool = self.agent_pool.write().await;
        agent_pool.release_task(agent_id, &task.id)?;

        let duration = start_time.elapsed();

        match result {
            Ok(output) => Ok(TaskResult {
                task_id: task.id.clone(),
                success: true,
                error: None,
                output: Some(output),
                duration,
            }),
            Err(e) => Ok(TaskResult {
                task_id: task.id.clone(),
                success: false,
                error: Some(e.to_string()),
                output: None,
                duration,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub workflow_id: WorkflowId,
    pub success: bool,
    pub task_results: HashMap<TaskId, TaskResult>,
    pub failed_tasks: Vec<TaskId>,
    pub duration: Duration,
    pub cortex_task_id: TaskId,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub error: Option<String>,
    pub output: Option<TaskOutput>,
    pub duration: Duration,
}
```

## Error Handling and Retry Logic

### Retry Strategy

```rust
/// Retry strategy for errors
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f32,
    pub retry_on_errors: Vec<ErrorType>,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            retry_on_errors: vec![
                ErrorType::Network,
                ErrorType::Timeout,
                ErrorType::ServiceUnavailable,
            ],
        }
    }
}

impl WorkflowExecutor {
    /// Executes task with retries
    async fn execute_with_retry(
        &self,
        agent: &Agent,
        task: &Task,
        session_id: &SessionId,
        context: Vec<Episode>,
        max_retries: u32,
    ) -> Result<TaskOutput> {
        let strategy = RetryStrategy::default();
        let mut attempt = 0;
        let mut delay = strategy.initial_delay;

        loop {
            attempt += 1;

            info!("Executing task {} (attempt {}/{})", task.id, attempt, max_retries);

            match self.execute_task_internal(agent, task, session_id, &context).await {
                Ok(output) => {
                    info!("Task {} completed successfully on attempt {}", task.id, attempt);
                    return Ok(output);
                }
                Err(e) => {
                    error!("Task {} failed on attempt {}: {}", task.id, attempt, e);

                    // Check if we need to retry
                    if attempt >= max_retries {
                        return Err(e);
                    }

                    if !self.should_retry(&e, &strategy) {
                        return Err(e);
                    }

                    // Exponential backoff
                    warn!("Retrying task {} after {:?}", task.id, delay);
                    tokio::time::sleep(delay).await;

                    delay = (delay.mul_f32(strategy.backoff_multiplier))
                        .min(strategy.max_delay);
                }
            }
        }
    }

    /// Determines if execution should be retried on error
    fn should_retry(&self, error: &OrchestrationError, strategy: &RetryStrategy) -> bool {
        let error_type = error.error_type();
        strategy.retry_on_errors.contains(&error_type)
    }

    /// Internal task execution (without retry logic)
    async fn execute_task_internal(
        &self,
        agent: &Agent,
        task: &Task,
        session_id: &SessionId,
        context: &[Episode],
    ) -> Result<TaskOutput> {
        // Send task to agent via message bus
        let message = Message::TaskAssignment {
            task: task.clone(),
            agent_id: agent.id.clone(),
            session_id: session_id.clone(),
            context: context.to_vec(),
        };

        self.message_bus.send(agent.id.clone(), message).await?;

        // Wait for result from agent (with timeout)
        let result = tokio::time::timeout(
            task.requirements.estimated_duration * 2,
            self.wait_for_task_completion(&task.id, &agent.id),
        ).await??;

        Ok(result)
    }

    /// Waits for task completion
    async fn wait_for_task_completion(
        &self,
        task_id: &TaskId,
        agent_id: &AgentId,
    ) -> Result<TaskOutput> {
        // Subscribe to messages from agent
        let mut receiver = self.message_bus.subscribe(agent_id.clone());

        loop {
            match receiver.recv().await {
                Ok(Message::TaskComplete { task_id: completed_id, result, .. }) => {
                    if completed_id == *task_id {
                        return Ok(result);
                    }
                }
                Ok(Message::TaskProgress { task_id: progress_id, progress, .. }) => {
                    if progress_id == *task_id {
                        info!("Task {} progress: {:.1}%", task_id, progress * 100.0);
                    }
                }
                Err(e) => {
                    return Err(OrchestrationError::Communication {
                        reason: format!("Failed to receive message: {}", e),
                    }.into());
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorType {
    Network,
    Timeout,
    ServiceUnavailable,
    InvalidInput,
    AgentError,
    SystemError,
}

impl OrchestrationError {
    pub fn error_type(&self) -> ErrorType {
        match self {
            Self::Communication { .. } => ErrorType::Network,
            Self::Timeout { .. } => ErrorType::Timeout,
            Self::CortexUnavailable { .. } => ErrorType::ServiceUnavailable,
            Self::InvalidTask { .. } => ErrorType::InvalidInput,
            Self::AgentFailure { .. } => ErrorType::AgentError,
            _ => ErrorType::SystemError,
        }
    }
}
```

## Cortex Integration for Task Management

### Cortex Task API Integration

```rust
impl WorkflowExecutor {
    /// Creates workflow in Cortex for tracking
    pub async fn create_workflow_in_cortex(&self, workflow: &Workflow) -> Result<TaskId> {
        // Create parent task for workflow
        let parent_task = self.cortex_bridge.create_task(TaskDefinition {
            title: workflow.name.clone(),
            description: workflow.description.clone(),
            workspace_id: "default".to_string(),
            estimated_hours: workflow.metadata.timeout.as_secs_f64() / 3600.0,
        }).await?;

        // Create subtasks for each task
        for task in &workflow.tasks {
            let subtask = self.cortex_bridge.create_task(TaskDefinition {
                title: task.name.clone(),
                description: format!("Type: {:?}", task.task_type),
                workspace_id: "default".to_string(),
                estimated_hours: task.requirements.estimated_duration.as_secs_f64() / 3600.0,
            }).await?;

            // Link subtask to parent through Cortex
            // (assuming Cortex API supports task hierarchy)
        }

        Ok(parent_task)
    }

    /// Updates workflow progress in Cortex
    pub async fn update_workflow_progress(&self, workflow_id: &WorkflowId, progress: f32) -> Result<()> {
        // Get corresponding task_id in Cortex
        // Update progress via Cortex API
        Ok(())
    }
}
```

## Workflow DSL (Domain Specific Language)

### DSL for describing workflows

```rust
/// DSL for declarative workflow description
pub mod workflow_dsl {
    use super::*;

    /// Builder for creating workflow
    pub struct WorkflowBuilder {
        workflow: Workflow,
    }

    impl WorkflowBuilder {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                workflow: Workflow {
                    id: WorkflowId::new(),
                    name: name.into(),
                    description: String::new(),
                    tasks: Vec::new(),
                    dependencies: HashMap::new(),
                    metadata: WorkflowMetadata {
                        created_at: Utc::now(),
                        created_by: AgentId::system(),
                        priority: Priority::Medium,
                        timeout: Duration::from_secs(3600),
                        max_retries: 3,
                    },
                },
            }
        }

        pub fn description(mut self, desc: impl Into<String>) -> Self {
            self.workflow.description = desc.into();
            self
        }

        pub fn priority(mut self, priority: Priority) -> Self {
            self.workflow.metadata.priority = priority;
            self
        }

        pub fn timeout(mut self, timeout: Duration) -> Self {
            self.workflow.metadata.timeout = timeout;
            self
        }

        pub fn task(mut self, task: Task) -> Self {
            self.workflow.tasks.push(task);
            self
        }

        pub fn depends_on(mut self, task_id: TaskId, depends_on: Vec<TaskId>) -> Self {
            self.workflow.dependencies.insert(task_id, depends_on);
            self
        }

        pub fn build(self) -> Result<Workflow> {
            // Validate workflow
            DagValidator::validate(&self.workflow)?;
            Ok(self.workflow)
        }
    }

    /// Builder for creating Task
    pub struct TaskBuilder {
        task: Task,
    }

    impl TaskBuilder {
        pub fn new(name: impl Into<String>, task_type: TaskType) -> Self {
            Self {
                task: Task {
                    id: TaskId::new(),
                    name: name.into(),
                    task_type,
                    requirements: TaskRequirements {
                        capabilities: Vec::new(),
                        estimated_duration: Duration::from_secs(300),
                        resources: ResourceRequirements {
                            cpu_cores: 1,
                            memory_mb: 512,
                            gpu_required: false,
                        },
                    },
                    input: TaskInput::default(),
                    status: TaskStatus::Pending,
                },
            }
        }

        pub fn capability(mut self, cap: Capability) -> Self {
            self.task.requirements.capabilities.push(cap);
            self
        }

        pub fn estimated_duration(mut self, duration: Duration) -> Self {
            self.task.requirements.estimated_duration = duration;
            self
        }

        pub fn resources(mut self, resources: ResourceRequirements) -> Self {
            self.task.requirements.resources = resources;
            self
        }

        pub fn input(mut self, input: TaskInput) -> Self {
            self.task.input = input;
            self
        }

        pub fn build(self) -> Task {
            self.task
        }
    }
}
```

### DSL Usage Examples

```rust
use workflow_dsl::{WorkflowBuilder, TaskBuilder};

/// Example: creating workflow for feature development
pub fn create_feature_development_workflow() -> Result<Workflow> {
    // Create tasks
    let design_task = TaskBuilder::new("Design API", TaskType::Development {
        language: "rust".to_string(),
        scope: vec!["src/api".to_string()],
    })
    .capability(Capability::CodeGeneration)
    .capability(Capability::ApiDesign)
    .estimated_duration(Duration::from_secs(1800))
    .build();

    let implement_task = TaskBuilder::new("Implement API", TaskType::Development {
        language: "rust".to_string(),
        scope: vec!["src/api".to_string()],
    })
    .capability(Capability::CodeGeneration)
    .estimated_duration(Duration::from_secs(3600))
    .build();

    let test_task = TaskBuilder::new("Write Tests", TaskType::Testing {
        test_type: TestType::Unit,
        coverage_threshold: 0.8,
    })
    .capability(Capability::Testing)
    .estimated_duration(Duration::from_secs(1800))
    .build();

    let review_task = TaskBuilder::new("Code Review", TaskType::Review {
        review_type: ReviewType::Comprehensive,
    })
    .capability(Capability::CodeReview)
    .estimated_duration(Duration::from_secs(900))
    .build();

    let doc_task = TaskBuilder::new("Write Documentation", TaskType::Documentation {
        format: DocFormat::Markdown,
    })
    .capability(Capability::Documentation)
    .estimated_duration(Duration::from_secs(600))
    .build();

    // Create workflow with dependencies
    let workflow = WorkflowBuilder::new("Feature: User Authentication")
        .description("Implement user authentication API with tests and documentation")
        .priority(Priority::High)
        .timeout(Duration::from_secs(7200))
        .task(design_task.clone())
        .task(implement_task.clone())
        .task(test_task.clone())
        .task(review_task.clone())
        .task(doc_task.clone())
        // Dependencies: implement depends on design
        .depends_on(implement_task.id.clone(), vec![design_task.id.clone()])
        // test depends on implement
        .depends_on(test_task.id.clone(), vec![implement_task.id.clone()])
        // review depends on test
        .depends_on(review_task.id.clone(), vec![test_task.id.clone()])
        // doc can run in parallel with test
        .depends_on(doc_task.id.clone(), vec![implement_task.id.clone()])
        .build()?;

    Ok(workflow)
}

/// Example: refactoring workflow
pub fn create_refactoring_workflow() -> Result<Workflow> {
    let analyze_task = TaskBuilder::new("Analyze Code", TaskType::Custom {
        handler: "code_analyzer".to_string(),
        config: serde_json::json!({
            "metrics": ["complexity", "duplication", "coverage"]
        }),
    })
    .capability(Capability::CodeAnalysis)
    .estimated_duration(Duration::from_secs(600))
    .build();

    let refactor_task = TaskBuilder::new("Refactor Code", TaskType::Refactoring {
        optimization_level: OptimizationLevel::Aggressive,
    })
    .capability(Capability::Refactoring)
    .estimated_duration(Duration::from_secs(2400))
    .build();

    let verify_task = TaskBuilder::new("Verify Changes", TaskType::Testing {
        test_type: TestType::Regression,
        coverage_threshold: 0.9,
    })
    .capability(Capability::Testing)
    .estimated_duration(Duration::from_secs(1200))
    .build();

    WorkflowBuilder::new("Refactoring: Improve Code Quality")
        .description("Analyze, refactor, and verify code improvements")
        .priority(Priority::Medium)
        .task(analyze_task.clone())
        .task(refactor_task.clone())
        .task(verify_task.clone())
        .depends_on(refactor_task.id.clone(), vec![analyze_task.id.clone()])
        .depends_on(verify_task.id.clone(), vec![refactor_task.id.clone()])
        .build()
}
```

## Metrics and Monitoring

```rust
/// Metrics for Orchestration Engine
pub struct ExecutorMetrics {
    pub workflows_executed: AtomicU64,
    pub workflows_succeeded: AtomicU64,
    pub workflows_failed: AtomicU64,
    pub total_execution_time: AtomicU64,
    pub tasks_executed: AtomicU64,
    pub tasks_retried: AtomicU64,
    pub avg_workflow_duration: AtomicF64,
}

impl ExecutorMetrics {
    pub fn record_workflow_execution(&self, workflow_id: WorkflowId, duration: Duration, success: bool) {
        self.workflows_executed.fetch_add(1, Ordering::Relaxed);

        if success {
            self.workflows_succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
            self.workflows_failed.fetch_add(1, Ordering::Relaxed);
        }

        let duration_ms = duration.as_millis() as u64;
        self.total_execution_time.fetch_add(duration_ms, Ordering::Relaxed);

        // Update average execution time
        let total_executed = self.workflows_executed.load(Ordering::Relaxed);
        let total_time = self.total_execution_time.load(Ordering::Relaxed);
        let avg = total_time as f64 / total_executed as f64;
        self.avg_workflow_duration.store(avg, Ordering::Relaxed);
    }

    pub fn export(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            workflows_executed: self.workflows_executed.load(Ordering::Relaxed),
            workflows_succeeded: self.workflows_succeeded.load(Ordering::Relaxed),
            workflows_failed: self.workflows_failed.load(Ordering::Relaxed),
            avg_workflow_duration_ms: self.avg_workflow_duration.load(Ordering::Relaxed),
            tasks_executed: self.tasks_executed.load(Ordering::Relaxed),
            tasks_retried: self.tasks_retried.load(Ordering::Relaxed),
        }
    }
}
```

---

The Orchestration Engine provides efficient execution of complex workflows with automatic dependency handling, parallel task execution, and integration with Cortex for state and memory management.
