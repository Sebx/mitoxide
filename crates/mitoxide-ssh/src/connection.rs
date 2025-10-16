//! SSH connection management

use crate::TransportError;
use tokio::process::Child;
use tracing::{debug, warn};

/// SSH connection wrapper
#[derive(Debug)]
pub struct Connection {
    /// SSH process handle
    ssh_process: Option<Child>,
    /// Connection state
    connected: bool,
}

impl Connection {
    /// Create a new connection from an SSH process
    pub fn new(ssh_process: Option<Child>) -> Self {
        let connected = ssh_process.is_some();
        Self {
            ssh_process,
            connected,
        }
    }
    
    /// Check if the connection is active
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    /// Get mutable reference to the SSH process
    pub fn process_mut(&mut self) -> Option<&mut Child> {
        self.ssh_process.as_mut()
    }
    
    /// Take ownership of the SSH process
    pub fn take_process(&mut self) -> Option<Child> {
        self.connected = false;
        self.ssh_process.take()
    }
    
    /// Close the connection
    pub async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(mut child) = self.ssh_process.take() {
            debug!("Closing SSH connection");
            
            // Try to terminate gracefully
            if let Err(e) = child.kill().await {
                warn!("Failed to kill SSH process: {}", e);
            }
            
            // Wait for the process to exit
            match child.wait().await {
                Ok(status) => {
                    debug!("SSH process exited with status: {}", status);
                }
                Err(e) => {
                    warn!("Error waiting for SSH process: {}", e);
                }
            }
        }
        
        self.connected = false;
        Ok(())
    }
    
    /// Get stdin handle for writing to the remote process
    pub fn stdin(&mut self) -> Option<&mut tokio::process::ChildStdin> {
        self.ssh_process.as_mut()?.stdin.as_mut()
    }
    
    /// Get stdout handle for reading from the remote process
    pub fn stdout(&mut self) -> Option<&mut tokio::process::ChildStdout> {
        self.ssh_process.as_mut()?.stdout.as_mut()
    }
    
    /// Get stderr handle for reading errors from the remote process
    pub fn stderr(&mut self) -> Option<&mut tokio::process::ChildStderr> {
        self.ssh_process.as_mut()?.stderr.as_mut()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(mut child) = self.ssh_process.take() {
            // Try to kill the process if it's still running
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_creation() {
        let conn = Connection::new(None);
        assert!(!conn.is_connected());
    }
    
    #[tokio::test]
    async fn test_connection_close() {
        let mut conn = Connection::new(None);
        let result = conn.close().await;
        assert!(result.is_ok());
        assert!(!conn.is_connected());
    }
}