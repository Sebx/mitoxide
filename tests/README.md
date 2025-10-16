# Mitoxide Integration and Constraint Tests

This directory contains comprehensive integration tests for the Mitoxide Docker test environment, including constraint testing scenarios that verify behavior under various system limitations.

## Test Categories

### Integration Tests (`integration_tests.rs`)

Basic functionality and connectivity tests:

- **Docker Environment Setup**: Verifies containers start correctly and are properly configured
- **SSH Connectivity**: Tests SSH connections to all containers
- **Basic Command Execution**: Verifies command execution over SSH
- **File Operations**: Tests file upload/download via SCP
- **Jump Host Connectivity**: Verifies multi-hop SSH connections through bastion
- **Performance Characteristics**: Measures connection and execution latency

### Constraint Tests (`constraint_tests.rs`)

Advanced testing scenarios for system constraints:

- **Read-only Filesystem Constraints**: Tests behavior with read-only root filesystem
- **Memory Limit Constraints**: Verifies memory limits are enforced
- **Network Isolation**: Tests network segmentation and jump host functionality
- **Resource Exhaustion Recovery**: Tests system behavior under resource pressure
- **Concurrent Connection Stress**: Verifies handling of multiple simultaneous connections
- **Container Restart Recovery**: Tests recovery after container restarts

## Test Environment

### Containers

| Container | Purpose | Constraints | Port |
|-----------|---------|-------------|------|
| `alpine_ro` | Read-only filesystem testing | 64MB RAM, read-only root, tmpfs /tmp | 2222 |
| `ubuntu_min` | Standard testing environment | None | 2223 |
| `bastion` | Jump host for multi-hop testing | None | 2224 |
| `backend_target` | Isolated backend target | Network isolation | Internal only |

### Network Topology

```
[Client] ──────────────────────────────────────────────────────────────┐
    │                                                                   │
    ├─── alpine_ro:2222 (constrained environment)                      │
    ├─── ubuntu_min:2223 (standard environment)                        │
    ├─── bastion:2224 (jump host)                                       │
    │                                                                   │
    └─── bastion:2224 ──── backend_target:22 (isolated backend)        │
                                                                        │
         [Frontend Network]    [Backend Network - Internal Only]       │
```

## Running Tests

### Prerequisites

1. **Docker**: Docker Desktop must be installed and running
2. **docker-compose**: Available in PATH
3. **SSH**: SSH client must be available
4. **Rust**: Cargo must be available for running tests

### Quick Start

```bash
# Setup and run all tests
./tests/run_constraint_tests.sh

# Or on Windows
./tests/run_constraint_tests.ps1
```

### Manual Setup

```bash
# 1. Generate SSH keys
mkdir -p docker/ssh_keys
ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N "" -C "mitoxide-test-key"

# 2. Start Docker environment
docker-compose up -d

# 3. Run specific test categories
cargo test --test integration_tests
cargo test --test constraint_tests

# 4. Run individual tests
cargo test --test constraint_tests test_readonly_filesystem_constraints -- --nocapture

# 5. Cleanup
docker-compose down
```

### Test Configuration

Tests can be configured via environment variables:

```bash
# SSH connection timeout (default: 30s)
export MITOXIDE_SSH_TIMEOUT=60

# Test key path (default: docker/ssh_keys/test_key)
export MITOXIDE_TEST_KEY_PATH=/path/to/test/key

# Docker compose file (default: docker-compose.yml)
export MITOXIDE_COMPOSE_FILE=docker-compose.test.yml
```

## Test Scenarios

### Read-only Filesystem Testing

Tests verify that:
- Root filesystem is truly read-only
- tmpfs mounts are writable with size limits
- System directories cannot be modified
- Applications handle read-only constraints gracefully

### Memory Constraint Testing

Tests verify that:
- Memory limits are properly enforced (64MB for alpine_ro)
- OOM killer activates under memory pressure
- Applications cannot allocate excessive memory
- System remains responsive under memory constraints

### Network Isolation Testing

Tests verify that:
- Backend network is isolated from direct access
- Jump host provides proper access to backend
- Network failures are handled gracefully
- Connection recovery works after network issues

### Resource Exhaustion Testing

Tests verify system behavior under:
- CPU exhaustion (high CPU load)
- Disk space exhaustion (tmpfs limits)
- File descriptor exhaustion
- Process limit exhaustion
- System recovery after resource pressure

### Concurrent Connection Testing

Tests verify that:
- Multiple simultaneous SSH connections work
- Performance remains acceptable under load
- No connection leaks or resource issues
- Error handling works under concurrent load

### Container Recovery Testing

Tests verify that:
- Containers restart properly
- SSH connectivity is restored after restart
- Constraints are maintained after restart
- No persistent state issues after restart

## Performance Expectations

### Latency Thresholds

- SSH connection establishment: < 5 seconds
- Command execution: < 2 seconds
- File transfer (1MB): < 10 seconds
- Jump host connection: < 10 seconds

### Throughput Expectations

- File transfer: > 1 MB/s (over SSH)
- Concurrent connections: 10+ simultaneous
- Command execution rate: > 5 commands/second

### Resource Limits

- alpine_ro memory: ~64MB total
- tmpfs size: 64MB maximum
- CPU limit: 0.5 cores for alpine_ro
- Connection timeout: 30 seconds default

## Troubleshooting

### Common Issues

1. **Docker not running**
   ```bash
   # Start Docker Desktop
   # Or on Linux: sudo systemctl start docker
   ```

2. **SSH keys missing**
   ```bash
   # Generate keys
   ssh-keygen -t rsa -b 2048 -f docker/ssh_keys/test_key -N ""
   ```

3. **Containers not starting**
   ```bash
   # Check logs
   docker-compose logs
   
   # Rebuild images
   docker-compose build --no-cache
   ```

4. **SSH connection failures**
   ```bash
   # Check container status
   docker-compose ps
   
   # Test manual SSH connection
   ssh -i docker/ssh_keys/test_key -p 2222 testuser@localhost
   ```

5. **Port conflicts**
   ```bash
   # Check port usage
   netstat -tulpn | grep :222
   
   # Modify ports in docker-compose.yml if needed
   ```

### Debug Mode

Run tests with verbose output:

```bash
# Rust test output
cargo test --test constraint_tests -- --nocapture

# Docker compose logs
docker-compose logs -f

# SSH debug output
ssh -vvv -i docker/ssh_keys/test_key -p 2222 testuser@localhost
```

### Performance Debugging

Monitor resource usage during tests:

```bash
# Container resource usage
docker stats

# System resource usage
htop

# Network connections
ss -tulpn | grep :222
```

## Contributing

When adding new tests:

1. Follow the existing test structure and naming conventions
2. Add appropriate assertions and error messages
3. Include cleanup code to prevent resource leaks
4. Update this README with new test descriptions
5. Ensure tests are deterministic and don't depend on external services
6. Add performance expectations for new scenarios

### Test Guidelines

- Use descriptive test names that explain what is being tested
- Include both positive and negative test cases
- Test error conditions and edge cases
- Verify cleanup and resource management
- Add appropriate timeouts to prevent hanging tests
- Use proper assertions with meaningful error messages