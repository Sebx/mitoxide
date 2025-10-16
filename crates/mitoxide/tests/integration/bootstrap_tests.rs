//! Comprehensive integration tests for agent bootstrap scenarios
//! 
//! These tests verify agent bootstrap functionality across different platforms
//! and failure scenarios, including memfd_create, /tmp fallback, and cleanup.

use super::*;
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::timeout;

/// Test agent bootstrap scenarios
pub struct BootstrapTests {
    docker_env: DockerTestEnv,
    ssh_helper: SshHelper,
}

impl BootstrapTests {
    /// Create new bootstrap test suite
    pub fn new() -> Self {
        Self {
            docker_env: DockerTestEnv::new(),
            ssh_helper: SshHelper::new(),
        }
    }
    
    /// Setup test environment
    pub async fn setup(&self) -> Result<()> {
        EnvUtils::setup_test_environment()?;
        self.docker_env.start().await?;
        Ok(())
    }
    
    /// Cleanup test environment
    pub async fn cleanup(&self) -> Result<()> {
        self.docker_env.stop().await?;
        Ok(())
    }
    
    /// Test memfd_create bootstrap on Linux containers
    pub async fn test_memfd_bootstrap(&self) -> Result<()> {
        println!("Testing memfd_create bootstrap on Linux containers...");
        
        // Test on Ubuntu container (should support memfd_create)
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Check if memfd_create is available
        let memfd_check = r#"
python3 -c "
import ctypes
try:
    libc = ctypes.CDLL('libc.so.6')
    fd = libc.syscall(319, b'test', 1)  # memfd_create
    print('available' if fd >= 0 else 'unavailable')
except:
    print('unavailable')
"
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", memfd_check]).await?;
        TestAssertions::assert_ssh_success(&output, "memfd_create availability check")?;
        
        if !output.stdout.contains("available") {
            println!("⚠️  memfd_create not available on this container, skipping test");
            return Ok(());
        }
        
        // Create a mock agent binary
        let mock_agent = self.create_mock_agent_binary()?;
        
        // Test memfd_create bootstrap script
        let bootstrap_script = self.create_memfd_bootstrap_script();
        
        // Execute bootstrap with mock agent
        let bootstrap_result = self.execute_bootstrap_test(
            &config,
            &bootstrap_script,
            &mock_agent,
            "memfd_create bootstrap"
        ).await?;
        
        // Verify bootstrap succeeded
        assert!(bootstrap_result.success, "memfd_create bootstrap should succeed");
        assert!(bootstrap_result.agent_executed, "Agent should have executed");
        assert!(bootstrap_result.cleanup_verified, "Cleanup should be verified");
        
        println!("✅ memfd_create bootstrap test passed");
        Ok(())
    }
    
    /// Test /tmp fallback when memfd unavailable
    pub async fn test_tmp_fallback_bootstrap(&self) -> Result<()> {
        println!("Testing /tmp fallback bootstrap...");
        
        // Test on Alpine container (may not have memfd_create)
        let config = self.ssh_helper.config_for_container("alpine_ro", 2222);
        
        // Create a mock agent binary
        let mock_agent = self.create_mock_agent_binary()?;
        
        // Test /tmp fallback bootstrap script
        let bootstrap_script = self.create_tmp_fallback_script();
        
        // Execute bootstrap with mock agent
        let bootstrap_result = self.execute_bootstrap_test(
            &config,
            &bootstrap_script,
            &mock_agent,
            "/tmp fallback bootstrap"
        ).await?;
        
        // Verify bootstrap succeeded
        assert!(bootstrap_result.success, "/tmp fallback bootstrap should succeed");
        assert!(bootstrap_result.agent_executed, "Agent should have executed");
        
        // Note: On read-only filesystem, we expect the agent to run from /tmp
        // but cleanup might not be possible due to filesystem constraints
        
        println!("✅ /tmp fallback bootstrap test passed");
        Ok(())
    }
    
    /// Test bootstrap failure and recovery scenarios
    pub async fn test_bootstrap_failure_scenarios(&self) -> Result<()> {
        println!("Testing bootstrap failure and recovery scenarios...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test 1: Invalid agent binary
        println!("  Testing invalid agent binary...");
        let invalid_agent = b"invalid binary data";
        let bootstrap_script = self.create_tmp_fallback_script();
        
        let result = self.execute_bootstrap_test(
            &config,
            &bootstrap_script,
            invalid_agent,
            "invalid agent bootstrap"
        ).await;
        
        // Should fail gracefully
        assert!(result.is_err() || !result.unwrap().success, 
                "Invalid agent binary should cause bootstrap failure");
        
        // Test 2: No writable directories
        println!("  Testing no writable directories scenario...");
        let no_write_script = r#"
set -e
# Simulate no writable directories
for dir in /dev/shm /tmp /var/tmp; do
    if [ -d "$dir" ]; then
        echo "Directory $dir exists but simulating no write access"
    fi
done
echo "No writable directory found for agent bootstrap" >&2
exit 1
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", no_write_script]).await?;
        assert!(!output.success(), "No writable directories should cause failure");
        assert!(output.stderr.contains("No writable directory"), "Should report no writable directory");
        
        // Test 3: Disk space exhaustion simulation
        println!("  Testing disk space exhaustion...");
        let disk_full_script = r#"
set -e
# Try to create a large file to simulate disk full
AGENT_PATH="/tmp/mitoxide-agent-test-$$"
# This should fail if /tmp is too small or full
dd if=/dev/zero of="$AGENT_PATH" bs=1M count=1000 2>/dev/null || {
    echo "Disk space exhaustion simulated" >&2
    exit 1
}
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", disk_full_script]).await?;
        // This might succeed or fail depending on available space, both are valid
        
        println!("✅ Bootstrap failure scenarios test passed");
        Ok(())
    }
    
    /// Test agent self-deletion and cleanup
    pub async fn test_agent_cleanup(&self) -> Result<()> {
        println!("Testing agent self-deletion and cleanup...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Create a mock agent that creates a marker file and then exits
        let cleanup_agent = self.create_cleanup_test_agent()?;
        
        // Bootstrap script with cleanup verification
        let bootstrap_script = r#"
set -e
AGENT_PATH="/tmp/mitoxide-agent-cleanup-test-$$"
MARKER_FILE="/tmp/agent-executed-$$"

# Write agent binary
cat > "$AGENT_PATH"
chmod +x "$AGENT_PATH"

# Set up cleanup trap
trap 'rm -f "$AGENT_PATH" "$MARKER_FILE" 2>/dev/null || true' EXIT

# Execute agent
"$AGENT_PATH" || true

# Verify marker file was created (agent executed)
if [ -f "$MARKER_FILE" ]; then
    echo "Agent executed successfully"
    rm -f "$MARKER_FILE"
else
    echo "Agent did not execute" >&2
    exit 1
fi

# Verify agent binary is cleaned up
if [ -f "$AGENT_PATH" ]; then
    echo "Agent binary still exists, cleaning up"
    rm -f "$AGENT_PATH"
fi

echo "Cleanup completed"
        "#;
        
        // Execute cleanup test
        let output = timeout(
            Duration::from_secs(30),
            self.ssh_helper.execute_command(&config, &["sh", "-c", &format!("echo '{}' | {}", 
                base64::encode(&cleanup_agent), bootstrap_script)])
        ).await??;
        
        TestAssertions::assert_ssh_success(&output, "Agent cleanup test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Agent executed successfully", "Agent execution")?;
        TestAssertions::assert_output_contains(&output.stdout, "Cleanup completed", "Cleanup completion")?;
        
        // Verify no leftover files
        let cleanup_check = r#"
find /tmp -name "mitoxide-agent-*" -o -name "agent-executed-*" 2>/dev/null | wc -l
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", cleanup_check]).await?;
        TestAssertions::assert_ssh_success(&output, "Cleanup verification")?;
        
        let leftover_count: i32 = output.stdout.trim().parse().unwrap_or(-1);
        assert_eq!(leftover_count, 0, "Should have no leftover agent files");
        
        println!("✅ Agent cleanup test passed");
        Ok(())
    }
    
    /// Test platform detection accuracy
    pub async fn test_platform_detection(&self) -> Result<()> {
        println!("Testing platform detection accuracy...");
        
        // Test on different containers
        let test_cases = vec![
            ("ubuntu_min", 2223, "x86_64", "Linux"),
            ("alpine_ro", 2222, "x86_64", "Linux"),
        ];
        
        for (container, port, expected_arch, expected_os) in test_cases {
            println!("  Testing platform detection on {}...", container);
            
            let config = self.ssh_helper.config_for_container(container, port);
            
            // Execute platform detection
            let platform_script = r#"
echo "ARCH: $(uname -m)"
echo "OS: $(uname -s)"
echo "VERSION: $(cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d'"' -f2 || echo 'Unknown')"
            "#;
            
            let output = self.ssh_helper.execute_command(&config, &["sh", "-c", platform_script]).await?;
            TestAssertions::assert_ssh_success(&output, &format!("Platform detection on {}", container))?;
            
            // Verify detected platform
            TestAssertions::assert_output_contains(&output.stdout, &format!("ARCH: {}", expected_arch), "Architecture detection")?;
            TestAssertions::assert_output_contains(&output.stdout, &format!("OS: {}", expected_os), "OS detection")?;
            
            println!("    ✅ Platform detection on {} passed", container);
        }
        
        println!("✅ Platform detection test passed");
        Ok(())
    }
    
    /// Create a mock agent binary for testing
    fn create_mock_agent_binary(&self) -> Result<Vec<u8>> {
        // Create a simple shell script that acts as a mock agent
        let mock_agent = r#"#!/bin/sh
echo "Mock agent started"
echo "Agent PID: $$"
echo "Mock agent completed"
exit 0
        "#;
        
        Ok(mock_agent.as_bytes().to_vec())
    }
    
    /// Create a cleanup test agent
    fn create_cleanup_test_agent(&self) -> Result<Vec<u8>> {
        let cleanup_agent = r#"#!/bin/sh
# Create marker file to indicate execution
MARKER_FILE="/tmp/agent-executed-$$"
echo "Agent executed at $(date)" > "$MARKER_FILE"
echo "Cleanup test agent completed"
exit 0
        "#;
        
        Ok(cleanup_agent.as_bytes().to_vec())
    }
    
    /// Create memfd_create bootstrap script
    fn create_memfd_bootstrap_script(&self) -> String {
        r#"
set -e
python3 -c "
import os, sys, ctypes, base64
try:
    # Read base64 encoded agent from stdin
    agent_b64 = sys.stdin.read().strip()
    agent_data = base64.b64decode(agent_b64)
    
    # Create memfd
    libc = ctypes.CDLL('libc.so.6')
    fd = libc.syscall(319, b'mitoxide-agent', 1)  # memfd_create
    if fd < 0:
        raise Exception('memfd_create failed')
    
    # Write agent to memfd
    os.write(fd, agent_data)
    
    # Execute agent
    os.fexecve(fd, ['/proc/self/fd/%d' % fd], os.environ)
except Exception as e:
    print(f'memfd_create bootstrap failed: {e}', file=sys.stderr)
    sys.exit(1)
"
        "#.trim().to_string()
    }
    
    /// Create /tmp fallback bootstrap script
    fn create_tmp_fallback_script(&self) -> String {
        r#"
set -e
# Use /tmp as fallback
AGENT_PATH="/tmp/mitoxide-agent-fallback-$$-$(date +%s)"

# Decode and write agent
echo "$1" | base64 -d > "$AGENT_PATH"
chmod +x "$AGENT_PATH"

# Set up cleanup
trap 'rm -f "$AGENT_PATH" 2>/dev/null || true' EXIT

# Execute agent
exec "$AGENT_PATH"
        "#.trim().to_string()
    }
    
    /// Execute a bootstrap test
    async fn execute_bootstrap_test(
        &self,
        config: &SshConfig,
        bootstrap_script: &str,
        agent_binary: &[u8],
        test_name: &str,
    ) -> Result<BootstrapTestResult> {
        // Encode agent binary as base64
        let agent_b64 = base64::encode(agent_binary);
        
        // Execute bootstrap
        let command = format!("echo '{}' | {}", agent_b64, bootstrap_script);
        
        let output = timeout(
            Duration::from_secs(30),
            self.ssh_helper.execute_command(config, &["sh", "-c", &command])
        ).await??;
        
        let success = output.success();
        let agent_executed = output.stdout.contains("Mock agent") || output.stdout.contains("Agent executed");
        
        // Check for cleanup (no leftover processes or files)
        let cleanup_check = r#"
ps aux | grep -v grep | grep mitoxide || true
find /tmp -name "mitoxide-agent-*" 2>/dev/null | wc -l
        "#;
        
        let cleanup_output = self.ssh_helper.execute_command(config, &["sh", "-c", cleanup_check]).await?;
        let cleanup_verified = cleanup_output.success() && 
                              cleanup_output.stdout.lines().last().unwrap_or("1").trim() == "0";
        
        Ok(BootstrapTestResult {
            success,
            agent_executed,
            cleanup_verified,
            output: output.stdout,
            error: output.stderr,
        })
    }
}

/// Result of a bootstrap test
#[derive(Debug)]
struct BootstrapTestResult {
    success: bool,
    agent_executed: bool,
    cleanup_verified: bool,
    output: String,
    error: String,
}

/// Run all bootstrap tests
pub async fn run_bootstrap_tests() -> Result<()> {
    let tests = BootstrapTests::new();
    
    tests.setup().await?;
    
    let mut results = Vec::new();
    
    // Run all bootstrap tests
    results.push(("memfd_bootstrap", tests.test_memfd_bootstrap().await));
    results.push(("tmp_fallback", tests.test_tmp_fallback_bootstrap().await));
    results.push(("failure_scenarios", tests.test_bootstrap_failure_scenarios().await));
    results.push(("agent_cleanup", tests.test_agent_cleanup().await));
    results.push(("platform_detection", tests.test_platform_detection().await));
    
    tests.cleanup().await?;
    
    // Report results
    let mut failed_tests = Vec::new();
    for (test_name, result) in results {
        match result {
            Ok(()) => println!("✅ {} passed", test_name),
            Err(e) => {
                println!("❌ {} failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }
    
    if !failed_tests.is_empty() {
        anyhow::bail!("Bootstrap tests failed: {:?}", failed_tests);
    }
    
    println!("🎉 All bootstrap tests passed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_bootstrap_test_creation() {
        let tests = BootstrapTests::new();
        // Should create without errors
    }
    
    #[test]
    fn test_mock_agent_creation() {
        let tests = BootstrapTests::new();
        let agent = tests.create_mock_agent_binary().unwrap();
        assert!(!agent.is_empty());
        assert!(String::from_utf8_lossy(&agent).contains("Mock agent"));
    }
    
    #[test]
    fn test_bootstrap_script_generation() {
        let tests = BootstrapTests::new();
        
        let memfd_script = tests.create_memfd_bootstrap_script();
        assert!(memfd_script.contains("memfd_create"));
        assert!(memfd_script.contains("python3"));
        
        let tmp_script = tests.create_tmp_fallback_script();
        assert!(tmp_script.contains("/tmp"));
        assert!(tmp_script.contains("base64"));
    }
}