//! Cortex Query Tool

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CortexQueryInput {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct CortexQueryOutput {
    pub results: Vec<serde_json::Value>,
}

pub struct CortexQueryTool;

impl CortexQueryTool {
    pub async fn query(&self, _input: CortexQueryInput) -> Result<CortexQueryOutput> {
        Ok(CortexQueryOutput {
            results: vec![],
        })
    }
}
