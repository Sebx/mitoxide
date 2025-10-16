# Mitoxide SSH

[![Crates.io](https://img.shields.io/crates/v/mitoxide-ssh.svg)](https://crates.io/crates/mitoxide-ssh)
[![Documentation](https://docs.rs/mitoxide-ssh/badge.svg)](https://docs.rs/mitoxide-ssh)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

SSH transport layer for Mitoxide - providing secure remote connections with connection pooling and jump host support.

## Features

- Multiple SSH backend support (libssh2, OpenSSH)
- Connection pooling and reuse
- Jump host and bastion support
- Automatic reconnection and recovery
- Efficient connection management

## Usage

This crate is primarily used internally by Mitoxide, but can be used standalone for SSH operations.

```rust
use mitoxide_ssh::{SshConfig, StdioTransport, Transport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SshConfig {
        host: "remote-host".to_string(),
        port: 22,
        user: "username".to_string(),
        key_path: "/path/to/key".to_string(),
        ..Default::default()
    };
    
    let mut transport = StdioTransport::new(config);
    let connection = transport.connect().await?;
    
    Ok(())
}
```

## Features

- `ssh2` (default) - Use libssh2 for SSH transport
- `openssh` - Use OpenSSH client for transport

## Documentation

- [API Documentation](https://docs.rs/mitoxide-ssh)
- [Main Mitoxide Documentation](https://docs.rs/mitoxide)
- [GitHub Repository](https://github.com/yourusername/mitoxide)

## License

This project is licensed under the MIT License.