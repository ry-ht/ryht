//! Model routing for optimal provider selection

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
    pub confidence: f32,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub enum ModelRequirements {
    LowestCost,
    FastestResponse,
    HighestQuality,
    Balanced,
}

pub struct ModelRouter {
    cache: Arc<RwLock<HashMap<String, ModelSelection>>>,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn select_model(
        &self,
        task_type: &str,
        requirements: ModelRequirements,
    ) -> Result<ModelSelection> {
        // Check cache
        if let Some(cached) = self.cache.read().await.get(task_type) {
            return Ok(cached.clone());
        }

        // Select based on requirements
        let selection = match requirements {
            ModelRequirements::LowestCost => ModelSelection {
                provider_id: "openai".to_string(),
                model_id: "gpt-3.5-turbo".to_string(),
                confidence: 0.8,
                rationale: "Lowest cost option".to_string(),
            },
            ModelRequirements::HighestQuality => ModelSelection {
                provider_id: "anthropic".to_string(),
                model_id: "claude-3-opus".to_string(),
                confidence: 0.9,
                rationale: "Highest quality for complex tasks".to_string(),
            },
            _ => ModelSelection {
                provider_id: "openai".to_string(),
                model_id: "gpt-4-turbo".to_string(),
                confidence: 0.85,
                rationale: "Balanced performance".to_string(),
            },
        };

        // Cache result
        self.cache
            .write()
            .await
            .insert(task_type.to_string(), selection.clone());

        Ok(selection)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}
