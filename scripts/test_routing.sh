#!/bin/bash
# Test script for routing functionality

echo "🚀 Mitoxide Routing Tests"
echo "========================="

# Check prerequisites
echo "Checking prerequisites..."

# Check Docker
if command -v docker &> /dev/null; then
    echo "✅ Docker is available"
else
    echo "❌ Docker is not available"
    exit 1
fi

# Check docker-compose
if command -v docker-compose &> /dev/null; then
    echo "✅ Docker Compose is available"
else
    echo "❌ Docker Compose is not available"
    exit 1
fi

# Check SSH keys
if [ -f "docker/ssh_keys/test_key" ]; then
    echo "✅ SSH keys are available"
else
    echo "❌ SSH keys not found"
    echo "Please run: docker/setup.sh"
    exit 1
fi

echo ""
echo "Running routing integration tests..."

# Run the routing tests
if cargo test --package mitoxide --test routing_integration_tests -- --nocapture; then
    echo ""
    echo "🎉 All routing tests passed!"
else
    echo ""
    echo "❌ Some routing tests failed"
    exit 1
fi

echo ""
echo "Routing test summary:"
echo "- Multi-hop SSH connections through bastion"
echo "- Connection routing and multiplexing"
echo "- Connection failure and recovery"
echo "- Load balancing and connection pooling"
echo "- Routing performance under load"