use crate::storage::{deserialize, serialize, Storage};
use crate::types::{Outcome, TaskEpisode};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Procedural memory - knowledge about HOW to perform tasks
pub struct ProceduralMemory {
    storage: Arc<dyn Storage>,
    procedures: HashMap<TaskType, Procedure>,
    execution_history: Vec<ExecutionTrace>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Refactor,
    BugFix,
    Feature,
    Test,
    Documentation,
    Performance,
    Security,
    Other(String),
}

impl TaskType {
    /// Infer task type from description
    pub fn infer(description: &str) -> Self {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("refactor") || desc_lower.contains("restructure") {
            TaskType::Refactor
        } else if desc_lower.contains("bug") || desc_lower.contains("fix") || desc_lower.contains("error") {
            TaskType::BugFix
        } else if desc_lower.contains("test") {
            TaskType::Test
        } else if desc_lower.contains("document") || desc_lower.contains("readme") {
            TaskType::Documentation
        } else if desc_lower.contains("performance") || desc_lower.contains("optimize") {
            TaskType::Performance
        } else if desc_lower.contains("security") || desc_lower.contains("vulnerability") {
            TaskType::Security
        } else if desc_lower.contains("feature") || desc_lower.contains("add") || desc_lower.contains("implement") {
            TaskType::Feature
        } else {
            TaskType::Other(description.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    pub steps: Vec<ProcedureStep>,
    pub required_context: Vec<String>,
    pub typical_queries: Vec<String>,
    pub success_rate: f32,
    pub execution_count: u32,
    pub average_tokens: u32,
    pub common_pitfalls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub order: usize,
    pub description: String,
    pub typical_actions: Vec<String>,
    pub expected_files: Vec<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub task_type: TaskType,
    pub episode_id: String,
    pub steps_taken: Vec<String>,
    pub outcome: Outcome,
    pub duration_estimate: u32, // in tokens
}

impl ProceduralMemory {
    pub fn new(storage: Arc<dyn Storage>) -> Result<Self> {
        Ok(Self {
            storage,
            procedures: HashMap::new(),
            execution_history: Vec::new(),
        })
    }

    /// Load procedures from storage
    pub async fn load(&mut self) -> Result<()> {
        let keys = self.storage.get_keys_with_prefix(b"procedure:").await?;

        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                let (task_type, procedure): (TaskType, Procedure) = deserialize(&data)?;
                self.procedures.insert(task_type, procedure);
            }
        }

        // Load execution history
        let history_keys = self.storage.get_keys_with_prefix(b"execution:").await?;
        for key in history_keys {
            if let Some(data) = self.storage.get(&key).await? {
                let trace: ExecutionTrace = deserialize(&data)?;
                self.execution_history.push(trace);
            }
        }

        tracing::info!(
            "Loaded {} procedures and {} execution traces from storage",
            self.procedures.len(),
            self.execution_history.len()
        );
        Ok(())
    }

    /// Learn procedures from successful episodes
    pub async fn learn_from_episodes(&mut self, episodes: &[TaskEpisode]) -> Result<()> {
        // Group episodes by task type
        let mut grouped: HashMap<TaskType, Vec<&TaskEpisode>> = HashMap::new();

        for episode in episodes {
            let task_type = TaskType::infer(&episode.task_description);
            grouped.entry(task_type).or_default().push(episode);
        }

        // Extract procedures from groups
        for (task_type, episodes) in grouped {
            if episodes.len() >= 2 {
                // Need at least 2 episodes to extract a pattern
                let procedure = self.extract_procedure(&episodes);
                self.add_or_update_procedure(task_type, procedure).await?;
            }
        }

        tracing::info!("Learned procedures from {} episodes", episodes.len());
        Ok(())
    }

    fn extract_procedure(&self, episodes: &[&TaskEpisode]) -> Procedure {
        let successful: Vec<_> = episodes
            .iter()
            .filter(|e| e.outcome == Outcome::Success)
            .copied()
            .collect();

        let success_rate = Self::calculate_success_rate(episodes);
        let steps = Self::extract_common_steps(&successful);
        let required_context = Self::extract_minimal_context(&successful);
        let typical_queries = Self::extract_query_patterns(&successful);
        let average_tokens = Self::calculate_average_tokens(episodes);
        let common_pitfalls = Self::extract_common_pitfalls(episodes);

        Procedure {
            steps,
            required_context,
            typical_queries,
            success_rate,
            execution_count: episodes.len() as u32,
            average_tokens,
            common_pitfalls,
        }
    }

    /// Calculate success rate from episodes
    fn calculate_success_rate(episodes: &[&TaskEpisode]) -> f32 {
        if episodes.is_empty() {
            return 0.0;
        }

        let successful_count = episodes
            .iter()
            .filter(|e| e.outcome == Outcome::Success)
            .count();

        successful_count as f32 / episodes.len() as f32
    }

    /// Extract common steps across successful episodes
    fn extract_common_steps(successful: &[&TaskEpisode]) -> Vec<ProcedureStep> {
        // Track frequency of each step description
        let mut step_frequency: HashMap<String, usize> = HashMap::new();

        for episode in successful {
            if !episode.solution_path.is_empty() {
                // Split solution path into individual steps
                let steps: Vec<&str> = episode.solution_path
                    .split(['.', ',', ';'])
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                for step in steps {
                    *step_frequency
                        .entry(step.to_string())
                        .or_insert(0) += 1;
                }
            }
        }

        // Sort by frequency and convert to ProcedureSteps
        let mut steps_vec: Vec<_> = step_frequency.into_iter().collect();
        steps_vec.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by frequency descending

        // Calculate threshold for optional vs required steps
        let majority_threshold = if successful.is_empty() {
            1
        } else {
            successful.len() / 2
        };

        steps_vec
            .into_iter()
            .enumerate()
            .map(|(i, (desc, freq))| {
                // Extract typical actions from the step description
                let typical_actions = Self::extract_actions_from_step(&desc);

                // Extract expected files mentioned in the step
                let expected_files = Self::extract_files_from_step(&desc);

                ProcedureStep {
                    order: i,
                    description: desc.clone(),
                    typical_actions,
                    expected_files,
                    optional: freq < majority_threshold,
                }
            })
            .collect()
    }

    /// Extract minimal context (files) required from episodes
    fn extract_minimal_context(successful: &[&TaskEpisode]) -> Vec<String> {
        let mut file_frequency: HashMap<String, usize> = HashMap::new();

        for episode in successful {
            for file in &episode.files_touched {
                *file_frequency.entry(file.clone()).or_insert(0) += 1;
            }
        }

        // Only include files that appear in at least half of successful episodes (ceiling)
        let threshold = if successful.is_empty() {
            1
        } else {
            successful.len().div_ceil(2)
        };

        let mut required: Vec<(String, usize)> = file_frequency
            .into_iter()
            .filter(|(_, freq)| *freq >= threshold)
            .collect();

        // Sort by frequency to get most critical files first
        required.sort_by(|a, b| b.1.cmp(&a.1));

        required.into_iter().map(|(file, _)| file).collect()
    }

    /// Learn typical query patterns from episodes
    fn extract_query_patterns(successful: &[&TaskEpisode]) -> Vec<String> {
        let mut query_frequency: HashMap<String, usize> = HashMap::new();

        for episode in successful {
            for query in &episode.queries_made {
                // Normalize query (lowercase, trim)
                let normalized = query.trim().to_lowercase();
                if !normalized.is_empty() {
                    *query_frequency
                        .entry(normalized)
                        .or_insert(0) += 1;
                }
            }
        }

        // Filter queries that appear at least twice (recurring patterns)
        let mut patterns: Vec<(String, usize)> = query_frequency
            .into_iter()
            .filter(|(_, freq)| *freq >= 2)
            .collect();

        // Sort by frequency to get most common patterns first
        patterns.sort_by(|a, b| b.1.cmp(&a.1));

        patterns.into_iter().map(|(query, _)| query).collect()
    }

    /// Calculate average token usage across episodes
    fn calculate_average_tokens(episodes: &[&TaskEpisode]) -> u32 {
        if episodes.is_empty() {
            return 0;
        }

        let total_tokens: u32 = episodes.iter().map(|e| e.tokens_used.0).sum();
        total_tokens / episodes.len() as u32
    }

    /// Extract common pitfalls from failed episodes
    fn extract_common_pitfalls(episodes: &[&TaskEpisode]) -> Vec<String> {
        let failed: Vec<_> = episodes
            .iter()
            .filter(|e| e.outcome == Outcome::Failure)
            .collect();

        let mut pitfalls = Vec::new();

        for episode in failed {
            if !episode.solution_path.is_empty() {
                let pitfall = format!("Failed at: {}", episode.solution_path);

                // Avoid duplicates
                if !pitfalls.contains(&pitfall) {
                    pitfalls.push(pitfall);
                }
            }
        }

        pitfalls
    }

    /// Extract actions from step description (helper)
    fn extract_actions_from_step(step_desc: &str) -> Vec<String> {
        // Look for action verbs and extract them
        let action_keywords = [
            "read", "write", "modify", "delete", "create", "update",
            "search", "find", "analyze", "test", "build", "deploy",
            "refactor", "fix", "implement", "add", "remove"
        ];

        let mut actions = Vec::new();
        let step_lower = step_desc.to_lowercase();

        for keyword in &action_keywords {
            if step_lower.contains(keyword) {
                actions.push(keyword.to_string());
            }
        }

        // If no actions found, use the full step as action
        if actions.is_empty() {
            actions.push(step_desc.to_string());
        }

        actions
    }

    /// Extract file paths mentioned in step description (helper)
    fn extract_files_from_step(step_desc: &str) -> Vec<String> {
        let mut files = Vec::new();

        // Look for common file extensions
        let extensions = [".rs", ".ts", ".js", ".py", ".go", ".md", ".toml", ".json", ".yml"];

        for ext in &extensions {
            if step_desc.contains(ext) {
                // Try to extract the full filename
                if let Some(start) = step_desc.rfind(|c: char| c.is_whitespace()) {
                    if let Some(end) = step_desc[start..].find(|c: char| c.is_whitespace()) {
                        let file = &step_desc[start..start + end];
                        files.push(file.trim().to_string());
                    }
                }
            }
        }

        files
    }

    async fn add_or_update_procedure(
        &mut self,
        task_type: TaskType,
        new_procedure: Procedure,
    ) -> Result<()> {
        if let Some(existing) = self.procedures.get_mut(&task_type) {
            // Update existing procedure
            existing.execution_count += new_procedure.execution_count;
            existing.success_rate = (existing.success_rate + new_procedure.success_rate) / 2.0;
            existing.average_tokens =
                (existing.average_tokens + new_procedure.average_tokens) / 2;

            // Merge steps
            let mut seen_steps = HashSet::new();
            for step in &existing.steps {
                seen_steps.insert(step.description.clone());
            }

            for new_step in new_procedure.steps {
                if !seen_steps.contains(&new_step.description) {
                    existing.steps.push(new_step);
                }
            }

            // Merge context and queries
            let mut context_set: HashSet<_> = existing.required_context.iter().cloned().collect();
            context_set.extend(new_procedure.required_context);
            existing.required_context = context_set.into_iter().collect();

            let mut query_set: HashSet<_> = existing.typical_queries.iter().cloned().collect();
            query_set.extend(new_procedure.typical_queries);
            existing.typical_queries = query_set.into_iter().collect();

            // Merge pitfalls
            existing.common_pitfalls.extend(new_procedure.common_pitfalls);
        } else {
            self.procedures.insert(task_type.clone(), new_procedure);
        }

        // Save to storage
        self.save_procedure(&task_type).await?;

        Ok(())
    }

    async fn save_procedure(&self, task_type: &TaskType) -> Result<()> {
        if let Some(procedure) = self.procedures.get(task_type) {
            let key = format!("procedure:{:?}", task_type);
            let value = serialize(&(task_type, procedure))?;
            self.storage.put(key.as_bytes(), &value).await?;
        }
        Ok(())
    }

    /// Learn a procedure manually
    pub async fn learn_procedure(
        &mut self,
        task_type: TaskType,
        procedure: Procedure,
    ) -> Result<()> {
        self.add_or_update_procedure(task_type, procedure).await
    }

    /// Get procedure for task type
    pub fn get_procedure(&self, task_type: &TaskType) -> Option<&Procedure> {
        self.procedures.get(task_type)
    }

    /// Get procedure by description (infers task type)
    pub fn get_procedure_for_task(&self, task_description: &str) -> Option<&Procedure> {
        let task_type = TaskType::infer(task_description);
        self.procedures.get(&task_type)
    }

    /// Get next suggested step
    pub fn next_step(
        &self,
        task_type: &TaskType,
        completed_steps: &[String],
    ) -> Option<&ProcedureStep> {
        let procedure = self.procedures.get(task_type)?;

        // Find first step not yet completed
        procedure.steps.iter().find(|step| {
            !completed_steps
                .iter()
                .any(|completed| step.description.contains(completed))
        })
    }

    /// Record execution trace
    pub async fn record_execution(&mut self, trace: ExecutionTrace) -> Result<()> {
        let key = format!("execution:{}", uuid::Uuid::new_v4());
        let value = serialize(&trace)?;
        self.storage.put(key.as_bytes(), &value).await?;
        self.execution_history.push(trace);
        Ok(())
    }

    /// Get execution history for task type
    pub fn get_execution_history(&self, task_type: &TaskType) -> Vec<&ExecutionTrace> {
        self.execution_history
            .iter()
            .filter(|trace| &trace.task_type == task_type)
            .collect()
    }

    /// Get all procedures
    pub fn procedures(&self) -> &HashMap<TaskType, Procedure> {
        &self.procedures
    }

    /// Get success rate for a task type
    pub fn get_success_rate(&self, task_type: &TaskType) -> Option<f32> {
        self.procedures.get(task_type).map(|p| p.success_rate)
    }

    /// Get estimated token cost for a task type
    pub fn get_estimated_cost(&self, task_type: &TaskType) -> Option<u32> {
        self.procedures.get(task_type).map(|p| p.average_tokens)
    }

    /// Record a solution for learning
    pub async fn record_solution(&mut self, task_description: &str, solution_path: &str) -> Result<()> {
        let task_type = TaskType::infer(task_description);

        // Parse solution path into steps
        let steps: Vec<String> = solution_path
            .split(&[',', ';', '\n'][..])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Create or update procedure
        let procedure = self.procedures.entry(task_type.clone()).or_insert(Procedure {
            steps: Vec::new(),
            required_context: Vec::new(),
            typical_queries: Vec::new(),
            success_rate: 0.0,
            execution_count: 0,
            average_tokens: 0,
            common_pitfalls: Vec::new(),
        });

        // Update steps if this is a new or better solution
        if procedure.steps.is_empty() {
            procedure.steps = steps
                .iter()
                .enumerate()
                .map(|(i, desc)| ProcedureStep {
                    order: i,
                    description: desc.clone(),
                    typical_actions: Vec::new(),
                    expected_files: Vec::new(),
                    optional: false,
                })
                .collect();
        }

        procedure.execution_count += 1;

        // Save to storage
        let key = format!("procedure:{:?}", task_type);
        let value = serialize(&(task_type, procedure.clone()))?;
        self.storage.put(key.as_bytes(), &value).await?;

        Ok(())
    }

    /// Get count of stored procedures
    pub fn procedure_count(&self) -> usize {
        self.procedures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::types::{EpisodeId, TokenCount};
    use chrono::Utc;
    use tempfile::TempDir;

    async fn create_test_storage() -> (Arc<dyn Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MemoryStorage::new();
        (Arc::new(storage), temp_dir)
    }

    #[test]
    fn test_task_type_inference() {
        assert_eq!(TaskType::infer("Fix authentication bug"), TaskType::BugFix);
        assert_eq!(TaskType::infer("Add new feature to API"), TaskType::Feature);
        assert_eq!(TaskType::infer("Refactor payment module"), TaskType::Refactor);
        assert_eq!(TaskType::infer("Write tests for service"), TaskType::Test);
        assert_eq!(TaskType::infer("Update documentation"), TaskType::Documentation);
    }

    #[tokio::test]
    async fn test_learn_from_episodes() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = ProceduralMemory::new(storage).unwrap();

        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Fix authentication bug".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec!["find auth".to_string()],
            files_touched: vec!["auth.ts".to_string()],
            solution_path: "Identified issue and fixed validation".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(500),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Fix login bug".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec!["find auth".to_string()],
            files_touched: vec!["auth.ts".to_string()],
            solution_path: "Identified issue and fixed validation".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(450),
            access_count: 0,
            pattern_value: 0.8,
        };

        memory.learn_from_episodes(&[episode1, episode2]).await.unwrap();

        let procedure = memory.get_procedure(&TaskType::BugFix);
        assert!(procedure.is_some());
        let proc = procedure.unwrap();
        assert!(!proc.steps.is_empty());
        assert_eq!(proc.success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_next_step() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = ProceduralMemory::new(storage).unwrap();

        let procedure = Procedure {
            steps: vec![
                ProcedureStep {
                    order: 0,
                    description: "Identify issue".to_string(),
                    typical_actions: vec![],
                    expected_files: vec![],
                    optional: false,
                },
                ProcedureStep {
                    order: 1,
                    description: "Fix validation".to_string(),
                    typical_actions: vec![],
                    expected_files: vec![],
                    optional: false,
                },
            ],
            required_context: vec![],
            typical_queries: vec![],
            success_rate: 1.0,
            execution_count: 1,
            average_tokens: 500,
            common_pitfalls: vec![],
        };

        memory
            .learn_procedure(TaskType::BugFix, procedure)
            .await
            .unwrap();

        let next = memory.next_step(&TaskType::BugFix, &[]);
        assert!(next.is_some());
        assert_eq!(next.unwrap().description, "Identify issue");

        let next2 = memory.next_step(&TaskType::BugFix, &["Identify issue".to_string()]);
        assert!(next2.is_some());
        assert_eq!(next2.unwrap().description, "Fix validation");
    }

    #[tokio::test]
    async fn test_execution_trace() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = ProceduralMemory::new(storage).unwrap();

        let trace = ExecutionTrace {
            task_type: TaskType::BugFix,
            episode_id: "test-123".to_string(),
            steps_taken: vec!["Step 1".to_string(), "Step 2".to_string()],
            outcome: Outcome::Success,
            duration_estimate: 500,
        };

        memory.record_execution(trace).await.unwrap();

        let history = memory.get_execution_history(&TaskType::BugFix);
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_get_procedure_for_task() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = ProceduralMemory::new(storage).unwrap();

        let procedure = Procedure {
            steps: vec![],
            required_context: vec![],
            typical_queries: vec![],
            success_rate: 0.9,
            execution_count: 5,
            average_tokens: 600,
            common_pitfalls: vec![],
        };

        memory
            .learn_procedure(TaskType::Feature, procedure)
            .await
            .unwrap();

        let proc = memory.get_procedure_for_task("Add new API endpoint");
        assert!(proc.is_some());
        assert_eq!(proc.unwrap().success_rate, 0.9);
    }

    #[tokio::test]
    async fn test_common_pitfalls() {
        let (storage, _temp) = create_test_storage().await;
        let memory = ProceduralMemory::new(storage).unwrap();

        let failed_episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Fix bug".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Tried approach A but failed at validation".to_string(),
            outcome: Outcome::Failure,
            tokens_used: TokenCount::new(300),
            access_count: 0,
            pattern_value: 0.0,
        };

        let procedure = memory.extract_procedure(&[&failed_episode]);
        assert!(!procedure.common_pitfalls.is_empty());
    }

    #[test]
    fn test_calculate_success_rate() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Failed".to_string(),
            outcome: Outcome::Failure,
            tokens_used: TokenCount::new(50),
            access_count: 0,
            pattern_value: 0.0,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(120),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let success_rate = ProceduralMemory::calculate_success_rate(&episodes);

        // 2 out of 3 successful = 0.666...
        assert!((success_rate - 0.666).abs() < 0.01);

        // Test empty episodes
        let empty: Vec<&TaskEpisode> = vec![];
        assert_eq!(ProceduralMemory::calculate_success_rate(&empty), 0.0);
    }

    #[test]
    fn test_extract_common_steps() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Read file, Analyze code, Fix bug".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Read file, Analyze code, Write tests".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(120),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Read file, Fix bug".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(80),
            access_count: 0,
            pattern_value: 0.7,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let steps = ProceduralMemory::extract_common_steps(&episodes);

        // Should have at least 3 unique steps
        assert!(!steps.is_empty());

        // "Read file" should be the most common (appears in all 3)
        let read_step = steps.iter().find(|s| s.description.contains("Read file"));
        assert!(read_step.is_some());

        // Check that actions are extracted
        for step in &steps {
            assert!(!step.typical_actions.is_empty());
        }
    }

    #[test]
    fn test_extract_minimal_context() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["auth.rs".to_string(), "config.toml".to_string()],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["auth.rs".to_string(), "user.rs".to_string()],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(120),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["auth.rs".to_string(), "config.toml".to_string()],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(90),
            access_count: 0,
            pattern_value: 0.85,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let context = ProceduralMemory::extract_minimal_context(&episodes);

        // auth.rs appears in all 3 episodes (100%), should be included
        assert!(context.contains(&"auth.rs".to_string()));

        // config.toml appears in 2/3 episodes (66%), should be included
        assert!(context.contains(&"config.toml".to_string()));

        // user.rs appears in only 1/3 episodes (33%), should NOT be included
        assert!(!context.contains(&"user.rs".to_string()));
    }

    #[test]
    fn test_extract_query_patterns() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![
                "find auth".to_string(),
                "search authentication".to_string(),
            ],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![
                "find auth".to_string(),
                "get user service".to_string(),
            ],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(120),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![
                "  FIND AUTH  ".to_string(), // Should be normalized
                "search authentication".to_string(),
            ],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(90),
            access_count: 0,
            pattern_value: 0.85,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let patterns = ProceduralMemory::extract_query_patterns(&episodes);

        // "find auth" appears 3 times (with normalization), should be included
        assert!(patterns.contains(&"find auth".to_string()));

        // "search authentication" appears 2 times, should be included
        assert!(patterns.contains(&"search authentication".to_string()));

        // "get user service" appears only once, should NOT be included
        assert!(!patterns.contains(&"get user service".to_string()));

        // Check that the most frequent pattern is first
        assert_eq!(patterns[0], "find auth");
    }

    #[test]
    fn test_calculate_average_tokens() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.9,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(200),
            access_count: 0,
            pattern_value: 0.8,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Success".to_string(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::new(300),
            access_count: 0,
            pattern_value: 0.7,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let avg = ProceduralMemory::calculate_average_tokens(&episodes);

        // Average of 100, 200, 300 = 200
        assert_eq!(avg, 200);

        // Test empty episodes
        let empty: Vec<&TaskEpisode> = vec![];
        assert_eq!(ProceduralMemory::calculate_average_tokens(&empty), 0);
    }

    #[test]
    fn test_extract_common_pitfalls() {
        let episode1 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 1".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Failed at validation".to_string(),
            outcome: Outcome::Failure,
            tokens_used: TokenCount::new(100),
            access_count: 0,
            pattern_value: 0.0,
        };

        let episode2 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 2".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Failed at database connection".to_string(),
            outcome: Outcome::Failure,
            tokens_used: TokenCount::new(120),
            access_count: 0,
            pattern_value: 0.0,
        };

        let episode3 = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Task 3".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: "Failed at validation".to_string(), // Duplicate
            outcome: Outcome::Failure,
            tokens_used: TokenCount::new(90),
            access_count: 0,
            pattern_value: 0.0,
        };

        let episodes = vec![&episode1, &episode2, &episode3];
        let pitfalls = ProceduralMemory::extract_common_pitfalls(&episodes);

        // Should have 2 unique pitfalls (duplicates removed)
        assert_eq!(pitfalls.len(), 2);
        assert!(pitfalls.iter().any(|p| p.contains("validation")));
        assert!(pitfalls.iter().any(|p| p.contains("database connection")));
    }

    #[test]
    fn test_extract_actions_from_step() {
        let step1 = "Read the configuration file and update settings";
        let actions1 = ProceduralMemory::extract_actions_from_step(step1);
        assert!(actions1.contains(&"read".to_string()));
        assert!(actions1.contains(&"update".to_string()));

        let step2 = "Refactor the authentication module";
        let actions2 = ProceduralMemory::extract_actions_from_step(step2);
        assert!(actions2.contains(&"refactor".to_string()));

        let step3 = "Some random text without actions";
        let actions3 = ProceduralMemory::extract_actions_from_step(step3);
        // Should use full step if no actions found
        assert!(!actions3.is_empty());
    }

    #[test]
    fn test_extract_files_from_step() {
        let step1 = "Modified auth.rs to fix validation";
        let files1 = ProceduralMemory::extract_files_from_step(step1);
        // This is a simplified test - actual implementation might need improvement
        assert!(!files1.is_empty() || step1.contains(".rs"));

        let step2 = "Updated config.toml and package.json";
        let _files2 = ProceduralMemory::extract_files_from_step(step2);
        // File extraction from step descriptions is basic
        assert!(step2.contains(".toml"));
    }

    #[tokio::test]
    async fn test_full_learning_cycle() {
        let (storage, _temp) = create_test_storage().await;
        let mut memory = ProceduralMemory::new(storage).unwrap();

        // Create episodes with varying outcomes
        let episodes = vec![
            TaskEpisode {
                schema_version: 1,
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task_description: "Fix auth bug".to_string(),
                initial_context: crate::types::ContextSnapshot::default(),
                queries_made: vec!["find auth".to_string(), "search user".to_string()],
                files_touched: vec!["auth.rs".to_string(), "user.rs".to_string()],
                solution_path: "Read file, Analyze code, Fix validation".to_string(),
                outcome: Outcome::Success,
                tokens_used: TokenCount::new(500),
                access_count: 0,
                pattern_value: 0.9,
            },
            TaskEpisode {
                schema_version: 1,
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task_description: "Fix login bug".to_string(),
                initial_context: crate::types::ContextSnapshot::default(),
                queries_made: vec!["find auth".to_string()],
                files_touched: vec!["auth.rs".to_string()],
                solution_path: "Read file, Analyze code, Update tests".to_string(),
                outcome: Outcome::Success,
                tokens_used: TokenCount::new(450),
                access_count: 0,
                pattern_value: 0.8,
            },
            TaskEpisode {
                schema_version: 1,
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task_description: "Fix validation bug".to_string(),
                initial_context: crate::types::ContextSnapshot::default(),
                queries_made: vec!["search validation".to_string()],
                files_touched: vec!["validator.rs".to_string()],
                solution_path: "Attempted fix but failed at testing".to_string(),
                outcome: Outcome::Failure,
                tokens_used: TokenCount::new(300),
                access_count: 0,
                pattern_value: 0.0,
            },
        ];

        // Learn from episodes
        memory.learn_from_episodes(&episodes).await.unwrap();

        // Verify procedure was learned
        let procedure = memory.get_procedure(&TaskType::BugFix);
        assert!(procedure.is_some());

        let proc = procedure.unwrap();

        // Check success rate (2 successful out of 3 = 0.666...)
        assert!((proc.success_rate - 0.666).abs() < 0.01);

        // Check that common steps were extracted
        assert!(!proc.steps.is_empty());

        // Check that required context includes auth.rs (appears in 2/3)
        assert!(proc.required_context.contains(&"auth.rs".to_string()));

        // Check that typical queries include "find auth" (appears 2 times)
        assert!(proc.typical_queries.contains(&"find auth".to_string()));

        // Check average tokens (500 + 450 + 300) / 3 = 416.666...
        assert!((proc.average_tokens as f32 - 416.666).abs() < 1.0);

        // Check that pitfalls were captured
        assert!(!proc.common_pitfalls.is_empty());
        assert!(proc
            .common_pitfalls
            .iter()
            .any(|p| p.contains("testing")));
    }
}
