//! Message Bus for agent communication

use super::*;
use crate::agents::AgentId;

pub struct MessageBus {
    channels: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<Message>>>>,
    topics: Arc<RwLock<HashMap<String, broadcast::Sender<Message>>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            topics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_agent(&self, agent_id: AgentId) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.write().await.insert(agent_id, tx);
        rx
    }

    pub async fn send(&self, target: AgentId, message: Message) -> Result<()> {
        let channels = self.channels.read().await;
        let tx = channels
            .get(&target)
            .ok_or_else(|| CoordinationError::AgentNotFound(target.to_string()))?;

        tx.send(message).map_err(|_| CoordinationError::SendFailed {
            target: target.to_string(),
        })?;

        Ok(())
    }

    pub async fn publish(&self, topic: String, message: Message) -> Result<usize> {
        let tx = self.get_or_create_topic(topic.clone()).await;
        let count = tx.receiver_count();

        tx.send(message).map_err(|_| CoordinationError::PublishFailed {
            topic: topic.clone(),
        })?;

        Ok(count)
    }

    pub async fn subscribe(&self, topic: String) -> broadcast::Receiver<Message> {
        let tx = self.get_or_create_topic(topic).await;
        tx.subscribe()
    }

    async fn get_or_create_topic(&self, topic: String) -> broadcast::Sender<Message> {
        let mut topics = self.topics.write().await;

        if let Some(tx) = topics.get(&topic) {
            tx.clone()
        } else {
            let (tx, _) = broadcast::channel(1000);
            topics.insert(topic, tx.clone());
            tx
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    TaskAssignment {
        task_id: String,
        agent_id: AgentId,
    },
    TaskProgress {
        task_id: String,
        progress: f32,
    },
    TaskComplete {
        task_id: String,
        result: serde_json::Value,
    },
    SystemEvent {
        event_type: String,
        data: serde_json::Value,
    },
}
