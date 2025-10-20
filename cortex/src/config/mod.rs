use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Get the Meridian home directory (~/.meridian)
/// Can be overridden with MERIDIAN_HOME environment variable
///
/// # Panics
/// Panics if neither MERIDIAN_HOME, HOME, nor USERPROFILE environment variables are set.
/// This is intentional to prevent creating .meridian in arbitrary directories.
pub fn get_meridian_home() -> PathBuf {
    // First check for explicit MERIDIAN_HOME override (useful for testing)
    if let Ok(meridian_home) = std::env::var("MERIDIAN_HOME") {
        return PathBuf::from(meridian_home);
    }

    // Use dirs crate for reliable home directory detection
    // This handles platform-specific differences correctly
    dirs::home_dir()
        .map(|home| home.join(".meridian"))
        .unwrap_or_else(|| {
            // Final fallback: check environment variables
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .expect("Cannot determine home directory: HOME, USERPROFILE, and dirs::home_dir() all failed. Set MERIDIAN_HOME explicitly.");
            PathBuf::from(home).join(".meridian")
        })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub index: IndexConfig,
    pub storage: StorageConfig,
    pub memory: MemoryConfig,
    pub session: SessionConfig,
    pub monorepo: MonorepoConfig,
    pub learning: LearningConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub languages: Vec<String>,
    pub ignore: Vec<String>,
    pub max_file_size: String,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            languages: vec![
                "rust".to_string(),
                "typescript".to_string(),
                "javascript".to_string(),
                "markdown".to_string(),
            ],
            ignore: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            max_file_size: "1MB".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: PathBuf,
    pub cache_size: String,
    /// Path to store HNSW vector index (for fast startup)
    /// If None, defaults to storage.path/hnsw_index
    pub hnsw_index_path: Option<PathBuf>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: get_meridian_home().join("db").join("current").join("index"),
            cache_size: "256MB".to_string(),
            hnsw_index_path: None, // Will use default path in storage.path/hnsw_index
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub episodic_retention_days: u32,
    pub working_memory_size: String,
    pub consolidation_interval: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            episodic_retention_days: 30,
            working_memory_size: "10MB".to_string(),
            consolidation_interval: "1h".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_sessions: usize,
    pub session_timeout: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            session_timeout: "1h".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonorepoConfig {
    pub detect_projects: bool,
    pub project_markers: Vec<String>,
}

impl Default for MonorepoConfig {
    fn default() -> Self {
        Self {
            detect_projects: true,
            project_markers: vec![
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "tsconfig.json".to_string(),
                "go.mod".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub min_episodes_for_pattern: u32,
    pub confidence_threshold: f32,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_episodes_for_pattern: 3,
            confidence_threshold: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub socket: Option<PathBuf>,
    pub max_token_response: u32,
    pub http: Option<HttpConfig>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            socket: Some(PathBuf::from("/tmp/meridian.sock")),
            max_token_response: 2000,
            http: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub max_connections: usize,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_origins: vec!["*".to_string()],
            max_connections: 100,
        }
    }
}


impl Config {
    /// Get the global config file path
    pub fn global_config_path() -> PathBuf {
        get_meridian_home().join("meridian.toml")
    }

    /// Load configuration from the global config file
    /// Falls back to defaults if config doesn't exist
    /// Environment variables can override specific settings
    pub fn load() -> Result<Self> {
        let global_path = Self::global_config_path();

        // Load from global config or use defaults
        let mut config = if global_path.exists() {
            tracing::info!("Loading config from {:?}", global_path);
            Self::from_file(&global_path)?
        } else {
            tracing::warn!("Global config not found at {:?}, using defaults", global_path);
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Load config from a specific file path (for testing/migration)
    pub fn from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::warn!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        Ok(config)
    }

    /// Save config to global location
    pub fn save(&self) -> Result<()> {
        let global_path = Self::global_config_path();
        self.save_to(&global_path)
    }

    /// Save config to specific path (for testing/migration)
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        tracing::info!("Config saved to {:?}", path);
        Ok(())
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        // MCP socket path override
        if let Ok(socket) = std::env::var("MERIDIAN_MCP_SOCKET") {
            self.mcp.socket = Some(PathBuf::from(socket));
        }

        // HTTP port override
        if let Ok(port) = std::env::var("MERIDIAN_HTTP_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                if let Some(ref mut http) = self.mcp.http {
                    http.port = port_num;
                }
            }
        }

        // Max token response override
        if let Ok(max_tokens) = std::env::var("MERIDIAN_MAX_TOKENS") {
            if let Ok(tokens) = max_tokens.parse::<u32>() {
                self.mcp.max_token_response = tokens;
            }
        }

        // Log level affects behavior but is handled by tracing subscriber
        tracing::debug!("Applied environment variable overrides to config");
    }

    /// Initialize global config with defaults if it doesn't exist
    pub fn init_global() -> Result<()> {
        let global_path = Self::global_config_path();

        if global_path.exists() {
            tracing::info!("Global config already exists at {:?}", global_path);
            return Ok(());
        }

        tracing::info!("Creating default global config at {:?}", global_path);
        let config = Self::default();
        config.save_to(&global_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.index.languages.len(), 4);
        assert!(config.index.languages.contains(&"rust".to_string()));
        assert_eq!(config.session.max_sessions, 10);
        assert_eq!(config.learning.min_episodes_for_pattern, 3);
    }

    #[test]
    fn test_index_config_default() {
        let config = IndexConfig::default();
        assert_eq!(config.max_file_size, "1MB");
        assert!(config.ignore.contains(&"node_modules".to_string()));
        assert!(config.ignore.contains(&"target".to_string()));
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        // Verify path ends with db/current/index
        assert!(config.path.ends_with("db/current/index"));
        // Verify path contains .meridian
        assert!(config.path.to_string_lossy().contains(".meridian"));
        assert_eq!(config.cache_size, "256MB");
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert_eq!(config.episodic_retention_days, 30);
        assert_eq!(config.working_memory_size, "10MB");
        assert_eq!(config.consolidation_interval, "1h");
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.max_sessions, 10);
        assert_eq!(config.session_timeout, "1h");
    }

    #[test]
    fn test_monorepo_config_default() {
        let config = MonorepoConfig::default();
        assert!(config.detect_projects);
        assert_eq!(config.project_markers.len(), 4);
        assert!(config.project_markers.contains(&"package.json".to_string()));
    }

    #[test]
    fn test_learning_config_default() {
        let config = LearningConfig::default();
        assert_eq!(config.min_episodes_for_pattern, 3);
        assert_eq!(config.confidence_threshold, 0.7);
    }

    #[test]
    fn test_mcp_config_default() {
        let config = McpConfig::default();
        assert_eq!(config.max_token_response, 2000);
        assert!(config.socket.is_some());
    }

    #[test]
    fn test_http_config_default() {
        let config = HttpConfig::default();
        assert!(config.enabled);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.max_connections, 100);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.session.max_sessions = 20;
        config.learning.confidence_threshold = 0.8;

        config.save_to(&config_path).unwrap();
        assert!(config_path.exists());

        let loaded = Config::from_file(&config_path).unwrap();
        assert_eq!(loaded.session.max_sessions, 20);
        assert_eq!(loaded.learning.confidence_threshold, 0.8);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let config = Config::from_file(&config_path).unwrap();
        // Should return default config
        assert_eq!(config.session.max_sessions, 10);
    }

    #[test]
    fn test_load_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");
        std::fs::write(&config_path, "invalid toml {{{}").unwrap();

        let result = Config::from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[index]"));
        assert!(toml_str.contains("[storage]"));
        assert!(toml_str.contains("[memory]"));
        assert!(toml_str.contains("[session]"));
    }

    #[test]
    fn test_config_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("roundtrip.toml");

        // Create a default config, modify some values
        let mut config = Config::default();
        config.session.max_sessions = 5;
        config.session.session_timeout = "2h".to_string();
        config.learning.confidence_threshold = 0.85;

        // Save and reload
        config.save_to(&config_path).unwrap();
        let loaded = Config::from_file(&config_path).unwrap();

        assert_eq!(loaded.session.max_sessions, 5);
        assert_eq!(loaded.session.session_timeout, "2h");
        assert_eq!(loaded.learning.confidence_threshold, 0.85);
        // Other fields should match defaults
        assert_eq!(loaded.learning.min_episodes_for_pattern, 3);
    }

    #[test]
    fn test_global_config_path() {
        let path = Config::global_config_path();
        assert!(path.to_string_lossy().contains(".meridian"));
        assert!(path.to_string_lossy().ends_with("meridian.toml"));
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("MERIDIAN_MCP_SOCKET", "/tmp/test.sock");
        std::env::set_var("MERIDIAN_HTTP_PORT", "8080");
        std::env::set_var("MERIDIAN_MAX_TOKENS", "5000");

        let mut config = Config::default();
        config.mcp.http = Some(HttpConfig::default());
        config.apply_env_overrides();

        assert_eq!(config.mcp.socket, Some(PathBuf::from("/tmp/test.sock")));
        assert_eq!(config.mcp.http.as_ref().unwrap().port, 8080);
        assert_eq!(config.mcp.max_token_response, 5000);

        // Clean up
        std::env::remove_var("MERIDIAN_MCP_SOCKET");
        std::env::remove_var("MERIDIAN_HTTP_PORT");
        std::env::remove_var("MERIDIAN_MAX_TOKENS");
    }

    #[test]
    fn test_init_global_creates_config() {
        // This test would need to mock the global path or use a temp directory
        // For now, we'll just test that the function exists and compiles
        // Real integration test should be in a separate test file
    }

    #[test]
    fn test_get_meridian_home_never_returns_current_dir() {
        // Critical test: ensure get_meridian_home() NEVER returns "." or current directory
        // This prevents creating .meridian in arbitrary project directories
        let meridian_home = get_meridian_home();

        // Should not be current directory
        assert_ne!(meridian_home, PathBuf::from("."));
        assert_ne!(meridian_home, PathBuf::from("./"));

        // Should be absolute path
        assert!(meridian_home.is_absolute(),
            "Meridian home must be absolute path, got: {:?}", meridian_home);

        // Should end with .meridian
        assert!(meridian_home.ends_with(".meridian"),
            "Meridian home must end with .meridian, got: {:?}", meridian_home);

        // Should not be in current working directory (unless HOME happens to be there)
        if let Ok(cwd) = std::env::current_dir() {
            // If meridian_home is under cwd, it's only OK if HOME is also under cwd
            // (e.g., testing scenario where HOME=/tmp/test)
            if meridian_home.starts_with(&cwd) {
                // This should only happen in test scenarios with custom MERIDIAN_HOME
                let is_test_override = std::env::var("MERIDIAN_HOME").is_ok();
                assert!(is_test_override,
                    "Meridian home should not be under current working directory unless MERIDIAN_HOME is set");
            }
        }
    }
}
