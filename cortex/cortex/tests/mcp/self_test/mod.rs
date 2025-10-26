//! Cortex Self-Test Suite
//!
//! These tests validate that cortex can successfully ingest and understand
//! its own codebase - the ultimate proof that all functionality works correctly.
//!
//! ## Test Phases
//!
//! ### Phase 1: Complete Ingestion
//! - Load entire cortex workspace
//! - Parse all Rust files
//! - Extract code units
//! - Build dependency graph
//! - Verify all expected crates found
//!
//! ### Phase 2: Deep Analysis and Navigation
//! - Code navigation (find definitions, references, call hierarchies)
//! - Type hierarchy traversal
//! - Dependency analysis and impact assessment
//! - Circular dependency detection
//! - Semantic search capabilities
//!
//! ### Phase 3: Code Manipulation
//! - Safely modify cortex codebase in VFS
//! - Add/rename/extract functions
//! - Create structs and implement traits
//! - Verify syntax and compilation
//! - Prove code manipulation tools work correctly
//!
//! ## Running the Tests
//!
//! These tests are ignored by default due to their comprehensive nature.
//! Run them explicitly with:
//!
//! ```bash
//! # Run Phase 1 (Ingestion)
//! cargo test --test phase1_ingestion -- --ignored --nocapture
//!
//! # Run Phase 2 (Navigation)
//! cargo test --test phase2_navigation -- --ignored --nocapture
//!
//! # Run Phase 3 (Manipulation)
//! cargo test --test phase3_manipulation -- --ignored --nocapture
//!
//! # Run all phases
//! cargo test --package cortex self_test -- --ignored --nocapture
//! ```
//!
//! The `--nocapture` flag is recommended to see the detailed progress reports.

pub mod phase1_ingestion;
pub mod phase2_navigation;
pub mod phase3_manipulation;
