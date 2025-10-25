//! Axon - Multi-Agent System Framework
//!
//! Axon provides a comprehensive framework for building, orchestrating, and managing
//! multi-agent systems with advanced coordination patterns, consensus mechanisms,
//! and integration with Claude Code SDK.
//!
//! # Architecture
//!
//! - `cc` - Claude Code SDK integration for agent communication
//! - `agents` - Agent types and implementations
//! - `orchestration` - Orchestration engine for coordinating agents
//! - `coordination` - Coordination patterns and protocols
//! - `consensus` - Consensus mechanisms for multi-agent decisions
//! - `intelligence` - Intelligence layer for agent reasoning
//! - `monitoring` - Performance monitoring and optimization
//! - `quality` - Quality assurance and validation

#![warn(missing_docs)]

// Claude Code SDK module (original cc-sdk code)
pub mod cc;

// Multi-agent system modules
pub mod agents;
pub mod orchestration;
pub mod coordination;
pub mod consensus;
pub mod intelligence;
pub mod monitoring;
pub mod quality;

// Re-export key types from cc module for backward compatibility
pub use cc::{
    ClaudeClient,
    Error,
    Result,
    prelude::*,
};

// Re-export multi-agent system types
pub use agents::Agent;
pub use orchestration::Orchestrator;
pub use coordination::CoordinationPattern;
pub use consensus::ConsensusProtocol;

/// Axon version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");