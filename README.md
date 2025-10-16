# Mitoxide

A Rust library for remote execution and automation inspired by Mitogen.

## Project Structure

This is a Cargo workspace containing the following crates:

- **mitoxide** - Main SDK and client library
- **mitoxide-agent** - Remote agent binary
- **mitoxide-proto** - Protocol definitions and codec
- **mitoxide-ssh** - SSH transport layer
- **mitoxide-wasm** - WASM runtime support

## Requirements

- Rust 1.82+
- SSH client for remote connections

## Building

```bash
cargo build --workspace
```

## Testing

```bash
cargo test --workspace
```

## Features

- `ssh2` - Use libssh2 for SSH transport (default)
- `openssh` - Use OpenSSH client for transport
- `wasm` - Enable WASM runtime support
- `sudo` - Enable privilege escalation
- `docker` - Enable container execution
- `k8s` - Enable Kubernetes integration
- `lxc` - Enable LXC support

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.