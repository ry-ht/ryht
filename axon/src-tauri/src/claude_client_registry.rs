use cc_sdk::{ClaudeClient, core::state::Connected};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Registry for managing active Claude client sessions
///
/// This registry keeps track of all active ClaudeClient instances,
/// allowing us to:
/// - Reuse clients across multiple commands
/// - Track active sessions
/// - Clean up clients when sessions complete
pub struct ClaudeClientRegistry {
    /// Map of session_id to ClaudeClient
    clients: Arc<Mutex<HashMap<String, Arc<ClaudeClient<Connected>>>>>,
}

impl ClaudeClientRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new client with the given session ID
    ///
    /// If a client with this session ID already exists, it will be replaced
    pub async fn register_client(&self, session_id: String, client: Arc<ClaudeClient<Connected>>) {
        let mut clients = self.clients.lock().await;
        log::info!("Registering Claude client for session: {}", session_id);
        clients.insert(session_id, client);
    }

    /// Get a client by session ID
    ///
    /// Returns None if no client exists for this session
    pub async fn get_client(&self, session_id: &str) -> Option<Arc<ClaudeClient<Connected>>> {
        let clients = self.clients.lock().await;
        clients.get(session_id).cloned()
    }

    /// Remove a client from the registry
    ///
    /// This should be called when a session completes or is cancelled
    pub async fn remove_client(&self, session_id: &str) -> Option<Arc<ClaudeClient<Connected>>> {
        let mut clients = self.clients.lock().await;
        log::info!("Removing Claude client for session: {}", session_id);
        clients.remove(session_id)
    }

    /// List all active session IDs
    pub async fn list_active_sessions(&self) -> Vec<String> {
        let clients = self.clients.lock().await;
        clients.keys().cloned().collect()
    }

    /// Get the number of active sessions
    pub async fn active_count(&self) -> usize {
        let clients = self.clients.lock().await;
        clients.len()
    }

    /// Clear all clients from the registry
    ///
    /// This is primarily for cleanup purposes
    pub async fn clear(&self) {
        let mut clients = self.clients.lock().await;
        log::info!("Clearing all Claude clients from registry");
        clients.clear();
    }
}

impl Default for ClaudeClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tauri state wrapper for ClaudeClientRegistry
pub struct ClaudeClientRegistryState(pub Arc<ClaudeClientRegistry>);

impl Default for ClaudeClientRegistryState {
    fn default() -> Self {
        Self(Arc::new(ClaudeClientRegistry::new()))
    }
}

impl ClaudeClientRegistryState {
    /// Create a new registry state
    pub fn new() -> Self {
        Self::default()
    }
}
