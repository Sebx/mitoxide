# Mitoxide Agent

[![Crates.io](https://img.shields.io/crates/v/mitoxide-agent.svg)](https://crates.io/crates/mitoxide-agent)
[![Documentation](https://docs.rs/mitoxide-agent/badge.svg)](https://docs.rs/mitoxide-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Remote agent binary for Mitoxide - the lightweight agent that runs on remote systems to execute commands and manage resources.

## Features

- Self-bootstrapping agent deployment
- Multi-platform support (Linux, macOS, Windows)
- Privilege escalation handling
- PTY operations for interactive commands
- Resource constraint handling
- WASM module execution support

## Installation

Install the agent binary:

```bash
cargo install mitoxide-agent
```

## Usage

The agent is typically deployed automatically by Mitoxide, but can be run manually:

```bash
# Run agent with default settings
mitoxide-agent

# Run with custom configuration
mitoxide-agent --config /path/to/config.json

# Run in debug mode
RUST_LOG=debug mitoxide-agent
```

## Configuration

The agent can be configured via environment variables or configuration file:

```json
{
  "max_memory": "512MB",
  "max_cpu_time": "30s",
  "allowed_commands": ["*"],
  "wasm_enabled": true,
  "sudo_enabled": false
}
```

## Security

The agent includes several security features:
- Command allowlist/blocklist
- Resource limits (memory, CPU, disk)
- Privilege escalation controls
- Sandboxed WASM execution

## Documentation

- [API Documentation](https://docs.rs/mitoxide-agent)
- [Main Mitoxide Documentation](https://docs.rs/mitoxide)
- [GitHub Repository](https://github.com/yourusername/mitoxide)

## License

This project is licensed under the MIT License.