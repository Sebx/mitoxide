//! Integration tests for agent bootstrap scenarios
//! 
//! Tests memfd_create bootstrap, /tmp fallback, failure scenarios, and cleanup

mod integration;

use integration::bootstrap_tests::*;
use anyhow::Result;

/// Test memfd_create bootstrap on Linux containers
#[tokio::test]
async fn test_memfd_create_bootstrap() -> Result<()> {
    let tests = BootstrapTests::new();
    tests.setup().await?;
    
    let result = tests.test_memfd_bootstrap().await;
    
    tests.cleanup().await?;
    result
}

/// Test /tmp fallback when memfd unavailable
#[tokio::test]
async fn test_tmp_fallback_bootstrap() -> Result<()> {
    let tests = BootstrapTests::new();
    tests.setup().await?;
    
    let result = tests.test_tmp_fallback_bootstrap().await;
    
    tests.cleanup().await?;
    result
}

/// Test bootstrap failure and recovery scenarios
#[tokio::test]
async fn test_bootstrap_failure_scenarios() -> Result<()> {
    let tests = BootstrapTests::new();
    tests.setup().await?;
    
    let result = tests.test_bootstrap_failure_scenarios().await;
    
    tests.cleanup().await?;
    result
}

/// Test agent self-deletion and cleanup
#[tokio::test]
async fn test_agent_self_deletion_cleanup() -> Result<()> {
    let tests = BootstrapTests::new();
    tests.setup().await?;
    
    let result = tests.test_agent_cleanup().await;
    
    tests.cleanup().await?;
    result
}

/// Test platform detection accuracy
#[tokio::test]
async fn test_platform_detection_accuracy() -> Result<()> {
    let tests = BootstrapTests::new();
    tests.setup().await?;
    
    let result = tests.test_platform_detection().await;
    
    tests.cleanup().await?;
    result
}

/// Run comprehensive bootstrap test suite
#[tokio::test]
async fn test_comprehensive_bootstrap_suite() -> Result<()> {
    run_bootstrap_tests().await
}