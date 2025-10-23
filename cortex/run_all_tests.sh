#!/bin/bash

################################################################################
# Cortex Workspace Test Runner
#
# Comprehensive test suite that runs ALL tests in the workspace and verifies
# 100% pass rate with detailed reporting.
#
# Usage:
#   ./run_all_tests.sh [OPTIONS]
#
# Options:
#   --fast         Skip long-running tests (benchmarks, ultimate tests)
#   --verbose      Show detailed test output
#   --package PKG  Test only specific package
#   --clean        Run cargo clean before building
#   --bench        Include benchmark tests
#   --help         Show this help message
#
# Exit Codes:
#   0 - All tests passed
#   1 - One or more tests failed
#   2 - Setup/build error
################################################################################

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

# Export proper PATH
export PATH=/Users/taaliman/.cargo/bin:/usr/local/bin:/bin:/usr/bin:$PATH
export RUST_BACKTRACE=1

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Flags
FAST_MODE=false
VERBOSE=false
SPECIFIC_PACKAGE=""
CLEAN_BUILD=false
RUN_BENCHMARKS=false

# Test tracking
TOTAL_PACKAGES=0
PASSED_PACKAGES=0
FAILED_PACKAGES=0
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
IGNORED_TESTS=0
START_TIME=$(date +%s)

# Failed tests details
declare -a FAILED_PACKAGE_NAMES=()
declare -a FAILED_TEST_DETAILS=()

# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

print_header() {
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${CYAN}$1${NC}"
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_section() {
    echo ""
    echo -e "${BOLD}${BLUE}▶ $1${NC}"
    echo -e "${BLUE}$(date '+%Y-%m-%d %H:%M:%S')${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${CYAN}ℹ${NC} $1"
}

show_help() {
    cat << EOF
Cortex Workspace Test Runner

Usage: $0 [OPTIONS]

Options:
  --fast         Skip long-running tests (benchmarks, ultimate tests)
  --verbose      Show detailed test output
  --package PKG  Test only specific package (e.g., cortex-core)
  --clean        Run cargo clean before building
  --bench        Include benchmark tests
  --help         Show this help message

Examples:
  $0                                    # Run all tests
  $0 --fast                             # Quick test run
  $0 --package cortex-core              # Test single package
  $0 --clean --verbose                  # Clean build with verbose output
  $0 --bench                            # Include benchmarks

Exit Codes:
  0 - All tests passed
  1 - One or more tests failed
  2 - Setup/build error

EOF
    exit 0
}

# Parse test output to extract statistics
parse_test_output() {
    local output="$1"
    local pkg_name="$2"

    # Extract test results from cargo output
    # Format: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
    if echo "$output" | grep -q "test result:"; then
        local result_line=$(echo "$output" | grep "test result:" | tail -1)

        local passed=$(echo "$result_line" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' || echo "0")
        local failed=$(echo "$result_line" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' || echo "0")
        local ignored=$(echo "$result_line" | grep -oE '[0-9]+ ignored' | grep -oE '[0-9]+' || echo "0")

        TOTAL_TESTS=$((TOTAL_TESTS + passed + failed + ignored))
        PASSED_TESTS=$((PASSED_TESTS + passed))
        FAILED_TESTS=$((FAILED_TESTS + failed))
        IGNORED_TESTS=$((IGNORED_TESTS + ignored))

        if [ "$failed" -eq 0 ]; then
            print_success "$pkg_name: $passed passed, $ignored ignored"
            PASSED_PACKAGES=$((PASSED_PACKAGES + 1))
            return 0
        else
            print_error "$pkg_name: $passed passed, $failed failed, $ignored ignored"
            FAILED_PACKAGES=$((FAILED_PACKAGES + 1))
            FAILED_PACKAGE_NAMES+=("$pkg_name")

            # Extract failed test names
            local failed_tests=$(echo "$output" | grep -E "^test .* \.\.\. FAILED$" | sed 's/test //' | sed 's/ \.\.\. FAILED//' || echo "")
            if [ -n "$failed_tests" ]; then
                FAILED_TEST_DETAILS+=("$pkg_name: $failed_tests")
            fi
            return 1
        fi
    else
        print_warning "$pkg_name: Could not parse test output"
        return 1
    fi
}

# Run a cargo test command
run_test() {
    local test_name="$1"
    shift
    local cmd=("$@")

    TOTAL_PACKAGES=$((TOTAL_PACKAGES + 1))

    print_section "Testing: $test_name"

    if [ "$VERBOSE" = true ]; then
        echo -e "${CYAN}Command: ${cmd[*]}${NC}"
        if "${cmd[@]}"; then
            print_success "PASSED: $test_name"
            PASSED_PACKAGES=$((PASSED_PACKAGES + 1))
            return 0
        else
            print_error "FAILED: $test_name"
            FAILED_PACKAGES=$((FAILED_PACKAGES + 1))
            FAILED_PACKAGE_NAMES+=("$test_name")
            return 1
        fi
    else
        local output
        if output=$("${cmd[@]}" 2>&1); then
            parse_test_output "$output" "$test_name"
            return $?
        else
            print_error "FAILED: $test_name"
            echo "$output" | tail -20
            FAILED_PACKAGES=$((FAILED_PACKAGES + 1))
            FAILED_PACKAGE_NAMES+=("$test_name")
            FAILED_TEST_DETAILS+=("$test_name: Build or test execution failed")
            return 1
        fi
    fi
}

# ============================================================================
# PARSE ARGUMENTS
# ============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --fast)
            FAST_MODE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --package)
            SPECIFIC_PACKAGE="$2"
            shift 2
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --bench)
            RUN_BENCHMARKS=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

# ============================================================================
# MAIN EXECUTION
# ============================================================================

print_header "CORTEX WORKSPACE TEST RUNNER"

echo ""
print_info "Fast Mode: $FAST_MODE"
print_info "Verbose: $VERBOSE"
print_info "Clean Build: $CLEAN_BUILD"
print_info "Run Benchmarks: $RUN_BENCHMARKS"
if [ -n "$SPECIFIC_PACKAGE" ]; then
    print_info "Specific Package: $SPECIFIC_PACKAGE"
fi
echo ""

# ============================================================================
# PHASE 1: ENVIRONMENT SETUP
# ============================================================================

print_section "Phase 1: Environment Setup"

# Check for cargo
if ! command -v cargo &> /dev/null; then
    print_error "cargo not found in PATH"
    print_info "Expected PATH: $PATH"
    exit 2
fi
print_success "cargo found: $(cargo --version)"

# Check for rustc
if ! command -v rustc &> /dev/null; then
    print_error "rustc not found in PATH"
    exit 2
fi
print_success "rustc found: $(rustc --version)"

print_success "Environment setup complete"

# ============================================================================
# PHASE 2: CLEAN BUILD (OPTIONAL)
# ============================================================================

if [ "$CLEAN_BUILD" = true ]; then
    print_section "Phase 2: Clean Build"

    if cargo clean; then
        print_success "Workspace cleaned"
    else
        print_error "Failed to clean workspace"
        exit 2
    fi
fi

# ============================================================================
# PHASE 3: BUILD WORKSPACE
# ============================================================================

print_section "Phase 3: Build Workspace"

if [ "$VERBOSE" = true ]; then
    if cargo build --workspace --lib; then
        print_success "Workspace built successfully"
    else
        print_error "Workspace build failed"
        exit 2
    fi
else
    print_info "Building workspace libraries..."
    if cargo build --workspace --lib --quiet 2>&1 | tail -5; then
        print_success "Workspace built successfully"
    else
        print_error "Workspace build failed"
        exit 2
    fi
fi

# ============================================================================
# PHASE 4: RUN TESTS
# ============================================================================

print_header "RUNNING TESTS"

# Track overall test success
ALL_TESTS_PASSED=true

# Define packages and tests
declare -a UNIT_TEST_PACKAGES=(
    "cortex-core"
    "cortex-storage"
    "cortex-vfs"
    "cortex-memory"
    "cortex-ingestion"
    "cortex-semantic"
    "cortex-parser"
    "cortex-cli"
)

# Individual package integration tests
declare -a INTEGRATION_TESTS=(
    # cortex-vfs integration tests
    "cortex-vfs:test_helper_methods"
    "cortex-vfs:external_loader_verification"
    "cortex-vfs:fork_management_verification"
    "cortex-vfs:ingestion_pipeline_verification"
    "cortex-vfs:integration_tests"
    "cortex-vfs:test_ingestion_pipeline"
    "cortex-vfs:test_vfs_comprehensive"
    "cortex-vfs:test_vfs_reference_counting"
    "cortex-vfs:unit_tests"
    "cortex-vfs:vfs_correctness_verification"

    # cortex-semantic integration tests
    "cortex-semantic:hnsw_integration"
    "cortex-semantic:integration_tests"
    "cortex-semantic:test_semantic_search_e2e"

    # cortex-cli integration tests
    "cortex-cli:api_integration_tests"
    "cortex-cli:comprehensive_tests"
    "cortex-cli:integration_tests"
    "cortex-cli:mcp_integration_tests"
    "cortex-cli:mcp_tools_unit_tests"
    "cortex-cli:test_db_commands"
)

# Long-running integration tests
declare -a LONG_RUNNING_TESTS=(
    "cortex-cli:ultimate_cortex_test"
)

# Workspace-level integration tests
declare -a WORKSPACE_INTEGRATION_TESTS=(
    "cortex-integration-tests:cross_crate_integration"
    "cortex-integration-tests:e2e_workflow_tests"
    "cortex-integration-tests:test_complete_workflow"
    "cortex-integration-tests:test_multi_agent_scenario"
    "cortex-integration-tests:test_memory_consolidation"
    "cortex-integration-tests:test_production_load"
    "cortex-integration-tests:test_surrealdb_integration"
    "cortex-integration-tests:reality_check_e2e"
    "cortex-integration-tests:e2e_real_project"
    "cortex-integration-tests:test_multi_agent_advanced"
    "cortex-integration-tests:test_complete_e2e_workflows"
    "cortex-integration-tests:test_token_efficiency"
    "cortex-integration-tests:test_performance_benchmarks"
    "cortex-integration-tests:test_multi_agent_realistic_scenarios"
    "cortex-integration-tests:test_ast_correctness"
    "cortex-integration-tests:test_token_efficiency_benchmark"
    "cortex-integration-tests:test_mcp_stress"
    "cortex-integration-tests:test_real_world_development"
    "cortex-integration-tests:test_memory_system_integration"
    "cortex-integration-tests:test_mcp_tools_e2e_workflows"
    "cortex-integration-tests:test_performance_regression"
    "cortex-integration-tests:test_token_efficiency_measured"
    "cortex-integration-tests:e2e_cortex_self_test_phase1_ingestion"
    "cortex-integration-tests:comprehensive_workflow_verification"
    "cortex-integration-tests:e2e_cortex_complete"
)

# Benchmark tests (all in cortex-semantic)
declare -a BENCHMARK_TESTS=(
    "cortex-semantic:embedding_bench"
    "cortex-semantic:search_bench"
    "cortex-semantic:search_performance"
    "cortex-semantic:hnsw_comparison"
)

# Workspace-level benchmarks
declare -a WORKSPACE_BENCHMARK_TESTS=(
    "cortex-integration-tests:performance_benchmarks"
    "cortex-integration-tests:e2e_workflows"
)

# ============================================================================
# RUN UNIT TESTS
# ============================================================================

if [ -z "$SPECIFIC_PACKAGE" ] || [[ " ${UNIT_TEST_PACKAGES[@]} " =~ " ${SPECIFIC_PACKAGE} " ]]; then
    print_header "UNIT TESTS"

    for package in "${UNIT_TEST_PACKAGES[@]}"; do
        if [ -n "$SPECIFIC_PACKAGE" ] && [ "$package" != "$SPECIFIC_PACKAGE" ]; then
            continue
        fi

        if ! run_test "$package (unit tests)" cargo test --package "$package" --lib; then
            ALL_TESTS_PASSED=false
        fi
    done
fi

# ============================================================================
# RUN INTEGRATION TESTS
# ============================================================================

if [ -z "$SPECIFIC_PACKAGE" ]; then
    print_header "INTEGRATION TESTS"

    for test in "${INTEGRATION_TESTS[@]}"; do
        IFS=':' read -r package test_name <<< "$test"

        if ! run_test "$package::$test_name" cargo test --package "$package" --test "$test_name"; then
            ALL_TESTS_PASSED=false
        fi
    done
fi

# ============================================================================
# RUN WORKSPACE INTEGRATION TESTS
# ============================================================================

if [ -z "$SPECIFIC_PACKAGE" ]; then
    print_header "WORKSPACE INTEGRATION TESTS"

    for test in "${WORKSPACE_INTEGRATION_TESTS[@]}"; do
        IFS=':' read -r package test_name <<< "$test"

        # Skip long-running tests in fast mode
        if [ "$FAST_MODE" = true ]; then
            # Check if this is a long-running test (e2e, production, stress tests)
            if [[ "$test_name" == *"e2e"* || "$test_name" == *"production"* || "$test_name" == *"stress"* || "$test_name" == *"load"* ]]; then
                print_warning "Skipping (fast mode): $package::$test_name"
                continue
            fi
        fi

        if ! run_test "$package::$test_name" cargo test --package "$package" --test "$test_name"; then
            ALL_TESTS_PASSED=false
        fi
    done
fi

# ============================================================================
# RUN LONG-RUNNING TESTS (UNLESS --fast)
# ============================================================================

if [ "$FAST_MODE" = false ] && [ -z "$SPECIFIC_PACKAGE" ]; then
    print_header "LONG-RUNNING TESTS"

    for test in "${LONG_RUNNING_TESTS[@]}"; do
        IFS=':' read -r package test_name <<< "$test"

        if ! run_test "$package::$test_name" cargo test --package "$package" --test "$test_name"; then
            ALL_TESTS_PASSED=false
        fi
    done
else
    if [ "$FAST_MODE" = true ]; then
        print_warning "Skipping long-running tests (--fast mode)"
    fi
fi

# ============================================================================
# RUN BENCHMARKS (IF REQUESTED)
# ============================================================================

if [ "$RUN_BENCHMARKS" = true ] && [ -z "$SPECIFIC_PACKAGE" ]; then
    print_header "PACKAGE BENCHMARKS"

    for test in "${BENCHMARK_TESTS[@]}"; do
        IFS=':' read -r package bench_name <<< "$test"

        # Note: Benchmarks don't affect pass/fail status
        print_section "Benchmark: $package::$bench_name"
        if cargo bench --package "$package" --bench "$bench_name"; then
            print_success "Benchmark completed: $package::$bench_name"
        else
            print_warning "Benchmark failed: $package::$bench_name"
        fi
    done

    print_header "WORKSPACE BENCHMARKS"

    for test in "${WORKSPACE_BENCHMARK_TESTS[@]}"; do
        IFS=':' read -r package bench_name <<< "$test"

        print_section "Benchmark: $package::$bench_name"
        if cargo bench --package "$package" --bench "$bench_name"; then
            print_success "Benchmark completed: $package::$bench_name"
        else
            print_warning "Benchmark failed: $package::$bench_name"
        fi
    done
fi

# ============================================================================
# PHASE 5: GENERATE REPORT
# ============================================================================

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
MINUTES=$((DURATION / 60))
SECONDS=$((DURATION % 60))

print_header "TEST REPORT"

echo ""
print_section "Summary"
echo -e "${BOLD}Total Packages Tested:${NC} $TOTAL_PACKAGES"
echo -e "${BOLD}Packages Passed:${NC} ${GREEN}$PASSED_PACKAGES${NC}"
echo -e "${BOLD}Packages Failed:${NC} ${RED}$FAILED_PACKAGES${NC}"
echo ""
echo -e "${BOLD}Total Tests:${NC} $TOTAL_TESTS"
echo -e "${BOLD}Tests Passed:${NC} ${GREEN}$PASSED_TESTS${NC}"
echo -e "${BOLD}Tests Failed:${NC} ${RED}$FAILED_TESTS${NC}"
echo -e "${BOLD}Tests Ignored:${NC} ${YELLOW}$IGNORED_TESTS${NC}"
echo ""
echo -e "${BOLD}Duration:${NC} ${MINUTES}m ${SECONDS}s"

if [ ${#FAILED_PACKAGE_NAMES[@]} -gt 0 ]; then
    echo ""
    print_section "Failed Packages"
    for pkg in "${FAILED_PACKAGE_NAMES[@]}"; do
        echo -e "  ${RED}✗${NC} $pkg"
    done
fi

if [ ${#FAILED_TEST_DETAILS[@]} -gt 0 ]; then
    echo ""
    print_section "Failed Test Details"
    for detail in "${FAILED_TEST_DETAILS[@]}"; do
        echo -e "  ${RED}▸${NC} $detail"
    done
fi

# Calculate pass rate
if [ "$TOTAL_TESTS" -gt 0 ]; then
    PASS_RATE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo ""
    print_section "Test Coverage"
    echo -e "${BOLD}Pass Rate:${NC} $PASS_RATE% ($PASSED_TESTS/$TOTAL_TESTS)"

    if [ "$PASS_RATE" -eq 100 ]; then
        echo -e "${GREEN}${BOLD}🎉 100% PASS RATE ACHIEVED! 🎉${NC}"
    else
        echo -e "${RED}${BOLD}❌ TARGET 100% PASS RATE NOT MET${NC}"
    fi
fi

echo ""
print_header "END OF TEST RUN"
echo ""

# ============================================================================
# EXIT WITH APPROPRIATE CODE
# ============================================================================

if [ "$ALL_TESTS_PASSED" = true ] && [ "$FAILED_PACKAGES" -eq 0 ]; then
    print_success "All tests passed successfully!"
    exit 0
else
    print_error "One or more tests failed. See report above for details."
    exit 1
fi
