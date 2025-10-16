//! # Mitoxide WASM Runtime
//!
//! WASM module loading and execution support for Mitoxide.

#![warn(missing_docs)]

/// WASM module loading and validation
pub mod module;

/// WASM execution runtime
pub mod runtime;

/// WASM-specific error types
pub mod error;

/// Test utilities for WASM modules
pub mod test_utils;

pub use module::{WasmModule, ModuleMetadata, WasmCapability, WasmImport};
pub use runtime::{WasmRuntime, WasmContext, WasmConfig};
pub use error::WasmError;