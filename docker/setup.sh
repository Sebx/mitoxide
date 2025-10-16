#!/bin/bash
set -e

echo "🐳 Setting up Mitoxide Docker Test Environment"
echo "=============================================="

# Check Docker availability
./docker/check_docker.sh

# Generate SSH keys if they don't exist
if [ ! -f docker/ssh_keys/test_key ]; then
    echo "🔑 Generating SSH keys..."
    ./docker/generate_keys.sh
else
    echo "✅ SSH keys already exist"
fi

# Build Docker images
echo "🏗️  Building Docker images..."
docker-compose build

# Start containers
echo "🚀 Starting containers..."
docker-compose up -d

# Wait for containers to be ready
echo "⏳ Waiting for containers to be ready..."
sleep 10

# Test connectivity
echo "🔍 Testing SSH connectivity..."
cd docker
make test-connectivity

echo ""
echo "✅ Docker test environment is ready!"
echo ""
echo "Available containers:"
echo "  - alpine_ro:    localhost:2222 (read-only filesystem, memory constrained)"
echo "  - ubuntu_min:   localhost:2223 (standard Ubuntu environment)"
echo "  - bastion:      localhost:2224 (jump host for backend access)"
echo "  - backend_target: (accessible only through bastion)"
echo ""
echo "SSH key: docker/ssh_keys/test_key"
echo "SSH user: testuser"
echo ""
echo "Usage examples:"
echo "  ssh -i docker/ssh_keys/test_key -p 2222 testuser@localhost  # Alpine RO"
echo "  ssh -i docker/ssh_keys/test_key -p 2223 testuser@localhost  # Ubuntu Min"
echo "  ssh -i docker/ssh_keys/test_key -p 2224 testuser@localhost  # Bastion"
echo ""
echo "Management commands:"
echo "  cd docker && make status     # Check container status"
echo "  cd docker && make logs       # View all logs"
echo "  cd docker && make down       # Stop containers"
echo "  cd docker && make clean      # Clean up everything"