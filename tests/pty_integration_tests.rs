//! Integration tests for privilege escalation and PTY operations
//! 
//! Tests sudo prompts, PTY operations, failure scenarios, and security

mod integration;

use integration::pty_tests::*;
use anyhow::Result;

/// Test sudo prompt detection and handling
#[tokio::test]
async fn test_sudo_prompt_detection_handling() -> Result<()> {
    let tests = PtyTests::new();
    tests.setup().await?;
    
    let result = tests.test_sudo_prompt_detection().await;
    
    tests.cleanup().await?;
    result
}

/// Test PTY operations with interactive commands
#[tokio::test]
async fn test_pty_interactive_command_operations() -> Result<()> {
    let tests = PtyTests::new();
    tests.setup().await?;
    
    let result = tests.test_pty_interactive_operations().await;
    
    tests.cleanup().await?;
    result
}

/// Test privilege escalation failure scenarios
#[tokio::test]
async fn test_privilege_escalation_failure_scenarios() -> Result<()> {
    let tests = PtyTests::new();
    tests.setup().await?;
    
    let result = tests.test_privilege_escalation_failures().await;
    
    tests.cleanup().await?;
    result
}

/// Test credential handling and security
#[tokio::test]
async fn test_credential_handling_and_security() -> Result<()> {
    let tests = PtyTests::new();
    tests.setup().await?;
    
    let result = tests.test_credential_handling_security().await;
    
    tests.cleanup().await?;
    result
}

/// Test comprehensive PTY and privilege escalation workflow
#[tokio::test]
async fn test_comprehensive_pty_privilege_workflow() -> Result<()> {
    let tests = PtyTests::new();
    tests.setup().await?;
    
    let result = tests.test_comprehensive_pty_workflow().await;
    
    tests.cleanup().await?;
    result
}

/// Run comprehensive PTY and privilege escalation test suite
#[tokio::test]
async fn test_comprehensive_pty_suite() -> Result<()> {
    run_pty_tests().await
}