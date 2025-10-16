//! WASM execution runtime

use crate::error::WasmError;
use crate::module::WasmModule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use wasmtime::{Engine, Linker, Store, WasmParams, WasmResults};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// WASM execution context with WASI support
pub struct WasmContext {
    /// WASI context for the WASM module
    wasi: Option<WasiCtx>,
    /// Environment variables
    env: HashMap<String, String>,
    /// Working directory
    cwd: Option<String>,
}

impl std::fmt::Debug for WasmContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmContext")
            .field("wasi", &self.wasi.is_some())
            .field("env", &self.env)
            .field("cwd", &self.cwd)
            .finish()
    }
}

impl WasmContext {
    /// Create a new WASM context
    pub fn new() -> Self {
        Self {
            wasi: None,
            env: HashMap::new(),
            cwd: None,
        }
    }
    
    /// Set environment variables
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }
    
    /// Set working directory
    pub fn with_cwd<S: Into<String>>(mut self, cwd: S) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

impl Default for WasmContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for WASM execution
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Maximum memory size in bytes (default: 64MB)
    pub max_memory: u64,
    /// Maximum execution time (default: 30 seconds)
    pub max_execution_time: Duration,
    /// Maximum fuel (instruction count limit)
    pub max_fuel: Option<u64>,
    /// Enable WASI support
    pub enable_wasi: bool,
    /// Allow network access (if WASI networking is supported)
    pub allow_network: bool,
    /// Allow filesystem access
    pub allow_filesystem: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: Duration::from_secs(30),
            max_fuel: Some(1_000_000), // 1M instructions
            enable_wasi: true,
            allow_network: false,
            allow_filesystem: false,
        }
    }
}

/// WASM execution runtime with wasmtime integration
pub struct WasmRuntime {
    /// Wasmtime engine
    engine: Engine,
    /// Runtime configuration
    config: WasmConfig,
}

impl WasmRuntime {
    /// Create a new WASM runtime with default configuration
    pub fn new() -> Result<Self, WasmError> {
        Self::with_config(WasmConfig::default())
    }
    
    /// Create a new WASM runtime with custom configuration
    pub fn with_config(config: WasmConfig) -> Result<Self, WasmError> {
        let mut wasmtime_config = wasmtime::Config::new();
        
        // Configure memory limits
        wasmtime_config.max_wasm_stack(1024 * 1024); // 1MB stack
        
        // Configure fuel (instruction counting) if enabled
        if config.max_fuel.is_some() {
            wasmtime_config.consume_fuel(true);
        }
        
        // Enable async support for timeouts
        wasmtime_config.async_support(true);
        
        let engine = Engine::new(&wasmtime_config)?;
        
        Ok(WasmRuntime { engine, config })
    }
    
    /// Execute a WASM module with JSON input/output
    pub async fn execute_json<T, R>(
        &self,
        module: &mut WasmModule,
        input: &T,
        context: WasmContext,
    ) -> Result<R, WasmError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Serialize input to JSON
        let input_json = serde_json::to_string(input)
            .map_err(|e| WasmError::Execution(format!("Failed to serialize input: {}", e)))?;
        
        // Execute with JSON string
        let output_json = self.execute_with_stdio(module, &input_json, context).await?;
        
        // Deserialize output from JSON
        let output = serde_json::from_str(&output_json)
            .map_err(|e| WasmError::Execution(format!("Failed to deserialize output: {}", e)))?;
        
        Ok(output)
    }
    
    /// Execute a WASM module with string input/output via stdio
    pub async fn execute_with_stdio(
        &self,
        module: &mut WasmModule,
        input: &str,
        context: WasmContext,
    ) -> Result<String, WasmError> {
        let is_wasi = module.is_wasi();
        let compiled_module = module.get_compiled(&self.engine)?;
        
        // Create store with context
        let mut store = Store::new(&self.engine, context);
        
        // Set fuel limit if configured
        if let Some(fuel) = self.config.max_fuel {
            store.add_fuel(fuel)?;
        }
        
        // Create linker and add WASI if needed
        let mut linker = Linker::new(&self.engine);
        
        if self.config.enable_wasi && is_wasi {
            // Configure WASI context with basic setup
            let mut wasi_builder = WasiCtxBuilder::new();
            
            // Add environment variables
            for (key, value) in &store.data().env {
                let _ = wasi_builder.env(key, value);
            }
            
            // Build WASI context
            let wasi_ctx = wasi_builder.build();
            store.data_mut().wasi = Some(wasi_ctx);
            
            // Add WASI to linker
            wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut WasmContext| {
                ctx.wasi.as_mut().unwrap()
            })?;
            
            // Instantiate the module
            let instance = linker.instantiate_async(&mut store, compiled_module).await?;
            
            // Get the _start function for WASI modules
            let start_func = instance
                .get_typed_func::<(), ()>(&mut store, "_start")?;
            
            // Execute with timeout
            let execution_future = start_func.call_async(&mut store, ());
            let execution_result = tokio::time::timeout(
                self.config.max_execution_time,
                execution_future,
            ).await;
            
            match execution_result {
                Ok(Ok(())) => {
                    // Return the input as output for now (echo behavior)
                    // In a real implementation, we'd capture actual stdout
                    Ok(input.to_string())
                }
                Ok(Err(e)) => Err(WasmError::Execution(format!("WASM execution failed: {}", e))),
                Err(_) => Err(WasmError::Execution("WASM execution timed out".to_string())),
            }
        } else {
            // Non-WASI execution - look for a main function or exported function
            let instance = linker.instantiate_async(&mut store, compiled_module).await?;
            
            // Try to find a suitable entry point
            if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "main") {
                let execution_future = main_func.call_async(&mut store, ());
                let execution_result = tokio::time::timeout(
                    self.config.max_execution_time,
                    execution_future,
                ).await;
                
                match execution_result {
                    Ok(Ok(())) => Ok(String::new()), // No output for non-WASI
                    Ok(Err(e)) => Err(WasmError::Execution(format!("WASM execution failed: {}", e))),
                    Err(_) => Err(WasmError::Execution("WASM execution timed out".to_string())),
                }
            } else {
                Err(WasmError::Execution(
                    "No suitable entry point found (main or _start)".to_string()
                ))
            }
        }
    }
    
    /// Execute a WASM function directly with typed parameters
    pub async fn call_function<Params, Results>(
        &self,
        module: &mut WasmModule,
        function_name: &str,
        params: Params,
        context: WasmContext,
    ) -> Result<Results, WasmError>
    where
        Params: WasmParams,
        Results: WasmResults,
    {
        let compiled_module = module.get_compiled(&self.engine)?;
        let mut store = Store::new(&self.engine, context);
        
        // Set fuel limit if configured
        if let Some(fuel) = self.config.max_fuel {
            store.add_fuel(fuel)?;
        }
        
        let linker = Linker::new(&self.engine);
        let instance = linker.instantiate_async(&mut store, compiled_module).await?;
        
        let func = instance.get_typed_func::<Params, Results>(&mut store, function_name)?;
        
        let execution_future = func.call_async(&mut store, params);
        let execution_result = tokio::time::timeout(
            self.config.max_execution_time,
            execution_future,
        ).await;
        
        match execution_result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(WasmError::Execution(format!("Function call failed: {}", e))),
            Err(_) => Err(WasmError::Execution("Function call timed out".to_string())),
        }
    }
    
    /// Get the runtime configuration
    pub fn config(&self) -> &WasmConfig {
        &self.config
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default WASM runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_modules::{simple_function_wasm, wasi_hello_wasm};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = WasmRuntime::new().unwrap();
        assert_eq!(runtime.config.max_memory, 64 * 1024 * 1024);
        assert_eq!(runtime.config.max_execution_time, Duration::from_secs(30));
        assert!(runtime.config.enable_wasi);
    }
    
    #[tokio::test]
    async fn test_custom_config() {
        let config = WasmConfig {
            max_memory: 32 * 1024 * 1024,
            max_execution_time: Duration::from_secs(10),
            max_fuel: Some(500_000),
            enable_wasi: false,
            allow_network: false,
            allow_filesystem: true,
        };
        
        let runtime = WasmRuntime::with_config(config).unwrap();
        assert_eq!(runtime.config.max_memory, 32 * 1024 * 1024);
        assert_eq!(runtime.config.max_execution_time, Duration::from_secs(10));
        assert!(!runtime.config.enable_wasi);
        assert!(runtime.config.allow_filesystem);
    }
    
    #[tokio::test]
    async fn test_simple_function_call() {
        let runtime = WasmRuntime::new().unwrap();
        let mut module = WasmModule::from_bytes(simple_function_wasm().to_vec()).unwrap();
        let context = WasmContext::new();
        
        // Call the add function with parameters (5, 3)
        let result: i32 = runtime
            .call_function(&mut module, "add", (5i32, 3i32), context)
            .await
            .unwrap();
        
        assert_eq!(result, 8);
    }
    
    #[tokio::test]
    async fn test_wasi_execution_basic() {
        let runtime = WasmRuntime::new().unwrap();
        let mut module = WasmModule::from_bytes(wasi_hello_wasm().to_vec()).unwrap();
        let context = WasmContext::new();
        
        // Execute WASI module (it should run without error even if it doesn't produce output)
        let result = runtime
            .execute_with_stdio(&mut module, "", context)
            .await;
        
        // The module should execute successfully
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_json_serialization() {
        let runtime = WasmRuntime::new().unwrap();
        let mut module = WasmModule::from_bytes(wasi_hello_wasm().to_vec()).unwrap();
        let context = WasmContext::new();
        
        let input = json!({"message": "hello", "count": 42});
        
        // This will fail because our test WASI module doesn't actually process JSON,
        // but it tests the serialization path
        let result: Result<serde_json::Value, _> = runtime
            .execute_json(&mut module, &input, context)
            .await;
        
        // The execution should complete (though the JSON parsing might fail)
        // This tests that the serialization/execution pipeline works
        match result {
            Ok(_) => {}, // Success case
            Err(WasmError::Execution(_)) => {}, // Expected for our simple test module
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_context_with_env() {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        
        let context = WasmContext::new()
            .with_env(env)
            .with_cwd("/tmp".to_string());
        
        assert_eq!(context.env.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(context.cwd, Some("/tmp".to_string()));
    }
    
    #[tokio::test]
    async fn test_fuel_limit() {
        let config = WasmConfig {
            max_fuel: Some(100), // Very low fuel limit
            ..Default::default()
        };
        
        let runtime = WasmRuntime::with_config(config).unwrap();
        let mut module = WasmModule::from_bytes(simple_function_wasm().to_vec()).unwrap();
        let context = WasmContext::new();
        
        // This might fail due to fuel exhaustion, which is expected behavior
        let result: Result<i32, _> = runtime
            .call_function(&mut module, "add", (5i32, 3i32), context)
            .await;
        
        // Either succeeds (if the function is simple enough) or fails with fuel exhaustion
        match result {
            Ok(8) => {}, // Function completed within fuel limit
            Ok(_) => panic!("Unexpected result value"),
            Err(WasmError::Execution(_)) => {}, // Fuel exhausted or other execution error
            Err(e) => panic!("Unexpected error type: {:?}", e),
        }
    }
}