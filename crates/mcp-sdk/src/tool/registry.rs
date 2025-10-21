//! Tool registry for managing tool registration and lookup.
//!
//! This module provides a thread-safe registry for storing and accessing tools.

use super::{Tool, ToolDefinition};
use crate::error::{RegistryError, ToolError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe registry for managing tools.
///
/// `ToolRegistry` provides a central place to register and look up tools.
/// It uses `Arc<RwLock<HashMap>>` to ensure thread-safe access across async tasks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
/// use mcp_server::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct EchoTool;
///
/// #[async_trait]
/// impl Tool for EchoTool {
///     fn name(&self) -> &str { "echo" }
///     fn input_schema(&self) -> Value { json!({}) }
///     async fn execute(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
///         Ok(ToolResult::success_text("echo"))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let registry = ToolRegistry::new();
///     registry.register(EchoTool).await.unwrap();
///
///     assert!(registry.has("echo").await);
///     assert_eq!(registry.list().await.len(), 1);
/// }
/// ```
///
/// ## Duplicate Detection
///
/// ```
/// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
/// use mcp_server::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn name(&self) -> &str { "my_tool" }
///     fn input_schema(&self) -> Value { json!({}) }
///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
///         Ok(ToolResult::success_text(""))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let registry = ToolRegistry::new();
///     registry.register(MyTool).await.unwrap();
///
///     // Attempting to register again will fail
///     let result = registry.register(MyTool).await;
///     assert!(result.is_err());
/// }
/// ```
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &"<HashMap<String, Arc<dyn Tool>>>")
            .finish()
    }
}

impl ToolRegistry {
    /// Creates a new empty tool registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a tool in the registry.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError::AlreadyRegistered` if a tool with the same name
    /// is already registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     registry.register(MyTool).await.unwrap();
    /// }
    /// ```
    pub async fn register<T: Tool + 'static>(&self, tool: T) -> Result<(), ToolError> {
        let mut tools = self.tools.write().await;
        let name = tool.name().to_string();

        if tools.contains_key(&name) {
            return Err(RegistryError::DuplicateTool(name).into());
        }

        tools.insert(name, Arc::new(tool));
        Ok(())
    }

    /// Registers a tool from an Arc.
    ///
    /// This is useful when you already have an Arc-wrapped tool.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError::AlreadyRegistered` if a tool with the same name
    /// is already registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    /// use std::sync::Arc;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     let tool = Arc::new(MyTool);
    ///     registry.register_arc(tool).await.unwrap();
    /// }
    /// ```
    pub async fn register_arc(&self, tool: Arc<dyn Tool>) -> Result<(), ToolError> {
        let mut tools = self.tools.write().await;
        let name = tool.name().to_string();

        if tools.contains_key(&name) {
            return Err(RegistryError::DuplicateTool(name).into());
        }

        tools.insert(name, tool);
        Ok(())
    }

    /// Gets a tool by name.
    ///
    /// Returns `None` if the tool is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     registry.register(MyTool).await.unwrap();
    ///
    ///     let tool = registry.get("my_tool").await;
    ///     assert!(tool.is_some());
    ///
    ///     let missing = registry.get("nonexistent").await;
    ///     assert!(missing.is_none());
    /// }
    /// ```
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Checks if a tool with the given name exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     registry.register(MyTool).await.unwrap();
    ///
    ///     assert!(registry.has("my_tool").await);
    ///     assert!(!registry.has("nonexistent").await);
    /// }
    /// ```
    pub async fn has(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// Lists all registered tools as tool definitions.
    ///
    /// Returns a vector of `ToolDefinition` containing the metadata for each tool.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct Tool1;
    /// struct Tool2;
    ///
    /// #[async_trait]
    /// impl Tool for Tool1 {
    ///     fn name(&self) -> &str { "tool1" }
    ///     fn description(&self) -> Option<&str> { Some("First tool") }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[async_trait]
    /// impl Tool for Tool2 {
    ///     fn name(&self) -> &str { "tool2" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     registry.register(Tool1).await.unwrap();
    ///     registry.register(Tool2).await.unwrap();
    ///
    ///     let definitions = registry.list().await;
    ///     assert_eq!(definitions.len(), 2);
    /// }
    /// ```
    pub async fn list(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
                output_schema: tool.output_schema(),
            })
            .collect()
    }

    /// Returns the number of registered tools.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let registry = ToolRegistry::new();
    ///     assert_eq!(registry.count().await, 0);
    ///
    ///     registry.register(MyTool).await.unwrap();
    ///     assert_eq!(registry.count().await, 1);
    /// }
    /// ```
    pub async fn count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ToolContext, ToolResult};
    use async_trait::async_trait;
    use serde_json::json;

    struct TestTool {
        name: String,
    }

    impl TestTool {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> Option<&str> {
            Some("A test tool")
        }

        fn input_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {}
            })
        }

        async fn execute(
            &self,
            _input: serde_json::Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success_text("test"))
        }
    }

    #[tokio::test]
    async fn test_registry_new() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_register_tool() {
        let registry = ToolRegistry::new();
        let tool = TestTool::new("test_tool");

        registry.register(tool).await.unwrap();
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_register_duplicate_tool() {
        let registry = ToolRegistry::new();

        registry.register(TestTool::new("duplicate")).await.unwrap();
        let result = registry.register(TestTool::new("duplicate")).await;

        assert!(result.is_err());
        match result {
            Err(ToolError::AlreadyRegistered(name)) => {
                assert_eq!(name, "duplicate");
            }
            _ => panic!("Expected AlreadyRegistered error"),
        }
    }

    #[tokio::test]
    async fn test_get_tool() {
        let registry = ToolRegistry::new();
        registry.register(TestTool::new("my_tool")).await.unwrap();

        let tool = registry.get("my_tool").await;
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "my_tool");
    }

    #[tokio::test]
    async fn test_get_nonexistent_tool() {
        let registry = ToolRegistry::new();
        let tool = registry.get("nonexistent").await;
        assert!(tool.is_none());
    }

    #[tokio::test]
    async fn test_has_tool() {
        let registry = ToolRegistry::new();
        registry.register(TestTool::new("exists")).await.unwrap();

        assert!(registry.has("exists").await);
        assert!(!registry.has("does_not_exist").await);
    }

    #[tokio::test]
    async fn test_list_tools() {
        let registry = ToolRegistry::new();
        registry.register(TestTool::new("tool1")).await.unwrap();
        registry.register(TestTool::new("tool2")).await.unwrap();
        registry.register(TestTool::new("tool3")).await.unwrap();

        let tools = registry.list().await;
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"tool1"));
        assert!(names.contains(&"tool2"));
        assert!(names.contains(&"tool3"));
    }

    #[tokio::test]
    async fn test_list_empty_registry() {
        let registry = ToolRegistry::new();
        let tools = registry.list().await;
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_count() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.count().await, 0);

        registry.register(TestTool::new("tool1")).await.unwrap();
        assert_eq!(registry.count().await, 1);

        registry.register(TestTool::new("tool2")).await.unwrap();
        assert_eq!(registry.count().await, 2);
    }

    #[tokio::test]
    async fn test_registry_clone() {
        let registry1 = ToolRegistry::new();
        registry1.register(TestTool::new("tool1")).await.unwrap();

        let registry2 = registry1.clone();
        assert!(registry2.has("tool1").await);

        // Both registries share the same underlying storage
        registry2.register(TestTool::new("tool2")).await.unwrap();
        assert!(registry1.has("tool2").await);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let registry = Arc::new(ToolRegistry::new());

        let mut handles = vec![];
        for i in 0..10 {
            let registry = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                registry
                    .register(TestTool::new(format!("tool{}", i)))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(registry.count().await, 10);
    }

    #[tokio::test]
    async fn test_tool_definition_properties() {
        let registry = ToolRegistry::new();
        registry.register(TestTool::new("my_tool")).await.unwrap();

        let tools = registry.list().await;
        assert_eq!(tools.len(), 1);

        let def = &tools[0];
        assert_eq!(def.name, "my_tool");
        assert_eq!(def.description, Some("A test tool".to_string()));
        assert!(def.input_schema.is_object());
        assert!(def.output_schema.is_none());
    }
}
