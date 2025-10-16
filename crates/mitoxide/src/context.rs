//! Execution context for remote operations

use crate::{Result, MitoxideError, Router};
use mitoxide_proto::{Message, Request, Response};
// use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use tracing::debug;
use uuid::Uuid;

/// Execution context for remote operations
pub struct Context {
    /// Session ID this context belongs to
    session_id: Uuid,
    /// Router for sending requests
    router: Arc<Router>,
}

impl Context {
    /// Create a new context
    pub(crate) fn new(session_id: Uuid, router: Arc<Router>) -> Result<Self> {
        Ok(Self {
            session_id,
            router,
        })
    }
    
    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
    
    /// Execute a process on the remote host
    pub async fn proc_exec(&self, command: &[&str]) -> Result<ProcessOutput> {
        let cmd: Vec<String> = command.iter().map(|s| s.to_string()).collect();
        
        debug!("Executing process: {:?}", cmd);
        
        let request = Request::process_exec(
            cmd,
            std::collections::HashMap::new(),
            None,
            None,
            Some(300), // 5 minute default timeout
        );
        
        let response = self.send_request(request).await?;
        
        match response {
            Response::ProcessResult { exit_code, stdout, stderr, duration_ms, .. } => {
                Ok(ProcessOutput {
                    exit_code,
                    stdout,
                    stderr,
                    duration: Duration::from_millis(duration_ms),
                })
            }
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("Process execution failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Execute a process with environment variables and working directory
    pub async fn proc_exec_with_env(
        &self,
        command: &[&str],
        env: std::collections::HashMap<String, String>,
        cwd: Option<&Path>,
        stdin: Option<&[u8]>,
    ) -> Result<ProcessOutput> {
        let cmd: Vec<String> = command.iter().map(|s| s.to_string()).collect();
        
        debug!("Executing process with env: {:?}", cmd);
        
        let request = Request::process_exec(
            cmd,
            env,
            cwd.map(|p| p.to_path_buf()),
            stdin.map(|data| Bytes::copy_from_slice(data)),
            Some(300),
        );
        
        let response = self.send_request(request).await?;
        
        match response {
            Response::ProcessResult { exit_code, stdout, stderr, duration_ms, .. } => {
                Ok(ProcessOutput {
                    exit_code,
                    stdout,
                    stderr,
                    duration: Duration::from_millis(duration_ms),
                })
            }
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("Process execution failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Upload a file to the remote host
    pub async fn put(&self, local_path: &Path, remote_path: &Path) -> Result<u64> {
        debug!("Uploading file: {:?} -> {:?}", local_path, remote_path);
        
        // Read local file
        let content = tokio::fs::read(local_path).await
            .map_err(|e| MitoxideError::Agent(format!("Failed to read local file: {}", e)))?;
        
        let request = Request::file_put(
            remote_path.to_path_buf(),
            Bytes::from(content),
            None, // Use default permissions
            true, // Create parent directories
        );
        
        let response = self.send_request(request).await?;
        
        match response {
            Response::FilePutResult { bytes_written, .. } => Ok(bytes_written),
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("File upload failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Download a file from the remote host
    pub async fn get(&self, remote_path: &Path, local_path: &Path) -> Result<u64> {
        debug!("Downloading file: {:?} -> {:?}", remote_path, local_path);
        
        let request = Request::file_get(remote_path.to_path_buf(), None);
        let response = self.send_request(request).await?;
        
        match response {
            Response::FileContent { content, .. } => {
                // Create parent directories if needed
                if let Some(parent) = local_path.parent() {
                    tokio::fs::create_dir_all(parent).await
                        .map_err(|e| MitoxideError::Agent(format!("Failed to create local directory: {}", e)))?;
                }
                
                // Write file content
                tokio::fs::write(local_path, &content).await
                    .map_err(|e| MitoxideError::Agent(format!("Failed to write local file: {}", e)))?;
                
                Ok(content.len() as u64)
            }
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("File download failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Call a JSON RPC method on the remote host
    pub async fn call_json<T, R>(&self, method: &str, params: &T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        debug!("Calling JSON RPC method: {}", method);
        
        let params_json = serde_json::to_vec(params)
            .map_err(|e| MitoxideError::Protocol(format!("Failed to serialize params: {}", e)))?;
        
        let request = Request::JsonCall {
            id: Uuid::new_v4(),
            method: method.to_string(),
            params: Bytes::from(params_json),
        };
        
        let response = self.send_request(request).await?;
        
        match response {
            Response::JsonResult { result, .. } => {
                serde_json::from_slice(&result)
                    .map_err(|e| MitoxideError::Protocol(format!("Failed to deserialize result: {}", e)))
            }
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("JSON RPC call failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Execute a WASM module on the remote host
    #[cfg(feature = "wasm")]
    pub async fn call_wasm<T, R>(&self, module: &[u8], input: &T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        debug!("Executing WASM module");
        
        let input_json = serde_json::to_vec(input)
            .map_err(|e| MitoxideError::Protocol(format!("Failed to serialize WASM input: {}", e)))?;
        
        let request = Request::WasmExec {
            id: Uuid::new_v4(),
            module: Bytes::copy_from_slice(module),
            input: Bytes::from(input_json),
            timeout: Some(60), // 1 minute default timeout
        };
        
        let response = self.send_request(request).await?;
        
        match response {
            Response::WasmResult { output, .. } => {
                serde_json::from_slice(&output)
                    .map_err(|e| MitoxideError::Protocol(format!("Failed to deserialize WASM output: {}", e)))
            }
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("WASM execution failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Ping the remote host to test connectivity
    pub async fn ping(&self) -> Result<Duration> {
        debug!("Pinging remote host");
        
        let start = Instant::now();
        let request = Request::ping();
        let response = self.send_request(request).await?;
        let duration = start.elapsed();
        
        match response {
            Response::Pong { .. } => Ok(duration),
            Response::Error { error, .. } => {
                Err(MitoxideError::Agent(format!("Ping failed: {}", error.message)))
            }
            _ => Err(MitoxideError::Protocol("Unexpected response type".to_string())),
        }
    }
    
    /// Send a request and wait for response
    async fn send_request(&self, request: Request) -> Result<Response> {
        let message = Message::request(request);
        self.router.send_message(message).await
    }
}

/// Process execution output
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// Exit code of the process
    pub exit_code: i32,
    /// Standard output
    pub stdout: Bytes,
    /// Standard error
    pub stderr: Bytes,
    /// Execution duration
    pub duration: Duration,
}

impl ProcessOutput {
    /// Check if the process succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
    
    /// Get stdout as UTF-8 string
    pub fn stdout_string(&self) -> Result<String> {
        String::from_utf8(self.stdout.to_vec())
            .map_err(|e| MitoxideError::Protocol(format!("Invalid UTF-8 in stdout: {}", e)))
    }
    
    /// Get stderr as UTF-8 string
    pub fn stderr_string(&self) -> Result<String> {
        String::from_utf8(self.stderr.to_vec())
            .map_err(|e| MitoxideError::Protocol(format!("Invalid UTF-8 in stderr: {}", e)))
    }
}

#[cfg(test)]
mod tests;