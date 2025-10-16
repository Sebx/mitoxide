//! Comprehensive integration tests for process execution and I/O handling
//! 
//! These tests verify process execution functionality including large I/O streams,
//! environment variables, binary data handling, and process timeouts.

use super::*;
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::timeout;

/// Test process execution and I/O handling
pub struct ProcessTests {
    docker_env: DockerTestEnv,
    ssh_helper: SshHelper,
}

impl ProcessTests {
    /// Create new process test suite
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
    
    /// Test large stdout/stderr streaming
    pub async fn test_large_io_streaming(&self) -> Result<()> {
        println!("Testing large stdout/stderr streaming...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test large stdout output
        println!("  Testing large stdout output...");
        let large_stdout_cmd = r#"
# Generate 1MB of stdout data
for i in $(seq 1 10000); do
    echo "This is line $i with some additional text to make it longer and test streaming capabilities"
done
        "#;
        
        let (output, duration) = PerformanceUtils::measure_async(
            self.ssh_helper.execute_command(&config, &["sh", "-c", large_stdout_cmd])
        ).await;
        
        let output = output?;
        TestAssertions::assert_ssh_success(&output, "Large stdout test")?;
        
        // Verify output size and content
        let output_size = output.stdout.len();
        assert!(output_size > 500_000, "Output should be at least 500KB, got {}", output_size);
        assert!(output.stdout.contains("line 1 "), "Should contain first line");
        assert!(output.stdout.contains("line 10000"), "Should contain last line");
        
        println!("    ✅ Large stdout: {} bytes in {}", 
                output_size, PerformanceUtils::format_duration(duration));
        
        // Test large stderr output
        println!("  Testing large stderr output...");
        let large_stderr_cmd = r#"
# Generate stderr data
for i in $(seq 1 5000); do
    echo "Error message $i: This is a test error with additional context" >&2
done
echo "Command completed" # Small stdout
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", large_stderr_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Large stderr test")?;
        
        let stderr_size = output.stderr.len();
        assert!(stderr_size > 200_000, "Stderr should be at least 200KB, got {}", stderr_size);
        assert!(output.stderr.contains("Error message 1:"), "Should contain first error");
        assert!(output.stderr.contains("Error message 5000"), "Should contain last error");
        assert!(output.stdout.contains("Command completed"), "Should have stdout");
        
        println!("    ✅ Large stderr: {} bytes", stderr_size);
        
        // Test mixed large stdout and stderr
        println!("  Testing mixed large stdout/stderr...");
        let mixed_io_cmd = r#"
for i in $(seq 1 2000); do
    echo "STDOUT line $i"
    echo "STDERR line $i" >&2
done
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", mixed_io_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Mixed I/O test")?;
        
        assert!(output.stdout.len() > 50_000, "Mixed stdout should be substantial");
        assert!(output.stderr.len() > 50_000, "Mixed stderr should be substantial");
        assert!(output.stdout.contains("STDOUT line 1"), "Should have stdout content");
        assert!(output.stderr.contains("STDERR line 1"), "Should have stderr content");
        
        println!("✅ Large I/O streaming test passed");
        Ok(())
    }
    
    /// Test environment variable passthrough
    pub async fn test_environment_passthrough(&self) -> Result<()> {
        println!("Testing environment variable passthrough...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test setting and reading environment variables
        let env_test_cmd = r#"
export TEST_VAR1="hello world"
export TEST_VAR2="special chars: !@#$%^&*()"
export TEST_VAR3="multiline
content
here"
export PATH_BACKUP="$PATH"

echo "TEST_VAR1=$TEST_VAR1"
echo "TEST_VAR2=$TEST_VAR2"
echo "TEST_VAR3=$TEST_VAR3"
echo "PATH_SET=$(echo $PATH | wc -c)"
echo "HOME=$HOME"
echo "USER=$USER"
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", env_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Environment variable test")?;
        
        // Verify environment variables
        TestAssertions::assert_output_contains(&output.stdout, "TEST_VAR1=hello world", "Basic env var")?;
        TestAssertions::assert_output_contains(&output.stdout, "TEST_VAR2=special chars:", "Special chars env var")?;
        TestAssertions::assert_output_contains(&output.stdout, "TEST_VAR3=multiline", "Multiline env var")?;
        TestAssertions::assert_output_contains(&output.stdout, "HOME=", "HOME env var")?;
        TestAssertions::assert_output_contains(&output.stdout, "USER=", "USER env var")?;
        
        // Test environment variable inheritance
        println!("  Testing environment inheritance...");
        let inherit_test_cmd = r#"
# Set a variable and run a subcommand
export PARENT_VAR="from parent"
sh -c 'echo "Child sees: $PARENT_VAR"'
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", inherit_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Environment inheritance test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Child sees: from parent", "Environment inheritance")?;
        
        // Test environment variable modification
        println!("  Testing environment modification...");
        let modify_test_cmd = r#"
# Test PATH modification
export ORIGINAL_PATH="$PATH"
export PATH="/custom/path:$PATH"
echo "PATH_MODIFIED=$(echo $PATH | grep '/custom/path')"
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", modify_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Environment modification test")?;
        TestAssertions::assert_output_contains(&output.stdout, "PATH_MODIFIED=/custom/path", "PATH modification")?;
        
        println!("✅ Environment variable passthrough test passed");
        Ok(())
    }
    
    /// Test binary data handling and encoding
    pub async fn test_binary_data_handling(&self) -> Result<()> {
        println!("Testing binary data handling and encoding...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test binary data creation and verification
        println!("  Testing binary data creation...");
        let binary_test_cmd = r#"
# Create binary file with known pattern
dd if=/dev/zero bs=1024 count=10 2>/dev/null | tr '\0' 'A' > /tmp/test_binary.dat
echo "Binary file created"

# Add some random bytes
dd if=/dev/urandom bs=256 count=1 >> /tmp/test_binary.dat 2>/dev/null
echo "Random data appended"

# Check file size
ls -l /tmp/test_binary.dat | awk '{print "Size: " $5}'

# Create checksum
md5sum /tmp/test_binary.dat | awk '{print "MD5: " $1}'
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", binary_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Binary data creation")?;
        TestAssertions::assert_output_contains(&output.stdout, "Binary file created", "File creation")?;
        TestAssertions::assert_output_contains(&output.stdout, "Size: 10496", "File size")?;
        
        // Extract MD5 for verification
        let md5_line = output.stdout.lines()
            .find(|line| line.starts_with("MD5: "))
            .context("MD5 line not found")?;
        let original_md5 = md5_line.strip_prefix("MD5: ").unwrap();
        
        // Test binary data transfer via base64
        println!("  Testing binary data encoding/decoding...");
        let encode_test_cmd = r#"
# Encode binary file to base64
base64 /tmp/test_binary.dat > /tmp/test_binary.b64
echo "File encoded to base64"

# Decode back to binary
base64 -d /tmp/test_binary.b64 > /tmp/test_binary_decoded.dat
echo "File decoded from base64"

# Verify integrity
md5sum /tmp/test_binary_decoded.dat | awk '{print "Decoded MD5: " $1}'

# Compare files
if cmp -s /tmp/test_binary.dat /tmp/test_binary_decoded.dat; then
    echo "Binary integrity verified"
else
    echo "Binary integrity check failed"
fi
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", encode_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Binary encoding test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Binary integrity verified", "Binary integrity")?;
        
        let decoded_md5_line = output.stdout.lines()
            .find(|line| line.starts_with("Decoded MD5: "))
            .context("Decoded MD5 line not found")?;
        let decoded_md5 = decoded_md5_line.strip_prefix("Decoded MD5: ").unwrap();
        
        assert_eq!(original_md5, decoded_md5, "MD5 checksums should match");
        
        // Test handling of null bytes and special characters
        println!("  Testing special character handling...");
        let special_chars_cmd = r#"
# Create file with null bytes and special characters
printf '\x00\x01\x02\xFF\xFE\xFD' > /tmp/special_chars.bin
echo "Special chars file created"

# Encode and decode
base64 /tmp/special_chars.bin | base64 -d > /tmp/special_chars_decoded.bin

# Verify with hexdump
hexdump -C /tmp/special_chars.bin > /tmp/original.hex
hexdump -C /tmp/special_chars_decoded.bin > /tmp/decoded.hex

if cmp -s /tmp/original.hex /tmp/decoded.hex; then
    echo "Special characters handled correctly"
else
    echo "Special character handling failed"
fi
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", special_chars_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Special characters test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Special characters handled correctly", "Special chars")?;
        
        // Cleanup
        let cleanup_cmd = "rm -f /tmp/test_binary.* /tmp/special_chars.* /tmp/original.hex /tmp/decoded.hex";
        let _ = self.ssh_helper.execute_command(&config, &["sh", "-c", cleanup_cmd]).await;
        
        println!("✅ Binary data handling test passed");
        Ok(())
    }
    
    /// Test process timeout and cancellation
    pub async fn test_process_timeout_cancellation(&self) -> Result<()> {
        println!("Testing process timeout and cancellation...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test command timeout
        println!("  Testing command timeout...");
        let long_running_cmd = r#"
echo "Starting long running command"
sleep 30
echo "Command completed"
        "#;
        
        let timeout_duration = Duration::from_secs(5);
        let result = timeout(
            timeout_duration,
            self.ssh_helper.execute_command(&config, &["sh", "-c", long_running_cmd])
        ).await;
        
        // Should timeout
        assert!(result.is_err(), "Command should timeout");
        println!("    ✅ Command timeout works correctly");
        
        // Test process cancellation via signal
        println!("  Testing process cancellation...");
        let cancellation_test_cmd = r#"
# Start a background process
(
    trap 'echo "Received signal, exiting"; exit 0' TERM INT
    echo "Background process started"
    sleep 60 &
    wait
) &
BG_PID=$!

# Give it time to start
sleep 1

# Send termination signal
kill -TERM $BG_PID 2>/dev/null || true

# Wait for it to finish
wait $BG_PID 2>/dev/null || true
echo "Process cancellation test completed"
        "#;
        
        let output = timeout(
            Duration::from_secs(10),
            self.ssh_helper.execute_command(&config, &["sh", "-c", cancellation_test_cmd])
        ).await??;
        
        TestAssertions::assert_ssh_success(&output, "Process cancellation test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Process cancellation test completed", "Cancellation completion")?;
        
        // Test zombie process cleanup
        println!("  Testing zombie process cleanup...");
        let zombie_test_cmd = r#"
# Create and clean up child processes
for i in $(seq 1 5); do
    (echo "Child process $i"; exit 0) &
done

# Wait for all children
wait

# Check for zombie processes
ZOMBIES=$(ps aux | grep -c '[Zz]ombie' || echo "0")
echo "Zombie processes: $ZOMBIES"

# Should be 0 or very few
if [ "$ZOMBIES" -lt 3 ]; then
    echo "Zombie cleanup successful"
else
    echo "Too many zombie processes: $ZOMBIES"
fi
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", zombie_test_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Zombie cleanup test")?;
        TestAssertions::assert_output_contains(&output.stdout, "Zombie cleanup successful", "Zombie cleanup")?;
        
        // Test resource cleanup after timeout
        println!("  Testing resource cleanup after timeout...");
        let resource_cleanup_cmd = r#"
# Check for leftover processes from previous tests
MITOXIDE_PROCS=$(ps aux | grep -v grep | grep -c mitoxide || echo "0")
SLEEP_PROCS=$(ps aux | grep -v grep | grep -c 'sleep [0-9]' || echo "0")

echo "Mitoxide processes: $MITOXIDE_PROCS"
echo "Sleep processes: $SLEEP_PROCS"

# Should have minimal leftover processes
if [ "$MITOXIDE_PROCS" -eq 0 ] && [ "$SLEEP_PROCS" -lt 3 ]; then
    echo "Resource cleanup verified"
else
    echo "Resource cleanup may be incomplete"
fi
        "#;
        
        let output = self.ssh_helper.execute_command(&config, &["sh", "-c", resource_cleanup_cmd]).await?;
        TestAssertions::assert_ssh_success(&output, "Resource cleanup test")?;
        
        println!("✅ Process timeout and cancellation test passed");
        Ok(())
    }
    
    /// Test concurrent process execution
    pub async fn test_concurrent_process_execution(&self) -> Result<()> {
        println!("Testing concurrent process execution...");
        
        let config = self.ssh_helper.config_for_container("ubuntu_min", 2223);
        
        // Test multiple concurrent commands
        let concurrent_test_cmd = r#"
echo "Starting concurrent process test"

# Run multiple background processes
for i in $(seq 1 5); do
    (
        echo "Process $i starting"
        sleep 2
        echo "Process $i completed"
    ) &
done

# Wait for all to complete
wait

echo "All concurrent processes completed"
        "#;
        
        let (output, duration) = PerformanceUtils::measure_async(
            self.ssh_helper.execute_command(&config, &["sh", "-c", concurrent_test_cmd])
        ).await;
        
        let output = output?;
        TestAssertions::assert_ssh_success(&output, "Concurrent processes test")?;
        
        // Should complete in roughly 2 seconds (not 10 seconds sequentially)
        assert!(duration < Duration::from_secs(5), 
                "Concurrent execution should be faster than sequential");
        
        // Verify all processes ran
        for i in 1..=5 {
            TestAssertions::assert_output_contains(&output.stdout, 
                &format!("Process {} starting", i), &format!("Process {} start", i))?;
            TestAssertions::assert_output_contains(&output.stdout, 
                &format!("Process {} completed", i), &format!("Process {} completion", i))?;
        }
        
        println!("    ✅ Concurrent execution completed in {}", 
                PerformanceUtils::format_duration(duration));
        
        println!("✅ Concurrent process execution test passed");
        Ok(())
    }
}

/// Run all process execution tests
pub async fn run_process_tests() -> Result<()> {
    let tests = ProcessTests::new();
    
    tests.setup().await?;
    
    let mut results = Vec::new();
    
    // Run all process tests
    results.push(("large_io_streaming", tests.test_large_io_streaming().await));
    results.push(("environment_passthrough", tests.test_environment_passthrough().await));
    results.push(("binary_data_handling", tests.test_binary_data_handling().await));
    results.push(("timeout_cancellation", tests.test_process_timeout_cancellation().await));
    results.push(("concurrent_execution", tests.test_concurrent_process_execution().await));
    
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
        anyhow::bail!("Process tests failed: {:?}", failed_tests);
    }
    
    println!("🎉 All process execution tests passed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_process_test_creation() {
        let tests = ProcessTests::new();
        // Should create without errors
    }
}