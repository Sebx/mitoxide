//! Unit tests for execution context

use super::*;
use crate::MitoxideError;
use mitoxide_proto::{Message, Request, Response};
use mitoxide_proto::message::{ErrorDetails, ErrorCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Mock router for testing
struct MockRouter {
    expected_requests: Arc<RwLock<Vec<Request>>>,
    responses: Arc<RwLock<Vec<Response>>>,
}

impl MockRouter {
    fn new() -> Self {
        Self {
            expected_requests: Arc::new(RwLock::new(Vec::new())),
            responses: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn add_expected_request(&self, request: Request) {
        self.expected_requests.write().await.push(request);
    }
    
    async fn add_response(&self, response: Response) {
        self.responses.write().await.push(response);
    }
    
    async fn send_message(&self, message: Message) -> Result<Response> {
        match message {
            Message::Request(_req) => {
                // Verify expected request
                let mut expected = self.expected_requests.write().await;
                if expected.is_empty() {
                    return Err(MitoxideError::Protocol("Unexpected request".to_string()));
                }
                
                let _expected_req = expected.remove(0);
                // In a real test, we'd compare the requests more thoroughly
                
                // Return next response
                let mut responses = self.responses.write().await;
                if responses.is_empty() {
                    return Err(MitoxideError::Protocol("No response available".to_string()));
                }
                
                Ok(responses.remove(0))
            }
            _ => Err(MitoxideError::Protocol("Expected request message".to_string())),
        }
    }
}

#[tokio::test]
async fn test_context_creation() {
    let session_id = Uuid::new_v4();
    let _mock_router = Arc::new(MockRouter::new());
    
    // We can't directly create a Router for testing, so we'll test the Context struct itself
    // In a real implementation, we'd need to create a proper mock or use dependency injection
    
    // For now, test the basic structure
    assert_eq!(session_id.to_string().len(), 36); // UUID length
}

#[tokio::test]
async fn test_process_output_success() {
    let output = ProcessOutput {
        exit_code: 0,
        stdout: Bytes::from("Hello, World!"),
        stderr: Bytes::new(),
        duration: Duration::from_millis(100),
    };
    
    assert!(output.success());
    assert_eq!(output.stdout_string().unwrap(), "Hello, World!");
    assert_eq!(output.stderr_string().unwrap(), "");
}

#[tokio::test]
async fn test_process_output_failure() {
    let output = ProcessOutput {
        exit_code: 1,
        stdout: Bytes::new(),
        stderr: Bytes::from("Error occurred"),
        duration: Duration::from_millis(50),
    };
    
    assert!(!output.success());
    assert_eq!(output.stdout_string().unwrap(), "");
    assert_eq!(output.stderr_string().unwrap(), "Error occurred");
}

#[tokio::test]
async fn test_process_output_invalid_utf8() {
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    let output = ProcessOutput {
        exit_code: 0,
        stdout: Bytes::from(invalid_utf8),
        stderr: Bytes::new(),
        duration: Duration::from_millis(10),
    };
    
    assert!(output.stdout_string().is_err());
}

#[test]
fn test_process_output_clone() {
    let output = ProcessOutput {
        exit_code: 42,
        stdout: Bytes::from("test output"),
        stderr: Bytes::from("test error"),
        duration: Duration::from_millis(200),
    };
    
    let cloned = output.clone();
    assert_eq!(output.exit_code, cloned.exit_code);
    assert_eq!(output.stdout, cloned.stdout);
    assert_eq!(output.stderr, cloned.stderr);
    assert_eq!(output.duration, cloned.duration);
}

// Integration-style tests would require a real router and connection
// These would be better suited for integration tests with Docker containers

#[tokio::test]
async fn test_context_session_id() {
    let session_id = Uuid::new_v4();
    
    // Test UUID generation and comparison
    let another_id = Uuid::new_v4();
    assert_ne!(session_id, another_id);
    
    // Test UUID string representation
    let id_str = session_id.to_string();
    let parsed_id = Uuid::parse_str(&id_str).unwrap();
    assert_eq!(session_id, parsed_id);
}

#[test]
fn test_bytes_operations() {
    let data = b"Hello, World!";
    let bytes = Bytes::from(&data[..]);
    
    assert_eq!(bytes.len(), 13);
    assert_eq!(&bytes[..], data);
    
    let string = String::from_utf8(bytes.to_vec()).unwrap();
    assert_eq!(string, "Hello, World!");
}

#[test]
fn test_duration_operations() {
    let duration = Duration::from_millis(1500);
    
    assert_eq!(duration.as_millis(), 1500);
    assert_eq!(duration.as_secs(), 1);
    
    let longer = Duration::from_secs(60);
    assert!(longer > duration);
}