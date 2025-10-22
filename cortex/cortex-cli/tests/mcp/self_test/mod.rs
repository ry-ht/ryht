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
//! ### Phase 2: Deep Analysis (Future)
//! - Semantic search within cortex codebase
//! - Cross-reference validation
//! - Complex dependency queries
//!
//! ### Phase 3: Self-Modification (Future)
//! - Generate documentation for cortex
//! - Suggest improvements to cortex code
//! - Refactor cortex modules
//!
//! ## Running the Tests
//!
//! These tests are ignored by default due to their comprehensive nature.
//! Run them explicitly with:
//!
//! ```bash
//! cargo test --test phase1_ingestion -- --ignored --nocapture
//! ```
//!
//! The `--nocapture` flag is recommended to see the detailed progress reports.

pub mod phase1_ingestion;
