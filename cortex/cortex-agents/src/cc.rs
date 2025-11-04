//! Claude CLI integration stubs
//!
//! TODO: Implement actual Claude CLI integration in future phases

use serde::{Deserialize, Serialize};
use futures::Stream;
use std::pin::Pin;

/// Claude CLI query options
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub system_prompt: Option<String>,
}

impl ClaudeCodeOptions {
    pub fn builder() -> ClaudeCodeOptionsBuilder {
        ClaudeCodeOptionsBuilder::default()
    }
}

/// Builder for ClaudeCodeOptions
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptionsBuilder {
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    system_prompt: Option<String>,
}

impl ClaudeCodeOptionsBuilder {
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn build(self) -> ClaudeCodeOptions {
        ClaudeCodeOptions {
            model: self.model,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            system_prompt: self.system_prompt,
        }
    }
}

/// Options module for compatibility
pub mod options {
    // SystemPrompt type for compatibility - agents just pass strings
    // so we don't need a complex enum
}

/// Message for Claude CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Content block in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    pub text: String,
}

/// Messages module
pub mod messages {
    pub use super::ContentBlock;
}

/// Query Claude CLI (stub implementation)
pub async fn query(
    _prompt: &str,
    _options: ClaudeCodeOptions,
) -> Result<Pin<Box<dyn Stream<Item = Result<ContentBlock, String>> + Send>>, String> {
    // TODO: Implement actual Claude CLI integration
    Err("Claude CLI integration not yet implemented".to_string())
}
