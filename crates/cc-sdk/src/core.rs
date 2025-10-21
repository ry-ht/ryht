//! Core types and traits for the Claude Code SDK.
//!
//! This module provides the foundational types, traits, and type-state markers
//! used throughout the SDK to ensure compile-time safety and correctness.
//!
//! # Type-State Pattern
//!
//! The SDK uses the type-state pattern to enforce valid state transitions at
//! compile time. For example, a client must discover a binary before connecting,
//! and must be connected before sending messages.
//!
//! # Examples
//!
//! ```rust
//! use cc_sdk::core::state::*;
//!
//! // Type-safe state progression
//! struct Client<S = NoBinary> {
//!     state: std::marker::PhantomData<S>,
//! }
//!
//! impl Client<NoBinary> {
//!     fn with_binary(self) -> Client<WithBinary> {
//!         Client { state: std::marker::PhantomData }
//!     }
//! }
//!
//! impl Client<WithBinary> {
//!     fn connect(self) -> Client<Connected> {
//!         Client { state: std::marker::PhantomData }
//!     }
//! }
//! ```

use std::fmt;
use std::path::PathBuf;

/// Type-state markers for compile-time safety.
///
/// These marker types are used with the type-state pattern to ensure that
/// operations are only performed when the client is in a valid state.
///
/// The typical progression is:
/// `NoBinary` → `WithBinary` → `Configured` → `Connected`
pub mod state {
    /// Initial state: no binary has been discovered.
    ///
    /// In this state, the client cannot connect or perform operations.
    /// The next valid state is `WithBinary` after binary discovery.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NoBinary;

    /// Binary has been discovered and validated.
    ///
    /// In this state, the client has a valid binary path but hasn't been
    /// configured yet. The next valid state is `Configured`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WithBinary;

    /// Client has been configured with options.
    ///
    /// In this state, the client has both a binary and configuration options,
    /// but hasn't established a connection. The next valid state is `Connected`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Configured;

    /// Client is connected and ready for operations.
    ///
    /// In this state, the client can send messages, manage sessions, and
    /// perform all operations. The client can transition to `Disconnected`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Connected;

    /// Client is disconnected.
    ///
    /// In this state, the client cannot perform operations that require a
    /// connection. The client can transition back to `Connected` via reconnection.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Disconnected;
}

/// Newtype wrapper for session IDs.
///
/// This provides type safety by preventing accidental mixing of session IDs
/// with other string types.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::core::SessionId;
///
/// let session_id = SessionId::new("abc123");
/// assert_eq!(session_id.as_str(), "abc123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new session ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random session ID.
    ///
    /// Uses UUID v4 format for globally unique identifiers.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the session ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Newtype wrapper for binary paths.
///
/// This provides type safety and convenience methods for working with
/// Claude CLI binary paths.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::core::BinaryPath;
/// use std::path::PathBuf;
///
/// let path = BinaryPath::new("/usr/local/bin/claude");
/// assert_eq!(path.as_path(), &PathBuf::from("/usr/local/bin/claude"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryPath(PathBuf);

impl BinaryPath {
    /// Create a new binary path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Get the path as a `PathBuf` reference.
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    /// Convert into the inner `PathBuf`.
    pub fn into_inner(self) -> PathBuf {
        self.0
    }

    /// Check if the binary exists.
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Check if the binary is executable.
    ///
    /// On Unix systems, checks the executable bit. On other systems,
    /// checks if the file exists.
    pub fn is_executable(&self) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            self.0
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }

        #[cfg(not(unix))]
        {
            self.exists()
        }
    }
}

impl fmt::Display for BinaryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl From<PathBuf> for BinaryPath {
    fn from(p: PathBuf) -> Self {
        Self(p)
    }
}

impl From<&str> for BinaryPath {
    fn from(s: &str) -> Self {
        Self(PathBuf::from(s))
    }
}

impl AsRef<PathBuf> for BinaryPath {
    fn as_ref(&self) -> &PathBuf {
        &self.0
    }
}

impl AsRef<std::path::Path> for BinaryPath {
    fn as_ref(&self) -> &std::path::Path {
        self.0.as_ref()
    }
}

/// Newtype wrapper for model IDs.
///
/// This provides type safety for Claude model identifiers.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::core::ModelId;
///
/// let model = ModelId::new("claude-sonnet-4-5-20250929");
/// assert_eq!(model.as_str(), "claude-sonnet-4-5-20250929");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelId(String);

impl ModelId {
    /// Create a new model ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the model ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Check if this is a Sonnet model.
    pub fn is_sonnet(&self) -> bool {
        self.0.contains("sonnet")
    }

    /// Check if this is an Opus model.
    pub fn is_opus(&self) -> bool {
        self.0.contains("opus")
    }

    /// Check if this is a Haiku model.
    pub fn is_haiku(&self) -> bool {
        self.0.contains("haiku")
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ModelId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Common well-known model IDs.
pub mod models {
    use super::ModelId;

    /// Claude Sonnet 4.5 (latest)
    pub const SONNET_4_5: ModelId = ModelId(String::new());

    /// Helper to create model ID for Sonnet 4.5
    pub fn sonnet_4_5() -> ModelId {
        ModelId::new("claude-sonnet-4-5-20250929")
    }

    /// Helper to create model ID for Sonnet 3.5
    pub fn sonnet_3_5() -> ModelId {
        ModelId::new("claude-3-5-sonnet-20241022")
    }

    /// Helper to create model ID for Opus 3
    pub fn opus_3() -> ModelId {
        ModelId::new("claude-3-opus-20240229")
    }

    /// Helper to create model ID for Haiku 3.5
    pub fn haiku_3_5() -> ModelId {
        ModelId::new("claude-3-5-haiku-20241022")
    }
}

/// Version information for the Claude CLI binary.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::core::Version;
///
/// let version = Version::parse("0.2.5").unwrap();
/// assert_eq!(version.major, 0);
/// assert_eq!(version.minor, 2);
/// assert_eq!(version.patch, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
    /// Pre-release identifier (e.g., "alpha", "beta")
    pub pre: Option<String>,
}

impl Version {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    /// Create a new version with pre-release identifier.
    pub fn with_pre(major: u32, minor: u32, patch: u32, pre: impl Into<String>) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: Some(pre.into()),
        }
    }

    /// Parse a version string.
    ///
    /// Supports formats like "1.2.3", "1.2.3-alpha", "v1.2.3".
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim().trim_start_matches('v');
        let (version_part, pre) = if let Some((v, p)) = s.split_once('-') {
            (v, Some(p.to_string()))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// Check if this version satisfies a requirement.
    ///
    /// Simple version checking: supports ">=", ">", "=", "<", "<=".
    pub fn satisfies(&self, requirement: &str) -> bool {
        let requirement = requirement.trim();

        if let Some(req) = requirement.strip_prefix(">=") {
            if let Ok(req_ver) = Self::parse(req.trim()) {
                return self >= &req_ver;
            }
        } else if let Some(req) = requirement.strip_prefix('>') {
            if let Ok(req_ver) = Self::parse(req.trim()) {
                return self > &req_ver;
            }
        } else if let Some(req) = requirement.strip_prefix("<=") {
            if let Ok(req_ver) = Self::parse(req.trim()) {
                return self <= &req_ver;
            }
        } else if let Some(req) = requirement.strip_prefix('<') {
            if let Ok(req_ver) = Self::parse(req.trim()) {
                return self < &req_ver;
            }
        } else if let Some(req) = requirement.strip_prefix('=') {
            if let Ok(req_ver) = Self::parse(req.trim()) {
                return self == &req_ver;
            }
        } else if let Ok(req_ver) = Self::parse(requirement) {
            return self == &req_ver;
        }

        false
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id() {
        let id = SessionId::new("test-123");
        assert_eq!(id.as_str(), "test-123");
        assert_eq!(id.to_string(), "test-123");

        let id2 = SessionId::from("test-456");
        assert_eq!(id2.as_str(), "test-456");

        let generated = SessionId::generate();
        assert!(!generated.as_str().is_empty());
    }

    #[test]
    fn test_binary_path() {
        let path = BinaryPath::new("/usr/local/bin/claude");
        assert_eq!(path.as_path(), &PathBuf::from("/usr/local/bin/claude"));
        assert_eq!(path.to_string(), "/usr/local/bin/claude");
    }

    #[test]
    fn test_model_id() {
        let model = ModelId::new("claude-sonnet-4-5-20250929");
        assert_eq!(model.as_str(), "claude-sonnet-4-5-20250929");
        assert!(model.is_sonnet());
        assert!(!model.is_opus());
        assert!(!model.is_haiku());

        let opus = ModelId::new("claude-3-opus-20240229");
        assert!(opus.is_opus());
        assert!(!opus.is_sonnet());
    }

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre, None);

        let v = Version::parse("v2.0.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 1);

        let v = Version::parse("1.0.0-alpha").unwrap();
        assert_eq!(v.pre, Some("alpha".to_string()));

        assert!(Version::parse("invalid").is_err());
        assert!(Version::parse("1.2").is_err());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 0, 1);
        let v3 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
        assert!(v2 == Version::new(1, 0, 1));
    }

    #[test]
    fn test_version_satisfies() {
        let v = Version::new(1, 2, 3);

        assert!(v.satisfies(">=1.0.0"));
        assert!(v.satisfies(">=1.2.3"));
        assert!(!v.satisfies(">=2.0.0"));

        assert!(v.satisfies(">1.0.0"));
        assert!(!v.satisfies(">1.2.3"));

        assert!(v.satisfies("=1.2.3"));
        assert!(v.satisfies("1.2.3"));
        assert!(!v.satisfies("=1.2.4"));

        assert!(v.satisfies("<=2.0.0"));
        assert!(v.satisfies("<2.0.0"));
        assert!(!v.satisfies("<1.0.0"));
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");

        let v = Version::with_pre(1, 0, 0, "beta");
        assert_eq!(v.to_string(), "1.0.0-beta");
    }

    #[test]
    fn test_state_markers() {
        use state::*;

        // Just ensure they compile and can be used
        let _no_binary: NoBinary = NoBinary;
        let _with_binary: WithBinary = WithBinary;
        let _configured: Configured = Configured;
        let _connected: Connected = Connected;
        let _disconnected: Disconnected = Disconnected;
    }

    #[test]
    fn test_model_helpers() {
        let sonnet = models::sonnet_4_5();
        assert!(sonnet.is_sonnet());

        let opus = models::opus_3();
        assert!(opus.is_opus());

        let haiku = models::haiku_3_5();
        assert!(haiku.is_haiku());
    }
}
