//! WASM module loading and validation

use crate::error::WasmError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use wasmtime::{Engine, Module};

/// WASM module capabilities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmCapability {
    /// WASI filesystem access
    WasiFs,
    /// WASI environment variables
    WasiEnv,
    /// WASI command line arguments
    WasiArgs,
    /// WASI standard I/O
    WasiStdio,
    /// WASI networking (if supported)
    WasiNet,
    /// Custom host functions
    HostFunctions,
}

/// WASM module metadata extracted from the module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// SHA256 hash of the module bytes
    pub hash: String,
    /// Size of the module in bytes
    pub size: usize,
    /// Detected capabilities required by the module
    pub capabilities: HashSet<WasmCapability>,
    /// Exported functions from the module
    pub exports: Vec<String>,
    /// Imported functions required by the module
    pub imports: Vec<WasmImport>,
    /// Whether the module is WASI-compatible
    pub is_wasi: bool,
}

/// Information about a WASM import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmImport {
    /// Module name (e.g., "wasi_snapshot_preview1")
    pub module: String,
    /// Function name
    pub name: String,
}

/// WASM module wrapper with validation and metadata
#[derive(Debug, Clone)]
pub struct WasmModule {
    /// Module bytecode
    pub bytes: Vec<u8>,
    /// Module metadata
    pub metadata: ModuleMetadata,
    /// Compiled wasmtime module (cached)
    compiled: Option<Module>,
}

impl WasmModule {
    /// Load a WASM module from bytes with validation
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, WasmError> {
        // Pre-validate basic format before attempting to parse
        Self::validate_basic_format(&bytes)?;
        
        let metadata = Self::extract_metadata(&bytes)?;
        Self::validate_module(&bytes, &metadata)?;
        
        Ok(WasmModule {
            bytes,
            metadata,
            compiled: None,
        })
    }
    
    /// Load a WASM module from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, WasmError> {
        let bytes = fs::read(path)?;
        Self::from_bytes(bytes)
    }
    
    /// Get the compiled wasmtime module, compiling if necessary
    pub fn get_compiled(&mut self, engine: &Engine) -> Result<&Module, WasmError> {
        if self.compiled.is_none() {
            let module = Module::from_binary(engine, &self.bytes)?;
            self.compiled = Some(module);
        }
        Ok(self.compiled.as_ref().unwrap())
    }
    
    /// Get the module hash
    pub fn hash(&self) -> &str {
        &self.metadata.hash
    }
    
    /// Check if the module requires a specific capability
    pub fn requires_capability(&self, capability: &WasmCapability) -> bool {
        self.metadata.capabilities.contains(capability)
    }
    
    /// Check if the module is WASI-compatible
    pub fn is_wasi(&self) -> bool {
        self.metadata.is_wasi
    }
    
    /// Extract metadata from WASM module bytes
    fn extract_metadata(bytes: &[u8]) -> Result<ModuleMetadata, WasmError> {
        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());
        
        // Create a temporary engine for parsing
        let engine = Engine::default();
        let module = Module::from_binary(&engine, bytes)
            .map_err(|e| WasmError::ModuleLoad(e.to_string()))?;
        
        let mut capabilities = HashSet::new();
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        let mut is_wasi = false;
        
        // Extract exports
        for export in module.exports() {
            exports.push(export.name().to_string());
        }
        
        // Extract imports and detect capabilities
        for import in module.imports() {
            let import_info = WasmImport {
                module: import.module().to_string(),
                name: import.name().to_string(),
            };
            
            // Detect WASI imports
            if import.module().starts_with("wasi_") {
                is_wasi = true;
                
                // Detect specific WASI capabilities
                match import.name() {
                    name if name.starts_with("fd_") => {
                        capabilities.insert(WasmCapability::WasiFs);
                        capabilities.insert(WasmCapability::WasiStdio);
                    }
                    name if name.starts_with("environ_") => {
                        capabilities.insert(WasmCapability::WasiEnv);
                    }
                    name if name.starts_with("args_") => {
                        capabilities.insert(WasmCapability::WasiArgs);
                    }
                    name if name.starts_with("sock_") => {
                        capabilities.insert(WasmCapability::WasiNet);
                    }
                    _ => {}
                }
            } else if import.module() != "env" {
                // Non-standard imports indicate custom host functions
                capabilities.insert(WasmCapability::HostFunctions);
            }
            
            imports.push(import_info);
        }
        
        // If it's WASI, ensure basic WASI capabilities are marked
        if is_wasi {
            capabilities.insert(WasmCapability::WasiStdio);
        }
        
        Ok(ModuleMetadata {
            hash,
            size: bytes.len(),
            capabilities,
            exports,
            imports,
            is_wasi,
        })
    }
    
    /// Validate basic WASM format before parsing
    fn validate_basic_format(bytes: &[u8]) -> Result<(), WasmError> {
        // Check minimum size
        if bytes.len() < 8 {
            return Err(WasmError::InvalidFormat(
                "WASM module too small (minimum 8 bytes)".to_string()
            ));
        }
        
        // Validate WASM magic number
        if &bytes[0..4] != b"\0asm" {
            return Err(WasmError::InvalidFormat(
                "Invalid WASM magic number".to_string()
            ));
        }
        
        // Check module size limits (e.g., 64MB max)
        const MAX_MODULE_SIZE: usize = 64 * 1024 * 1024;
        if bytes.len() > MAX_MODULE_SIZE {
            return Err(WasmError::ModuleValidation(format!(
                "Module too large: {} bytes (max: {} bytes)",
                bytes.len(), MAX_MODULE_SIZE
            )));
        }
        
        Ok(())
    }
    
    /// Validate the WASM module
    fn validate_module(_bytes: &[u8], metadata: &ModuleMetadata) -> Result<(), WasmError> {
        // Check for unsupported capabilities
        if metadata.capabilities.contains(&WasmCapability::WasiNet) {
            return Err(WasmError::UnsupportedCapability(
                "WASI networking is not supported".to_string()
            ));
        }
        
        // Ensure WASI modules have required exports
        if metadata.is_wasi && !metadata.exports.contains(&"_start".to_string()) {
            return Err(WasmError::ModuleValidation(
                "WASI module must export '_start' function".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_modules::{minimal_wasm, simple_function_wasm, wasi_hello_wasm, INVALID_MAGIC_WASM};
    
    #[test]
    fn test_minimal_wasm_module() {
        let module = WasmModule::from_bytes(minimal_wasm().to_vec()).unwrap();
        assert_eq!(module.metadata.size, minimal_wasm().len());
        assert!(!module.is_wasi());
        assert!(module.metadata.exports.is_empty());
        assert!(module.metadata.imports.is_empty());
    }
    
    #[test]
    fn test_simple_function_wasm() {
        let module = WasmModule::from_bytes(simple_function_wasm().to_vec()).unwrap();
        assert!(!module.is_wasi());
        assert!(module.metadata.exports.contains(&"add".to_string()));
        assert!(!module.requires_capability(&WasmCapability::WasiStdio));
    }
    
    #[test]
    fn test_wasi_module_detection() {
        let module = WasmModule::from_bytes(wasi_hello_wasm().to_vec()).unwrap();
        assert!(module.is_wasi());
        assert!(module.metadata.exports.contains(&"_start".to_string()));
        assert!(module.metadata.exports.contains(&"memory".to_string()));
        assert!(module.requires_capability(&WasmCapability::WasiStdio));
        
        // Check that WASI imports are detected
        let has_fd_write = module.metadata.imports.iter()
            .any(|imp| imp.module == "wasi_snapshot_preview1" && imp.name == "fd_write");
        assert!(has_fd_write);
        
        let has_environ_get = module.metadata.imports.iter()
            .any(|imp| imp.module == "wasi_snapshot_preview1" && imp.name == "environ_get");
        assert!(has_environ_get);
        
        // Should detect environment capability
        assert!(module.requires_capability(&WasmCapability::WasiEnv));
    }
    
    #[test]
    fn test_invalid_wasm_magic() {
        let result = WasmModule::from_bytes(INVALID_MAGIC_WASM.to_vec());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::InvalidFormat(_)));
    }
    
    #[test]
    fn test_empty_bytes() {
        let empty_bytes = vec![];
        let result = WasmModule::from_bytes(empty_bytes);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::InvalidFormat(_)));
    }
    
    #[test]
    fn test_module_too_large() {
        // Create a module that's too large
        let mut large_bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic
        large_bytes.extend(vec![0x01, 0x00, 0x00, 0x00]); // version
        large_bytes.extend(vec![0x00; 65 * 1024 * 1024]); // > 64MB of padding
        
        let result = WasmModule::from_bytes(large_bytes);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::ModuleValidation(_)));
    }
    
    #[test]
    fn test_hash_calculation() {
        let module1 = WasmModule::from_bytes(minimal_wasm().to_vec()).unwrap();
        let module2 = WasmModule::from_bytes(simple_function_wasm().to_vec()).unwrap();
        
        // Different modules should have different hashes
        assert_ne!(module1.hash(), module2.hash());
        
        // Same module should have same hash
        let module1_copy = WasmModule::from_bytes(minimal_wasm().to_vec()).unwrap();
        assert_eq!(module1.hash(), module1_copy.hash());
    }
    
    #[test]
    fn test_capability_detection() {
        let wasi_module = WasmModule::from_bytes(wasi_hello_wasm().to_vec()).unwrap();
        
        // Should detect WASI stdio capability
        assert!(wasi_module.requires_capability(&WasmCapability::WasiStdio));
        
        // Should detect environment capability
        assert!(wasi_module.requires_capability(&WasmCapability::WasiEnv));
        
        // Should not detect networking (not in this module)
        assert!(!wasi_module.requires_capability(&WasmCapability::WasiNet));
        
        let simple_module = WasmModule::from_bytes(simple_function_wasm().to_vec()).unwrap();
        
        // Simple module should not require WASI capabilities
        assert!(!simple_module.requires_capability(&WasmCapability::WasiStdio));
    }
    
    #[test]
    fn test_compiled_module_caching() {
        let mut module = WasmModule::from_bytes(minimal_wasm().to_vec()).unwrap();
        let engine = wasmtime::Engine::default();
        
        // First compilation
        let _compiled1 = module.get_compiled(&engine).unwrap();
        
        // Check that the module is now cached
        assert!(module.compiled.is_some());
        
        // Second call should return cached version without recompiling
        let _compiled2 = module.get_compiled(&engine).unwrap();
        
        // Module should still be cached
        assert!(module.compiled.is_some());
    }
    
    #[test]
    fn test_from_file_nonexistent() {
        let result = WasmModule::from_file("/nonexistent/path/module.wasm");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::Io(_)));
    }
}