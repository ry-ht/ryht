//! End-to-End Workflow Tests
//!
//! This module contains comprehensive E2E tests that simulate real development workflows
//! using Cortex MCP tools. Each workflow demonstrates:
//!
//! - Complete task execution from start to finish
//! - Multiple MCP tools working in sequence
//! - Real code manipulation, navigation, and testing
//! - Token efficiency measurements vs traditional approaches
//! - Detailed logging and metrics
//!
//! ## Available Workflows
//!
//! ### 1. Add Feature (`workflow_add_feature.rs`)
//! Simulates adding an authentication feature to an existing service:
//! - Create workspace and import project
//! - Analyze existing auth module structure
//! - Create new login and registration functions
//! - Generate comprehensive tests
//! - Generate documentation
//! - Export and verify compilation
//!
//! **Key Metrics:**
//! - 9 workflow steps
//! - 50-60% token savings
//! - Automated test and doc generation
//!
//! ### 2. Fix Bug (`workflow_fix_bug.rs`)
//! Simulates fixing an off-by-one bug in batch processing:
//! - Search for bug-related code using semantic search
//! - Navigate to problematic function
//! - Analyze dependencies and call sites
//! - Apply precise code fix
//! - Run tests to verify fix
//! - Check code quality improvements
//!
//! **Key Metrics:**
//! - 8 workflow steps
//! - 40-50% token savings
//! - Faster bug location and fixing
//!
//! ### 3. Refactor Module (`workflow_refactor_module.rs`)
//! Simulates refactoring a monolithic calculator into organized modules:
//! - Analyze code structure and dependencies
//! - Extract functions to separate modules
//! - Rename symbols for better clarity
//! - Update all references automatically
//! - Reorganize imports
//! - Verify no regressions via tests
//!
//! **Key Metrics:**
//! - 10 workflow steps
//! - 3+ modules created
//! - 45-55% token savings
//! - Improved code organization
//!
//! ### 4. Add Tests (`workflow_add_tests.rs`)
//! Simulates adding comprehensive test coverage to under-tested code:
//! - Identify untested functions
//! - Generate unit tests for core functions
//! - Create integration tests
//! - Add property-based tests for edge cases
//! - Create test fixtures and helpers
//! - Measure coverage improvements
//!
//! **Key Metrics:**
//! - 9 workflow steps
//! - 15+ tests generated
//! - Coverage improvement from ~20% to >80%
//! - 60-70% token savings
//!
//! ## Usage
//!
//! Run all E2E workflow tests:
//! ```bash
//! cargo test --test '*' e2e::
//! ```
//!
//! Run specific workflow:
//! ```bash
//! cargo test --test '*' test_workflow_add_authentication_feature
//! cargo test --test '*' test_workflow_fix_off_by_one_bug
//! cargo test --test '*' test_workflow_refactor_monolithic_module
//! cargo test --test '*' test_workflow_add_comprehensive_test_coverage
//! ```
//!
//! ## Efficiency Analysis
//!
//! All workflows measure token efficiency by comparing:
//! - **Traditional Approach**: Manual file reading, editing, testing
//! - **Cortex MCP Approach**: Targeted tool usage with semantic understanding
//!
//! Expected token savings range from 40% to 70% depending on workflow complexity.
//!
//! ## Test Structure
//!
//! Each workflow test follows this pattern:
//!
//! 1. **Setup**: Create realistic project with TempDir
//! 2. **Metrics**: Initialize workflow metrics tracker
//! 3. **Steps**: Execute 8-10 sequential workflow steps
//! 4. **Verification**: Validate results at each step
//! 5. **Summary**: Print comprehensive metrics and efficiency analysis
//!
//! ## Implementation Details
//!
//! - Uses real MCP tools (not mocks)
//! - Creates actual file structures and code
//! - Measures real token usage
//! - Validates exported code compiles
//! - Tracks execution time for each step
//!
//! ## Benefits Demonstrated
//!
//! These workflows showcase Cortex MCP's advantages:
//! - **Precision**: Targeted code changes without full file reads
//! - **Intelligence**: Semantic search and dependency analysis
//! - **Automation**: Auto-generated tests and documentation
//! - **Efficiency**: Significant token savings
//! - **Safety**: Verification at each step
//! - **Speed**: Faster development cycles

mod workflow_add_feature;
mod workflow_fix_bug;
mod workflow_refactor_module;
mod workflow_add_tests;
