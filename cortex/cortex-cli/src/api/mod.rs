//! REST API module for Cortex
//!
//! Provides HTTP/REST API endpoints for all Cortex functionality.

pub mod server;
pub mod routes;
pub mod middleware;
pub mod types;
pub mod error;
pub mod websocket;
pub mod db_schema;

#[cfg(test)]
mod tests;

pub use server::RestApiServer;
pub use types::{ApiResponse, ApiMetadata};
pub use error::{ApiError, ApiResult};
pub use websocket::{WsManager, WsEvent};
