//! WASM-specific error types

use thiserror::Error;

/// WASM-specific errors
#[derive(Debug, Error)]
pub enum WasmError {
    /// Module loading error
    #[error("Module loading error: {0}")]
    ModuleLoad(String),
    
    /// Module validation error
    #[error("Module validation error: {0}")]
    ModuleValidation(String),
    
    /// Invalid module format
    #[error("Invalid module format: {0}")]
    InvalidFormat(String),
    
    /// Unsupported capability
    #[error("Unsupported capability: {0}")]
    UnsupportedCapability(String),
    
    /// Execution error
    #[error("Execution error: {0}")]
    Execution(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Wasmtime error
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),
}