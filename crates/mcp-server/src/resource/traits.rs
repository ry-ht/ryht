//! Resource trait definition.
//!
//! This module defines the core [`Resource`] trait that all MCP resources must implement.

use async_trait::async_trait;

use crate::error::ResourceError;
use super::{ResourceContent, ResourceContext};
use super::uri::matches_pattern;

/// A resource that can be read by MCP clients.
///
/// Resources represent URI-addressable content such as files, database records,
/// API responses, or any other data that can be retrieved via a URI pattern.
///
/// # URI Patterns
///
/// Resources use URI patterns to match incoming requests. Patterns support:
/// - **Exact matches**: `"app://config"`
/// - **Wildcards**: `"db://users/*"` matches `"db://users/123"`
/// - **Glob patterns**: `"file:///*.txt"` matches all `.txt` files
///
/// # Examples
///
/// ## Simple Static Resource
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// struct HelloResource;
///
/// #[async_trait]
/// impl Resource for HelloResource {
///     fn uri_pattern(&self) -> &str {
///         "hello://world"
///     }
///
///     fn name(&self) -> Option<&str> {
///         Some("Hello World")
///     }
///
///     fn description(&self) -> Option<&str> {
///         Some("A simple greeting resource")
///     }
///
///     fn mime_type(&self) -> Option<&str> {
///         Some("text/plain")
///     }
///
///     async fn read(
///         &self,
///         _uri: &str,
///         _context: &ResourceContext,
///     ) -> Result<ResourceContent, ResourceError> {
///         Ok(ResourceContent::text("Hello, World!", "text/plain"))
///     }
/// }
/// ```
///
/// ## Dynamic Resource with Pattern Matching
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// struct DocumentResource;
///
/// #[async_trait]
/// impl Resource for DocumentResource {
///     fn uri_pattern(&self) -> &str {
///         "docs://*"
///     }
///
///     fn name(&self) -> Option<&str> {
///         Some("Documentation")
///     }
///
///     async fn read(
///         &self,
///         uri: &str,
///         _context: &ResourceContext,
///     ) -> Result<ResourceContent, ResourceError> {
///         let doc_id = uri.strip_prefix("docs://")
///             .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;
///
///         // Fetch document content (simulated)
///         let content = format!("# Documentation for {}\n\nContent here...", doc_id);
///
///         Ok(ResourceContent::text(content, "text/markdown"))
///     }
/// }
/// ```
///
/// ## Binary Resource
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// struct ImageResource;
///
/// #[async_trait]
/// impl Resource for ImageResource {
///     fn uri_pattern(&self) -> &str {
///         "images://*.png"
///     }
///
///     fn mime_type(&self) -> Option<&str> {
///         Some("image/png")
///     }
///
///     async fn read(
///         &self,
///         uri: &str,
///         _context: &ResourceContext,
///     ) -> Result<ResourceContent, ResourceError> {
///         // Load image data (simulated)
///         let image_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
///
///         Ok(ResourceContent::blob(image_data, "image/png"))
///     }
/// }
/// ```
#[async_trait]
pub trait Resource: Send + Sync {
    /// Returns the URI pattern that this resource matches.
    ///
    /// The pattern can include wildcards (`*`) and glob patterns for flexible matching.
    ///
    /// # Examples
    ///
    /// - `"app://config"` - Exact match
    /// - `"db://users/*"` - Matches any user ID
    /// - `"file:///*.txt"` - Matches all .txt files
    fn uri_pattern(&self) -> &str;

    /// Returns an optional human-readable name for this resource.
    ///
    /// This name may be displayed to users or used for documentation purposes.
    ///
    /// # Default
    ///
    /// Returns `None` by default.
    fn name(&self) -> Option<&str> {
        None
    }

    /// Returns an optional description of this resource.
    ///
    /// The description should explain what this resource provides and how it can be used.
    ///
    /// # Default
    ///
    /// Returns `None` by default.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the MIME type of the content returned by this resource.
    ///
    /// Common MIME types:
    /// - `"text/plain"` - Plain text
    /// - `"text/markdown"` - Markdown
    /// - `"application/json"` - JSON
    /// - `"image/png"` - PNG image
    /// - `"application/octet-stream"` - Binary data
    ///
    /// # Default
    ///
    /// Returns `None` by default, which indicates the MIME type should be
    /// determined from the content itself.
    fn mime_type(&self) -> Option<&str> {
        None
    }

    /// Checks if the given URI matches this resource's pattern.
    ///
    /// This method uses glob-style pattern matching to determine if a URI
    /// should be handled by this resource.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI to check against this resource's pattern
    ///
    /// # Returns
    ///
    /// `true` if the URI matches this resource's pattern, `false` otherwise.
    ///
    /// # Default Implementation
    ///
    /// The default implementation uses glob pattern matching via [`matches_pattern`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::Resource;
    /// # use mcp_server::resource::{ResourceContent, ResourceContext};
    /// # use mcp_server::error::ResourceError;
    /// # use async_trait::async_trait;
    /// #
    /// # struct UserResource;
    /// # #[async_trait]
    /// # impl Resource for UserResource {
    /// #     fn uri_pattern(&self) -> &str { "db://users/*" }
    /// #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    /// #         Ok(ResourceContent::text("user", "application/json"))
    /// #     }
    /// # }
    ///
    /// let resource = UserResource;
    ///
    /// assert!(resource.matches("db://users/123"));
    /// assert!(resource.matches("db://users/alice"));
    /// assert!(!resource.matches("db://posts/123"));
    /// ```
    fn matches(&self, uri: &str) -> bool {
        matches_pattern(self.uri_pattern(), uri)
    }

    /// Reads the resource content for the given URI.
    ///
    /// This method is called when a client requests to read a resource. It should
    /// return the resource content as either text or binary data.
    ///
    /// # Arguments
    ///
    /// * `uri` - The full URI being requested (guaranteed to match this resource's pattern)
    /// * `context` - Additional context about the request (session info, etc.)
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - `Ok(ResourceContent)` - The resource content (text or blob)
    /// - `Err(ResourceError)` - An error if the resource cannot be read
    ///
    /// # Errors
    ///
    /// Common errors include:
    /// - [`ResourceError::NotFound`] - Resource doesn't exist
    /// - [`ResourceError::InvalidUri`] - URI is malformed
    /// - [`ResourceError::ReadFailed`] - Failed to read content
    /// - [`ResourceError::Internal`] - Internal error occurred
    ///
    /// # Examples
    ///
    /// See trait-level documentation for complete examples.
    async fn read(
        &self,
        uri: &str,
        context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestResource {
        pattern: &'static str,
        name: Option<&'static str>,
    }

    #[async_trait]
    impl Resource for TestResource {
        fn uri_pattern(&self) -> &str {
            self.pattern
        }

        fn name(&self) -> Option<&str> {
            self.name
        }

        async fn read(
            &self,
            _uri: &str,
            _context: &ResourceContext,
        ) -> Result<ResourceContent, ResourceError> {
            Ok(ResourceContent::text("test content", "text/plain"))
        }
    }

    #[test]
    fn test_resource_uri_pattern() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        assert_eq!(resource.uri_pattern(), "test://resource");
    }

    #[test]
    fn test_resource_name() {
        let resource = TestResource {
            pattern: "test://resource",
            name: Some("Test Resource"),
        };
        assert_eq!(resource.name(), Some("Test Resource"));
    }

    #[test]
    fn test_resource_name_default() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        assert_eq!(resource.name(), None);
    }

    #[test]
    fn test_resource_description_default() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        assert_eq!(resource.description(), None);
    }

    #[test]
    fn test_resource_mime_type_default() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        assert_eq!(resource.mime_type(), None);
    }

    #[test]
    fn test_resource_matches_exact() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        assert!(resource.matches("test://resource"));
        assert!(!resource.matches("test://other"));
    }

    #[test]
    fn test_resource_matches_wildcard() {
        let resource = TestResource {
            pattern: "test://users/*",
            name: None,
        };
        assert!(resource.matches("test://users/123"));
        assert!(resource.matches("test://users/alice"));
        assert!(!resource.matches("test://posts/123"));
    }

    #[tokio::test]
    async fn test_resource_read() {
        let resource = TestResource {
            pattern: "test://resource",
            name: None,
        };
        let context = ResourceContext::default();
        let content = resource.read("test://resource", &context).await.unwrap();

        match content {
            ResourceContent::Text { text, .. } => assert_eq!(text, "test content"),
            _ => panic!("Expected text content"),
        }
    }
}
