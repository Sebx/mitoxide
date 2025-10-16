# Docker Test Environment

This directory contains Docker containers and configuration for testing Mitoxide in various constrained environments.

## Containers

### alpine_ro
- **Purpose**: Test read-only filesystem constraints
- **Base**: Alpine Linux 3.18
- **Constraints**: 
  - Read-only root filesystem
  - 64MB memory limit
  - 64MB tmpfs for /tmp
  - 0.5 CPU limit
- **Port**: 2222
- **User**: testuser

### ubuntu_min
- **Purpose**: Standard Ubuntu environment for general testing
- **Base**: Ubuntu 22.04
- **Features**:
  - SSH server
  - sudo access for testuser
  - Standard filesystem access
- **Port**: 2223
- **User**: testuser

### bastion
- **Purpose**: Jump host for multi-hop SSH testing
- **Base**: Ubuntu 22.04
- **Features**:
  - SSH server with forwarding enabled
  - Access to both frontend and backend networks
  - Pre-configured SSH client for backend access
- **Port**: 2224
- **User**: testuser

### backend_target
- **Purpose**: Backend target accessible only through bastion
- **Base**: Ubuntu 22.04
- **Features**:
  - Only accessible through bastion host
  - Isolated backend network
- **Port**: None (internal only)
- **User**: testuser

## Networks

- **mitoxide_test**: Standard test network for direct access containers
- **mitoxide_frontend**: Frontend network for bastion host
- **mitoxide_backend**: Isolated backend network (internal only)

## Usage

### Setup
```bash
# Generate SSH keys (if not already done)
./generate_keys.sh

# Build all containers
make build

# Start all containers
make up
```

### Testing Connectivity
```bash
# Test direct SSH connections
make test-connectivity

# Test jump host functionality
make test-jump

# Check container status
make status
```

### Development
```bash
# View logs for all containers
make logs

# View logs for specific container
make logs-alpine_ro

# Execute shell in container
make shell-ubuntu_min

# Restart specific container
make restart-bastion
```

### Cleanup
```bash
# Stop containers
make down

# Clean up everything
make clean
```

## SSH Configuration

All containers use the same SSH key pair located in `ssh_keys/`:
- Private key: `ssh_keys/test_key`
- Public key: `ssh_keys/test_key.pub`

SSH connections use:
- User: `testuser`
- Authentication: Public key only (no passwords)
- Host key checking: Disabled for testing

## Testing Scenarios

### Read-only Filesystem (alpine_ro)
- Test agent bootstrap with memfd_create
- Test /tmp fallback when memfd unavailable
- Verify proper cleanup of temporary files

### Memory Constraints (alpine_ro)
- Test behavior under 64MB memory limit
- Verify graceful handling of memory pressure
- Test agent memory usage patterns

### Jump Host Routing (bastion + backend_target)
- Test multi-hop SSH connections
- Verify connection routing through bastion
- Test connection failure and recovery scenarios

### Privilege Escalation (ubuntu_min, bastion)
- Test sudo functionality
- Verify PTY handling for interactive commands
- Test credential handling and security

## Container Details

### Resource Limits
- **alpine_ro**: 64MB RAM, 0.5 CPU, read-only filesystem
- **ubuntu_min**: No limits (standard testing)
- **bastion**: No limits (jump host functionality)
- **backend_target**: No limits (isolated target)

### Network Topology
```
[Client] -> [bastion:2224] -> [backend_target:22]
[Client] -> [alpine_ro:2222]
[Client] -> [ubuntu_min:2223]
```

### SSH Key Management
- All containers share the same authorized_keys
- Bastion has both public and private keys for forwarding
- Keys are mounted read-only from host filesystem