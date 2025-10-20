//! MCP Server and Client Capabilities
//!
//! This module defines the capability negotiation types used during MCP initialization.
//!
//! # Overview
//!
//! Capabilities allow servers and clients to negotiate which features they support.
//! During the initialization handshake, both parties exchange their capabilities.
//!
//! # Examples
//!
//! ```
//! use mcp_server::protocol::{ServerCapabilities, ToolsCapability, ResourcesCapability};
//!
//! // Server with tools and resources
//! let capabilities = ServerCapabilities::builder()
//!     .with_tools(ToolsCapability { list_changed: Some(true) })
//!     .with_resources(ResourcesCapability { subscribe: Some(true), list_changed: Some(true) })
//!     .build();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server capabilities advertised during initialization
///
/// Capabilities inform clients which features the server supports.
///
/// # MCP Spec 2025-03-26
///
/// As per the specification, capabilities include:
/// - `tools`: Tool execution support
/// - `resources`: Resource reading support
/// - `prompts`: Prompt templates support
/// - `logging`: Logging support
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{ServerCapabilities, ToolsCapability};
///
/// let mut capabilities = ServerCapabilities::default();
/// capabilities.tools = Some(ToolsCapability { list_changed: Some(true) });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ServerCapabilities {
    /// Tool execution capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Resource reading capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Prompt templates capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,

    /// Experimental or custom capabilities
    #[serde(flatten)]
    pub experimental: HashMap<String, serde_json::Value>,
}

/// Tools capability details
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ToolsCapability;
///
/// let capability = ToolsCapability {
///     list_changed: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolsCapability {
    /// Whether the server sends notifications when tool list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability details
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ResourcesCapability;
///
/// let capability = ResourcesCapability {
///     subscribe: Some(true),
///     list_changed: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourcesCapability {
    /// Whether the server supports resource subscriptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether the server sends notifications when resource list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompts capability details
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::PromptsCapability;
///
/// let capability = PromptsCapability {
///     list_changed: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptsCapability {
    /// Whether the server sends notifications when prompt list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Logging capability details
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::LoggingCapability;
///
/// let capability = LoggingCapability {};
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingCapability {
    // Currently no additional fields defined in spec
}

/// Client capabilities provided during initialization
///
/// Informs the server which features the client supports.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{ClientCapabilities, RootsCapability};
///
/// let capabilities = ClientCapabilities {
///     roots: Some(RootsCapability { list_changed: Some(true) }),
///     sampling: None,
///     experimental: Default::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ClientCapabilities {
    /// Roots capability (filesystem roots the client can access)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Sampling capability (for LLM sampling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,

    /// Experimental or custom capabilities
    #[serde(flatten)]
    pub experimental: HashMap<String, serde_json::Value>,
}

/// Roots capability details
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::RootsCapability;
///
/// let capability = RootsCapability {
///     list_changed: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RootsCapability {
    /// Whether the client sends notifications when roots list changes
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Sampling capability details
///
/// Indicates the client can perform LLM sampling on behalf of the server.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::SamplingCapability;
///
/// let capability = SamplingCapability {};
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamplingCapability {
    // Currently no additional fields defined in spec
}

impl ServerCapabilities {
    /// Create a new builder for ServerCapabilities
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::{ServerCapabilities, ToolsCapability};
    ///
    /// let capabilities = ServerCapabilities::builder()
    ///     .with_tools(ToolsCapability { list_changed: Some(true) })
    ///     .build();
    /// ```
    pub fn builder() -> ServerCapabilitiesBuilder {
        ServerCapabilitiesBuilder::default()
    }
}

/// Builder for ServerCapabilities
#[derive(Debug, Default)]
pub struct ServerCapabilitiesBuilder {
    capabilities: ServerCapabilities,
}

impl ServerCapabilitiesBuilder {
    /// Add tools capability
    pub fn with_tools(mut self, tools: ToolsCapability) -> Self {
        self.capabilities.tools = Some(tools);
        self
    }

    /// Add resources capability
    pub fn with_resources(mut self, resources: ResourcesCapability) -> Self {
        self.capabilities.resources = Some(resources);
        self
    }

    /// Add prompts capability
    pub fn with_prompts(mut self, prompts: PromptsCapability) -> Self {
        self.capabilities.prompts = Some(prompts);
        self
    }

    /// Add logging capability
    pub fn with_logging(mut self, logging: LoggingCapability) -> Self {
        self.capabilities.logging = Some(logging);
        self
    }

    /// Add an experimental capability
    pub fn with_experimental(mut self, key: String, value: serde_json::Value) -> Self {
        self.capabilities.experimental.insert(key, value);
        self
    }

    /// Build the ServerCapabilities
    pub fn build(self) -> ServerCapabilities {
        self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_server_capabilities_default() {
        let caps = ServerCapabilities::default();
        assert!(caps.tools.is_none());
        assert!(caps.resources.is_none());
        assert!(caps.prompts.is_none());
        assert!(caps.logging.is_none());
        assert!(caps.experimental.is_empty());
    }

    #[test]
    fn test_server_capabilities_builder() {
        let caps = ServerCapabilities::builder()
            .with_tools(ToolsCapability { list_changed: Some(true) })
            .with_resources(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(false),
            })
            .build();

        assert!(caps.tools.is_some());
        assert_eq!(caps.tools.as_ref().unwrap().list_changed, Some(true));
        assert!(caps.resources.is_some());
        assert_eq!(caps.resources.as_ref().unwrap().subscribe, Some(true));
    }

    #[test]
    fn test_tools_capability() {
        let cap = ToolsCapability {
            list_changed: Some(true),
        };
        assert_eq!(cap.list_changed, Some(true));
    }

    #[test]
    fn test_resources_capability() {
        let cap = ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(false),
        };
        assert_eq!(cap.subscribe, Some(true));
        assert_eq!(cap.list_changed, Some(false));
    }

    #[test]
    fn test_client_capabilities_default() {
        let caps = ClientCapabilities::default();
        assert!(caps.roots.is_none());
        assert!(caps.sampling.is_none());
        assert!(caps.experimental.is_empty());
    }

    #[test]
    fn test_serialization_server_capabilities() {
        let caps = ServerCapabilities::builder()
            .with_tools(ToolsCapability { list_changed: Some(true) })
            .build();

        let json = serde_json::to_value(&caps).unwrap();
        assert!(json["tools"]["listChanged"].as_bool().unwrap());
    }

    #[test]
    fn test_deserialization_server_capabilities() {
        let json = json!({
            "tools": {
                "listChanged": true
            },
            "resources": {
                "subscribe": true,
                "listChanged": false
            }
        });

        let caps: ServerCapabilities = serde_json::from_value(json).unwrap();
        assert!(caps.tools.is_some());
        assert_eq!(caps.tools.as_ref().unwrap().list_changed, Some(true));
        assert!(caps.resources.is_some());
        assert_eq!(caps.resources.as_ref().unwrap().subscribe, Some(true));
    }

    #[test]
    fn test_serialization_client_capabilities() {
        let caps = ClientCapabilities {
            roots: Some(RootsCapability {
                list_changed: Some(true),
            }),
            sampling: Some(SamplingCapability {}),
            experimental: HashMap::new(),
        };

        let json = serde_json::to_value(&caps).unwrap();
        assert!(json["roots"]["listChanged"].as_bool().unwrap());
        assert!(json["sampling"].is_object());
    }

    #[test]
    fn test_deserialization_client_capabilities() {
        let json = json!({
            "roots": {
                "listChanged": true
            }
        });

        let caps: ClientCapabilities = serde_json::from_value(json).unwrap();
        assert!(caps.roots.is_some());
        assert_eq!(caps.roots.as_ref().unwrap().list_changed, Some(true));
    }

    #[test]
    fn test_experimental_capabilities() {
        let mut caps = ServerCapabilities::default();
        caps.experimental.insert("custom".to_string(), json!({"enabled": true}));

        let json = serde_json::to_value(&caps).unwrap();
        assert!(json["custom"]["enabled"].as_bool().unwrap());

        let deserialized: ServerCapabilities = serde_json::from_value(json).unwrap();
        assert_eq!(
            deserialized.experimental.get("custom"),
            Some(&json!({"enabled": true}))
        );
    }

    #[test]
    fn test_omit_none_fields() {
        let caps = ServerCapabilities::default();
        let json = serde_json::to_value(&caps).unwrap();

        assert!(!json.get("tools").is_some());
        assert!(!json.get("resources").is_some());
        assert!(!json.get("prompts").is_some());
        assert!(!json.get("logging").is_some());
    }

    #[test]
    fn test_camel_case_serialization() {
        let cap = ToolsCapability {
            list_changed: Some(true),
        };
        let json = serde_json::to_value(&cap).unwrap();
        assert!(json.get("listChanged").is_some());
        assert!(!json.get("list_changed").is_some());
    }

    #[test]
    fn test_clone() {
        let original = ServerCapabilities::builder()
            .with_tools(ToolsCapability { list_changed: Some(true) })
            .build();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_debug() {
        let caps = ServerCapabilities::default();
        let debug_str = format!("{:?}", caps);
        assert!(debug_str.contains("ServerCapabilities"));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let caps = ServerCapabilities::builder()
            .with_tools(ToolsCapability { list_changed: Some(true) })
            .with_resources(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(false),
            })
            .with_prompts(PromptsCapability {
                list_changed: Some(true),
            })
            .with_logging(LoggingCapability {})
            .build();

        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: ServerCapabilities = serde_json::from_str(&json).unwrap();

        assert_eq!(caps, deserialized);
    }
}
