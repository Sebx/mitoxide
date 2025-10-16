//! Transport abstraction and implementations

use async_trait::async_trait;
use crate::{Connection, TransportError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{debug, info};

/// Transport abstraction for different connection types
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to the remote host
    async fn connect(&mut self) -> Result<Connection, TransportError>;
    
    /// Bootstrap the agent on the remote host
    async fn bootstrap_agent(&mut self, agent_binary: &[u8]) -> Result<(), TransportError>;
    
    /// Get connection information
    fn connection_info(&self) -> ConnectionInfo;
    
    /// Test connectivity to the remote host
    async fn test_connection(&mut self) -> Result<(), TransportError>;
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Remote hostname or IP
    pub host: String,
    /// Remote port
    pub port: u16,
    /// Username
    pub username: String,
    /// Connection type
    pub transport_type: TransportType,
}

/// Transport type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportType {
    /// SSH with subprocess
    SshSubprocess,
    /// SSH with libssh2
    SshLibssh2,
    /// Local process (for testing)
    Local,
}

/// SSH configuration
#[derive(Debug, Clone)]
pub struct SshConfig {
    /// Remote hostname or IP
    pub host: String,
    /// Remote port (default: 22)
    pub port: u16,
    /// Username
    pub username: String,
    /// SSH key path
    pub key_path: Option<PathBuf>,
    /// SSH options
    pub options: HashMap<String, String>,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Command timeout in seconds
    pub command_timeout: u64,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 22,
            username: "root".to_string(),
            key_path: None,
            options: HashMap::new(),
            connect_timeout: 30,
            command_timeout: 300,
        }
    }
}

/// SSH stdio transport implementation using subprocess
pub struct StdioTransport {
    /// SSH configuration
    config: SshConfig,
    /// Active SSH process
    ssh_process: Option<Child>,
    /// Connection state
    connected: bool,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new(config: SshConfig) -> Self {
        Self {
            config,
            ssh_process: None,
            connected: false,
        }
    }
    
    /// Build SSH command arguments
    fn build_ssh_args(&self) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(), "BatchMode=yes".to_string(),
            "-o".to_string(), "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(), format!("ConnectTimeout={}", self.config.connect_timeout),
            "-p".to_string(), self.config.port.to_string(),
        ];
        
        // Add SSH key if specified
        if let Some(key_path) = &self.config.key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }
        
        // Add custom options
        for (key, value) in &self.config.options {
            args.push("-o".to_string());
            args.push(format!("{}={}", key, value));
        }
        
        // Add target
        args.push(format!("{}@{}", self.config.username, self.config.host));
        
        args
    }
    
    /// Execute a command over SSH
    async fn execute_command(&mut self, command: &str) -> Result<String, TransportError> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push(command.to_string());
        
        debug!("Executing SSH command: ssh {}", ssh_args.join(" "));
        
        let output = Command::new("ssh")
            .args(&ssh_args)
            .output()
            .await
            .map_err(|e| TransportError::Connection(format!("Failed to execute SSH: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TransportError::Connection(format!("SSH command failed: {}", stderr)));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Start an interactive SSH session
    async fn start_interactive_session(&mut self) -> Result<Child, TransportError> {
        let ssh_args = self.build_ssh_args();
        
        debug!("Starting interactive SSH session: ssh {}", ssh_args.join(" "));
        
        let child = Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TransportError::Connection(format!("Failed to start SSH: {}", e)))?;
        
        Ok(child)
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn connect(&mut self) -> Result<Connection, TransportError> {
        if self.connected {
            return Ok(Connection::new(self.ssh_process.take()));
        }
        
        info!("Connecting to {}@{}:{}", self.config.username, self.config.host, self.config.port);
        
        // Test basic connectivity first
        self.test_connection().await?;
        
        // Start interactive session
        let child = self.start_interactive_session().await?;
        self.ssh_process = Some(child);
        self.connected = true;
        
        info!("Successfully connected to {}@{}", self.config.username, self.config.host);
        Ok(Connection::new(self.ssh_process.take()))
    }
    
    async fn bootstrap_agent(&mut self, agent_binary: &[u8]) -> Result<(), TransportError> {
        info!("Bootstrapping agent on {}@{}", self.config.username, self.config.host);
        
        // Detect platform
        let platform_info = self.execute_command("uname -m && uname -s").await?;
        debug!("Remote platform: {}", platform_info.trim());
        
        // Try to use memfd_create for in-memory execution (Linux only)
        let bootstrap_script = format!(
            r#"
            set -e
            
            # Try memfd_create approach first (Linux)
            if command -v python3 >/dev/null 2>&1; then
                python3 -c "
import os, sys
try:
    import ctypes
    libc = ctypes.CDLL('libc.so.6')
    fd = libc.syscall(319, b'mitoxide-agent', 1)  # memfd_create
    if fd >= 0:
        os.write(fd, sys.stdin.buffer.read())
        os.fexecve(fd, ['/proc/self/fd/%d' % fd], os.environ)
except:
    pass
# Fallback to temp file
import tempfile
with tempfile.NamedTemporaryFile(delete=False) as f:
    f.write(sys.stdin.buffer.read())
    f.flush()
    os.chmod(f.name, 0o755)
    os.execv(f.name, [f.name])
"
            elif [ -d /tmp ] && [ -w /tmp ]; then
                # Fallback to /tmp
                AGENT_PATH="/tmp/mitoxide-agent-$$"
                cat > "$AGENT_PATH"
                chmod +x "$AGENT_PATH"
                exec "$AGENT_PATH"
                rm -f "$AGENT_PATH" 2>/dev/null || true
            else
                echo "No suitable location for agent bootstrap" >&2
                exit 1
            fi
            "#
        );
        
        // Send agent binary through stdin
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("bash".to_string());
        
        let mut child = Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TransportError::Bootstrap(format!("Failed to start SSH for bootstrap: {}", e)))?;
        
        // Send bootstrap script and agent binary
        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            
            stdin.write_all(bootstrap_script.as_bytes()).await
                .map_err(|e| TransportError::Bootstrap(format!("Failed to write bootstrap script: {}", e)))?;
            
            stdin.write_all(agent_binary).await
                .map_err(|e| TransportError::Bootstrap(format!("Failed to write agent binary: {}", e)))?;
            
            stdin.shutdown().await
                .map_err(|e| TransportError::Bootstrap(format!("Failed to close stdin: {}", e)))?;
        }
        
        // Wait for bootstrap to complete
        let output = child.wait_with_output().await
            .map_err(|e| TransportError::Bootstrap(format!("Bootstrap process failed: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TransportError::Bootstrap(format!("Agent bootstrap failed: {}", stderr)));
        }
        
        info!("Agent successfully bootstrapped on {}@{}", self.config.username, self.config.host);
        Ok(())
    }
    
    fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            host: self.config.host.clone(),
            port: self.config.port,
            username: self.config.username.clone(),
            transport_type: TransportType::SshSubprocess,
        }
    }
    
    async fn test_connection(&mut self) -> Result<(), TransportError> {
        debug!("Testing connection to {}@{}", self.config.username, self.config.host);
        
        // Simple connectivity test
        let result = self.execute_command("echo 'connection_test'").await?;
        
        if !result.trim().contains("connection_test") {
            return Err(TransportError::Connection("Connection test failed".to_string()));
        }
        
        debug!("Connection test successful");
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if let Some(mut child) = self.ssh_process.take() {
            // Try to kill the SSH process gracefully
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
    #[test]
    fn test_ssh_config_default() {
        let config = SshConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
        assert_eq!(config.connect_timeout, 30);
        assert_eq!(config.command_timeout, 300);
    }
    
    #[test]
    fn test_stdio_transport_creation() {
        let config = SshConfig::default();
        let transport = StdioTransport::new(config.clone());
        
        let info = transport.connection_info();
        assert_eq!(info.host, config.host);
        assert_eq!(info.port, config.port);
        assert_eq!(info.username, config.username);
        assert_eq!(info.transport_type, TransportType::SshSubprocess);
    }
    
    #[test]
    fn test_ssh_args_building() {
        let mut config = SshConfig::default();
        config.host = "example.com".to_string();
        config.port = 2222;
        config.username = "testuser".to_string();
        config.key_path = Some(PathBuf::from("/path/to/key"));
        config.options.insert("ServerAliveInterval".to_string(), "60".to_string());
        
        let transport = StdioTransport::new(config);
        let args = transport.build_ssh_args();
        
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/path/to/key".to_string()));
        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"ServerAliveInterval=60".to_string()));
        assert!(args.contains(&"testuser@example.com".to_string()));
    }
    
    #[test]
    fn test_connection_info() {
        let config = SshConfig {
            host: "test.example.com".to_string(),
            port: 2222,
            username: "testuser".to_string(),
            ..Default::default()
        };
        
        let transport = StdioTransport::new(config);
        let info = transport.connection_info();
        
        assert_eq!(info.host, "test.example.com");
        assert_eq!(info.port, 2222);
        assert_eq!(info.username, "testuser");
        assert_eq!(info.transport_type, TransportType::SshSubprocess);
    }
    
    // Mock transport for testing
    #[cfg(test)]
    pub struct MockTransport {
        should_fail: bool,
        connection_info: ConnectionInfo,
    }
    
    #[cfg(test)]
    impl MockTransport {
        pub fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                connection_info: ConnectionInfo {
                    host: "mock.example.com".to_string(),
                    port: 22,
                    username: "mockuser".to_string(),
                    transport_type: TransportType::Local,
                },
            }
        }
    }
    
    #[cfg(test)]
    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<Connection, TransportError> {
            if self.should_fail {
                Err(TransportError::Connection("Mock connection failed".to_string()))
            } else {
                Ok(Connection::new(None))
            }
        }
        
        async fn bootstrap_agent(&mut self, _agent_binary: &[u8]) -> Result<(), TransportError> {
            if self.should_fail {
                Err(TransportError::Bootstrap("Mock bootstrap failed".to_string()))
            } else {
                Ok(())
            }
        }
        
        fn connection_info(&self) -> ConnectionInfo {
            self.connection_info.clone()
        }
        
        async fn test_connection(&mut self) -> Result<(), TransportError> {
            if self.should_fail {
                Err(TransportError::Connection("Mock test failed".to_string()))
            } else {
                Ok(())
            }
        }
    }
    
    #[tokio::test]
    async fn test_mock_transport_success() {
        let mut transport = MockTransport::new(false);
        
        assert!(transport.test_connection().await.is_ok());
        assert!(transport.connect().await.is_ok());
        assert!(transport.bootstrap_agent(b"test").await.is_ok());
    }
    
    #[tokio::test]
    async fn test_mock_transport_failure() {
        let mut transport = MockTransport::new(true);
        
        assert!(transport.test_connection().await.is_err());
        assert!(transport.connect().await.is_err());
        assert!(transport.bootstrap_agent(b"test").await.is_err());
    }
}