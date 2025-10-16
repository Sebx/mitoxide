//! Utility functions for integration tests

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use anyhow::{Context, Result};
use tempfile::TempDir;

/// Test file operations and assertions
pub struct FileTestUtils;

impl FileTestUtils {
    /// Create a temporary test file with specified content
    pub fn create_test_file(content: &str) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let file_path = temp_dir.path().join("test_file.txt");
        
        fs::write(&file_path, content)
            .context("Failed to write test file")?;
        
        Ok((temp_dir, file_path))
    }
    
    /// Create a test file with binary content
    pub fn create_binary_test_file(data: &[u8]) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let file_path = temp_dir.path().join("test_binary.bin");
        
        fs::write(&file_path, data)
            .context("Failed to write binary test file")?;
        
        Ok((temp_dir, file_path))
    }
    
    /// Create a large test file for performance testing
    pub fn create_large_test_file(size_mb: usize) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        let file_path = temp_dir.path().join("large_test_file.dat");
        
        let chunk_size = 1024 * 1024; // 1MB chunks
        let chunk = vec![0u8; chunk_size];
        
        let mut file = fs::File::create(&file_path)
            .context("Failed to create large test file")?;
        
        use std::io::Write;
        for _ in 0..size_mb {
            file.write_all(&chunk)
                .context("Failed to write chunk to large test file")?;
        }
        
        Ok((temp_dir, file_path))
    }
    
    /// Verify file content matches expected
    pub fn verify_file_content(path: &Path, expected: &str) -> Result<bool> {
        let content = fs::read_to_string(path)
            .context("Failed to read file for verification")?;
        Ok(content == expected)
    }
    
    /// Verify binary file content matches expected
    pub fn verify_binary_file_content(path: &Path, expected: &[u8]) -> Result<bool> {
        let content = fs::read(path)
            .context("Failed to read binary file for verification")?;
        Ok(content == expected)
    }
    
    /// Get file size in bytes
    pub fn get_file_size(path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path)
            .context("Failed to get file metadata")?;
        Ok(metadata.len())
    }
    
    /// Check if file exists
    pub fn file_exists(path: &Path) -> bool {
        path.exists() && path.is_file()
    }
}

/// Performance measurement utilities
pub struct PerformanceUtils;

impl PerformanceUtils {
    /// Measure execution time of an operation
    pub async fn measure_async<F, T>(operation: F) -> (T, Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = SystemTime::now();
        let result = operation.await;
        let duration = start.elapsed().unwrap_or(Duration::ZERO);
        (result, duration)
    }
    
    /// Measure execution time of a synchronous operation
    pub fn measure_sync<F, T>(operation: F) -> (T, Duration)
    where
        F: FnOnce() -> T,
    {
        let start = SystemTime::now();
        let result = operation();
        let duration = start.elapsed().unwrap_or(Duration::ZERO);
        (result, duration)
    }
    
    /// Calculate throughput in MB/s
    pub fn calculate_throughput_mbps(bytes: u64, duration: Duration) -> f64 {
        if duration.is_zero() {
            return 0.0;
        }
        
        let mb = bytes as f64 / (1024.0 * 1024.0);
        let seconds = duration.as_secs_f64();
        mb / seconds
    }
    
    /// Format duration for display
    pub fn format_duration(duration: Duration) -> String {
        let millis = duration.as_millis();
        if millis < 1000 {
            format!("{}ms", millis)
        } else {
            format!("{:.2}s", duration.as_secs_f64())
        }
    }
}

/// Test assertion utilities
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that a command output indicates success
    pub fn assert_command_success(output: &crate::integration::CommandOutput, context: &str) -> Result<()> {
        if !output.success() {
            anyhow::bail!(
                "{} failed with exit code {}\nstdout: {}\nstderr: {}",
                context,
                output.exit_code,
                output.stdout,
                output.stderr
            );
        }
        Ok(())
    }
    
    /// Assert that SSH command output indicates success
    pub fn assert_ssh_success(output: &crate::integration::SshCommandOutput, context: &str) -> Result<()> {
        if !output.success() {
            anyhow::bail!(
                "SSH {} failed with exit code {}\nstdout: {}\nstderr: {}",
                context,
                output.exit_code,
                output.stdout,
                output.stderr
            );
        }
        Ok(())
    }
    
    /// Assert that output contains expected text
    pub fn assert_output_contains(output: &str, expected: &str, context: &str) -> Result<()> {
        if !output.contains(expected) {
            anyhow::bail!(
                "{}: Expected output to contain '{}', but got: {}",
                context,
                expected,
                output
            );
        }
        Ok(())
    }
    
    /// Assert that performance meets threshold
    pub fn assert_performance_threshold(
        actual: Duration,
        threshold: Duration,
        operation: &str,
    ) -> Result<()> {
        if actual > threshold {
            anyhow::bail!(
                "{} took {} but threshold is {}",
                operation,
                PerformanceUtils::format_duration(actual),
                PerformanceUtils::format_duration(threshold)
            );
        }
        Ok(())
    }
    
    /// Assert that throughput meets minimum requirement
    pub fn assert_throughput_threshold(
        actual_mbps: f64,
        min_mbps: f64,
        operation: &str,
    ) -> Result<()> {
        if actual_mbps < min_mbps {
            anyhow::bail!(
                "{} achieved {:.2} MB/s but minimum is {:.2} MB/s",
                operation,
                actual_mbps,
                min_mbps
            );
        }
        Ok(())
    }
}

/// Environment setup utilities
pub struct EnvUtils;

impl EnvUtils {
    /// Check if Docker is available
    pub fn check_docker_available() -> Result<()> {
        use std::process::Command;
        
        let output = Command::new("docker")
            .args(&["--version"])
            .output()
            .context("Failed to check Docker availability")?;
        
        if !output.status.success() {
            anyhow::bail!("Docker is not available or not running");
        }
        
        Ok(())
    }
    
    /// Check if docker-compose is available
    pub fn check_docker_compose_available() -> Result<()> {
        use std::process::Command;
        
        let output = Command::new("docker-compose")
            .args(&["--version"])
            .output()
            .context("Failed to check docker-compose availability")?;
        
        if !output.status.success() {
            anyhow::bail!("docker-compose is not available");
        }
        
        Ok(())
    }
    
    /// Check if SSH keys exist
    pub fn check_ssh_keys_exist(key_path: &str) -> Result<()> {
        let private_key = Path::new(key_path);
        let public_key = Path::new(&format!("{}.pub", key_path));
        
        if !private_key.exists() {
            anyhow::bail!("SSH private key not found: {}", key_path);
        }
        
        if !public_key.exists() {
            anyhow::bail!("SSH public key not found: {}.pub", key_path);
        }
        
        Ok(())
    }
    
    /// Setup test environment prerequisites
    pub fn setup_test_environment() -> Result<()> {
        println!("Checking test environment prerequisites...");
        
        Self::check_docker_available()
            .context("Docker check failed")?;
        
        Self::check_docker_compose_available()
            .context("Docker Compose check failed")?;
        
        Self::check_ssh_keys_exist("docker/ssh_keys/test_key")
            .context("SSH keys check failed")?;
        
        println!("✅ Test environment prerequisites satisfied");
        Ok(())
    }
}

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate random text data
    pub fn random_text(size: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                 abcdefghijklmnopqrstuvwxyz\
                                 0123456789\n\t ";
        
        let mut rng = rand::thread_rng();
        (0..size)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Generate random binary data
    pub fn random_binary(size: usize) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..size).map(|_| rng.gen()).collect()
    }
    
    /// Generate structured test data
    pub fn structured_json_data(records: usize) -> String {
        let mut data = String::from("[\n");
        for i in 0..records {
            if i > 0 {
                data.push_str(",\n");
            }
            data.push_str(&format!(
                r#"  {{"id": {}, "name": "test_record_{}", "value": {}}}"#,
                i, i, i * 42
            ));
        }
        data.push_str("\n]");
        data
    }
}