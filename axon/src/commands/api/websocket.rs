//! WebSocket support for real-time agent updates

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    /// Agent status changed
    AgentStatusChange {
        agent_id: String,
        agent_name: String,
        status: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Agent created
    AgentCreated {
        agent_id: String,
        agent_name: String,
        agent_type: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Agent stopped
    AgentStopped {
        agent_id: String,
        agent_name: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Workflow progress update
    WorkflowProgress {
        workflow_id: String,
        workflow_name: String,
        status: String,
        progress: u8,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Workflow completed
    WorkflowCompleted {
        workflow_id: String,
        workflow_name: String,
        success: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// System metrics update
    MetricsUpdate {
        active_agents: usize,
        running_workflows: usize,
        total_tasks: usize,
        cpu_usage: f32,
        memory_usage: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Task update
    TaskUpdate {
        task_id: String,
        agent_id: String,
        status: String,
        progress: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// System alert
    SystemAlert {
        level: String, // "info", "warning", "error"
        message: String,
        component: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// WebSocket subscription message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsSubscriptionMessage {
    /// Subscribe to channels
    Subscribe { channels: Vec<String> },
    /// Unsubscribe from channels
    Unsubscribe { channels: Vec<String> },
    /// Ping for keepalive
    Ping,
}

/// WebSocket message to client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    /// Event notification
    Event { channel: String, event: WsEvent },
    /// Subscription confirmation
    Subscribed { channels: Vec<String> },
    /// Unsubscription confirmation
    Unsubscribed { channels: Vec<String> },
    /// Pong response
    Pong,
    /// Error message
    Error { message: String },
}

/// Connection info
#[derive(Debug, Clone)]
struct ConnectionInfo {
    id: String,
    user_id: Option<String>,
    subscribed_channels: Vec<String>,
}

/// WebSocket manager
#[derive(Clone)]
pub struct WsManager {
    /// Broadcast channel for events
    event_tx: broadcast::Sender<(String, WsEvent)>,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
}

impl WsManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            event_tx,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Broadcast an event to a specific channel
    pub async fn broadcast(&self, channel: &str, event: WsEvent) {
        if let Err(e) = self.event_tx.send((channel.to_string(), event)) {
            error!("Failed to broadcast event: {}", e);
        }
    }

    /// Get active connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get subscribers for a channel
    pub async fn channel_subscribers(&self, channel: &str) -> usize {
        self.connections
            .read()
            .await
            .values()
            .filter(|conn| conn.subscribed_channels.contains(&channel.to_string()))
            .count()
    }
}

/// WebSocket context
#[derive(Clone)]
pub struct WsContext {
    pub manager: WsManager,
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<WsContext>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, ctx))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, ctx: WsContext) {
    let connection_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", connection_id);

    let (sender, receiver) = socket.split();

    let conn_info = ConnectionInfo {
        id: connection_id.clone(),
        user_id: None, // Can be set via authentication
        subscribed_channels: Vec::new(),
    };

    ctx.manager
        .connections
        .write()
        .await
        .insert(connection_id.clone(), conn_info);

    let event_rx = ctx.manager.event_tx.subscribe();

    let send_task = tokio::spawn(send_events(
        sender,
        event_rx,
        connection_id.clone(),
        ctx.manager.clone(),
    ));

    let recv_task = tokio::spawn(receive_messages(
        receiver,
        connection_id.clone(),
        ctx.manager.clone(),
    ));

    tokio::select! {
        _ = send_task => {
            debug!("Send task completed for connection {}", connection_id);
        }
        _ = recv_task => {
            debug!("Receive task completed for connection {}", connection_id);
        }
    }

    ctx.manager.connections.write().await.remove(&connection_id);
    info!("WebSocket connection closed: {}", connection_id);
}

/// Send events to client
async fn send_events(
    mut sender: SplitSink<WebSocket, Message>,
    mut event_rx: broadcast::Receiver<(String, WsEvent)>,
    connection_id: String,
    manager: WsManager,
) {
    loop {
        match event_rx.recv().await {
            Ok((channel, event)) => {
                let subscribed = {
                    let connections = manager.connections.read().await;
                    connections
                        .get(&connection_id)
                        .map(|conn| conn.subscribed_channels.contains(&channel))
                        .unwrap_or(false)
                };

                if subscribed {
                    let message = WsClientMessage::Event {
                        channel: channel.clone(),
                        event,
                    };

                    if let Ok(json) = serde_json::to_string(&message) {
                        if let Err(e) = sender.send(Message::Text(json.into())).await {
                            error!("Failed to send message to {}: {}", connection_id, e);
                            break;
                        }
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("Connection {} lagged by {} messages", connection_id, n);
            }
            Err(broadcast::error::RecvError::Closed) => {
                debug!("Event channel closed for connection {}", connection_id);
                break;
            }
        }
    }
}

/// Receive messages from client
async fn receive_messages(
    mut receiver: SplitStream<WebSocket>,
    connection_id: String,
    manager: WsManager,
) {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_client_message(&text, &connection_id, &manager).await {
                    error!("Error handling message from {}: {}", connection_id, e);
                }
            }
            Ok(Message::Close(_)) => {
                debug!("Received close message from {}", connection_id);
                break;
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                debug!("Received ping/pong from {}", connection_id);
            }
            Ok(_) => {}
            Err(e) => {
                error!("WebSocket error for {}: {}", connection_id, e);
                break;
            }
        }
    }
}

/// Handle client subscription messages
async fn handle_client_message(
    text: &str,
    connection_id: &str,
    manager: &WsManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg: WsSubscriptionMessage = serde_json::from_str(text)?;

    match msg {
        WsSubscriptionMessage::Subscribe { channels } => {
            debug!("Connection {} subscribing to {:?}", connection_id, channels);

            let mut connections = manager.connections.write().await;
            if let Some(conn) = connections.get_mut(connection_id) {
                for channel in &channels {
                    if !conn.subscribed_channels.contains(channel) {
                        conn.subscribed_channels.push(channel.clone());
                    }
                }
            }

            info!(
                "Connection {} subscribed to {} channels",
                connection_id,
                channels.len()
            );
        }
        WsSubscriptionMessage::Unsubscribe { channels } => {
            debug!(
                "Connection {} unsubscribing from {:?}",
                connection_id, channels
            );

            let mut connections = manager.connections.write().await;
            if let Some(conn) = connections.get_mut(connection_id) {
                conn.subscribed_channels.retain(|c| !channels.contains(c));
            }

            info!(
                "Connection {} unsubscribed from {} channels",
                connection_id,
                channels.len()
            );
        }
        WsSubscriptionMessage::Ping => {
            debug!("Received ping from {}", connection_id);
        }
    }

    Ok(())
}

/// Channel helper functions
pub mod channels {
    /// All agents channel
    pub fn agents() -> String {
        "agents".to_string()
    }

    /// Specific agent channel
    pub fn agent(agent_id: &str) -> String {
        format!("agent:{}", agent_id)
    }

    /// All workflows channel
    pub fn workflows() -> String {
        "workflows".to_string()
    }

    /// Specific workflow channel
    pub fn workflow(workflow_id: &str) -> String {
        format!("workflow:{}", workflow_id)
    }

    /// System metrics channel
    pub fn metrics() -> String {
        "metrics".to_string()
    }

    /// System alerts channel
    pub fn alerts() -> String {
        "alerts".to_string()
    }

    /// Tasks channel
    pub fn tasks() -> String {
        "tasks".to_string()
    }

    /// User-specific channel
    pub fn user(user_id: &str) -> String {
        format!("user:{}", user_id)
    }
}

/// Create WebSocket routes
pub fn websocket_routes(manager: WsManager) -> Router {
    let ctx = WsContext { manager };

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(ctx)
}
