//! Settings file loading with scope precedence.
//!
//! This module handles loading settings from various scopes (user, project, local)
//! and merging them with the correct precedence.

use std::fs;
use std::path::{Path, PathBuf};
use serde_json;

use crate::cc::error::{Error, SettingsError};
use crate::cc::result::Result;

use crate::cc::types::{ClaudeSettings, SettingsScope};

/// Load settings from multiple scopes with precedence.
///
/// Settings are loaded from the specified scopes and merged with the following precedence:
/// Local > Project > User
///
/// # Arguments
///
/// * `scopes` - The scopes to load from, in order of precedence (highest first)
/// * `project_path` - Optional project path, required for Project and Local scopes
///
/// # Returns
///
/// A merged `ClaudeSettings` object combining all loaded settings.
///
/// # Errors
///
/// Returns an error if a settings file exists but cannot be parsed.
/// Missing settings files are silently skipped.
pub async fn load_settings(
    scopes: &[SettingsScope],
    project_path: Option<PathBuf>,
) -> Result<ClaudeSettings> {
    let mut merged = ClaudeSettings::new();

    // Process scopes in reverse order (lowest precedence first)
    // This ensures higher precedence settings override lower ones
    let mut scopes_reversed = scopes.to_vec();
    scopes_reversed.reverse();

    for scope in scopes_reversed {
        if let Some(settings) = load_scope_settings(scope, project_path.as_ref()).await? {
            merged.merge(settings);
        }
    }

    Ok(merged)
}

/// Load settings from a single scope.
async fn load_scope_settings(
    scope: SettingsScope,
    project_path: Option<&PathBuf>,
) -> Result<Option<ClaudeSettings>> {
    let file_path = match scope.file_path(project_path) {
        Some(path) => path,
        None => return Ok(None),
    };

    load_settings_file(&file_path).await
}

/// Load settings from a file.
async fn load_settings_file(file_path: &Path) -> Result<Option<ClaudeSettings>> {
    // Return None if file doesn't exist
    if !file_path.exists() {
        return Ok(None);
    }

    let file_path = file_path.to_path_buf();

    // Load and parse in blocking task
    let settings = tokio::task::spawn_blocking(move || -> Result<ClaudeSettings> {
        let content = fs::read_to_string(&file_path).map_err(|e| {
            Error::Settings(SettingsError::ParseError {
                path: file_path.clone(),
                reason: format!("Failed to read file: {}", e),
                source: None,
            })
        })?;

        let settings: ClaudeSettings = serde_json::from_str(&content).map_err(|e| {
            Error::Settings(SettingsError::ParseError {
                path: file_path.clone(),
                reason: format!("Invalid JSON: {}", e),
                source: Some(e),
            })
        })?;

        Ok(settings)
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    Ok(Some(settings))
}

/// Save settings to a specific scope.
///
/// # Arguments
///
/// * `scope` - The scope to save to
/// * `settings` - The settings to save
/// * `project_path` - Optional project path, required for Project and Local scopes
///
/// # Errors
///
/// Returns an error if the file cannot be written or serialized.
pub async fn save_settings(
    scope: SettingsScope,
    settings: &ClaudeSettings,
    project_path: Option<PathBuf>,
) -> Result<()> {
    let file_path = scope
        .file_path(project_path.as_ref())
        .ok_or_else(|| {
            Error::Settings(SettingsError::InvalidScope {
                scope: format!("{:?}", scope),
                reason: "Cannot determine file path for scope".to_string(),
            })
        })?;

    save_settings_file(&file_path, settings).await
}

/// Save settings to a file.
async fn save_settings_file(file_path: &Path, settings: &ClaudeSettings) -> Result<()> {
    let file_path = file_path.to_path_buf();
    let settings = settings.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::Settings(SettingsError::WriteError {
                    path: file_path.clone(),
                    reason: format!("Failed to create directory: {}", e),
                })
            })?;
        }

        // Serialize settings with pretty formatting
        let content = serde_json::to_string_pretty(&settings).map_err(|e| {
            Error::Settings(SettingsError::WriteError {
                path: file_path.clone(),
                reason: format!("Failed to serialize: {}", e),
            })
        })?;

        // Write to file
        fs::write(&file_path, content).map_err(|e| {
            Error::Settings(SettingsError::WriteError {
                path: file_path.clone(),
                reason: format!("Failed to write file: {}", e),
            })
        })?;

        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    Ok(())
}

/// Load settings with default scopes (all scopes in precedence order).
pub async fn load_default_settings(project_path: Option<PathBuf>) -> Result<ClaudeSettings> {
    load_settings(&SettingsScope::all_ordered(), project_path).await
}
