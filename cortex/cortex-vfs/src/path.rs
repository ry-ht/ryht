//! Virtual path implementation that's independent of physical location.
//!
//! Virtual paths are always relative to the repository root, not tied to any
//! physical filesystem location. This enables path-agnostic operations where
//! the same virtual path can be materialized to different physical locations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// A virtual path that's independent of physical location.
///
/// Virtual paths are always stored as relative paths from the repository root.
/// They never contain absolute paths or physical filesystem references.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VirtualPath {
    segments: Vec<String>,
    is_absolute: bool,
}

impl VirtualPath {
    /// Create a new virtual path from a string.
    ///
    /// The path is automatically normalized to be relative to the repository root.
    /// Leading slashes are removed, and the path is split into segments.
    pub fn new(path: &str) -> Result<Self, VirtualPathError> {
        // Normalize path
        let path = path.trim();
        if path.is_empty() {
            return Ok(Self::root());
        }

        // Remove leading slash to make relative
        let path = path.trim_start_matches('/');

        // Split into segments and filter out empty and "." segments
        let segments: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .map(|s| s.to_string())
            .collect();

        // Check for invalid characters
        for segment in &segments {
            if segment.contains('\0') {
                return Err(VirtualPathError::InvalidCharacter('\0'));
            }
        }

        Ok(Self {
            segments,
            is_absolute: false,
        })
    }

    /// Create a root path.
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
            is_absolute: false,
        }
    }

    /// Create a virtual path from a physical path, making it relative to the given base.
    pub fn from_physical(physical: &Path, base: &Path) -> Result<Self, VirtualPathError> {
        let relative = physical
            .strip_prefix(base)
            .map_err(|_| VirtualPathError::NotRelativeTo(base.to_path_buf()))?;

        let path_str = relative
            .to_str()
            .ok_or_else(|| VirtualPathError::InvalidUtf8)?;

        Self::new(path_str)
    }

    /// Join this path with another segment or path.
    pub fn join(&self, other: &str) -> Result<Self, VirtualPathError> {
        let mut segments = self.segments.clone();

        let other = other.trim_start_matches('/');
        for segment in other.split('/') {
            if segment.is_empty() || segment == "." {
                continue;
            }
            if segment == ".." {
                segments.pop();
            } else {
                if segment.contains('\0') {
                    return Err(VirtualPathError::InvalidCharacter('\0'));
                }
                segments.push(segment.to_string());
            }
        }

        Ok(Self {
            segments,
            is_absolute: false,
        })
    }

    /// Get the parent path, if any.
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            return None;
        }

        let mut segments = self.segments.clone();
        segments.pop();

        Some(Self {
            segments,
            is_absolute: false,
        })
    }

    /// Get the file name (last segment), if any.
    pub fn file_name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Get the extension of the file, if any.
    pub fn extension(&self) -> Option<&str> {
        self.file_name().and_then(|name| {
            let parts: Vec<&str> = name.rsplitn(2, '.').collect();
            if parts.len() == 2 && !parts[1].is_empty() {
                Some(parts[0])
            } else {
                None
            }
        })
    }

    /// Check if this path starts with another path.
    pub fn starts_with(&self, base: &VirtualPath) -> bool {
        if base.segments.len() > self.segments.len() {
            return false;
        }

        self.segments
            .iter()
            .zip(base.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Check if this is the root path.
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the number of segments in this path.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Check if this path is empty (root).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the segments of this path.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Convert to a string representation with leading slash.
    pub fn to_string_with_slash(&self) -> String {
        if self.segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.segments.join("/"))
        }
    }

    /// Convert to a PathBuf relative to a physical base path.
    pub fn to_physical(&self, base: &Path) -> PathBuf {
        let mut physical = base.to_path_buf();
        for segment in &self.segments {
            physical.push(segment);
        }
        physical
    }

    /// Normalize the path by resolving ".." and "." components.
    pub fn normalize(self) -> Self {
        let mut normalized = Vec::new();

        for segment in self.segments {
            if segment == ".." {
                normalized.pop();
            } else if segment != "." {
                normalized.push(segment);
            }
        }

        Self {
            segments: normalized,
            is_absolute: false,
        }
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            write!(f, "/")
        } else {
            write!(f, "{}", self.segments.join("/"))
        }
    }
}

impl From<String> for VirtualPath {
    fn from(s: String) -> Self {
        Self::new(&s).unwrap_or_else(|_| Self::root())
    }
}

impl From<&str> for VirtualPath {
    fn from(s: &str) -> Self {
        Self::new(s).unwrap_or_else(|_| Self::root())
    }
}

impl AsRef<VirtualPath> for VirtualPath {
    fn as_ref(&self) -> &VirtualPath {
        self
    }
}

/// Errors that can occur when working with virtual paths.
#[derive(Debug, Clone, thiserror::Error)]
pub enum VirtualPathError {
    #[error("Invalid character in path: {0}")]
    InvalidCharacter(char),

    #[error("Path is not relative to base: {0}")]
    NotRelativeTo(PathBuf),

    #[error("Path contains invalid UTF-8")]
    InvalidUtf8,

    #[error("Path escapes repository root")]
    EscapesRoot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_path() {
        let path = VirtualPath::new("src/main.rs").unwrap();
        assert_eq!(path.segments(), &["src", "main.rs"]);
        assert_eq!(path.to_string(), "src/main.rs");
    }

    #[test]
    fn test_path_with_leading_slash() {
        let path = VirtualPath::new("/src/main.rs").unwrap();
        assert_eq!(path.segments(), &["src", "main.rs"]);
        assert!(!path.is_absolute); // Always relative
    }

    #[test]
    fn test_root_path() {
        let path = VirtualPath::root();
        assert!(path.is_root());
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_join() {
        let path = VirtualPath::new("src").unwrap();
        let joined = path.join("main.rs").unwrap();
        assert_eq!(joined.segments(), &["src", "main.rs"]);
    }

    #[test]
    fn test_parent() {
        let path = VirtualPath::new("src/lib/main.rs").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.segments(), &["src", "lib"]);

        let root = VirtualPath::root();
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_file_name() {
        let path = VirtualPath::new("src/main.rs").unwrap();
        assert_eq!(path.file_name(), Some("main.rs"));

        let root = VirtualPath::root();
        assert_eq!(root.file_name(), None);
    }

    #[test]
    fn test_extension() {
        let path = VirtualPath::new("src/main.rs").unwrap();
        assert_eq!(path.extension(), Some("rs"));

        let no_ext = VirtualPath::new("Makefile").unwrap();
        assert_eq!(no_ext.extension(), None);
    }

    #[test]
    fn test_starts_with() {
        let path = VirtualPath::new("src/lib/main.rs").unwrap();
        let base = VirtualPath::new("src/lib").unwrap();
        assert!(path.starts_with(&base));

        let other = VirtualPath::new("tests").unwrap();
        assert!(!path.starts_with(&other));
    }

    #[test]
    fn test_normalize() {
        let path = VirtualPath::new("src/../lib/./main.rs").unwrap();
        let normalized = path.normalize();
        assert_eq!(normalized.segments(), &["lib", "main.rs"]);
    }

    #[test]
    fn test_to_physical() {
        let vpath = VirtualPath::new("src/main.rs").unwrap();
        let base = Path::new("/home/user/project");
        let physical = vpath.to_physical(base);
        assert_eq!(physical, PathBuf::from("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_from_physical() {
        let physical = Path::new("/home/user/project/src/main.rs");
        let base = Path::new("/home/user/project");
        let vpath = VirtualPath::from_physical(physical, base).unwrap();
        assert_eq!(vpath.segments(), &["src", "main.rs"]);
    }

    #[test]
    fn test_display() {
        let path = VirtualPath::new("src/main.rs").unwrap();
        assert_eq!(format!("{}", path), "src/main.rs");

        let root = VirtualPath::root();
        assert_eq!(format!("{}", root), "/");
    }
}
