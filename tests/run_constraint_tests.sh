#!/bin/bash
set -e

echo "🧪 Running Mitoxide Constraint Tests"
echo "===================================="

# Check prerequisites
echo "Checking prerequisites..."
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed"
    exit 1
fi

if ! docker info &> /dev/null; then
    echo "❌ Docker daemon is not running"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose is not available"
    exit 1
fi

echo "✅ Prerequisites satisfied"

# Setup test environment
echo ""
echo "Setting up test environment..."
cd "$(dirname "$0")/.."

# Generate SSH keys if needed
if [ ! -f docker/ssh_keys/test_key ]; then
    echo "Generating SSH keys..."
    mkdir -p docker/ssh_keys
    ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N "" -C "mitoxide-test-key"
fi

# Build and start containers
echo "Building and starting Docker containers..."
docker-compose build
docker-compose up -d

# Wait for containers to be ready
echo "Waiting for containers to be ready..."
sleep 10

# Run constraint tests
echo ""
echo "Running constraint tests..."
echo "=========================="

# Test categories
TESTS=(
    "test_readonly_filesystem_constraints"
    "test_memory_limit_constraints" 
    "test_network_isolation"
    "test_resource_exhaustion_recovery"
    "test_concurrent_connection_stress"
    "test_container_restart_recovery"
)

PASSED=0
FAILED=0
FAILED_TESTS=()

for test in "${TESTS[@]}"; do
    echo ""
    echo "Running $test..."
    if cargo test --test constraint_tests "$test" -- --nocapture; then
        echo "✅ $test PASSED"
        ((PASSED++))
    else
        echo "❌ $test FAILED"
        ((FAILED++))
        FAILED_TESTS+=("$test")
    fi
done

# Run integration tests as well
echo ""
echo "Running integration tests..."
if cargo test --test integration_tests -- --nocapture; then
    echo "✅ Integration tests PASSED"
    ((PASSED++))
else
    echo "❌ Integration tests FAILED"
    ((FAILED++))
    FAILED_TESTS+=("integration_tests")
fi

# Cleanup
echo ""
echo "Cleaning up..."
docker-compose down

# Summary
echo ""
echo "Test Summary"
echo "============"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo "Total:  $((PASSED + FAILED))"

if [ $FAILED -gt 0 ]; then
    echo ""
    echo "Failed tests:"
    for test in "${FAILED_TESTS[@]}"; do
        echo "  - $test"
    done
    echo ""
    echo "❌ Some tests failed"
    exit 1
else
    echo ""
    echo "✅ All tests passed!"
    exit 0
fi