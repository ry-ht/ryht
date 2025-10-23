//! Comprehensive Infrastructure and Ingestion Tests
//!
//! This module contains comprehensive self-tests for Cortex infrastructure and
//! the ability to ingest and understand its own codebase.
//!
//! ## Test Categories
//!
//! ### Infrastructure Tests (`infrastructure_tests.rs`)
//! Tests core infrastructure components:
//! - SurrealDB lifecycle (start, stop, restart, health checks)
//! - Connection pooling (round-robin, least-loaded, sticky)
//! - VFS initialization and cache management
//! - Memory system initialization (5 tiers)
//! - Semantic search initialization (embeddings, HNSW index)
//! - Configuration management
//! - Error recovery mechanisms
//!
//! ### Ingestion Tests (`ingestion_tests.rs`)
//! Tests loading entire Cortex project:
//! - Load all 8 Cortex crates into VFS
//! - Parse 300+ Rust files with tree-sitter
//! - Build complete semantic graph
//! - Generate embeddings for all code units
//! - Populate all 5 memory tiers
//! - Verify statistics (file count, LOC, functions, structs)
//! - Test incremental loading and updates
//! - Measure ingestion performance (<60 seconds target)
//!
//! ### Self-Modification Tests (`self_modification_tests.rs`)
//! The ultimate tests - Cortex modifying and improving itself:
//! - Add new MCP tools to its own codebase
//! - Optimize its own performance
//! - Detect and fix bugs in itself
//! - Improve its own architecture
//! - Add tests to itself
//! - Enhance its own documentation
//! - Upgrade its own dependencies
//! - Multi-agent collaborative self-improvement
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all comprehensive tests
//! cargo test --test comprehensive_tests
//!
//! # Run with output
//! cargo test --test comprehensive_tests -- --nocapture
//!
//! # Run long-running tests (marked with #[ignore])
//! cargo test --test comprehensive_tests -- --ignored --nocapture
//!
//! # Run specific test
//! cargo test --test comprehensive_tests test_surrealdb_lifecycle -- --nocapture
//!
//! # Run self-modification tests
//! export PATH=/Users/taaliman/.cargo/bin:/usr/local/bin:/bin:/usr/bin:$PATH
//! cargo test --test comprehensive_tests self_modification -- --ignored --nocapture
//! ```

pub mod infrastructure_tests;
pub mod ingestion_tests;
pub mod rust_development_tests;
pub mod typescript_development_tests;
pub mod self_modification_tests;
pub mod tool_tests;
pub mod advanced_tool_tests;
pub mod integration_tests;
pub mod performance_tests;
pub mod materialization_tests;
pub mod correctness_tests;
pub mod stress_tests;
