# Mitoxide Examples

This directory contains practical examples demonstrating various features of Mitoxide.

## Running Examples

To run an example:

```bash
cargo run --example basic_usage
```

## Available Examples

### `basic_usage.rs`
Demonstrates core Mitoxide functionality:
- SSH connection establishment
- Remote command execution
- File operations (read/write)
- Concurrent command execution
- Error handling patterns

### `jump_host.rs` (Coming Soon)
Shows how to use jump hosts and bastion servers:
- Multi-hop SSH connections
- Connection routing through bastions
- Complex network topologies

### `wasm_execution.rs` (Coming Soon)
WebAssembly module execution:
- Loading and executing WASM modules
- JSON input/output handling
- Resource limits and sandboxing

### `deployment_automation.rs` (Coming Soon)
Real-world deployment scenario:
- Application deployment workflow
- Configuration management
- Service management
- Health checks and rollback

### `monitoring_setup.rs` (Coming Soon)
Infrastructure monitoring setup:
- Installing monitoring agents
- Configuring metrics collection
- Log aggregation setup

## Prerequisites

Before running examples, ensure you have:

1. **SSH Access**: Valid SSH credentials to a remote host
2. **Network Connectivity**: Ability to reach the target host
3. **Permissions**: Appropriate permissions for the operations being performed

## Configuration

Most examples use placeholder values like `user@remote-host`. Replace these with your actual SSH details:

```rust
// Replace this
let session = Session::ssh("user@remote-host").await?;

// With your actual details
let session = Session::ssh("myuser@192.168.1.100").await?;
```

## SSH Key Setup

For passwordless authentication, set up SSH keys:

```bash
# Generate SSH key pair (if you don't have one)
ssh-keygen -t rsa -b 4096 -C "your_email@example.com"

# Copy public key to remote host
ssh-copy-id user@remote-host

# Test connection
ssh user@remote-host echo "Connection successful"
```

## Docker Testing Environment

For testing examples without a real remote host, use the provided Docker environment:

```bash
# Start test containers
docker-compose up -d

# Run examples against test containers
cargo run --example basic_usage
# (modify the example to use localhost:2223)

# Cleanup
docker-compose down
```

## Troubleshooting

### Connection Issues
- Verify SSH connectivity: `ssh user@host`
- Check firewall settings
- Ensure SSH service is running on target host

### Permission Issues
- Verify user permissions on remote host
- Check file/directory permissions
- Use `sudo` feature if elevated privileges needed

### Network Issues
- Test network connectivity: `ping host`
- Check DNS resolution
- Verify port accessibility: `telnet host 22`

## Contributing Examples

We welcome contributions of new examples! When adding examples:

1. **Focus on Real Use Cases**: Examples should solve actual problems
2. **Include Documentation**: Add clear comments and explanations
3. **Handle Errors**: Show proper error handling patterns
4. **Test Thoroughly**: Ensure examples work as documented
5. **Update This README**: Add your example to the list above

### Example Template

```rust
//! Brief description of what this example demonstrates
//! 
//! This example shows how to:
//! - Feature 1
//! - Feature 2
//! - Feature 3

use mitoxide::Session;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Your example code here
    Ok(())
}
```

## Getting Help

If you have questions about the examples:

- 💬 [Discord Server](https://discord.gg/YOUR_INVITE)
- 🐛 [GitHub Issues](https://github.com/yourusername/mitoxide/issues)
- 💡 [GitHub Discussions](https://github.com/yourusername/mitoxide/discussions)
- 📚 [Documentation](https://docs.rs/mitoxide)