//! SSH connection helpers for integration tests

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Context, Result};
use tokio::time::timeout;

/// SSH connection configuration
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub jump_host: Option<Box<SshConfig>>,
}

impl SshConfig {
    /// Create SSH config for direct connection
    pub fn direct(host: &str, port: u16, user: &str, key_path: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            user: user.to_string(),
            key_path: key_path.to_string(),
            jump_host: None,
        }
    }
    
    /// Create SSH config with jump host
    pub fn with_jump_host(host: &str, port: u16, user: &str, key_path: &str, jump_host: SshConfig) -> Self {
        Self {
            host: host.to_string(),
            port,
            user: user.to_string(),
            key_path: key_path.to_string(),
            jump_host: Some(Box::new(jump_host)),
        }
    }
    
    /// Get connection string for this SSH config
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.user, self.host, self.port)
    }
}

/// SSH connection helper for integration tests
#[derive(Clone)]
pub struct SshHelper {
    default_key_path: String,
    default_user: String,
    connection_timeout: Duration,
}

impl SshHelper {
    /// Create new SSH helper with default configuration
    pub fn new() -> Self {
        Self {
            default_key_path: "docker/ssh_keys/test_key".to_string(),
            default_user: "testuser".to_string(),
            connection_timeout: Duration::from_secs(30),
        }
    }
    
    /// Create SSH helper with custom configuration
    pub fn with_config(key_path: &str, user: &str, timeout: Duration) -> Self {
        Self {
            default_key_path: key_path.to_string(),
            default_user: user.to_string(),
            connection_timeout: timeout,
        }
    }
    
    /// Test SSH connectivity to a host
    pub async fn test_connectivity(&self, config: &SshConfig) -> Result<bool> {
        let result = timeout(
            self.connection_timeout,
            self.execute_command(config, &["echo", "connection_test"])
        ).await;
        
        match result {
            Ok(Ok(output)) => Ok(output.success()),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout
        }
    }
    
    /// Execute a command over SSH
    pub async fn execute_command(&self, config: &SshConfig, command: &[&str]) -> Result<SshCommandOutput> {
        let mut ssh_cmd = Command::new("ssh");
        
        // Add SSH options
        ssh_cmd.args(&[
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=10",
            "-o", "ServerAliveInterval=5",
            "-o", "ServerAliveCountMax=3",
        ]);
        
        // Add identity file
        ssh_cmd.args(&["-i", &config.key_path]);
        
        // Add port
        ssh_cmd.args(&["-p", &config.port.to_string()]);
        
        // Add jump host if specified
        if let Some(jump_host) = &config.jump_host {
            let jump_string = format!("{}@{}:{}", jump_host.user, jump_host.host, jump_host.port);
            ssh_cmd.args(&["-J", &jump_string]);
        }
        
        // Add target host
        ssh_cmd.arg(&config.connection_string());
        
        // Add command
        ssh_cmd.args(command);
        
        let output = ssh_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute SSH command")?;
        
        Ok(SshCommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
    
    /// Copy file to remote host
    pub async fn copy_file_to(&self, config: &SshConfig, local_path: &Path, remote_path: &str) -> Result<()> {
        let mut scp_cmd = Command::new("scp");
        
        // Add SSH options
        scp_cmd.args(&[
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=10",
        ]);
        
        // Add identity file
        scp_cmd.args(&["-i", &config.key_path]);
        
        // Add port
        scp_cmd.args(&["-P", &config.port.to_string()]);
        
        // Add jump host if specified
        if let Some(jump_host) = &config.jump_host {
            let jump_string = format!("{}@{}:{}", jump_host.user, jump_host.host, jump_host.port);
            scp_cmd.args(&["-o", &format!("ProxyJump={}", jump_string)]);
        }
        
        // Add source and destination
        scp_cmd.arg(local_path);
        scp_cmd.arg(&format!("{}:{}", config.connection_string(), remote_path));
        
        let output = scp_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to copy file via SCP")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SCP failed: {}", stderr);
        }
        
        Ok(())
    }
    
    /// Copy file from remote host
    pub async fn copy_file_from(&self, config: &SshConfig, remote_path: &str, local_path: &Path) -> Result<()> {
        let mut scp_cmd = Command::new("scp");
        
        // Add SSH options
        scp_cmd.args(&[
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=10",
        ]);
        
        // Add identity file
        scp_cmd.args(&["-i", &config.key_path]);
        
        // Add port
        scp_cmd.args(&["-P", &config.port.to_string()]);
        
        // Add jump host if specified
        if let Some(jump_host) = &config.jump_host {
            let jump_string = format!("{}@{}:{}", jump_host.user, jump_host.host, jump_host.port);
            scp_cmd.args(&["-o", &format!("ProxyJump={}", jump_string)]);
        }
        
        // Add source and destination
        scp_cmd.arg(&format!("{}:{}", config.connection_string(), remote_path));
        scp_cmd.arg(local_path);
        
        let output = scp_cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to copy file via SCP")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SCP failed: {}", stderr);
        }
        
        Ok(())
    }
    
    /// Create SSH config for container
    pub fn config_for_container(&self, container: &str, port: u16) -> SshConfig {
        SshConfig::direct("localhost", port, &self.default_user, &self.default_key_path)
    }
    
    /// Create SSH config for jump host connection
    pub fn config_for_jump_host(&self, target_host: &str, bastion_port: u16) -> SshConfig {
        let jump_host = SshConfig::direct("localhost", bastion_port, &self.default_user, &self.default_key_path);
        SshConfig::with_jump_host(target_host, 22, &self.default_user, &self.default_key_path, jump_host)
    }
    
    /// Test all container connections
    pub async fn test_all_containers(&self) -> Result<TestResults> {
        let mut results = TestResults::new();
        
        // Test direct connections
        let containers = vec![
            ("alpine_ro", 2222),
            ("ubuntu_min", 2223),
            ("bastion", 2224),
        ];
        
        for (name, port) in containers {
            let config = self.config_for_container(name, port);
            let success = self.test_connectivity(&config).await?;
            results.add_result(name, success);
            
            if success {
                println!("✅ {} connection successful", name);
            } else {
                println!("❌ {} connection failed", name);
            }
        }
        
        // Test jump host connection
        let jump_config = self.config_for_jump_host("mitoxide_backend_target", 2224);
        let jump_success = self.test_connectivity(&jump_config).await?;
        results.add_result("backend_target_via_jump", jump_success);
        
        if jump_success {
            println!("✅ backend_target via jump host successful");
        } else {
            println!("❌ backend_target via jump host failed");
        }
        
        Ok(results)
    }
}

impl Default for SshHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// SSH command execution output
#[derive(Debug, Clone)]
pub struct SshCommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl SshCommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Test results for SSH connectivity
#[derive(Debug, Clone)]
pub struct TestResults {
    results: std::collections::HashMap<String, bool>,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            results: std::collections::HashMap::new(),
        }
    }
    
    pub fn add_result(&mut self, name: &str, success: bool) {
        self.results.insert(name.to_string(), success);
    }
    
    pub fn get_result(&self, name: &str) -> Option<bool> {
        self.results.get(name).copied()
    }
    
    pub fn all_successful(&self) -> bool {
        self.results.values().all(|&success| success)
    }
    
    pub fn success_count(&self) -> usize {
        self.results.values().filter(|&&success| success).count()
    }
    
    pub fn total_count(&self) -> usize {
        self.results.len()
    }
    
    pub fn failed_tests(&self) -> Vec<String> {
        self.results
            .iter()
            .filter(|(_, &success)| !success)
            .map(|(name, _)| name.clone())
            .collect()
    }
}