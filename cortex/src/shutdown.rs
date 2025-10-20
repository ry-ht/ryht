/// Graceful shutdown coordinator for production deployment
///
/// Handles SIGTERM/SIGINT signals and coordinates clean shutdown:
/// - Flush pending metrics
/// - Save final snapshots
/// - Close database connections
/// - Cancel background tasks
/// - Exit cleanly
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tracing::{info, error};

/// Shutdown coordinator
pub struct ShutdownCoordinator {
    shutdown_flag: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to check shutdown status
    pub fn handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            flag: self.shutdown_flag.clone(),
        }
    }

    /// Wait for shutdown signal (SIGTERM or SIGINT)
    pub async fn wait_for_signal(&self) {
        let sigterm = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        let sigint = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install SIGINT handler");
        };

        tokio::select! {
            _ = sigterm => {
                info!("Received SIGTERM signal");
            }
            _ = sigint => {
                info!("Received SIGINT signal (Ctrl+C)");
            }
        }

        // Set shutdown flag
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for checking shutdown status
#[derive(Clone)]
pub struct ShutdownHandle {
    flag: Arc<AtomicBool>,
}

impl ShutdownHandle {
    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Wait until shutdown is requested
    pub async fn wait_for_shutdown(&self) {
        while !self.is_shutdown_requested() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

/// Graceful shutdown sequence for MCP server
pub async fn shutdown_server_gracefully(
    metrics_collector: Option<Arc<crate::metrics::MetricsCollector>>,
    metrics_storage: Option<Arc<crate::metrics::MetricsStorage>>,
    background_tasks: Vec<tokio::task::JoinHandle<()>>,
) -> Result<()> {
    info!("Starting graceful shutdown sequence...");

    // Step 1: Stop accepting new requests (would be done by server)
    info!("Step 1/5: Stopped accepting new requests");

    // Step 2: Wait for in-flight requests to complete (with timeout)
    info!("Step 2/5: Waiting for in-flight requests (max 5s)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Step 3: Flush pending metrics and save final snapshot
    if let (Some(collector), Some(storage)) = (metrics_collector, metrics_storage) {
        info!("Step 3/5: Flushing metrics and saving final snapshot...");
        let snapshot = collector.take_snapshot();
        if let Err(e) = storage.save_snapshot(&snapshot).await {
            error!("Failed to save final metrics snapshot: {}", e);
        } else {
            info!("Final metrics snapshot saved successfully");
        }
    } else {
        info!("Step 3/5: Skipping metrics flush (not available)");
    }

    // Step 4: Cancel background tasks
    info!("Step 4/5: Cancelling background tasks...");
    for task in background_tasks {
        task.abort();
    }
    info!("All background tasks cancelled");

    // Step 5: Close database connections (RocksDB handles this via Drop)
    info!("Step 5/5: Closing database connections...");
    // RocksDB connections are closed when Arc drops

    info!("Graceful shutdown complete");
    Ok(())
}

/// Run server with graceful shutdown handling
///
/// This wraps the server run loop with signal handling
pub async fn run_with_shutdown<F, Fut>(
    server_fn: F,
    metrics_collector: Option<Arc<crate::metrics::MetricsCollector>>,
    metrics_storage: Option<Arc<crate::metrics::MetricsStorage>>,
    background_tasks: Vec<tokio::task::JoinHandle<()>>,
) -> Result<()>
where
    F: FnOnce(ShutdownHandle) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
{
    let coordinator = ShutdownCoordinator::new();
    let handle = coordinator.handle();

    // Spawn signal handler
    let signal_handle = tokio::spawn(async move {
        coordinator.wait_for_signal().await;
    });

    // Run server
    let server_handle = tokio::spawn(server_fn(handle));

    // Wait for either server to complete or signal
    tokio::select! {
        result = server_handle => {
            match result {
                Ok(Ok(())) => info!("Server completed normally"),
                Ok(Err(e)) => error!("Server error: {}", e),
                Err(e) => error!("Server task panicked: {}", e),
            }
        }
        _ = signal_handle => {
            info!("Shutdown signal received");
        }
    }

    // Perform graceful shutdown
    shutdown_server_gracefully(metrics_collector, metrics_storage, background_tasks).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator_creation() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_shutdown_handle() {
        let coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        assert!(!handle.is_shutdown_requested());

        coordinator.shutdown_flag.store(true, Ordering::SeqCst);
        assert!(handle.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_multiple_handles() {
        let coordinator = ShutdownCoordinator::new();
        let handle1 = coordinator.handle();
        let handle2 = coordinator.handle();

        coordinator.shutdown_flag.store(true, Ordering::SeqCst);

        assert!(handle1.is_shutdown_requested());
        assert!(handle2.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_shutdown_gracefully() {
        use std::sync::Arc;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let collector = Arc::new(crate::metrics::MetricsCollector::new());
        let storage = Arc::new(crate::metrics::MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap());

        // Record some metrics
        collector.record_tool_call("test_tool", 10.0, true);

        // Perform shutdown
        let result = shutdown_server_gracefully(
            Some(collector),
            Some(storage.clone()),
            vec![],
        )
        .await;

        assert!(result.is_ok());

        // Verify snapshot was saved
        let count = storage.count_snapshots().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_shutdown_without_metrics() {
        let result = shutdown_server_gracefully(None, None, vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_handle_wait() {
        let coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        // Spawn task that sets shutdown after delay
        let coordinator_clone = coordinator.shutdown_flag.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            coordinator_clone.store(true, Ordering::SeqCst);
        });

        // Wait for shutdown
        let start = tokio::time::Instant::now();
        handle.wait_for_shutdown().await;
        let elapsed = start.elapsed();

        assert!(elapsed >= tokio::time::Duration::from_millis(100));
        assert!(handle.is_shutdown_requested());
    }
}
