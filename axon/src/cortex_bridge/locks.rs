//! Lock management for agent coordination
//!
//! This module provides distributed locking mechanisms for coordinating
//! multiple agents working on shared resources.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct AcquireLockRequest {
    pub entity_id: String,
    pub lock_type: String,
    pub agent_id: String,
    pub session_id: String,
    pub scope: String,
    pub timeout: u32,
    pub wait: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcquireLockResponse {
    pub lock_id: String,
    pub acquired_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockStatus {
    pub lock_id: String,
    pub entity_id: String,
    pub lock_type: String,
    pub agent_id: String,
    pub session_id: String,
    pub acquired_at: String,
    pub expires_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListLocksResponse {
    pub locks: Vec<LockStatus>,
}

// ============================================================================
// Lock Manager
// ============================================================================

/// Lock manager for distributed coordination
pub struct LockManager {
    client: CortexClient,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Acquire a lock on an entity
    pub async fn acquire_lock(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<LockId> {
        self.acquire_lock_with_options(
            entity_id,
            lock_type,
            agent_id,
            session_id,
            300, // 5 minutes default timeout
            true, // Wait for lock
        )
        .await
    }

    /// Acquire a lock with custom options
    pub async fn acquire_lock_with_options(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
        timeout: u32,
        wait: bool,
    ) -> Result<LockId> {
        let request = AcquireLockRequest {
            entity_id: entity_id.to_string(),
            lock_type: lock_type.to_string(),
            agent_id: agent_id.0.clone(),
            session_id: session_id.0.clone(),
            scope: "entity".to_string(),
            timeout,
            wait,
        };

        let response: AcquireLockResponse = self.client.post("/locks", &request).await?;

        let lock_id = LockId::from(response.lock_id);
        info!(
            "Acquired {} lock {} on entity {} for agent {}",
            lock_type, lock_id, entity_id, agent_id
        );

        Ok(lock_id)
    }

    /// Try to acquire a lock without waiting
    pub async fn try_acquire_lock(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<Option<LockId>> {
        match self
            .acquire_lock_with_options(entity_id, lock_type, agent_id, session_id, 0, false)
            .await
        {
            Ok(lock_id) => Ok(Some(lock_id)),
            Err(e) => {
                warn!("Failed to acquire lock immediately: {}", e);
                Ok(None)
            }
        }
    }

    /// Release a lock
    pub async fn release_lock(&self, lock_id: &LockId) -> Result<()> {
        let path = format!("/locks/{}", lock_id);
        let _: serde_json::Value = self.client.delete(&path).await?;

        info!("Released lock {}", lock_id);
        Ok(())
    }

    /// Get lock status
    pub async fn get_lock_status(&self, lock_id: &LockId) -> Result<LockStatus> {
        let path = format!("/locks/{}", lock_id);
        let status: LockStatus = self.client.get(&path).await?;

        Ok(status)
    }

    /// List all active locks
    pub async fn list_locks(&self) -> Result<Vec<LockStatus>> {
        let response: ListLocksResponse = self.client.get("/locks").await?;
        Ok(response.locks)
    }

    /// List locks for a specific agent
    pub async fn list_agent_locks(&self, agent_id: &AgentId) -> Result<Vec<LockStatus>> {
        let path = format!("/locks?agent_id={}", agent_id);
        let response: ListLocksResponse = self.client.get(&path).await?;

        Ok(response.locks)
    }

    /// List locks for a specific entity
    pub async fn list_entity_locks(&self, entity_id: &str) -> Result<Vec<LockStatus>> {
        let path = format!("/locks?entity_id={}", urlencoding::encode(entity_id));
        let response: ListLocksResponse = self.client.get(&path).await?;

        Ok(response.locks)
    }

    /// Check if an entity is locked
    pub async fn is_locked(&self, entity_id: &str) -> Result<bool> {
        let locks = self.list_entity_locks(entity_id).await?;
        Ok(!locks.is_empty())
    }

    /// Extend lock timeout
    pub async fn extend_lock(&self, lock_id: &LockId, additional_seconds: u32) -> Result<()> {
        #[derive(Serialize)]
        struct ExtendLockRequest {
            additional_seconds: u32,
        }

        let request = ExtendLockRequest {
            additional_seconds,
        };

        let path = format!("/locks/{}/extend", lock_id);
        let _: serde_json::Value = self.client.put(&path, &request).await?;

        info!("Extended lock {} by {} seconds", lock_id, additional_seconds);
        Ok(())
    }

    /// Release all locks for an agent
    pub async fn release_agent_locks(&self, agent_id: &AgentId) -> Result<u32> {
        let locks = self.list_agent_locks(agent_id).await?;
        let count = locks.len() as u32;

        for lock in locks {
            if let Err(e) = self.release_lock(&LockId(lock.lock_id)).await {
                warn!("Failed to release lock: {}", e);
            }
        }

        info!("Released {} locks for agent {}", count, agent_id);
        Ok(count)
    }

    /// Release all locks for a session
    pub async fn release_session_locks(&self, session_id: &SessionId) -> Result<u32> {
        let path = format!("/locks?session_id={}", session_id);
        let response: ListLocksResponse = self.client.get(&path).await?;
        let count = response.locks.len() as u32;

        for lock in response.locks {
            if let Err(e) = self.release_lock(&LockId(lock.lock_id)).await {
                warn!("Failed to release lock: {}", e);
            }
        }

        info!("Released {} locks for session {}", count, session_id);
        Ok(count)
    }
}

/// RAII guard for automatic lock release
pub struct LockGuard {
    lock_id: LockId,
    lock_manager: LockManager,
    released: bool,
}

impl LockGuard {
    /// Create a new lock guard
    pub fn new(lock_id: LockId, lock_manager: LockManager) -> Self {
        Self {
            lock_id,
            lock_manager,
            released: false,
        }
    }

    /// Get the lock ID
    pub fn lock_id(&self) -> &LockId {
        &self.lock_id
    }

    /// Manually release the lock
    pub async fn release(mut self) -> Result<()> {
        if !self.released {
            self.lock_manager.release_lock(&self.lock_id).await?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if !self.released {
            warn!(
                "LockGuard dropped without explicit release for lock {}",
                self.lock_id
            );
            // Note: Can't use async in Drop, so we just log a warning
            // The lock will expire based on its timeout
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cortex_bridge::CortexConfig;

    #[test]
    fn test_lock_type_display() {
        assert_eq!(LockType::Shared.to_string(), "shared");
        assert_eq!(LockType::Exclusive.to_string(), "exclusive");
    }

    #[test]
    fn test_acquire_lock_request_creation() {
        let request = AcquireLockRequest {
            entity_id: "test_entity".to_string(),
            lock_type: "shared".to_string(),
            agent_id: "agent1".to_string(),
            session_id: "session1".to_string(),
            scope: "entity".to_string(),
            timeout: 300,
            wait: true,
        };

        assert_eq!(request.entity_id, "test_entity");
        assert_eq!(request.lock_type, "shared");
        assert!(request.wait);
    }

    #[test]
    fn test_lock_guard_creation() {
        let lock_id = LockId("test_lock".to_string());
        let client = CortexClient::new(CortexConfig::default()).unwrap();
        let lock_manager = LockManager::new(client);
        let guard = LockGuard::new(lock_id.clone(), lock_manager);

        assert_eq!(guard.lock_id(), &lock_id);
        assert!(!guard.released);
    }
}
