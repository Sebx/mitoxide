# Docker Test Environment Makefile

.PHONY: build up down clean logs test-connectivity

# Build all Docker images
build:
	docker-compose build

# Start all containers
up:
	docker-compose up -d

# Stop all containers
down:
	docker-compose down

# Clean up containers, networks, and images
clean:
	docker-compose down -v --remove-orphans
	docker-compose rm -f
	docker system prune -f

# Show logs for all containers
logs:
	docker-compose logs -f

# Show logs for specific container
logs-%:
	docker-compose logs -f $*

# Test SSH connectivity to all containers
test-connectivity:
	@echo "Testing SSH connectivity to all containers..."
	@echo "Alpine RO (port 2222):"
	@ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i ssh_keys/test_key -p 2222 testuser@localhost "uname -a" || echo "Failed to connect to alpine_ro"
	@echo "Ubuntu Min (port 2223):"
	@ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i ssh_keys/test_key -p 2223 testuser@localhost "uname -a" || echo "Failed to connect to ubuntu_min"
	@echo "Bastion (port 2224):"
	@ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i ssh_keys/test_key -p 2224 testuser@localhost "uname -a" || echo "Failed to connect to bastion"

# Test jump host connectivity through bastion
test-jump:
	@echo "Testing jump host connectivity through bastion..."
	@ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i ssh_keys/test_key -p 2224 -J testuser@localhost:2224 testuser@mitoxide_backend_target "uname -a" || echo "Failed to connect through jump host"

# Show container status
status:
	docker-compose ps

# Execute shell in specific container
shell-%:
	docker-compose exec $* /bin/bash

# Restart specific container
restart-%:
	docker-compose restart $*