//! Task Delegation - Clear Task Boundaries and Objectives
//!
//! This module defines task delegation structures that provide workers with:
//! - Explicit objectives
//! - Defined output formats
//! - Tool/source guidance
//! - Clear boundaries to prevent duplicate work and gaps
//!
//! Following Anthropic's best practices: "Each agent knows exactly what they're
//! responsible for" to avoid duplicate work and ensure comprehensive coverage.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Task Delegation
// ============================================================================

/// Task delegation with explicit boundaries
///
/// Provides complete context for a worker to execute independently while
/// staying within defined scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDelegation {
    /// Unique task identifier
    pub task_id: String,

    /// Clear objective statement
    ///
    /// Example: "Investigate memory leaks in the user authentication module"
    pub objective: String,

    /// Expected output format specification
    pub output_format: super::strategy_library::OutputFormat,

    /// Tools the worker is allowed to use
    ///
    /// Prevents workers from using inappropriate tools
    pub allowed_tools: Vec<String>,

    /// Task boundaries defining scope and constraints
    pub boundaries: TaskBoundaries,

    /// Priority level (1-10, higher is more important)
    pub priority: u8,

    /// Required capabilities to execute this task
    pub required_capabilities: Vec<String>,

    /// Additional context for the task
    ///
    /// Provides background information without constraining approach
    pub context: serde_json::Value,
}

/// Task boundaries prevent scope creep and duplicate work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoundaries {
    /// Scope definition - what to focus on
    ///
    /// Example: ["user_auth module", "session management"]
    pub scope: Vec<String>,

    /// Explicit constraints - what NOT to do
    ///
    /// Example: ["Don't modify database schema", "Don't touch payment processing"]
    pub constraints: Vec<String>,

    /// Maximum tool calls allowed
    ///
    /// Prevents runaway execution
    pub max_tool_calls: usize,

    /// Task timeout
    ///
    /// Maximum duration before task is terminated
    pub timeout: Duration,
}

impl TaskDelegation {
    /// Create a new task delegation with builder pattern
    pub fn builder() -> TaskDelegationBuilder {
        TaskDelegationBuilder::new()
    }

    /// Validate that delegation has all required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.objective.is_empty() {
            return Err("Objective cannot be empty".to_string());
        }

        if self.boundaries.scope.is_empty() {
            return Err("Scope cannot be empty".to_string());
        }

        if self.boundaries.max_tool_calls == 0 {
            return Err("Max tool calls must be > 0".to_string());
        }

        if self.priority == 0 || self.priority > 10 {
            return Err("Priority must be between 1 and 10".to_string());
        }

        Ok(())
    }

    /// Check if task is within scope
    pub fn is_in_scope(&self, item: &str) -> bool {
        self.boundaries
            .scope
            .iter()
            .any(|s| item.contains(s) || s.contains(item))
    }

    /// Check if action is constrained
    pub fn is_constrained(&self, action: &str) -> bool {
        self.boundaries
            .constraints
            .iter()
            .any(|c| action.contains(c) || c.contains(action))
    }

    /// Check if tool is allowed
    pub fn is_tool_allowed(&self, tool: &str) -> bool {
        self.allowed_tools.is_empty() || self.allowed_tools.contains(&tool.to_string())
    }
}

/// Builder for creating task delegations
pub struct TaskDelegationBuilder {
    task_id: Option<String>,
    objective: Option<String>,
    output_format: Option<super::strategy_library::OutputFormat>,
    allowed_tools: Vec<String>,
    scope: Vec<String>,
    constraints: Vec<String>,
    max_tool_calls: Option<usize>,
    timeout: Option<Duration>,
    priority: u8,
    required_capabilities: Vec<String>,
    context: serde_json::Value,
}

impl TaskDelegationBuilder {
    pub fn new() -> Self {
        Self {
            task_id: None,
            objective: None,
            output_format: None,
            allowed_tools: Vec::new(),
            scope: Vec::new(),
            constraints: Vec::new(),
            max_tool_calls: None,
            timeout: None,
            priority: 5,
            required_capabilities: Vec::new(),
            context: serde_json::Value::Object(Default::default()),
        }
    }

    pub fn task_id(mut self, id: String) -> Self {
        self.task_id = Some(id);
        self
    }

    pub fn objective(mut self, obj: String) -> Self {
        self.objective = Some(obj);
        self
    }

    pub fn output_format(mut self, format: super::strategy_library::OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    pub fn add_tool(mut self, tool: String) -> Self {
        self.allowed_tools.push(tool);
        self
    }

    pub fn scope(mut self, scope: Vec<String>) -> Self {
        self.scope = scope;
        self
    }

    pub fn add_scope(mut self, item: String) -> Self {
        self.scope.push(item);
        self
    }

    pub fn constraints(mut self, constraints: Vec<String>) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn add_constraint(mut self, constraint: String) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn max_tool_calls(mut self, max: usize) -> Self {
        self.max_tool_calls = Some(max);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }

    pub fn required_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    pub fn context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    pub fn build(self) -> Result<TaskDelegation, String> {
        let task_id = self
            .task_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let objective = self.objective.ok_or("Objective is required")?;

        let output_format = self
            .output_format
            .unwrap_or_default();

        let max_tool_calls = self.max_tool_calls.ok_or("Max tool calls is required")?;
        let timeout = self.timeout.ok_or("Timeout is required")?;

        let delegation = TaskDelegation {
            task_id,
            objective,
            output_format,
            allowed_tools: self.allowed_tools,
            boundaries: TaskBoundaries {
                scope: self.scope,
                constraints: self.constraints,
                max_tool_calls,
                timeout,
            },
            priority: self.priority,
            required_capabilities: self.required_capabilities,
            context: self.context,
        };

        delegation.validate()?;

        Ok(delegation)
    }
}

impl Default for TaskDelegationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Task Templates
// ============================================================================

/// Pre-defined templates for common task types
pub struct TaskTemplates;

impl TaskTemplates {
    /// Create a code review task delegation
    pub fn code_review(file_path: &str, focus_areas: Vec<String>) -> TaskDelegation {
        TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: format!("Review code in {} for quality and correctness", file_path),
            output_format: super::strategy_library::OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "summary".to_string(),
                    "issues_found".to_string(),
                    "recommendations".to_string(),
                ],
                optional_sections: vec!["code_quality_score".to_string()],
                schema: None,
            },
            allowed_tools: vec![
                "code_reader".to_string(),
                "static_analyzer".to_string(),
                "complexity_analyzer".to_string(),
            ],
            boundaries: TaskBoundaries {
                scope: vec![file_path.to_string()],
                constraints: vec![
                    "Don't modify code".to_string(),
                    "Stay within file scope".to_string(),
                ],
                max_tool_calls: 10,
                timeout: Duration::from_secs(60),
            },
            priority: 7,
            required_capabilities: vec!["CodeReview".to_string()],
            context: serde_json::json!({
                "file": file_path,
                "focus_areas": focus_areas,
            }),
        }
    }

    /// Create a bug investigation task delegation
    pub fn bug_investigation(description: &str, affected_files: Vec<String>) -> TaskDelegation {
        TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: format!("Investigate bug: {}", description),
            output_format: super::strategy_library::OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "root_cause".to_string(),
                    "affected_areas".to_string(),
                    "proposed_fix".to_string(),
                ],
                optional_sections: vec!["reproduction_steps".to_string()],
                schema: None,
            },
            allowed_tools: vec![
                "code_reader".to_string(),
                "log_analyzer".to_string(),
                "trace_analyzer".to_string(),
            ],
            boundaries: TaskBoundaries {
                scope: affected_files.clone(),
                constraints: vec!["Don't modify production code".to_string()],
                max_tool_calls: 15,
                timeout: Duration::from_secs(120),
            },
            priority: 9,
            required_capabilities: vec![
                "CodeAnalysis".to_string(),
                "DebuggingAssistance".to_string(),
            ],
            context: serde_json::json!({
                "bug_description": description,
                "affected_files": affected_files,
            }),
        }
    }

    /// Create a research task delegation
    pub fn research(topic: &str, keywords: Vec<String>) -> TaskDelegation {
        TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: format!("Research topic: {}", topic),
            output_format: super::strategy_library::OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "summary".to_string(),
                    "key_findings".to_string(),
                    "sources".to_string(),
                ],
                optional_sections: vec!["recommendations".to_string()],
                schema: None,
            },
            allowed_tools: vec![
                "search".to_string(),
                "semantic_search".to_string(),
                "documentation_reader".to_string(),
            ],
            boundaries: TaskBoundaries {
                scope: vec![topic.to_string()],
                constraints: vec!["Focus on recent information (last 2 years)".to_string()],
                max_tool_calls: 20,
                timeout: Duration::from_secs(180),
            },
            priority: 5,
            required_capabilities: vec!["InformationRetrieval".to_string()],
            context: serde_json::json!({
                "topic": topic,
                "keywords": keywords,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_delegation_builder() {
        let delegation = TaskDelegation::builder()
            .objective("Test objective".to_string())
            .add_scope("test scope".to_string())
            .add_constraint("test constraint".to_string())
            .max_tool_calls(10)
            .timeout(Duration::from_secs(60))
            .priority(5)
            .build();

        assert!(delegation.is_ok());
        let d = delegation.unwrap();
        assert_eq!(d.objective, "Test objective");
        assert_eq!(d.priority, 5);
    }

    #[test]
    fn test_task_validation() {
        let delegation = TaskDelegation::builder()
            .objective("".to_string())
            .max_tool_calls(10)
            .timeout(Duration::from_secs(60))
            .build();

        assert!(delegation.is_err());
    }

    #[test]
    fn test_scope_checking() {
        let delegation = TaskDelegation::builder()
            .objective("Test".to_string())
            .add_scope("user_auth".to_string())
            .max_tool_calls(10)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        assert!(delegation.is_in_scope("user_auth.rs"));
        assert!(!delegation.is_in_scope("payment.rs"));
    }

    #[test]
    fn test_code_review_template() {
        let delegation = TaskTemplates::code_review("src/main.rs", vec!["security".to_string()]);
        assert!(delegation.objective.contains("Review code"));
        assert_eq!(delegation.priority, 7);
    }
}
