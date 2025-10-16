//! Message types and enums

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use bytes::Bytes;
use uuid::Uuid;

/// Top-level message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Request message
    Request(Request),
    /// Response message
    Response(Response),
}

impl Message {
    /// Create a request message
    pub fn request(req: Request) -> Self {
        Self::Request(req)
    }
    
    /// Create a response message
    pub fn response(resp: Response) -> Self {
        Self::Response(resp)
    }
    
    /// Get the request ID if this is a request
    pub fn request_id(&self) -> Option<Uuid> {
        match self {
            Self::Request(req) => Some(req.id()),
            Self::Response(resp) => Some(resp.request_id()),
        }
    }
}

/// Request message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Process execution request
    ProcessExec {
        /// Request ID for correlation
        id: Uuid,
        /// Command to execute
        command: Vec<String>,
        /// Environment variables
        env: HashMap<String, String>,
        /// Working directory
        cwd: Option<PathBuf>,
        /// Standard input data
        stdin: Option<Bytes>,
        /// Timeout in seconds
        timeout: Option<u64>,
    },
    
    /// File get operation
    FileGet {
        /// Request ID for correlation
        id: Uuid,
        /// Path to file
        path: PathBuf,
        /// Optional byte range (start, end)
        range: Option<(u64, u64)>,
    },
    
    /// File put operation
    FilePut {
        /// Request ID for correlation
        id: Uuid,
        /// Path to file
        path: PathBuf,
        /// File content
        content: Bytes,
        /// File mode (permissions)
        mode: Option<u32>,
        /// Create parent directories
        create_dirs: bool,
    },
    
    /// Directory listing
    DirList {
        /// Request ID for correlation
        id: Uuid,
        /// Directory path
        path: PathBuf,
        /// Include hidden files
        include_hidden: bool,
        /// Recursive listing
        recursive: bool,
    },
    
    /// WASM module execution
    WasmExec {
        /// Request ID for correlation
        id: Uuid,
        /// WASM module bytecode
        module: Bytes,
        /// JSON input data
        input: Bytes,
        /// Execution timeout in seconds
        timeout: Option<u64>,
    },
    
    /// JSON RPC call
    JsonCall {
        /// Request ID for correlation
        id: Uuid,
        /// Method name
        method: String,
        /// JSON parameters
        params: Bytes,
    },
    
    /// Ping request for health checking
    Ping {
        /// Request ID for correlation
        id: Uuid,
        /// Timestamp
        timestamp: u64,
    },
    
    /// PTY process execution with privilege escalation
    PtyExec {
        /// Request ID for correlation
        id: Uuid,
        /// Command to execute
        command: Vec<String>,
        /// Environment variables
        env: HashMap<String, String>,
        /// Working directory
        cwd: Option<PathBuf>,
        /// Privilege escalation method
        privilege: Option<PrivilegeEscalation>,
        /// Execution timeout in seconds
        timeout: Option<u64>,
    },
}

impl Request {
    /// Get the request ID
    pub fn id(&self) -> Uuid {
        match self {
            Self::ProcessExec { id, .. } => *id,
            Self::FileGet { id, .. } => *id,
            Self::FilePut { id, .. } => *id,
            Self::DirList { id, .. } => *id,
            Self::WasmExec { id, .. } => *id,
            Self::JsonCall { id, .. } => *id,
            Self::Ping { id, .. } => *id,
            Self::PtyExec { id, .. } => *id,
        }
    }
    
    /// Create a process execution request
    pub fn process_exec(
        command: Vec<String>,
        env: HashMap<String, String>,
        cwd: Option<PathBuf>,
        stdin: Option<Bytes>,
        timeout: Option<u64>,
    ) -> Self {
        Self::ProcessExec {
            id: Uuid::new_v4(),
            command,
            env,
            cwd,
            stdin,
            timeout,
        }
    }
    
    /// Create a file get request
    pub fn file_get(path: PathBuf, range: Option<(u64, u64)>) -> Self {
        Self::FileGet {
            id: Uuid::new_v4(),
            path,
            range,
        }
    }
    
    /// Create a file put request
    pub fn file_put(path: PathBuf, content: Bytes, mode: Option<u32>, create_dirs: bool) -> Self {
        Self::FilePut {
            id: Uuid::new_v4(),
            path,
            content,
            mode,
            create_dirs,
        }
    }
    
    /// Create a ping request
    pub fn ping() -> Self {
        Self::Ping {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Response message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Process execution result
    ProcessResult {
        /// Request ID this responds to
        request_id: Uuid,
        /// Exit code
        exit_code: i32,
        /// Standard output
        stdout: Bytes,
        /// Standard error
        stderr: Bytes,
        /// Execution duration in milliseconds
        duration_ms: u64,
    },
    
    /// File get result
    FileContent {
        /// Request ID this responds to
        request_id: Uuid,
        /// File content
        content: Bytes,
        /// File metadata
        metadata: FileMetadata,
    },
    
    /// File put result
    FilePutResult {
        /// Request ID this responds to
        request_id: Uuid,
        /// Bytes written
        bytes_written: u64,
    },
    
    /// Directory listing result
    DirListing {
        /// Request ID this responds to
        request_id: Uuid,
        /// Directory entries
        entries: Vec<DirEntry>,
    },
    
    /// WASM execution result
    WasmResult {
        /// Request ID this responds to
        request_id: Uuid,
        /// JSON output data
        output: Bytes,
        /// Execution duration in milliseconds
        duration_ms: u64,
    },
    
    /// JSON RPC result
    JsonResult {
        /// Request ID this responds to
        request_id: Uuid,
        /// JSON result
        result: Bytes,
    },
    
    /// Pong response
    Pong {
        /// Request ID this responds to
        request_id: Uuid,
        /// Original timestamp
        timestamp: u64,
        /// Response timestamp
        response_timestamp: u64,
    },
    
    /// PTY process execution result
    PtyResult {
        /// Request ID this responds to
        request_id: Uuid,
        /// Exit code
        exit_code: i32,
        /// Combined stdout/stderr output
        output: Bytes,
        /// Execution duration in milliseconds
        duration_ms: u64,
    },
    
    /// Error response
    Error {
        /// Request ID this responds to
        request_id: Uuid,
        /// Error details
        error: ErrorDetails,
    },
}

impl Response {
    /// Get the request ID this response corresponds to
    pub fn request_id(&self) -> Uuid {
        match self {
            Self::ProcessResult { request_id, .. } => *request_id,
            Self::FileContent { request_id, .. } => *request_id,
            Self::FilePutResult { request_id, .. } => *request_id,
            Self::DirListing { request_id, .. } => *request_id,
            Self::WasmResult { request_id, .. } => *request_id,
            Self::JsonResult { request_id, .. } => *request_id,
            Self::Pong { request_id, .. } => *request_id,
            Self::PtyResult { request_id, .. } => *request_id,
            Self::Error { request_id, .. } => *request_id,
        }
    }
    
    /// Create an error response
    pub fn error(request_id: Uuid, error: ErrorDetails) -> Self {
        Self::Error { request_id, error }
    }
    
    /// Create a pong response
    pub fn pong(request_id: Uuid, timestamp: u64) -> Self {
        Self::Pong {
            request_id,
            timestamp,
            response_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// File metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
    /// File permissions mode
    pub mode: u32,
    /// Last modified timestamp (Unix epoch)
    pub modified: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
}

/// Directory entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    /// Entry name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// File metadata
    pub metadata: FileMetadata,
}

/// Error details for error responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional context data
    pub context: HashMap<String, String>,
}

/// Privilege escalation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeEscalation {
    /// Escalation method
    pub method: PrivilegeMethod,
    /// Credentials for escalation
    pub credentials: Option<Credentials>,
    /// Custom prompt patterns to detect
    pub prompt_patterns: Vec<String>,
}

/// Privilege escalation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivilegeMethod {
    /// Use sudo
    Sudo,
    /// Use su
    Su,
    /// Use doas
    Doas,
    /// Custom command
    Custom(String),
}

/// Credentials for privilege escalation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// Username (for su)
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
}

/// Error codes for different types of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Invalid request format
    InvalidRequest,
    /// File not found
    FileNotFound,
    /// Permission denied
    PermissionDenied,
    /// Process execution failed
    ProcessFailed,
    /// WASM execution failed
    WasmFailed,
    /// Timeout occurred
    Timeout,
    /// Internal server error
    InternalError,
    /// Unsupported operation
    Unsupported,
    /// Resource exhausted
    ResourceExhausted,
    /// Privilege escalation failed
    PrivilegeEscalationFailed,
}

impl ErrorDetails {
    /// Create a new error details
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: HashMap::new(),
        }
    }
    
    /// Add context to the error
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    #[test]
    fn test_request_creation() {
        let req = Request::process_exec(
            vec!["echo".to_string(), "hello".to_string()],
            HashMap::new(),
            None,
            None,
            Some(30),
        );
        
        match req {
            Request::ProcessExec { command, timeout, .. } => {
                assert_eq!(command, vec!["echo", "hello"]);
                assert_eq!(timeout, Some(30));
            }
            _ => panic!("Expected ProcessExec request"),
        }
    }
    
    #[test]
    fn test_response_creation() {
        let request_id = Uuid::new_v4();
        let resp = Response::error(
            request_id,
            ErrorDetails::new(ErrorCode::FileNotFound, "File not found"),
        );
        
        match resp {
            Response::Error { request_id: resp_id, error } => {
                assert_eq!(resp_id, request_id);
                assert_eq!(error.code, ErrorCode::FileNotFound);
                assert_eq!(error.message, "File not found");
            }
            _ => panic!("Expected Error response"),
        }
    }
    
    #[test]
    fn test_message_request_id() {
        let req = Request::ping();
        let req_id = req.id();
        let msg = Message::request(req);
        
        assert_eq!(msg.request_id(), Some(req_id));
    }
    
    #[test]
    fn test_error_details_with_context() {
        let error = ErrorDetails::new(ErrorCode::ProcessFailed, "Command failed")
            .with_context("command", "ls")
            .with_context("exit_code", "1");
        
        assert_eq!(error.code, ErrorCode::ProcessFailed);
        assert_eq!(error.message, "Command failed");
        assert_eq!(error.context.get("command"), Some(&"ls".to_string()));
        assert_eq!(error.context.get("exit_code"), Some(&"1".to_string()));
    }
    
    #[test]
    fn test_message_serialization() {
        let req = Request::ping();
        let msg = Message::request(req);
        
        let serialized = rmp_serde::to_vec(&msg).unwrap();
        let deserialized: Message = rmp_serde::from_slice(&serialized).unwrap();
        
        assert_eq!(msg.request_id(), deserialized.request_id());
    }
    
    proptest! {
        #[test]
        fn test_request_id_consistency(
            command in prop::collection::vec("[a-zA-Z0-9]+", 1..5),
            timeout in prop::option::of(1u64..3600)
        ) {
            let req = Request::process_exec(
                command,
                HashMap::new(),
                None,
                None,
                timeout,
            );
            
            let id1 = req.id();
            let id2 = req.id();
            prop_assert_eq!(id1, id2);
        }
    }
}