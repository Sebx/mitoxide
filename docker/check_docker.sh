#!/bin/bash

echo "Checking Docker availability..."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed"
    echo "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop"
    exit 1
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    echo "❌ Docker daemon is not running"
    echo "Please start Docker Desktop and try again"
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose is not available"
    echo "Please install docker-compose or use 'docker compose' (newer versions)"
    exit 1
fi

echo "✅ Docker is available and running"
echo "Docker version: $(docker --version)"
echo "Docker Compose version: $(docker-compose --version)"

# Test basic Docker functionality
echo "Testing Docker functionality..."
if docker run --rm hello-world &> /dev/null; then
    echo "✅ Docker is working correctly"
else
    echo "❌ Docker test failed"
    exit 1
fi

echo "🚀 Ready to build and run test containers!"