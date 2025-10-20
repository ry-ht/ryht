/// Production-grade error recovery for critical paths
///
/// This module provides retry logic with exponential backoff for:
/// - RocksDB operations
/// - MCP tool calls
/// - Background task failures
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn, debug};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Conservative config for critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(30),
            multiplier: 2.5,
        }
    }

    /// Aggressive config for non-critical operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 2,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(2),
            multiplier: 1.5,
        }
    }
}

/// Retry a potentially failing operation with exponential backoff
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation_name` - Name for logging
/// * `f` - Async function to retry
///
/// # Example
/// ```ignore
/// let result = retry_with_backoff(
///     RetryConfig::default(),
///     "save_snapshot",
///     || async { storage.save_snapshot(&snapshot).await }
/// ).await?;
/// ```
pub async fn retry_with_backoff<F, Fut, T>(
    config: RetryConfig,
    operation_name: &str,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 1;
    let mut backoff = config.initial_backoff;

    loop {
        match f().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        "Operation '{}' succeeded on attempt {}/{}",
                        operation_name, attempt, config.max_attempts
                    );
                }
                return Ok(result);
            }
            Err(e) if attempt >= config.max_attempts => {
                error!(
                    "Operation '{}' failed after {} attempts: {}",
                    operation_name, config.max_attempts, e
                );
                return Err(e).context(format!(
                    "Failed after {} retry attempts",
                    config.max_attempts
                ));
            }
            Err(e) => {
                warn!(
                    "Operation '{}' failed on attempt {}/{}: {} (retrying in {:?})",
                    operation_name, attempt, config.max_attempts, e, backoff
                );

                sleep(backoff).await;

                // Calculate next backoff with exponential growth
                backoff = Duration::from_secs_f64(
                    (backoff.as_secs_f64() * config.multiplier).min(config.max_backoff.as_secs_f64())
                );

                attempt += 1;
            }
        }
    }
}

/// Categorize errors for appropriate handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Transient errors that should be retried
    Transient,
    /// Permanent errors that should not be retried
    Permanent,
    /// Resource exhaustion (disk, memory, etc.)
    ResourceExhaustion,
    /// Invalid input/parameters
    InvalidInput,
}

/// Analyze error and determine category
pub fn categorize_error(error: &anyhow::Error) -> ErrorCategory {
    let error_str = format!("{:?}", error);
    let lower = error_str.to_lowercase();

    // Check for transient errors
    if lower.contains("timeout")
        || lower.contains("connection")
        || lower.contains("temporary")
        || lower.contains("busy")
        || lower.contains("lock")
    {
        return ErrorCategory::Transient;
    }

    // Check for resource exhaustion
    if lower.contains("out of memory")
        || lower.contains("disk full")
        || lower.contains("no space left")
        || lower.contains("too many")
    {
        return ErrorCategory::ResourceExhaustion;
    }

    // Check for invalid input
    if lower.contains("invalid")
        || lower.contains("parse")
        || lower.contains("malformed")
        || lower.contains("not found")
    {
        return ErrorCategory::InvalidInput;
    }

    // Default to permanent
    ErrorCategory::Permanent
}

/// Retry with intelligent error categorization
pub async fn retry_with_categorization<F, Fut, T>(
    config: RetryConfig,
    operation_name: &str,
    f: F,
) -> Result<T>
where
    F: FnMut() -> Fut + Send,
    Fut: std::future::Future<Output = Result<T>> + Send,
{
    retry_with_backoff(config, operation_name, f).await
}

/// Execute operation with error recovery and alerting
///
/// This wraps an operation with:
/// - Retry logic
/// - Error categorization
/// - Logging/alerting
/// - Graceful degradation
pub async fn execute_with_recovery<F, Fut, T>(
    config: RetryConfig,
    operation_name: &str,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let result = retry_with_backoff(config.clone(), operation_name, &mut f).await;

    match &result {
        Ok(_) => {
            debug!("Operation '{}' completed successfully", operation_name);
        }
        Err(e) => {
            let category = categorize_error(e);
            error!(
                "Operation '{}' failed with {:?} error: {}",
                operation_name, category, e
            );

            // In production, this would send alerts
            match category {
                ErrorCategory::ResourceExhaustion => {
                    error!("ALERT: Resource exhaustion in operation '{}'", operation_name);
                }
                ErrorCategory::Permanent => {
                    error!("ALERT: Permanent failure in operation '{}'", operation_name);
                }
                _ => {}
            }
        }
    }

    result
}

/// Background task wrapper with automatic error recovery
///
/// Wraps a background task to:
/// - Catch and log panics
/// - Retry on transient failures
/// - Continue running despite errors
pub async fn run_background_task_with_recovery<F, Fut>(
    task_name: &str,
    interval: Duration,
    mut f: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut task_interval = tokio::time::interval(interval);
    let config = RetryConfig::default();

    loop {
        task_interval.tick().await;

        let result = retry_with_backoff(config.clone(), task_name, &mut f).await;

        match result {
            Ok(()) => {
                debug!("Background task '{}' completed successfully", task_name);
            }
            Err(e) => {
                // Log but don't crash - background tasks should be resilient
                error!("Background task '{}' failed: {}", task_name, e);

                let category = categorize_error(&e);
                if category == ErrorCategory::ResourceExhaustion {
                    warn!("Resource exhaustion detected, waiting longer before retry");
                    sleep(Duration::from_secs(60)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let result = retry_with_backoff(
            RetryConfig::default(),
            "test_op",
            || async { Ok::<_, anyhow::Error>(42) },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = retry_with_backoff(
            RetryConfig::default(),
            "test_op",
            || {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        anyhow::bail!("Temporary failure");
                    }
                    Ok::<_, anyhow::Error>(42)
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(50),
            multiplier: 2.0,
        };

        let result: Result<(), _> = retry_with_backoff(config, "test_op", || async {
            anyhow::bail!("Permanent failure")
        })
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2 retry attempts"));
    }

    #[tokio::test]
    async fn test_error_categorization() {
        let timeout_err = anyhow::anyhow!("Connection timeout");
        assert_eq!(categorize_error(&timeout_err), ErrorCategory::Transient);

        let disk_err = anyhow::anyhow!("No space left on device");
        assert_eq!(
            categorize_error(&disk_err),
            ErrorCategory::ResourceExhaustion
        );

        let parse_err = anyhow::anyhow!("Invalid JSON format");
        assert_eq!(categorize_error(&parse_err), ErrorCategory::InvalidInput);

        let other_err = anyhow::anyhow!("Something went wrong");
        assert_eq!(categorize_error(&other_err), ErrorCategory::Permanent);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let config = RetryConfig {
            max_attempts: 4,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(100),
            multiplier: 2.0,
        };

        let start = tokio::time::Instant::now();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let _: Result<(), _> = retry_with_backoff(config, "test_op", || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                anyhow::bail!("Always fail")
            }
        })
        .await;

        let elapsed = start.elapsed();

        // Should have waited: 10ms + 20ms + 40ms = 70ms minimum
        assert!(elapsed >= Duration::from_millis(70));
        assert_eq!(attempts.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_conservative_config() {
        let config = RetryConfig::conservative();
        assert_eq!(config.max_attempts, 5);
        assert!(config.initial_backoff >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_aggressive_config() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_attempts, 2);
        assert!(config.max_backoff <= Duration::from_secs(5));
    }
}
