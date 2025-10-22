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
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random session ID.
    ///
    /// Uses UUID v4 format for globally unique identifiers.
    #[inline]
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Get the session ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Validate that the session ID is non-empty.
    ///
    /// Returns `true` if the session ID is valid (non-empty).
    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
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

impl std::str::FromStr for SessionId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BinaryPath(PathBuf);

impl BinaryPath {
    /// Create a new binary path.
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Get the path as a `PathBuf` reference.
    #[inline]
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }

    /// Convert into the inner `PathBuf`.
    #[inline]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }

    /// Check if the binary exists.
    #[inline]
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

impl std::str::FromStr for BinaryPath {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(PathBuf::from(s)))
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ModelId(String);

impl ModelId {
    /// Create a new model ID.
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the model ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Check if this is a Sonnet model.
    #[inline]
    pub fn is_sonnet(&self) -> bool {
        self.0.contains("sonnet")
    }

    /// Check if this is an Opus model.
    #[inline]
    pub fn is_opus(&self) -> bool {
        self.0.contains("opus")
    }

    /// Check if this is a Haiku model.
    #[inline]
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

impl std::str::FromStr for ModelId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
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
/// This is a re-export of [`crate::binary::Version`] for API compatibility.
/// The binary module provides the canonical semantic versioning implementation
/// with full support for major.minor.patch, prerelease, and build metadata.
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
///
/// # Field Name Compatibility
///
/// The binary::Version uses `prerelease` as the field name (following semver spec),
/// not `pre`. Use the field accessor directly:
///
/// ```rust
/// use cc_sdk::core::Version;
///
/// let v = Version::parse("1.0.0-beta").unwrap();
/// assert_eq!(v.prerelease, Some("beta".to_string()));
/// ```
pub use crate::binary::Version;

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
        assert_eq!(v.prerelease, None);

        let v = Version::parse("2.0.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 1);

        let v = Version::parse("1.0.0-alpha").unwrap();
        assert_eq!(v.prerelease, Some("alpha".to_string()));

        assert!(Version::parse("invalid").is_none());
        assert!(Version::parse("1.2").is_none());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("1.0.1").unwrap();
        let v3 = Version::parse("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
        assert!(v2 == Version::parse("1.0.1").unwrap());
    }

    #[test]
    fn test_version_display() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.to_string(), "1.2.3");

        let v = Version::parse("1.0.0-beta").unwrap();
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

    // Property-based tests
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // SessionId property tests
        proptest! {
            #[test]
            fn session_id_roundtrip_string(s in "\\PC+") {
                let id = SessionId::new(s.clone());
                prop_assert_eq!(id.as_str(), s.as_str());
                prop_assert_eq!(id.into_inner(), s);
            }

            #[test]
            fn session_id_serialization_roundtrip(s in "\\PC+") {
                let id = SessionId::new(s);
                let json = serde_json::to_string(&id).unwrap();
                let deserialized: SessionId = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(id, deserialized);
            }

            #[test]
            fn session_id_validity(s in "\\PC*") {
                let id = SessionId::new(s.clone());
                prop_assert_eq!(id.is_valid(), !s.is_empty());
            }

            #[test]
            fn session_id_from_str_always_succeeds(s in "\\PC*") {
                let result: Result<SessionId, _> = s.parse();
                prop_assert!(result.is_ok());
            }

            // ModelId property tests
            #[test]
            fn model_id_roundtrip(s in "\\PC+") {
                let id = ModelId::new(s.clone());
                prop_assert_eq!(id.as_str(), s.as_str());
                prop_assert_eq!(id.into_inner(), s);
            }

            #[test]
            fn model_id_serialization_roundtrip(s in "\\PC+") {
                let id = ModelId::new(s);
                let json = serde_json::to_string(&id).unwrap();
                let deserialized: ModelId = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(id, deserialized);
            }

            #[test]
            fn model_id_classification_consistent(s in "\\PC+") {
                let id = ModelId::new(s.clone());
                let is_sonnet = id.is_sonnet();
                let is_opus = id.is_opus();
                let is_haiku = id.is_haiku();

                // Verify consistency with actual string content
                prop_assert_eq!(is_sonnet, s.contains("sonnet"));
                prop_assert_eq!(is_opus, s.contains("opus"));
                prop_assert_eq!(is_haiku, s.contains("haiku"));
            }

            // BinaryPath property tests
            #[test]
            fn binary_path_roundtrip(s in "[/a-zA-Z0-9._-]+") {
                let path = BinaryPath::new(s.clone());
                prop_assert_eq!(path.as_path(), &PathBuf::from(s.clone()));
                prop_assert_eq!(path.into_inner(), PathBuf::from(s));
            }

            #[test]
            fn binary_path_serialization_roundtrip(s in "[/a-zA-Z0-9._-]+") {
                let path = BinaryPath::new(s);
                let json = serde_json::to_string(&path).unwrap();
                let deserialized: BinaryPath = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(path, deserialized);
            }

            // Version property tests - delegated to binary::version module
            // See binary/version.rs for comprehensive property-based tests
            #[test]
            fn version_display_parse_roundtrip(
                major in 0u32..100,
                minor in 0u32..100,
                patch in 0u32..100
            ) {
                let version_str = format!("{}.{}.{}", major, minor, patch);
                if let Some(v) = Version::parse(&version_str) {
                    let s = v.to_string();
                    let parsed = Version::parse(&s).unwrap();
                    prop_assert_eq!(v, parsed);
                }
            }

            #[test]
            fn version_ordering_reflexive(major in 0u32..100, minor in 0u32..100, patch in 0u32..100) {
                let version_str = format!("{}.{}.{}", major, minor, patch);
                if let Some(v) = Version::parse(&version_str) {
                    prop_assert_eq!(v.cmp(&v), std::cmp::Ordering::Equal);
                    prop_assert!(v == v);
                }
            }

            #[test]
            fn version_major_dominates(
                major1 in 0u32..10,
                major2 in 0u32..10,
                minor in 0u32..100,
                patch in 0u32..100
            ) {
                if major1 != major2 {
                    let v1_str = format!("{}.{}.{}", major1, minor, patch);
                    let v2_str = format!("{}.{}.{}", major2, minor, patch);
                    if let (Some(v1), Some(v2)) = (Version::parse(&v1_str), Version::parse(&v2_str)) {
                        if major1 < major2 {
                            prop_assert!(v1 < v2);
                        } else {
                            prop_assert!(v1 > v2);
                        }
                    }
                }
            }
        }
    }
}
