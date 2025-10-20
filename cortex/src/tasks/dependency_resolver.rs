/// Task Dependency Resolver - Automatic prerequisite task resolution
///
/// This module provides intelligent task dependency resolution for agents:
/// - Detects prerequisite tasks that must be completed first
/// - Generates execution plans respecting dependency order
/// - Visualizes dependency graphs
/// - Auto-executes prerequisite tasks before main task
///
/// Example:
/// ```
/// let resolver = DependencyResolver::new(progress_manager);
/// let plan = resolver.create_execution_plan(task_id).await?;
/// // plan = [prerequisite_1, prerequisite_2, main_task, dependent_1]
/// ```

use crate::tasks::{TaskManager, TaskId, TaskStatus};
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Execution plan for task with dependencies
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Tasks in execution order (prerequisites first, then target, then dependents)
    pub tasks: Vec<TaskId>,
    /// Dependency graph (task_id -> depends_on)
    pub graph: HashMap<TaskId, Vec<TaskId>>,
    /// Tasks that must be completed before target
    pub prerequisites: Vec<TaskId>,
    /// Target task
    pub target: TaskId,
    /// Tasks that depend on target
    pub dependents: Vec<TaskId>,
}

/// Dependency resolver for automatic task ordering
pub struct DependencyResolver {
    progress_manager: Arc<RwLock<TaskManager>>,
}

impl DependencyResolver {
    pub fn new(progress_manager: Arc<RwLock<TaskManager>>) -> Self {
        Self { progress_manager }
    }

    /// Create execution plan for a task, including all prerequisites and dependents
    ///
    /// Returns tasks in topological order:
    /// 1. All prerequisite tasks (transitive dependencies)
    /// 2. Target task
    /// 3. All dependent tasks (tasks that depend on target)
    pub async fn create_execution_plan(&self, target_id: &TaskId) -> Result<ExecutionPlan> {
        let manager = self.progress_manager.read().await;

        // Get target task (validates it exists)
        let _target = manager.get_task(target_id).await?;

        // Build complete dependency graph
        let mut graph = HashMap::new();
        let mut visited = HashSet::new();
        self.build_dependency_graph(&manager, target_id, &mut graph, &mut visited).await?;

        // Find all prerequisites (transitive)
        let prerequisites = self.get_all_prerequisites(&manager, target_id).await?;

        // Find all dependents
        let dependents = manager.get_dependents(target_id).await?
            .into_iter()
            .map(|t| t.id)
            .collect();

        // Topological sort for execution order
        let tasks = self.topological_sort(&graph, target_id)?;

        Ok(ExecutionPlan {
            tasks,
            graph,
            prerequisites,
            target: target_id.clone(),
            dependents,
        })
    }

    /// Build dependency graph recursively
    fn build_dependency_graph<'a>(
        &'a self,
        manager: &'a TaskManager,
        task_id: &'a TaskId,
        graph: &'a mut HashMap<TaskId, Vec<TaskId>>,
        visited: &'a mut HashSet<TaskId>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            if visited.contains(task_id) {
                return Ok(());
            }
            visited.insert(task_id.clone());

            let task = manager.get_task(task_id).await?;
            graph.insert(task_id.clone(), task.depends_on.clone());

            // Recurse into dependencies
            for dep_id in &task.depends_on {
                self.build_dependency_graph(manager, dep_id, graph, visited).await?;
            }

            // Recurse into dependents
            let dependents = manager.get_dependents(task_id).await?;
            for dep_task in dependents {
                self.build_dependency_graph(manager, &dep_task.id, graph, visited).await?;
            }

            Ok(())
        })
    }

    /// Get all prerequisites (transitive dependencies) in execution order
    async fn get_all_prerequisites(
        &self,
        manager: &TaskManager,
        task_id: &TaskId,
    ) -> Result<Vec<TaskId>> {
        let mut prerequisites = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        let task = manager.get_task(task_id).await?;
        for dep_id in &task.depends_on {
            queue.push_back(dep_id.clone());
        }

        while let Some(dep_id) = queue.pop_front() {
            if visited.contains(&dep_id) {
                continue;
            }
            visited.insert(dep_id.clone());

            let dep_task = manager.get_task(&dep_id).await?;

            // Add to prerequisites
            prerequisites.push(dep_id.clone());

            // Add its dependencies to queue
            for transitive_dep in &dep_task.depends_on {
                if !visited.contains(transitive_dep) {
                    queue.push_back(transitive_dep.clone());
                }
            }
        }

        // Reverse to get execution order (deepest dependencies first)
        prerequisites.reverse();
        Ok(prerequisites)
    }

    /// Topological sort using Kahn's algorithm
    fn topological_sort(
        &self,
        graph: &HashMap<TaskId, Vec<TaskId>>,
        _target_id: &TaskId,
    ) -> Result<Vec<TaskId>> {
        // Build in-degree map
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();
        let mut adj_list: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        for (task_id, dependencies) in graph {
            in_degree.entry(task_id.clone()).or_insert(0);

            for dep_id in dependencies {
                adj_list.entry(dep_id.clone())
                    .or_default()
                    .push(task_id.clone());

                *in_degree.entry(task_id.clone()).or_insert(0) += 1;
            }
        }

        // Find nodes with no incoming edges
        let mut queue: VecDeque<_> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id.clone());

            // Reduce in-degree for neighbors
            if let Some(neighbors) = adj_list.get(&task_id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != graph.len() {
            anyhow::bail!("Cycle detected in task dependencies");
        }

        Ok(result)
    }

    /// Generate DOT graph for visualization
    pub async fn generate_dot_graph(&self, target_id: &TaskId) -> Result<String> {
        let plan = self.create_execution_plan(target_id).await?;
        let manager = self.progress_manager.read().await;

        let mut dot = String::from("digraph TaskDependencies {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Add nodes with status colors
        for task_id in &plan.tasks {
            let task = manager.get_task(task_id).await?;
            let color = match task.status {
                TaskStatus::Done => "green",
                TaskStatus::InProgress => "yellow",
                TaskStatus::Blocked => "red",
                TaskStatus::Pending => "lightblue",
                TaskStatus::Cancelled => "gray",
            };

            let label = task.title.replace("\"", "\\\"");
            let is_target = task_id == &plan.target;

            if is_target {
                dot.push_str(&format!(
                    "  \"{}\" [label=\"{}\", fillcolor={}, style=\"filled,bold\"];\n",
                    task_id.0, label, color
                ));
            } else {
                dot.push_str(&format!(
                    "  \"{}\" [label=\"{}\", fillcolor={}];\n",
                    task_id.0, label, color
                ));
            }
        }

        dot.push('\n');

        // Add edges
        for (task_id, dependencies) in &plan.graph {
            for dep_id in dependencies {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    dep_id.0, task_id.0
                ));
            }
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    /// Check if a task can be started (all prerequisites are done)
    pub async fn can_start(&self, task_id: &TaskId) -> Result<bool> {
        let manager = self.progress_manager.read().await;
        manager.can_start_task(task_id).await
    }

    /// Get next actionable task from execution plan
    ///
    /// Returns the first task in the plan that:
    /// - Is not yet done
    /// - Has all prerequisites completed
    pub async fn get_next_actionable_task(&self, target_id: &TaskId) -> Result<Option<TaskId>> {
        let plan = self.create_execution_plan(target_id).await?;
        let manager = self.progress_manager.read().await;

        for task_id in &plan.tasks {
            let task = manager.get_task(task_id).await?;

            // Skip completed or cancelled tasks
            if task.status == TaskStatus::Done || task.status == TaskStatus::Cancelled {
                continue;
            }

            // Check if can start (all dependencies met)
            if manager.can_start_task(task_id).await? {
                return Ok(Some(task_id.clone()));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskStorage;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;

    async fn create_test_resolver() -> (DependencyResolver, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MemoryStorage::new());
        let progress_storage = Arc::new(TaskStorage::new(storage));
        let progress_manager = Arc::new(RwLock::new(TaskManager::new(progress_storage)));

        let resolver = DependencyResolver::new(progress_manager.clone());
        (resolver, temp_dir)
    }

    #[tokio::test]
    async fn test_execution_plan_simple() {
        let (resolver, _temp) = create_test_resolver().await;
        let manager = resolver.progress_manager.clone();

        // Create task chain: C depends on B depends on A
        let task_a = manager.write().await.create_task(
            "Task A".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        let task_b = manager.write().await.create_task(
            "Task B".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        let task_c = manager.write().await.create_task(
            "Task C".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        // Add dependencies
        manager.write().await.add_dependency(&task_b, &task_a).await.unwrap();
        manager.write().await.add_dependency(&task_c, &task_b).await.unwrap();

        // Create execution plan for C
        let plan = resolver.create_execution_plan(&task_c).await.unwrap();

        // Should be: A, B, C
        assert_eq!(plan.tasks.len(), 3);
        assert_eq!(plan.prerequisites.len(), 2);
        assert_eq!(plan.target, task_c);

        // A should be first, C should be last
        assert_eq!(plan.tasks[0], task_a);
        assert_eq!(plan.tasks[2], task_c);
    }

    #[tokio::test]
    async fn test_get_next_actionable_task() {
        let (resolver, _temp) = create_test_resolver().await;
        let manager = resolver.progress_manager.clone();

        // Create chain: B depends on A
        let task_a = manager.write().await.create_task(
            "Task A".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        let task_b = manager.write().await.create_task(
            "Task B".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        manager.write().await.add_dependency(&task_b, &task_a).await.unwrap();

        // Next actionable should be A (no dependencies)
        let next = resolver.get_next_actionable_task(&task_b).await.unwrap();
        assert_eq!(next, Some(task_a.clone()));

        // Complete A
        manager.write().await.update_task(
            &task_a, None, None, None, Some(TaskStatus::Done), None, None, None, None, None
        ).await.unwrap();

        // Now next actionable should be B
        let next = resolver.get_next_actionable_task(&task_b).await.unwrap();
        assert_eq!(next, Some(task_b));
    }

    #[tokio::test]
    async fn test_dot_graph_generation() {
        let (resolver, _temp) = create_test_resolver().await;
        let manager = resolver.progress_manager.clone();

        let task_a = manager.write().await.create_task(
            "Task A".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        let task_b = manager.write().await.create_task(
            "Task B".to_string(), None, None, None, vec![], None, None
        ).await.unwrap();

        manager.write().await.add_dependency(&task_b, &task_a).await.unwrap();

        // Generate DOT graph
        let dot = resolver.generate_dot_graph(&task_b).await.unwrap();

        // Should contain digraph declaration and nodes
        assert!(dot.contains("digraph TaskDependencies"));
        assert!(dot.contains("Task A"));
        assert!(dot.contains("Task B"));
        assert!(dot.contains("->"));
    }
}
