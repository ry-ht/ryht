//! Server configuration types.
//!
//! This module provides configuration structures for MCP servers.

/// Configuration for an MCP server.
///
///  Contains server metadata including name, version, and protocol version.
/// These are used during the initialization handshake with clients.
///
/// # Examples
///
/// ```
/// use mcp_server::server::ServerConfig;
///
/// let config = ServerConfig::new("my-server", "1.0.0");
/// assert_eq!(config.name(), "my-server");
/// assert_eq!(config.version(), "1.0.0");
/// assert_eq!(config.protocol_version(), "2025-03-26");
/// ```
///
/// ## With Custom Protocol Version
///
/// ```
/// use mcp_server::server::ServerConfig;
///
/// let config = ServerConfig::builder()
///     .name("my-server")
///     .version("2.0.0")
///     .protocol_version("2025-03-26")
///     .build();
///
/// assert_eq!(config.protocol_version(), "2025-03-26");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Server name
    pub(crate) name: String,

    /// Server version
    pub(crate) version: String,

    /// MCP protocol version (defaults to "2025-03-26")
    pub(crate) protocol_version: String,
}

impl ServerConfig {
    /// Creates a new server configuration with the default protocol version.
    ///
    /// # Arguments
    ///
    /// * `name` - The server name
    /// * `version` - The server version
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::new("my-server", "1.0.0");
    /// assert_eq!(config.name(), "my-server");
    /// ```
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            protocol_version: "2025-03-26".to_string(),
        }
    }

    /// Creates a new builder for constructing a `ServerConfig`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::new()
    }

    /// Returns the server name.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::new("test-server", "1.0.0");
    /// assert_eq!(config.name(), "test-server");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the server version.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::new("test-server", "2.1.3");
    /// assert_eq!(config.version(), "2.1.3");
    /// ```
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the MCP protocol version.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::new("test-server", "1.0.0");
    /// assert_eq!(config.protocol_version(), "2025-03-26");
    /// ```
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }
}

/// Builder for constructing a `ServerConfig`.
///
/// Provides a fluent API for building server configurations with optional
/// protocol version customization.
///
/// # Examples
///
/// ```
/// use mcp_server::server::ServerConfig;
///
/// let config = ServerConfig::builder()
///     .name("my-server")
///     .version("1.0.0")
///     .protocol_version("2025-03-26")
///     .build();
///
/// assert_eq!(config.name(), "my-server");
/// assert_eq!(config.version(), "1.0.0");
/// assert_eq!(config.protocol_version(), "2025-03-26");
/// ```
#[derive(Debug, Default)]
pub struct ServerConfigBuilder {
    name: Option<String>,
    version: Option<String>,
    protocol_version: Option<String>,
}

impl ServerConfigBuilder {
    /// Creates a new `ServerConfigBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the server name.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the server version.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .name("my-server")
    ///     .version("2.0.0")
    ///     .build();
    /// ```
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the MCP protocol version.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .protocol_version("2025-03-26")
    ///     .build();
    /// ```
    pub fn protocol_version(mut self, protocol_version: impl Into<String>) -> Self {
        self.protocol_version = Some(protocol_version.into());
        self
    }

    /// Builds the `ServerConfig`.
    ///
    /// # Panics
    ///
    /// Panics if name or version is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerConfig;
    ///
    /// let config = ServerConfig::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    pub fn build(self) -> ServerConfig {
        ServerConfig {
            name: self.name.expect("Server name is required"),
            version: self.version.expect("Server version is required"),
            protocol_version: self.protocol_version.unwrap_or_else(|| "2025-03-26".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_new() {
        let config = ServerConfig::new("test-server", "1.0.0");
        assert_eq!(config.name(), "test-server");
        assert_eq!(config.version(), "1.0.0");
        assert_eq!(config.protocol_version(), "2025-03-26");
    }

    #[test]
    fn test_server_config_builder_basic() {
        let config = ServerConfig::builder()
            .name("builder-server")
            .version("2.0.0")
            .build();

        assert_eq!(config.name(), "builder-server");
        assert_eq!(config.version(), "2.0.0");
        assert_eq!(config.protocol_version(), "2025-03-26");
    }

    #[test]
    fn test_server_config_builder_custom_protocol() {
        let config = ServerConfig::builder()
            .name("custom-server")
            .version("1.5.0")
            .protocol_version("custom-version")
            .build();

        assert_eq!(config.name(), "custom-server");
        assert_eq!(config.version(), "1.5.0");
        assert_eq!(config.protocol_version(), "custom-version");
    }

    #[test]
    #[should_panic(expected = "Server name is required")]
    fn test_server_config_builder_missing_name() {
        ServerConfig::builder().version("1.0.0").build();
    }

    #[test]
    #[should_panic(expected = "Server version is required")]
    fn test_server_config_builder_missing_version() {
        ServerConfig::builder().name("test").build();
    }

    #[test]
    fn test_server_config_clone() {
        let config1 = ServerConfig::new("server", "1.0.0");
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_server_config_equality() {
        let config1 = ServerConfig::new("server", "1.0.0");
        let config2 = ServerConfig::new("server", "1.0.0");
        assert_eq!(config1, config2);

        let config3 = ServerConfig::new("other", "1.0.0");
        assert_ne!(config1, config3);
    }

    #[test]
    fn test_server_config_debug() {
        let config = ServerConfig::new("test", "1.0.0");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ServerConfig"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_config_builder_fluent_api() {
        let config = ServerConfig::builder()
            .name("fluent")
            .version("1.0.0")
            .protocol_version("2025-03-26")
            .build();

        assert_eq!(config.name(), "fluent");
        assert_eq!(config.version(), "1.0.0");
        assert_eq!(config.protocol_version(), "2025-03-26");
    }

    #[test]
    fn test_config_getters() {
        let config = ServerConfig::new("getter-test", "3.2.1");

        // Test all getters
        assert_eq!(config.name(), "getter-test");
        assert_eq!(config.version(), "3.2.1");
        assert_eq!(config.protocol_version(), "2025-03-26");

        // Verify they return &str
        let _name: &str = config.name();
        let _version: &str = config.version();
        let _protocol: &str = config.protocol_version();
    }
}
