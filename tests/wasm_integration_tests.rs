//! Integration tests for WASM execution end-to-end
//! 
//! Tests WASM modules, JSON I/O, sandboxing, and error handling

mod integration;

use integration::wasm_tests::*;
use anyhow::Result;

/// Test WASM module creation and validation
#[tokio::test]
async fn test_wasm_module_creation_validation() -> Result<()> {
    let tests = WasmTests::new();
    tests.setup().await?;
    
    let result = tests.test_wasm_module_creation().await;
    
    tests.cleanup().await?;
    result
}

/// Test JSON input/output serialization
#[tokio::test]
async fn test_json_input_output_serialization() -> Result<()> {
    let tests = WasmTests::new();
    tests.setup().await?;
    
    let result = tests.test_json_io_serialization().await;
    
    tests.cleanup().await?;
    result
}

/// Test WASM sandboxing and resource limits
#[tokio::test]
async fn test_wasm_sandboxing_resource_limits() -> Result<()> {
    let tests = WasmTests::new();
    tests.setup().await?;
    
    let result = tests.test_wasm_sandboxing_limits().await;
    
    tests.cleanup().await?;
    result
}

/// Test WASM error handling and recovery
#[tokio::test]
async fn test_wasm_error_handling_recovery() -> Result<()> {
    let tests = WasmTests::new();
    tests.setup().await?;
    
    let result = tests.test_wasm_error_handling().await;
    
    tests.cleanup().await?;
    result
}

/// Test end-to-end WASM execution workflow
#[tokio::test]
async fn test_end_to_end_wasm_execution_workflow() -> Result<()> {
    let tests = WasmTests::new();
    tests.setup().await?;
    
    let result = tests.test_end_to_end_wasm_workflow().await;
    
    tests.cleanup().await?;
    result
}

/// Run comprehensive WASM execution test suite
#[tokio::test]
async fn test_comprehensive_wasm_suite() -> Result<()> {
    run_wasm_tests().await
}