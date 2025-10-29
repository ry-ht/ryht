//! Configuration management for Axon
//!
//! This module provides Axon-specific configuration management that integrates
//! with the unified GlobalConfig from cortex-core.

use anyhow::{Context, Result};
use cortex_core::config::{GlobalConfig, AxonSection, ServerConfig, RuntimeConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;

/// Axon workspace-specific configuration
///
/// This extends the global AxonSection with workspace-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxonConfig {
    pub workspace_name: String,
    pub workspace_path: PathBuf,

    /// Cortex integration settings
    pub cortex: CortexConfig,
}

/// Cortex integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    pub enabled: bool,
    pub api_url: Option<String>,
    pub mcp_server_url: Option<String>,
    pub workspace: Option<String>,
}

impl Default for AxonConfig {
    fn default() -> Self {
        Self {
            workspace_name: String::from("default"),
            workspace_path: PathBuf::from("."),
            cortex: CortexConfig {
                enabled: true,
                api_url: Some(String::from("http://127.0.0.1:8080")),
                mcp_server_url: None,
                workspace: None,
            },
        }
    }
}

impl AxonConfig {
    /// Load workspace-specific configuration
    ///
    /// This loads the workspace config (.axon/config/workspace.toml) which contains
    /// workspace-specific settings. Global Axon settings are stored in ~/.ryht/config.toml
    pub fn load() -> Result<Self> {
        // Try to load from local .axon/config/workspace.toml
        if let Ok(config) = Self::load_from_path(&PathBuf::from(".axon/config/workspace.toml")) {
            return Ok(config);
        }

        // Return default config
        Ok(Self::default())
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    /// Load global Axon configuration from unified config
    pub async fn load_global() -> Result<AxonSection> {
        let global_config = GlobalConfig::load_or_create_default().await?;
        Ok(global_config.axon().clone())
    }

    /// Get the global server configuration
    pub async fn global_server_config() -> Result<ServerConfig> {
        let global_config = GlobalConfig::load_or_create_default().await?;
        Ok(global_config.axon().server.clone())
    }

    /// Get the global runtime configuration
    pub async fn global_runtime_config() -> Result<RuntimeConfig> {
        let global_config = GlobalConfig::load_or_create_default().await?;
        Ok(global_config.axon().runtime.clone())
    }

    /// Save configuration to workspace
    pub fn save(&self) -> Result<()> {
        self.save_to_path(&PathBuf::from(".axon/config/workspace.toml"))
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))
    }

    /// Get a configuration value by key
    pub fn get(&self, key: &str) -> Result<String> {
        match key {
            "workspace_name" => Ok(self.workspace_name.clone()),
            "workspace_path" => Ok(self.workspace_path.display().to_string()),
            "cortex.enabled" => Ok(self.cortex.enabled.to_string()),
            "cortex.api_url" => Ok(self.cortex.api_url.clone().unwrap_or_default()),
            _ => Err(anyhow::anyhow!("Unknown workspace configuration key: {}. Use global config for server/runtime settings.", key)),
        }
    }

    /// Set a configuration value by key
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "workspace_name" => {
                self.workspace_name = value.to_string();
            }
            "workspace_path" => {
                self.workspace_path = PathBuf::from(value);
            }
            "cortex.enabled" => {
                self.cortex.enabled = value.parse()
                    .context("Invalid boolean value")?;
            }
            "cortex.api_url" => {
                self.cortex.api_url = Some(value.to_string());
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown workspace configuration key: {}. Use global config for server/runtime settings.", key));
            }
        }
        Ok(())
    }

    /// Get runtime directory path (from global config)
    pub fn runtime_dir(&self) -> PathBuf {
        GlobalConfig::axon_dir().unwrap_or_else(|_| PathBuf::from("/tmp/.ryht/axon"))
    }

    /// Get logs directory path (workspace-local)
    pub fn logs_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/logs")
    }

    /// Get agents directory path (workspace-local)
    pub fn agents_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/agents")
    }

    /// Get workflows directory path (workspace-local)
    pub fn workflows_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/workflows")
    }

    /// Get global logs directory path
    pub fn global_logs_dir() -> PathBuf {
        GlobalConfig::axon_logs_dir().unwrap_or_else(|_| PathBuf::from("/tmp/.ryht/axon/logs"))
    }

    /// Get global agents directory path
    pub fn global_agents_dir() -> PathBuf {
        GlobalConfig::axon_agents_dir().unwrap_or_else(|_| PathBuf::from("/tmp/.ryht/axon/agents"))
    }

    /// Get global workflows directory path
    pub fn global_workflows_dir() -> PathBuf {
        GlobalConfig::axon_workflows_dir().unwrap_or_else(|_| PathBuf::from("/tmp/.ryht/axon/workflows"))
    }
}

/// Configuration Manager wrapper for CLI commands
pub struct ConfigManager {
    config: AxonConfig,
}

impl ConfigManager {
    /// Load configuration
    pub async fn load() -> Result<Self> {
        let config = AxonConfig::load()?;
        Ok(Self { config })
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<String> {
        self.config.get(key).ok()
    }

    /// Set a configuration value
    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        self.config.set(key, &value)
    }

    /// Save workspace configuration
    ///
    /// Note: For global Axon configuration (server, runtime), use GlobalConfig directly
    pub async fn save(&self, _global: bool) -> Result<()> {
        // Only save workspace config - global config is managed through GlobalConfig
        self.config.save()
    }

    /// Get all workspace configuration values
    pub fn all(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("workspace_name".to_string(), self.config.workspace_name.clone());
        map.insert("workspace_path".to_string(), self.config.workspace_path.display().to_string());
        map.insert("cortex.enabled".to_string(), self.config.cortex.enabled.to_string());
        if let Some(api_url) = &self.config.cortex.api_url {
            map.insert("cortex.api_url".to_string(), api_url.clone());
        }
        map
    }

    /// Get all configuration values including global settings
    pub async fn all_with_global(&self) -> Result<HashMap<String, String>> {
        let mut map = self.all();

        // Add global Axon configuration
        if let Ok(global_config) = GlobalConfig::load_or_create_default().await {
            let axon = global_config.axon();
            map.insert("server.host".to_string(), axon.server.host.clone());
            map.insert("server.port".to_string(), axon.server.port.to_string());
            map.insert("runtime.max_agents".to_string(), axon.runtime.max_agents.to_string());
            map.insert("runtime.agent_timeout_seconds".to_string(), axon.runtime.agent_timeout_seconds.to_string());
            map.insert("runtime.task_queue_size".to_string(), axon.runtime.task_queue_size.to_string());
            map.insert("runtime.enable_auto_recovery".to_string(), axon.runtime.enable_auto_recovery.to_string());
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AxonConfig::default();
        assert_eq!(config.workspace_name, "default");
        assert!(config.cortex.enabled);
    }

    #[test]
    fn test_config_get_set() {
        let mut config = AxonConfig::default();

        config.set("workspace_name", "test-workspace").unwrap();
        assert_eq!(config.get("workspace_name").unwrap(), "test-workspace");

        config.set("cortex.enabled", "false").unwrap();
        assert_eq!(config.get("cortex.enabled").unwrap(), "false");
    }

    #[tokio::test]
    async fn test_global_config_integration() {
        // Test that we can load global Axon config
        let result = AxonConfig::load_global().await;
        // It's ok if this fails in test environment
        if let Ok(axon_section) = result {
            assert_eq!(axon_section.server.host, "127.0.0.1");
            assert_eq!(axon_section.server.port, 9090);
        }
    }
}
