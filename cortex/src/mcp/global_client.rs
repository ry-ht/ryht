//! Global server client for MCP server integration
//!
//! This module provides an HTTP client for communicating with the global server.
//! It handles:
//! - Project registry queries
//! - Symbol search across global database
//! - Documentation retrieval
//! - Symbol updates to global server
//! - Connection pooling and retry logic

use crate::global::ProjectRegistry;
use crate::types::CodeSymbol;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Symbol query for global search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolQuery {
    /// Search query string
    pub query: String,

    /// Optional project filter
    pub project_id: Option<String>,

    /// Maximum results
    pub limit: usize,

    /// Search scope
    pub scope: SearchScope,
}

/// Search scope for symbol queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    /// Search only in current project
    Local,

    /// Search in dependencies
    Dependencies,

    /// Search globally across all projects
    Global,
}

/// Global server client
pub struct GlobalServerClient {
    /// Base URL for global server
    base_url: String,

    /// HTTP client with connection pooling
    client: reqwest::Client,

    /// Maximum retry attempts
    max_retries: u32,

    /// Base retry delay
    retry_delay: Duration,
}

impl GlobalServerClient {
    /// Create a new global server client
    pub fn new(base_url: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url,
            client,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        })
    }

    /// Create a new client with custom configuration
    pub fn with_config(base_url: String, timeout: Duration, max_retries: u32) -> Result<Self> {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url,
            client,
            max_retries,
            retry_delay: Duration::from_millis(100),
        })
    }

    /// Get a project by ID
    pub async fn get_project(&self, id: &str) -> Result<ProjectRegistry> {
        let url = format!("{}/api/projects/{}", self.base_url, id);

        self.retry_request(|| async {
            debug!("Fetching project: {}", id);

            let response = self.client
                .get(&url)
                .send()
                .await
                .context("Failed to send request")?;

            if !response.status().is_success() {
                bail!("Server returned error: {}", response.status());
            }

            response
                .json::<ProjectRegistry>()
                .await
                .context("Failed to parse response")
        })
        .await
    }

    /// Search for symbols across global database
    pub async fn search_symbols(&self, query: SymbolQuery) -> Result<Vec<CodeSymbol>> {
        let url = format!("{}/api/symbols/search", self.base_url);

        self.retry_request(|| async {
            debug!("Searching symbols: {:?}", query.query);

            let response = self.client
                .post(&url)
                .json(&query)
                .send()
                .await
                .context("Failed to send request")?;

            if !response.status().is_success() {
                bail!("Server returned error: {}", response.status());
            }

            response
                .json::<Vec<CodeSymbol>>()
                .await
                .context("Failed to parse response")
        })
        .await
    }

    /// Get documentation for a specific symbol in a project
    pub async fn get_documentation(&self, project_id: &str, symbol_id: &str) -> Result<String> {
        let url = format!("{}/api/projects/{}/docs/{}", self.base_url, project_id, symbol_id);

        self.retry_request(|| async {
            debug!("Fetching documentation: {}/{}", project_id, symbol_id);

            let response = self.client
                .get(&url)
                .send()
                .await
                .context("Failed to send request")?;

            if !response.status().is_success() {
                bail!("Server returned error: {}", response.status());
            }

            response
                .text()
                .await
                .context("Failed to parse response")
        })
        .await
    }

    /// Update symbols for a project
    pub async fn update_symbols(&self, project_id: &str, symbols: Vec<CodeSymbol>) -> Result<()> {
        let url = format!("{}/api/projects/{}/symbols", self.base_url, project_id);

        self.retry_request(|| async {
            debug!("Updating {} symbols for project {}", symbols.len(), project_id);

            let response = self.client
                .put(&url)
                .json(&symbols)
                .send()
                .await
                .context("Failed to send request")?;

            if !response.status().is_success() {
                bail!("Server returned error: {}", response.status());
            }

            Ok(())
        })
        .await
    }

    /// Check if global server is available
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/health", self.base_url);

        match tokio::time::timeout(Duration::from_secs(2), self.client.get(&url).send()).await {
            Ok(Ok(response)) => response.status().is_success(),
            Ok(Err(e)) => {
                debug!("Global server unavailable: {}", e);
                false
            }
            Err(_) => {
                debug!("Global server health check timed out");
                false
            }
        }
    }

    /// Retry a request with exponential backoff
    async fn retry_request<F, Fut, T>(&self, mut request_fn: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.max_retries {
            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if attempts < self.max_retries {
                        let delay = self.retry_delay * 2u32.pow(attempts - 1);
                        warn!(
                            "Request failed (attempt {}/{}), retrying in {:?}",
                            attempts, self.max_retries, delay
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Request failed after {} attempts", self.max_retries)))
    }
}

/// Create a shared global server client
pub fn create_global_client(base_url: String) -> Result<Arc<GlobalServerClient>> {
    Ok(Arc::new(GlobalServerClient::new(base_url)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to check if a test server is available
    async fn is_test_server_available() -> bool {
        // Try to connect to a test server on localhost:7878
        let client = reqwest::Client::new();
        let url = "http://localhost:7878/api/health";

        match tokio::time::timeout(Duration::from_secs(1), client.get(url).send()).await {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = GlobalServerClient::new("http://localhost:7878".to_string());
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_with_config() {
        let client = GlobalServerClient::with_config(
            "http://localhost:7878".to_string(),
            Duration::from_secs(10),
            5,
        );
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.max_retries, 5);
    }

    #[tokio::test]
    async fn test_is_available_when_server_down() {
        // Use a port that's unlikely to be in use
        let client = GlobalServerClient::new("http://localhost:19999".to_string()).unwrap();
        let available = client.is_available().await;

        // Server should not be available
        assert!(!available);
    }

    #[tokio::test]
    async fn test_get_project_server_down() {
        let client = GlobalServerClient::new("http://localhost:19999".to_string()).unwrap();
        let result = client.get_project("test-project").await;

        // Should fail when server is down
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_symbols_server_down() {
        let client = GlobalServerClient::new("http://localhost:19999".to_string()).unwrap();

        let query = SymbolQuery {
            query: "test".to_string(),
            project_id: None,
            limit: 10,
            scope: SearchScope::Global,
        };

        let result = client.search_symbols(query).await;

        // Should fail when server is down
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let client = GlobalServerClient::with_config(
            "http://localhost:19999".to_string(),
            Duration::from_millis(100),
            2, // Only 2 retries for faster test
        ).unwrap();

        let start = std::time::Instant::now();
        let result = client.get_project("test").await;
        let elapsed = start.elapsed();

        // Should fail after retries
        assert!(result.is_err());

        // Should have taken at least some time (with some tolerance for timing)
        // First attempt + 1 retry = 100ms base delay, but be lenient
        assert!(elapsed >= Duration::from_millis(50));
    }

    // Integration tests - only run when global server is available
    #[tokio::test]
    async fn test_integration_health_check() {
        if !is_test_server_available().await {
            eprintln!("Skipping integration test - global server not available");
            return;
        }

        let client = GlobalServerClient::new("http://localhost:7878".to_string()).unwrap();
        let available = client.is_available().await;

        assert!(available);
    }
}
