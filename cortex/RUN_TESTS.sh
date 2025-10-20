#!/bin/bash
# Cortex Test Suite Runner
# This script runs all tests and generates reports

set -e  # Exit on error

echo "=================================="
echo "Cortex Test Suite Runner"
echo "=================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if in cortex directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Please run this script from the cortex directory${NC}"
    exit 1
fi

echo "Running comprehensive test suite..."
echo ""

# Function to run tests for a crate
run_crate_tests() {
    local crate=$1
    echo -e "${YELLOW}Testing $crate...${NC}"
    if cargo test -p $crate --quiet; then
        echo -e "${GREEN}✓ $crate tests passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $crate tests failed${NC}"
        return 1
    fi
}

# Track failures
FAILED_CRATES=()

# Test each crate
echo "=================================="
echo "Unit and Integration Tests"
echo "=================================="
echo ""

for crate in cortex-core cortex-storage cortex-vfs cortex-ingestion cortex-memory cortex-semantic cortex-mcp cortex-cli; do
    if run_crate_tests $crate; then
        :
    else
        FAILED_CRATES+=($crate)
    fi
    echo ""
done

# Run E2E tests
echo "=================================="
echo "End-to-End Workflow Tests"
echo "=================================="
echo ""

echo -e "${YELLOW}Testing E2E workflows...${NC}"
if cargo test --test e2e_workflow_tests --quiet; then
    echo -e "${GREEN}✓ E2E tests passed${NC}"
else
    echo -e "${RED}✗ E2E tests failed${NC}"
    FAILED_CRATES+=("e2e")
fi
echo ""

# Run all workspace tests
echo "=================================="
echo "Complete Workspace Test"
echo "=================================="
echo ""

echo -e "${YELLOW}Running all workspace tests...${NC}"
if cargo test --workspace --quiet; then
    echo -e "${GREEN}✓ All workspace tests passed${NC}"
else
    echo -e "${RED}✗ Some workspace tests failed${NC}"
fi
echo ""

# Summary
echo "=================================="
echo "Test Summary"
echo "=================================="
echo ""

if [ ${#FAILED_CRATES[@]} -eq 0 ]; then
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Generate coverage report: cargo tarpaulin --workspace"
    echo "  2. Review TESTING.md for additional test commands"
    echo "  3. Check TEST_REPORT.md for detailed analysis"
    exit 0
else
    echo -e "${RED}Some tests failed:${NC}"
    for crate in "${FAILED_CRATES[@]}"; do
        echo -e "  ${RED}✗ $crate${NC}"
    done
    echo ""
    echo "Debugging tips:"
    echo "  1. Run failed crate tests with output: cargo test -p <crate> -- --nocapture"
    echo "  2. Run specific test: cargo test <test_name> -- --nocapture"
    echo "  3. Check TESTING.md for troubleshooting guide"
    exit 1
fi
