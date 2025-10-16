# Mitoxide

[![Crates.io](https://img.shields.io/crates/v/mitoxide.svg)](https://crates.io/crates/mitoxide)
[![Documentation](https://docs.rs/mitoxide/badge.svg)](https://docs.rs/mitoxide)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A blazingly fast Rust library for remote execution and automation inspired by Mitogen.

*Built with AWS Kiro and vibe-coding principles for an intuitive developer experience*

## Features

- 🔥 **Blazing Fast**: Zero-copy serialization and efficient connection pooling
- 🛡️ **Secure**: Built-in SSH transport with jump host support
- 🌐 **Multi-Platform**: Linux, macOS, Windows, and container support
- ⚡ **WASM Runtime**: Execute WebAssembly modules remotely
- 🔧 **Privilege Escalation**: Seamless sudo and privilege handling
- 🐳 **Container Ready**: Docker, Kubernetes, and LXC integration

## Quick Start

Add Mitoxide to your `Cargo.toml`:

```toml
[dependencies]
mitoxide = "0.1"
```

### Basic Usage

```rust
use mitoxide::Session;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to remote host
    let session = Session::ssh("user@remote-host").await?;
    let context = session.connect().await?;
    
    // Execute commands
    let output = context.proc_exec(&["uname", "-a"]).await?;
    println!("Remote OS: {}", output.stdout);
    
    // Transfer files
    context.file_write("/tmp/hello.txt", b"Hello, World!").await?;
    let content = context.file_read("/tmp/hello.txt").await?;
    
    Ok(())
}
```

## Documentation

- [API Documentation](https://docs.rs/mitoxide)
- [Examples](https://github.com/yourusername/mitoxide/tree/main/examples)
- [GitHub Repository](https://github.com/yourusername/mitoxide)

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/yourusername/mitoxide/blob/main/LICENSE) file for details.