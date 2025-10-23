//! Comprehensive Infrastructure and Ingestion Tests
//!
//! This test module contains comprehensive self-tests for Cortex infrastructure
//! and ingestion capabilities.
//!
//! Test Categories:
//! - Infrastructure Tests: SurrealDB, connection pooling, VFS, memory systems
//! - Ingestion Tests: Load entire Cortex project, parse all files, build semantic graph
//!
//! Run these tests with:
//! ```bash
//! cargo test --test comprehensive_tests
//! cargo test --test comprehensive_tests -- --ignored  # For long-running tests
//! ```

#[path = "mcp/comprehensive/mod.rs"]
mod comprehensive;
