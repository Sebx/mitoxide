# Mitoxide WASM

[![Crates.io](https://img.shields.io/crates/v/mitoxide-wasm.svg)](https://crates.io/crates/mitoxide-wasm)
[![Documentation](https://docs.rs/mitoxide-wasm/badge.svg)](https://docs.rs/mitoxide-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

WebAssembly runtime support for Mitoxide - enabling secure remote execution of WASM modules.

## Features

- WebAssembly module execution with Wasmtime
- Sandboxed execution environment
- JSON input/output serialization
- Resource limits and security controls
- WASI support for file system access

## Usage

This crate is primarily used internally by Mitoxide, but can be used standalone for WASM execution.

```rust
use mitoxide_wasm::{WasmRuntime, WasmModule};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = WasmRuntime::new()?;
    
    // Load WASM module
    let module_bytes = std::fs::read("module.wasm")?;
    let module = WasmModule::from_bytes(&module_bytes)?;
    
    // Execute with JSON input
    let input = serde_json::json!({"message": "Hello, WASM!"});
    let result = runtime.execute(&module, &input).await?;
    
    println!("Result: {}", result);
    Ok(())
}
```

## Security

The WASM runtime provides sandboxed execution with:
- Memory limits
- CPU time limits
- File system access controls
- Network access restrictions

## Documentation

- [API Documentation](https://docs.rs/mitoxide-wasm)
- [Main Mitoxide Documentation](https://docs.rs/mitoxide)
- [GitHub Repository](https://github.com/yourusername/mitoxide)

## License

This project is licensed under the MIT License.