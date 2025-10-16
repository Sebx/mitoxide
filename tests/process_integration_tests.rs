//! Integration tests for process execution and I/O handling
//! 
//! Tests large I/O streaming, environment variables, binary data, and timeouts

mod integration;

use integration::process_tests::*;
use anyhow::Result;

/// Test large stdout/stderr streaming
#[tokio::test]
async fn test_large_stdout_stderr_streaming() -> Result<()> {
    let tests = ProcessTests::new();
    tests.setup().await?;
    
    let result = tests.test_large_io_streaming().await;
    
    tests.cleanup().await?;
    result
}

/// Test environment variable passthrough
#[tokio::test]
async fn test_environment_variable_passthrough() -> Result<()> {
    let tests = ProcessTests::new();
    tests.setup().await?;
    
    let result = tests.test_environment_passthrough().await;
    
    tests.cleanup().await?;
    result
}

/// Test binary data handling and encoding
#[tokio::test]
async fn test_binary_data_handling_encoding() -> Result<()> {
    let tests = ProcessTests::new();
    tests.setup().await?;
    
    let result = tests.test_binary_data_handling().await;
    
    tests.cleanup().await?;
    result
}

/// Test process timeout and cancellation
#[tokio::test]
async fn test_process_timeout_and_cancellation() -> Result<()> {
    let tests = ProcessTests::new();
    tests.setup().await?;
    
    let result = tests.test_process_timeout_cancellation().await;
    
    tests.cleanup().await?;
    result
}

/// Test concurrent process execution
#[tokio::test]
async fn test_concurrent_process_execution() -> Result<()> {
    let tests = ProcessTests::new();
    tests.setup().await?;
    
    let result = tests.test_concurrent_process_execution().await;
    
    tests.cleanup().await?;
    result
}

/// Run comprehensive process execution test suite
#[tokio::test]
async fn test_comprehensive_process_suite() -> Result<()> {
    run_process_tests().await
}