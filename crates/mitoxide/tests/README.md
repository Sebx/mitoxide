# Mitoxide Integration Tests

This directory contains comprehensive integration tests for the Mitoxide library, focusing on real-world scenarios using Docker containers to simulate various remote environments.

## Test Suites

### Routing Tests (`routing_integration_tests.rs`)

Tests for jump host and routing functionality, including:

- **Multi-hop SSH connections**: Tests connections through bastion hosts to backend targets
- **Connection routing and multiplexing**: Verifies concurrent operations across multiple SSH connections
- **Connection failure and recovery**: Tests resilience to network failures and automatic recovery
- **Load balancing and connection pooling**: Validates efficient connection management under load
- **Routing performance**: Measures latency and throughput under various load conditions

### Other Test Suites

- **Bootstrap Tests**: Agent bootstrapping scenarios across different platforms
- **Process Tests**: Process execution and I/O handling
- **WASM Tests**: WebAssembly module execution end-to-end
- **PTY Tests**: Privilege escalation and interactive terminal operations

## Docker Test Environment

The tests use a Docker Compose setup with multiple containers:

- **ubuntu_min**: Standard Ubuntu container for general testing
- **alpine_ro**: Alpine container with read-only filesystem for constraint testing
- **bastion**: Jump host for multi-hop connection testing
- **backend_target**: Target accessible only through the bastion (isolated network)

## Running Tests

### Prerequisites

1. Docker and Docker Compose installed
2. SSH keys generated (run `docker/setup.ps1` or `docker/setup.sh`)
3. Rust toolchain with cargo

### Running Routing Tests

```bash
# Run all routing tests
cargo test --package mitoxide --test routing_integration_tests

# Run specific routing test
cargo test --package mitoxide --test routing_integration_tests test_multi_hop_connections

# Run with output
cargo test --package mitoxide --test routing_integration_tests -- --nocapture
```

### Using Test Scripts

```bash
# Windows PowerShell
.\scripts\test_routing.ps1

# Linux/macOS
./scripts/test_routing.sh
```

## Test Architecture

The integration tests follow a structured approach:

1. **Setup Phase**: Start Docker containers and verify SSH connectivity
2. **Test Execution**: Run specific test scenarios with proper error handling
3. **Cleanup Phase**: Stop containers and clean up resources
4. **Verification**: Assert expected outcomes and performance thresholds

### Key Components

- **DockerTestEnv**: Manages Docker container lifecycle
- **SshHelper**: Provides SSH connectivity utilities
- **TestAssertions**: Common assertion helpers for test validation
- **PerformanceUtils**: Performance measurement and benchmarking utilities

## Performance Thresholds

The routing tests include performance assertions:

- **P50 Latency**: < 2 seconds for SSH command execution
- **P95 Latency**: < 5 seconds for SSH command execution
- **Throughput**: > 2 operations per second for concurrent operations
- **Connection Time**: < 5 seconds for SSH connection establishment

## Troubleshooting

### Common Issues

1. **Docker not running**: Ensure Docker daemon is started
2. **Port conflicts**: Check that ports 2222-2224 are available
3. **SSH key issues**: Regenerate keys with setup scripts
4. **Network isolation**: Verify Docker networks are properly configured

### Debug Mode

Run tests with additional logging:

```bash
RUST_LOG=debug cargo test --package mitoxide --test routing_integration_tests -- --nocapture
```

### Manual Container Testing

```bash
# Start containers manually
docker-compose up -d

# Test SSH connectivity
ssh -i docker/ssh_keys/test_key -p 2223 testuser@localhost echo "test"

# Test jump host
ssh -i docker/ssh_keys/test_key -J testuser@localhost:2224 testuser@mitoxide_backend_target echo "jump test"
```

## Contributing

When adding new routing tests:

1. Follow the existing test structure and naming conventions
2. Include proper setup/cleanup in test methods
3. Add performance assertions where appropriate
4. Update this README with new test descriptions
5. Ensure tests are deterministic and can run in parallel

## Requirements Coverage

These tests verify the following requirements from the Mitoxide specification:

- **Requirement 5.1**: Client-side router for context lifecycle management
- **Requirement 5.2**: Multi-hop SSH connections through bastion hosts
- **Requirement 5.3**: Agent-side mini-router for fan-out operations
- **Requirement 10.4**: Comprehensive integration testing scenarios

The routing tests ensure that Mitoxide can handle complex network topologies and provide reliable remote execution capabilities across various connection scenarios.