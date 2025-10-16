# Mitoxide Proto

[![Crates.io](https://img.shields.io/crates/v/mitoxide-proto.svg)](https://crates.io/crates/mitoxide-proto)
[![Documentation](https://docs.rs/mitoxide-proto/badge.svg)](https://docs.rs/mitoxide-proto)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Protocol definitions and codec for Mitoxide - a fast Rust library for remote execution and automation.

## Features

- Efficient binary protocol with MessagePack
- Frame-based communication
- Request/response correlation
- Stream multiplexing support
- Zero-copy serialization where possible

## Usage

This crate is primarily used internally by Mitoxide, but can be used standalone for custom protocol implementations.

```rust
use mitoxide_proto::{Message, Request, Response, Frame, FrameCodec};

// Create a request message
let request = Request::ProcExec {
    id: uuid::Uuid::new_v4(),
    command: vec!["echo".to_string(), "hello".to_string()],
    env: std::collections::HashMap::new(),
    timeout: None,
};

let message = Message::Request(request);
```

## Documentation

- [API Documentation](https://docs.rs/mitoxide-proto)
- [Main Mitoxide Documentation](https://docs.rs/mitoxide)
- [GitHub Repository](https://github.com/yourusername/mitoxide)

## License

This project is licensed under the MIT License.