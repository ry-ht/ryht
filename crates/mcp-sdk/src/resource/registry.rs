//! Resource registry for managing registered resources.
//!
//! This module provides the [`ResourceRegistry`] for thread-safe resource management.

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::Resource;

/// A thread-safe registry for managing MCP resources.
///
/// The registry stores all registered resources and provides methods for:
/// - Registering new resources
/// - Finding resources by URI pattern
/// - Listing all registered resources
///
/// # Thread Safety
///
/// The registry uses `Arc<RwLock<>>` internally, making it safe to share
/// across multiple threads and async tasks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// # struct ConfigResource;
/// # #[async_trait]
/// # impl Resource for ConfigResource {
/// #     fn uri_pattern(&self) -> &str { "app://config" }
/// #     fn name(&self) -> Option<&str> { Some("Config") }
/// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
/// #         Ok(ResourceContent::text("config", "application/json"))
/// #     }
/// # }
/// #
/// # async fn example() {
/// let registry = ResourceRegistry::new();
///
/// // Register a resource
/// registry.register(ConfigResource).await;
///
/// // Find by URI
/// let resource = registry.find_by_uri("app://config").await;
/// assert!(resource.is_some());
///
/// // List all resources
/// let definitions = registry.list().await;
/// assert_eq!(definitions.len(), 1);
/// # }
/// ```
///
/// ## Multiple Resources
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// # struct ConfigResource;
/// # #[async_trait]
/// # impl Resource for ConfigResource {
/// #     fn uri_pattern(&self) -> &str { "app://config" }
/// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
/// #         Ok(ResourceContent::text("config", "application/json"))
/// #     }
/// # }
/// # struct UserResource;
/// # #[async_trait]
/// # impl Resource for UserResource {
/// #     fn uri_pattern(&self) -> &str { "db://users/*" }
/// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
/// #         Ok(ResourceContent::text("user", "application/json"))
/// #     }
/// # }
/// #
/// # async fn example() {
/// let registry = ResourceRegistry::new();
///
/// registry.register(ConfigResource).await;
/// registry.register(UserResource).await;
///
/// // Find specific user
/// let user_res = registry.find_by_uri("db://users/123").await;
/// assert!(user_res.is_some());
///
/// // Find config
/// let config_res = registry.find_by_uri("app://config").await;
/// assert!(config_res.is_some());
/// # }
/// ```
#[derive(Clone)]
pub struct ResourceRegistry {
    resources: Arc<RwLock<Vec<Arc<dyn Resource>>>>,
}

impl ResourceRegistry {
    /// Creates a new empty resource registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceRegistry;
    ///
    /// let registry = ResourceRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Registers a new resource with the registry.
    ///
    /// The resource will be available for URI matching and content retrieval.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// # struct MyResource;
    /// # #[async_trait]
    /// # impl Resource for MyResource {
    /// #     fn uri_pattern(&self) -> &str { "test://resource" }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("content", "text/plain"))
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = ResourceRegistry::new();
    /// registry.register(MyResource).await;
    /// # }
    /// ```
    pub async fn register<R: Resource + 'static>(&self, resource: R) {
        let mut resources = self.resources.write();
        resources.push(Arc::new(resource));
    }

    /// Registers an Arc-wrapped resource.
    ///
    /// This is useful when you already have an Arc-wrapped resource instance.
    ///
    /// # Arguments
    ///
    /// * `resource` - Arc-wrapped resource to register
    pub async fn register_arc(&self, resource: Arc<dyn Resource>) {
        let mut resources = self.resources.write();
        resources.push(resource);
    }

    /// Finds the first resource that matches the given URI.
    ///
    /// Resources are checked in the order they were registered. The first
    /// resource whose pattern matches the URI is returned.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI to match against registered resources
    ///
    /// # Returns
    ///
    /// The first matching resource, or `None` if no resource matches.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// # struct UserResource;
    /// # #[async_trait]
    /// # impl Resource for UserResource {
    /// #     fn uri_pattern(&self) -> &str { "db://users/*" }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("user", "application/json"))
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = ResourceRegistry::new();
    /// registry.register(UserResource).await;
    ///
    /// // Find matching resource
    /// let resource = registry.find_by_uri("db://users/123").await;
    /// assert!(resource.is_some());
    ///
    /// // No match
    /// let not_found = registry.find_by_uri("db://posts/123").await;
    /// assert!(not_found.is_none());
    /// # }
    /// ```
    pub async fn find_by_uri(&self, uri: &str) -> Option<Arc<dyn Resource>> {
        let resources = self.resources.read();
        resources.iter()
            .find(|resource| resource.matches(uri))
            .cloned()
    }

    /// Lists all registered resources as definitions.
    ///
    /// This returns metadata about all registered resources, suitable for
    /// responding to MCP `resources/list` requests.
    ///
    /// # Returns
    ///
    /// A vector of resource definitions containing URI patterns, names, descriptions,
    /// and MIME types for all registered resources.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// # struct ConfigResource;
    /// # #[async_trait]
    /// # impl Resource for ConfigResource {
    /// #     fn uri_pattern(&self) -> &str { "app://config" }
    /// #     fn name(&self) -> Option<&str> { Some("Config") }
    /// #     fn description(&self) -> Option<&str> { Some("Application configuration") }
    /// #     fn mime_type(&self) -> Option<&str> { Some("application/json") }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("config", "application/json"))
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = ResourceRegistry::new();
    /// registry.register(ConfigResource).await;
    ///
    /// let definitions = registry.list().await;
    /// assert_eq!(definitions.len(), 1);
    /// assert_eq!(definitions[0].uri, "app://config");
    /// assert_eq!(definitions[0].name, Some("Config".to_string()));
    /// # }
    /// ```
    pub async fn list(&self) -> Vec<ResourceDefinition> {
        let resources = self.resources.read();
        resources.iter()
            .map(|resource| ResourceDefinition {
                uri: resource.uri_pattern().to_string(),
                name: resource.name().map(|s| s.to_string()),
                description: resource.description().map(|s| s.to_string()),
                mime_type: resource.mime_type().map(|s| s.to_string()),
            })
            .collect()
    }

    /// Returns the number of registered resources.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// # struct MyResource;
    /// # #[async_trait]
    /// # impl Resource for MyResource {
    /// #     fn uri_pattern(&self) -> &str { "test://resource" }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("content", "text/plain"))
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = ResourceRegistry::new();
    /// assert_eq!(registry.count().await, 0);
    ///
    /// registry.register(MyResource).await;
    /// assert_eq!(registry.count().await, 1);
    /// # }
    /// ```
    pub async fn count(&self) -> usize {
        let resources = self.resources.read();
        resources.len()
    }

    /// Clears all registered resources.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext, ResourceRegistry};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// # struct MyResource;
    /// # #[async_trait]
    /// # impl Resource for MyResource {
    /// #     fn uri_pattern(&self) -> &str { "test://resource" }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("content", "text/plain"))
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = ResourceRegistry::new();
    /// registry.register(MyResource).await;
    /// assert_eq!(registry.count().await, 1);
    ///
    /// registry.clear().await;
    /// assert_eq!(registry.count().await, 0);
    /// # }
    /// ```
    pub async fn clear(&self) {
        let mut resources = self.resources.write();
        resources.clear();
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about a registered resource.
///
/// This struct contains the information about a resource that is returned
/// by the `resources/list` MCP method.
///
/// # Examples
///
/// ```rust
/// use mcp_server::resource::ResourceDefinition;
///
/// let def = ResourceDefinition {
///     uri: "app://config".to_string(),
///     name: Some("Config".to_string()),
///     description: Some("Application configuration".to_string()),
///     mime_type: Some("application/json".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceDefinition {
    /// The URI pattern for this resource
    pub uri: String,

    /// Optional human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional MIME type
    #[serde(skip_serializing_if = "Option::is_none", rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ResourceError;
    use crate::resource::{ResourceContent, ResourceContext};
    use async_trait::async_trait;

    struct TestResource {
        pattern: &'static str,
        name: Option<&'static str>,
        description: Option<&'static str>,
        mime_type: Option<&'static str>,
    }

    #[async_trait]
    impl Resource for TestResource {
        fn uri_pattern(&self) -> &str {
            self.pattern
        }

        fn name(&self) -> Option<&str> {
            self.name
        }

        fn description(&self) -> Option<&str> {
            self.description
        }

        fn mime_type(&self) -> Option<&str> {
            self.mime_type
        }

        async fn read(
            &self,
            _uri: &str,
            _context: &ResourceContext,
        ) -> Result<ResourceContent, ResourceError> {
            Ok(ResourceContent::text("test content", "text/plain"))
        }
    }

    #[tokio::test]
    async fn test_new_registry() {
        let registry = ResourceRegistry::new();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_register_resource() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_find_by_uri_exact_match() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        let found = registry.find_by_uri("test://resource").await;
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_find_by_uri_wildcard() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "db://users/*",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        assert!(registry.find_by_uri("db://users/123").await.is_some());
        assert!(registry.find_by_uri("db://users/alice").await.is_some());
        assert!(registry.find_by_uri("db://posts/123").await.is_none());
    }

    #[tokio::test]
    async fn test_find_by_uri_not_found() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        let not_found = registry.find_by_uri("other://resource").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_empty() {
        let registry = ResourceRegistry::new();
        let definitions = registry.list().await;
        assert_eq!(definitions.len(), 0);
    }

    #[tokio::test]
    async fn test_list_resources() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource1",
            name: Some("Resource 1"),
            description: Some("First resource"),
            mime_type: Some("text/plain"),
        }).await;

        registry.register(TestResource {
            pattern: "test://resource2",
            name: Some("Resource 2"),
            description: None,
            mime_type: None,
        }).await;

        let definitions = registry.list().await;
        assert_eq!(definitions.len(), 2);

        assert_eq!(definitions[0].uri, "test://resource1");
        assert_eq!(definitions[0].name, Some("Resource 1".to_string()));
        assert_eq!(definitions[0].description, Some("First resource".to_string()));
        assert_eq!(definitions[0].mime_type, Some("text/plain".to_string()));

        assert_eq!(definitions[1].uri, "test://resource2");
        assert_eq!(definitions[1].name, Some("Resource 2".to_string()));
        assert_eq!(definitions[1].description, None);
        assert_eq!(definitions[1].mime_type, None);
    }

    #[tokio::test]
    async fn test_clear() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        assert_eq!(registry.count().await, 1);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_multiple_resources() {
        let registry = ResourceRegistry::new();

        for _i in 0..10 {
            registry.register(TestResource {
                pattern: "test://resource",
                name: Some("Test"),
                description: None,
                mime_type: None,
            }).await;
        }

        assert_eq!(registry.count().await, 10);
    }

    #[tokio::test]
    async fn test_first_match_wins() {
        let registry = ResourceRegistry::new();

        // Register two resources with overlapping patterns
        registry.register(TestResource {
            pattern: "db://**",
            name: Some("First"),
            description: None,
            mime_type: None,
        }).await;

        registry.register(TestResource {
            pattern: "db://users/*",
            name: Some("Second"),
            description: None,
            mime_type: None,
        }).await;

        // First registered resource should match
        let resource = registry.find_by_uri("db://users/123").await.unwrap();
        assert_eq!(resource.name(), Some("First"));
    }

    #[tokio::test]
    async fn test_registry_clone() {
        let registry1 = ResourceRegistry::new();

        registry1.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        let registry2 = registry1.clone();
        assert_eq!(registry2.count().await, 1);
    }

    #[tokio::test]
    async fn test_default() {
        let registry = ResourceRegistry::default();
        assert_eq!(registry.count().await, 0);
    }

    #[test]
    fn test_resource_definition_serialization() {
        let def = ResourceDefinition {
            uri: "app://config".to_string(),
            name: Some("Config".to_string()),
            description: Some("Application config".to_string()),
            mime_type: Some("application/json".to_string()),
        };

        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"uri\":\"app://config\""));
        assert!(json.contains("\"name\":\"Config\""));
        assert!(json.contains("\"mimeType\":\"application/json\""));
    }

    #[test]
    fn test_resource_definition_deserialization() {
        let json = r#"{
            "uri": "app://config",
            "name": "Config",
            "description": "Application config",
            "mimeType": "application/json"
        }"#;

        let def: ResourceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.uri, "app://config");
        assert_eq!(def.name, Some("Config".to_string()));
        assert_eq!(def.description, Some("Application config".to_string()));
        assert_eq!(def.mime_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_resource_definition_omit_none() {
        let def = ResourceDefinition {
            uri: "app://config".to_string(),
            name: None,
            description: None,
            mime_type: None,
        };

        let json = serde_json::to_string(&def).unwrap();
        assert!(!json.contains("\"name\""));
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"mimeType\""));
    }

    #[tokio::test]
    async fn test_glob_patterns() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "file:///*.txt",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        assert!(registry.find_by_uri("file:///readme.txt").await.is_some());
        assert!(registry.find_by_uri("file:///doc.txt").await.is_some());
        assert!(registry.find_by_uri("file:///doc.md").await.is_none());
    }

    #[tokio::test]
    async fn test_read_through_registry() {
        let registry = ResourceRegistry::new();

        registry.register(TestResource {
            pattern: "test://resource",
            name: None,
            description: None,
            mime_type: None,
        }).await;

        let resource = registry.find_by_uri("test://resource").await.unwrap();
        let context = ResourceContext::default();
        let content = resource.read("test://resource", &context).await.unwrap();

        assert_eq!(content.as_text(), Some("test content"));
    }
}
