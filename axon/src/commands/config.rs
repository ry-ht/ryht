//! Configuration management for Axon

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxonConfig {
    pub workspace_name: String,
    pub workspace_path: PathBuf,

    /// REST API server configuration
    pub server: ServerConfig,

    /// Agent runtime configuration
    pub runtime: RuntimeConfig,

    /// Cortex integration settings
    pub cortex: CortexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_agents: usize,
    pub agent_timeout_seconds: u64,
    pub task_queue_size: usize,
    pub enable_auto_recovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    pub enabled: bool,
    pub mcp_server_url: Option<String>,
    pub workspace: Option<String>,
}

impl Default for AxonConfig {
    fn default() -> Self {
        Self {
            workspace_name: String::from("default"),
            workspace_path: PathBuf::from("."),
            server: ServerConfig {
                host: String::from("127.0.0.1"),
                port: 9090,
                workers: None,
            },
            runtime: RuntimeConfig {
                max_agents: 10,
                agent_timeout_seconds: 300,
                task_queue_size: 100,
                enable_auto_recovery: true,
            },
            cortex: CortexConfig {
                enabled: true,
                mcp_server_url: None,
                workspace: None,
            },
        }
    }
}

impl AxonConfig {
    /// Load configuration from default locations
    pub fn load() -> Result<Self> {
        // Try to load from local .axon/config/workspace.toml
        if let Ok(config) = Self::load_from_path(&PathBuf::from(".axon/config/workspace.toml")) {
            return Ok(config);
        }

        // Try to load from global config
        if let Ok(config) = Self::load_global() {
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

    /// Load global configuration
    pub fn load_global() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        let config_path = home.join(".ryht/axon/config.toml");
        Self::load_from_path(&config_path)
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

    /// Save global configuration
    pub fn save_global(&self) -> Result<()> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        let config_path = home.join(".ryht/axon/config.toml");
        self.save_to_path(&config_path)
    }

    /// Get a configuration value by key
    pub fn get(&self, key: &str) -> Result<String> {
        match key {
            "workspace_name" => Ok(self.workspace_name.clone()),
            "workspace_path" => Ok(self.workspace_path.display().to_string()),
            "server.host" => Ok(self.server.host.clone()),
            "server.port" => Ok(self.server.port.to_string()),
            "runtime.max_agents" => Ok(self.runtime.max_agents.to_string()),
            "cortex.enabled" => Ok(self.cortex.enabled.to_string()),
            _ => Err(anyhow::anyhow!("Unknown configuration key: {}", key)),
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
            "server.host" => {
                self.server.host = value.to_string();
            }
            "server.port" => {
                self.server.port = value.parse()
                    .context("Invalid port number")?;
            }
            "runtime.max_agents" => {
                self.runtime.max_agents = value.parse()
                    .context("Invalid max_agents value")?;
            }
            "cortex.enabled" => {
                self.cortex.enabled = value.parse()
                    .context("Invalid boolean value")?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        }
        Ok(())
    }

    /// Get runtime directory path
    pub fn runtime_dir(&self) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(".ryht/axon")
    }

    /// Get logs directory path
    pub fn logs_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/logs")
    }

    /// Get agents directory path
    pub fn agents_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/agents")
    }

    /// Get workflows directory path
    pub fn workflows_dir(&self) -> PathBuf {
        self.workspace_path.join(".axon/workflows")
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

    /// Save configuration
    pub async fn save(&self, global: bool) -> Result<()> {
        if global {
            self.config.save_global()
        } else {
            self.config.save()
        }
    }

    /// Get all configuration values
    pub fn all(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("workspace_name".to_string(), self.config.workspace_name.clone());
        map.insert("workspace_path".to_string(), self.config.workspace_path.display().to_string());
        map.insert("server.host".to_string(), self.config.server.host.clone());
        map.insert("server.port".to_string(), self.config.server.port.to_string());
        map.insert("runtime.max_agents".to_string(), self.config.runtime.max_agents.to_string());
        map.insert("cortex.enabled".to_string(), self.config.cortex.enabled.to_string());
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AxonConfig::default();
        assert_eq!(config.workspace_name, "default");
        assert_eq!(config.server.port, 9090);
        assert!(config.cortex.enabled);
    }

    #[test]
    fn test_config_get_set() {
        let mut config = AxonConfig::default();

        config.set("workspace_name", "test-workspace").unwrap();
        assert_eq!(config.get("workspace_name").unwrap(), "test-workspace");

        config.set("server.port", "8080").unwrap();
        assert_eq!(config.get("server.port").unwrap(), "8080");
    }
}
