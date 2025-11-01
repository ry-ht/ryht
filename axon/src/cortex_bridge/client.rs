//! HTTP client for Cortex REST API
//!
//! This module provides a robust HTTP client with retry logic, error handling,
//! and response unwrapping for the Cortex API.

use super::models::*;
use cortex_core::config::GlobalConfig;
use reqwest::{Client as HttpClient, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Cortex client error types
#[derive(Debug, Error)]
pub enum CortexError {
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Cortex service unavailable
    #[error("Cortex unavailable: {0}")]
    CortexUnavailable(String),

    /// Cortex API error
    #[error("Cortex error: {0}")]
    CortexError(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(SessionId),

    /// Invalid response format
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Lock acquisition failed
    #[error("Lock acquisition failed: {0}")]
    LockFailed(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
}

impl From<reqwest::Error> for CortexError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            CortexError::Timeout(err.to_string())
        } else if err.is_connect() {
            CortexError::CortexUnavailable(err.to_string())
        } else {
            CortexError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for CortexError {
    fn from(err: serde_json::Error) -> Self {
        CortexError::SerializationError(err.to_string())
    }
}

/// Result type for Cortex operations
pub type Result<T> = std::result::Result<T, CortexError>;

/// Configuration for Cortex client
#[derive(Debug, Clone)]
pub struct CortexConfig {
    /// Base URL for Cortex API
    pub base_url: String,
    /// API version
    pub api_version: String,
    /// Authentication token (optional)
    pub auth_token: Option<String>,
    /// Cache size in MB
    pub cache_size_mb: usize,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// Reconnect WebSocket on disconnect
    pub reconnect_websocket: bool,
}

impl Default for CortexConfig {
    /// Create a default CortexConfig with hardcoded fallback values.
    ///
    /// **Important**: This uses fallback defaults and should only be used
    /// when GlobalConfig is not available. Prefer using `CortexConfig::from_global_config()`
    /// to get configuration from GlobalConfig.
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_version: "v3".to_string(),
            auth_token: None,
            cache_size_mb: 100,
            cache_ttl_seconds: 3600,
            connection_pool_size: 10,
            request_timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_websocket: true,
            reconnect_websocket: true,
        }
    }
}

impl CortexConfig {
    /// Create a CortexConfig from GlobalConfig
    ///
    /// This is the preferred way to create a CortexConfig as it reads
    /// from the global configuration file.
    pub async fn from_global_config() -> Result<Self> {
        let config = GlobalConfig::load_or_create_default()
            .await
            .map_err(|e| CortexError::CortexError(format!("Failed to load GlobalConfig: {}", e)))?;

        Ok(Self {
            base_url: format!(
                "http://{}:{}",
                config.cortex().server.host,
                config.cortex().server.port
            ),
            api_version: "v3".to_string(),
            auth_token: None,
            cache_size_mb: config.cortex().cache.memory_size_mb as usize,
            cache_ttl_seconds: config.cortex().cache.ttl_seconds,
            connection_pool_size: config.cortex().pool.max_connections as usize,
            request_timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_websocket: true,
            reconnect_websocket: true,
        })
    }
}

/// Internal Cortex HTTP client
#[derive(Clone)]
pub(crate) struct CortexClient {
    client: HttpClient,
    base_url: String,
    config: CortexConfig,
}

impl CortexClient {
    /// Create a new Cortex client
    pub fn new(config: CortexConfig) -> Result<Self> {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .pool_max_idle_per_host(config.connection_pool_size)
            .build()?;

        let base_url = format!("{}/{}", config.base_url, config.api_version);

        Ok(Self {
            client,
            base_url,
            config,
        })
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the HTTP client
    pub fn http_client(&self) -> &HttpClient {
        &self.client
    }

    /// Get the configuration
    pub fn config(&self) -> &CortexConfig {
        &self.config
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| CortexError::CortexUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CortexError::CortexUnavailable(format!(
                "Health check failed: {}",
                response.status()
            )));
        }

        let health: HealthStatus = response.json().await?;
        info!("Cortex health check passed: status={}", health.status);
        Ok(health)
    }

    /// Unwrap Cortex API response envelope
    pub async fn unwrap_response<T: DeserializeOwned>(response: Response) -> Result<T> {
        #[derive(Deserialize)]
        struct ApiResponse<T> {
            success: bool,
            data: Option<T>,
            error: Option<String>,
        }

        let status = response.status();

        // For non-success HTTP status codes, handle errors
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("HTTP error {}: {}", status, error_text);
            return Err(CortexError::CortexError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        // Try to parse as API envelope
        let text = response.text().await?;
        debug!("Response body: {}", text);

        let envelope: ApiResponse<T> = serde_json::from_str(&text).map_err(|e| {
            error!("Failed to parse response: {}", e);
            CortexError::InvalidResponse(format!("Failed to parse response: {}", e))
        })?;

        if !envelope.success {
            let error_msg = envelope
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            error!("API error: {}", error_msg);
            return Err(CortexError::CortexError(error_msg));
        }

        envelope.data.ok_or_else(|| {
            error!("Missing data in successful response");
            CortexError::InvalidResponse("Missing data in response".to_string())
        })
    }

    /// Execute request with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = Duration::from_millis(self.config.retry_delay_ms);

        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Request succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(e) if attempt < self.config.max_retries && Self::is_retryable(&e) => {
                    warn!(
                        "Request failed (attempt {}/{}): {}",
                        attempt + 1,
                        self.config.max_retries,
                        e
                    );

                    tokio::time::sleep(delay).await;
                    delay = delay.mul_f32(2.0); // Exponential backoff
                    attempt += 1;
                }
                Err(e) => {
                    error!("Request failed permanently: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Check if error is retryable
    fn is_retryable(error: &CortexError) -> bool {
        matches!(
            error,
            CortexError::NetworkError(_)
                | CortexError::Timeout(_)
                | CortexError::CortexUnavailable(_)
        )
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("GET {}", url);

        let response = self.client.get(&url).send().await?;
        Self::unwrap_response(response).await
    }

    /// Make a POST request
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("POST {}", url);

        let response = self.client.post(&url).json(body).send().await?;
        Self::unwrap_response(response).await
    }

    /// Make a PUT request
    pub async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("PUT {}", url);

        let response = self.client.put(&url).json(body).send().await?;
        Self::unwrap_response(response).await
    }

    /// Make a DELETE request
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("DELETE {}", url);

        let response = self.client.delete(&url).send().await?;
        Self::unwrap_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CortexConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.api_version, "v3");
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_is_retryable() {
        assert!(CortexClient::is_retryable(&CortexError::NetworkError(
            "test".to_string()
        )));
        assert!(CortexClient::is_retryable(&CortexError::Timeout(
            "test".to_string()
        )));
        assert!(CortexClient::is_retryable(
            &CortexError::CortexUnavailable("test".to_string())
        ));
        assert!(!CortexClient::is_retryable(&CortexError::CortexError(
            "test".to_string()
        )));
    }
}
