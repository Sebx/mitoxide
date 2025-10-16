//! Test utilities for WASM module testing

pub mod test_modules {
    use std::sync::OnceLock;
    
    /// Generate test WASM modules using wat
    fn generate_minimal_wasm() -> Vec<u8> {
        wat::parse_str("(module)").unwrap()
    }
    
    fn generate_simple_function_wasm() -> Vec<u8> {
        wat::parse_str(r#"
            (module
              (func $add (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add)
              (export "add" (func $add)))
        "#).unwrap()
    }
    
    fn generate_wasi_hello_wasm() -> Vec<u8> {
        wat::parse_str(r#"
            (module
              (import "wasi_snapshot_preview1" "fd_write" 
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "environ_get"
                (func $environ_get (param i32 i32) (result i32)))
              (func $_start
                nop)
              (export "_start" (func $_start))
              (memory 1)
              (export "memory" (memory 0)))
        "#).unwrap()
    }
    
    // Use OnceLock to cache the generated WASM modules
    static MINIMAL_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static SIMPLE_FUNCTION_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    static WASI_HELLO_WASM: OnceLock<Vec<u8>> = OnceLock::new();
    
    /// A minimal valid WASM module that does nothing
    pub fn minimal_wasm() -> &'static [u8] {
        MINIMAL_WASM.get_or_init(generate_minimal_wasm)
    }
    
    /// A WASM module with a simple function export
    pub fn simple_function_wasm() -> &'static [u8] {
        SIMPLE_FUNCTION_WASM.get_or_init(generate_simple_function_wasm)
    }
    
    /// A WASI-compatible WASM module with _start export
    pub fn wasi_hello_wasm() -> &'static [u8] {
        WASI_HELLO_WASM.get_or_init(generate_wasi_hello_wasm)
    }
    
    /// Invalid WASM with wrong magic number
    pub const INVALID_MAGIC_WASM: &[u8] = &[
        0xFF, 0xFF, 0xFF, 0xFF, // wrong magic
        0x01, 0x00, 0x00, 0x00, // version
    ];
}