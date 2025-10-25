//! Task scheduling and execution planning

use super::*;

pub struct TaskScheduler;

impl TaskScheduler {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_schedule(&self, workflow: &Workflow) -> Result<ExecutionSchedule> {
        let sorted_tasks = self.topological_sort(workflow)?;

        Ok(ExecutionSchedule {
            workflow_id: workflow.id.clone(),
            sorted_tasks,
            estimated_duration: workflow.metadata.timeout,
        })
    }

    fn topological_sort(&self, workflow: &Workflow) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for task in &workflow.tasks {
            in_degree.insert(task.id.clone(), 0);
        }

        for deps in workflow.dependencies.values() {
            for dep in deps {
                *in_degree.get_mut(dep).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|&(_, &degree)| degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id.clone());

            if let Some(deps) = workflow.dependencies.get(&task_id) {
                for dep in deps {
                    let degree = in_degree.get_mut(dep).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        if result.len() != workflow.tasks.len() {
            return Err(OrchestrationError::InvalidDag {
                reason: "Cycle detected".to_string(),
            });
        }

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionSchedule {
    pub workflow_id: String,
    pub sorted_tasks: Vec<String>,
    pub estimated_duration: Duration,
}
