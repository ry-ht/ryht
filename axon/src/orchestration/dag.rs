//! DAG validation and analysis

use super::*;

pub struct DagValidator;

impl DagValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, workflow: &Workflow) -> Result<()> {
        self.check_cycles(&workflow.dependencies)?;
        self.check_dependencies_exist(workflow)?;
        Ok(())
    }

    fn check_cycles(&self, deps: &HashMap<String, Vec<String>>) -> Result<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in deps.keys() {
            if !visited.contains(task_id) {
                if self.has_cycle(task_id, deps, &mut visited, &mut rec_stack) {
                    return Err(OrchestrationError::CycleDetected {
                        task_id: task_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    fn has_cycle(
        &self,
        task_id: &str,
        deps: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        if let Some(dependencies) = deps.get(task_id) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if self.has_cycle(dep, deps, visited, rec_stack) {
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

    fn check_dependencies_exist(&self, workflow: &Workflow) -> Result<()> {
        let task_ids: HashSet<_> = workflow.tasks.iter().map(|t| t.id.as_str()).collect();

        for (task_id, deps) in &workflow.dependencies {
            if !task_ids.contains(task_id.as_str()) {
                return Err(OrchestrationError::TaskNotFound {
                    task_id: task_id.clone(),
                });
            }

            for dep_id in deps {
                if !task_ids.contains(dep_id.as_str()) {
                    return Err(OrchestrationError::DependencyNotFound {
                        task_id: task_id.clone(),
                        dependency_id: dep_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}
