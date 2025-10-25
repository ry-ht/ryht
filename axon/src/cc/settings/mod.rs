//! Settings management module.
//!
//! This module provides functionality for loading and saving Claude Code settings
//! from various scopes (user, project, local) with proper precedence.
//!
//! # Examples
//!
//! ```no_run
//! use crate::cc::settings::{load_settings, save_settings, SettingsScope, ClaudeSettings};
//! use std::path::PathBuf;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Load settings from all scopes
//! let project_path = PathBuf::from("/path/to/project");
//! let settings = load_settings(
//!     &[SettingsScope::User, SettingsScope::Project],
//!     Some(project_path.clone())
//! ).await?;
//!
//! println!("Default model: {:?}", settings.default_model);
//!
//! // Modify and save settings
//! let mut new_settings = ClaudeSettings::new();
//! new_settings.default_model = Some("claude-sonnet-4-5-20250929".to_string());
//! save_settings(SettingsScope::User, &new_settings, None).await?;
//! # Ok(())
//! # }
//! ```

mod loader;
mod types;

// Re-export public API
pub use loader::{load_settings, save_settings, load_default_settings};
pub use types::{ClaudeSettings, HookConfig, SettingsScope};
