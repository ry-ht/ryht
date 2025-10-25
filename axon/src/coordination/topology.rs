//! Network topology management

use super::*;
use crate::agents::AgentId;

#[derive(Debug, Clone)]
pub enum Topology {
    Star {
        coordinator: AgentId,
        workers: Vec<AgentId>,
    },
    Mesh {
        nodes: Vec<AgentId>,
    },
    Pipeline {
        stages: Vec<AgentId>,
    },
}

pub struct TopologyManager {
    current: Arc<RwLock<Topology>>,
}

impl TopologyManager {
    pub fn new() -> Self {
        Self {
            current: Arc::new(RwLock::new(Topology::Mesh { nodes: Vec::new() })),
        }
    }

    pub async fn set_topology(&self, topology: Topology) {
        *self.current.write().await = topology;
    }

    pub async fn get_topology(&self) -> Topology {
        self.current.read().await.clone()
    }
}
