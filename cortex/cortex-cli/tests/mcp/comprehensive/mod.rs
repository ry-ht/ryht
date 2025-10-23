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
//! ```

pub mod infrastructure_tests;
pub mod ingestion_tests;
