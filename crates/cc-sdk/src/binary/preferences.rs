//! User preference persistence for binary selection.
//!
//! This module provides optional preference persistence without requiring
//! specific database dependencies. Applications can implement their own
//! storage backends by implementing the `PreferenceStore` trait.
//!
//! # Examples
//!
//! ## Using File-Based Storage
//!
//! ```no_run
//! use cc_sdk::binary::preferences::{FilePreferenceStore, PreferenceStore};
//! use std::path::PathBuf;
//!
//! let store = FilePreferenceStore::new(PathBuf::from("/tmp/claude-prefs.json"));
//!
//! // Store a preference
//! store.set_preferred_path("/usr/local/bin/claude").unwrap();
//!
//! // Retrieve the preference
//! if let Some(path) = store.get_preferred_path().unwrap() {
//!     println!("Preferred Claude: {}", path);
//! }
//! ```
//!
//! ## Custom Storage Backend
//!
//! ```
//! use cc_sdk::binary::preferences::PreferenceStore;
//! use std::collections::HashMap;
//! use std::sync::{Arc, Mutex};
//!
//! struct MemoryStore {
//!     data: Arc<Mutex<HashMap<String, String>>>,
//! }
//!
//! impl PreferenceStore for MemoryStore {
//!     fn get_preferred_path(&self) -> Result<Option<String>, String> {
//!         Ok(self.data.lock().unwrap().get("path").cloned())
//!     }
//!
//!     fn set_preferred_path(&self, path: &str) -> Result<(), String> {
//!         self.data.lock().unwrap().insert("path".to_string(), path.to_string());
//!         Ok(())
//!     }
//!
//!     fn get_installation_preference(&self) -> Result<Option<String>, String> {
//!         Ok(self.data.lock().unwrap().get("preference").cloned())
//!     }
//!
//!     fn set_installation_preference(&self, preference: &str) -> Result<(), String> {
//!         self.data.lock().unwrap().insert("preference".to_string(), preference.to_string());
//!         Ok(())
//!     }
//!
//!     fn clear(&self) -> Result<(), String> {
//!         self.data.lock().unwrap().clear();
//!         Ok(())
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Trait for storing and retrieving user preferences for binary selection.
///
/// Applications can implement this trait to integrate with their own storage
/// backends (database, file system, key-value store, etc.).
pub trait PreferenceStore: Send + Sync {
    /// Get the user's preferred Claude binary path.
    ///
    /// Returns `None` if no preference is set.
    fn get_preferred_path(&self) -> Result<Option<String>, String>;

    /// Set the user's preferred Claude binary path.
    fn set_preferred_path(&self, path: &str) -> Result<(), String>;

    /// Get the user's installation type preference.
    ///
    /// Common values: "system", "nvm", "homebrew", "custom"
    fn get_installation_preference(&self) -> Result<Option<String>, String>;

    /// Set the user's installation type preference.
    fn set_installation_preference(&self, preference: &str) -> Result<(), String>;

    /// Clear all stored preferences.
    fn clear(&self) -> Result<(), String>;
}

/// File-based preference storage using JSON.
///
/// This is a simple implementation that stores preferences in a JSON file.
/// It's suitable for applications that don't have a database.
///
/// # Thread Safety
///
/// This implementation is thread-safe and can be shared across threads.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::preferences::{FilePreferenceStore, PreferenceStore};
/// use std::path::PathBuf;
///
/// let store = FilePreferenceStore::new(PathBuf::from("~/.config/claude/prefs.json"));
/// store.set_preferred_path("/usr/local/bin/claude").unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct FilePreferenceStore {
    path: PathBuf,
}

impl FilePreferenceStore {
    /// Create a new file-based preference store.
    ///
    /// The file will be created if it doesn't exist. The parent directory
    /// must exist.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Read preferences from the file.
    fn read(&self) -> Result<HashMap<String, String>, String> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let contents = fs::read_to_string(&self.path)
            .map_err(|e| format!("Failed to read preferences: {}", e))?;

        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }

        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse preferences: {}", e))
    }

    /// Write preferences to the file.
    fn write(&self, prefs: &HashMap<String, String>) -> Result<(), String> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create preference directory: {}", e))?;
        }

        let contents = serde_json::to_string_pretty(prefs)
            .map_err(|e| format!("Failed to serialize preferences: {}", e))?;

        fs::write(&self.path, contents)
            .map_err(|e| format!("Failed to write preferences: {}", e))
    }
}

impl PreferenceStore for FilePreferenceStore {
    fn get_preferred_path(&self) -> Result<Option<String>, String> {
        let prefs = self.read()?;
        Ok(prefs.get("claude_binary_path").cloned())
    }

    fn set_preferred_path(&self, path: &str) -> Result<(), String> {
        let mut prefs = self.read()?;
        prefs.insert("claude_binary_path".to_string(), path.to_string());
        self.write(&prefs)
    }

    fn get_installation_preference(&self) -> Result<Option<String>, String> {
        let prefs = self.read()?;
        Ok(prefs.get("installation_preference").cloned())
    }

    fn set_installation_preference(&self, preference: &str) -> Result<(), String> {
        let mut prefs = self.read()?;
        prefs.insert("installation_preference".to_string(), preference.to_string());
        self.write(&prefs)
    }

    fn clear(&self) -> Result<(), String> {
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|e| format!("Failed to remove preferences file: {}", e))?;
        }
        Ok(())
    }
}

/// Get the default preference store path.
///
/// Returns a path in the user's home directory: `~/.config/claude-sdk/preferences.json`
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::preferences::default_preference_path;
///
/// if let Some(path) = default_preference_path() {
///     println!("Preferences stored at: {}", path.display());
/// }
/// ```
pub fn default_preference_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        Some(PathBuf::from(home).join(".config/claude-sdk/preferences.json"))
    } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
        // Windows
        Some(PathBuf::from(user_profile).join(".config\\claude-sdk\\preferences.json"))
    } else {
        None
    }
}

/// Create a default file-based preference store.
///
/// Uses the default preference path from [`default_preference_path`].
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::preferences::default_file_store;
///
/// if let Some(store) = default_file_store() {
///     store.set_preferred_path("/usr/local/bin/claude").unwrap();
/// }
/// ```
pub fn default_file_store() -> Option<FilePreferenceStore> {
    default_preference_path().map(FilePreferenceStore::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    // Simple in-memory store for testing
    #[derive(Clone)]
    struct MemoryStore {
        data: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MemoryStore {
        fn new() -> Self {
            Self {
                data: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    impl PreferenceStore for MemoryStore {
        fn get_preferred_path(&self) -> Result<Option<String>, String> {
            Ok(self.data.lock().unwrap().get("path").cloned())
        }

        fn set_preferred_path(&self, path: &str) -> Result<(), String> {
            self.data.lock().unwrap().insert("path".to_string(), path.to_string());
            Ok(())
        }

        fn get_installation_preference(&self) -> Result<Option<String>, String> {
            Ok(self.data.lock().unwrap().get("preference").cloned())
        }

        fn set_installation_preference(&self, preference: &str) -> Result<(), String> {
            self.data.lock().unwrap().insert("preference".to_string(), preference.to_string());
            Ok(())
        }

        fn clear(&self) -> Result<(), String> {
            self.data.lock().unwrap().clear();
            Ok(())
        }
    }

    #[test]
    fn test_memory_store() {
        let store = MemoryStore::new();

        // Initially empty
        assert_eq!(store.get_preferred_path().unwrap(), None);
        assert_eq!(store.get_installation_preference().unwrap(), None);

        // Set values
        store.set_preferred_path("/usr/local/bin/claude").unwrap();
        store.set_installation_preference("homebrew").unwrap();

        // Retrieve values
        assert_eq!(
            store.get_preferred_path().unwrap(),
            Some("/usr/local/bin/claude".to_string())
        );
        assert_eq!(
            store.get_installation_preference().unwrap(),
            Some("homebrew".to_string())
        );

        // Clear
        store.clear().unwrap();
        assert_eq!(store.get_preferred_path().unwrap(), None);
        assert_eq!(store.get_installation_preference().unwrap(), None);
    }

    #[test]
    fn test_file_store() {
        use std::env;

        let temp_dir = env::temp_dir();
        let test_file = temp_dir.join(format!("test-prefs-{}.json", std::process::id()));

        // Clean up any existing file
        let _ = fs::remove_file(&test_file);

        let store = FilePreferenceStore::new(test_file.clone());

        // Initially empty
        assert_eq!(store.get_preferred_path().unwrap(), None);

        // Set and get
        store.set_preferred_path("/test/path").unwrap();
        assert_eq!(
            store.get_preferred_path().unwrap(),
            Some("/test/path".to_string())
        );

        // Create a new store pointing to the same file
        let store2 = FilePreferenceStore::new(test_file.clone());
        assert_eq!(
            store2.get_preferred_path().unwrap(),
            Some("/test/path".to_string())
        );

        // Clean up
        store.clear().unwrap();
        assert!(!test_file.exists());
    }

    #[test]
    fn test_default_preference_path() {
        let path = default_preference_path();
        assert!(path.is_some());

        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("claude-sdk"));
            assert!(p.to_string_lossy().contains("preferences.json"));
        }
    }
}
