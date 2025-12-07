#!/bin/bash

# Privacy Bridge - Comprehensive Test Runner
# Usage: ./run_tests.sh [test_suite]

set -e

echo "=================================="
echo "Privacy Bridge Test Suite"
echo "=================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
FOUNDRY_PROFILE=${FOUNDRY_PROFILE:-default}
VERBOSITY=${VERBOSITY:-vvv}

# Function to run tests with proper formatting
run_test_suite() {
    local test_file=$1
    local test_name=$2
    
    echo -e "${YELLOW}Running: $test_name${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    if forge test --match-path "$test_file" -$VERBOSITY; then
        echo -e "${GREEN}✓ $test_name PASSED${NC}"
    else
        echo -e "${RED}✗ $test_name FAILED${NC}"
        exit 1
    fi
    
    echo ""
}

# Parse command line arguments
TEST_SUITE=${1:-all}

case $TEST_SUITE in
    "poseidon"|"hash")
        echo "Running Poseidon Hasher Tests..."
        run_test_suite "test/PoseidonHasher.t.sol" "Poseidon Hasher Tests"
        ;;
        
    "pool"|"intent")
        echo "Running Intent Pool Tests..."
        run_test_suite "test/PrivateIntentPool.t.sol" "Private Intent Pool Tests"
        ;;
        
    "settlement"|"claim")
        echo "Running Settlement Tests..."
        run_test_suite "test/PrivateSettlement.t.sol" "Private Settlement Tests"
        ;;
        
    "integration"|"e2e")
        echo "Running Integration Tests..."
        run_test_suite "test/Integration.t.sol" "End-to-End Integration Tests"
        ;;
        
    "invariant"|"fuzz")
        echo "Running Invariant Tests..."
        echo "⚠️  This may take several minutes..."
        forge test --match-path "test/Invariants.t.sol" -$VERBOSITY
        ;;
        
    "all")
        echo "Running ALL Test Suites..."
        echo ""
        
        run_test_suite "test/PoseidonHasher.t.sol" "Poseidon Hasher Tests"
        run_test_suite "test/PrivateIntentPool.t.sol" "Private Intent Pool Tests"
        run_test_suite "test/PrivateSettlement.t.sol" "Private Settlement Tests"
        run_test_suite "test/Integration.t.sol" "Integration Tests"
        
        echo -e "${YELLOW}Running Invariant Tests (may take 5-10 minutes)...${NC}"
        forge test --match-path "test/Invariants.t.sol" -vv
        ;;
        
    "quick")
        echo "Running Quick Test Suite (no invariants)..."
        forge test --match-path "test/PoseidonHasher.t.sol" --match-path "test/PrivateIntentPool.t.sol" --match-path "test/PrivateSettlement.t.sol" -vv
        ;;
        
    "gas")
        echo "Running Gas Benchmark Tests..."
        forge test --gas-report
        ;;
        
    "coverage")
        echo "Generating Code Coverage Report..."
        forge coverage --report lcov
        forge coverage --report summary
        ;;
        
    *)
        echo -e "${RED}Unknown test suite: $TEST_SUITE${NC}"
        echo ""
        echo "Available test suites:"
        echo "  all         - Run all tests (default)"
        echo "  poseidon    - Poseidon hasher tests"
        echo "  pool        - Intent pool tests"
        echo "  settlement  - Settlement tests"
        echo "  integration - End-to-end tests"
        echo "  invariant   - Invariant/fuzz tests"
        echo "  quick       - Quick tests (no invariants)"
        echo "  gas         - Gas benchmarks"
        echo "  coverage    - Coverage report"
        exit 1
        ;;
esac

echo ""
echo "=================================="
echo -e "${GREEN}All Tests Completed Successfully!${NC}"
echo "=================================="
echo ""

# Generate summary
echo "Test Summary:"
forge test --summary 2>/dev/null || echo "Summary not available"


# Make executable
# chmod +x run_tests.sh

# Run all tests
# ./run_tests.sh all

# Run specific suite
# ./run_tests.sh poseidon
# ./run_tests.sh pool
# ./run_tests.sh settlement
# ./run_tests.sh integration
# ./run_tests.sh invariant

# Quick tests (no invariants)
# ./run_tests.sh quick

# Gas benchmarks
# ./run_tests.sh gas

# Coverage report
# ./run_tests.sh coverage

