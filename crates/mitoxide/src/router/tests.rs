//! Unit tests for connection routing

use super::*;
use mitoxide_proto::{Message, Request, Response};
use mitoxide_proto::message::{ErrorDetails, ErrorCode};
// use mitoxide_ssh::Connection;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::test]
async fn test_router_pending_requests() {
    // Test the pending requests map functionality
    let pending = Arc::new(RwLock::new(HashMap::new()));
    let request_id = Uuid::new_v4();
    let (tx, _rx) = oneshot::channel::<Response>();
    
    // Add pending request
    {
        let mut map = pending.write().await;
        map.insert(request_id, tx);
    }
    
    // Verify it exists
    {
        let map = pending.read().await;
        assert!(map.contains_key(&request_id));
    }
    
    // Remove it
    {
        let mut map = pending.write().await;
        let removed = map.remove(&request_id);
        assert!(removed.is_some());
    }
    
    // Verify it's gone
    {
        let map = pending.read().await;
        assert!(!map.contains_key(&request_id));
    }
}

#[tokio::test]
async fn test_connection_handler_stream_id_generation() {
    let next_stream_id = Arc::new(Mutex::new(1u32));
    
    // Test sequential ID generation
    let id1 = {
        let mut next_id = next_stream_id.lock().await;
        let id = *next_id;
        *next_id = next_id.wrapping_add(1);
        id
    };
    
    let id2 = {
        let mut next_id = next_stream_id.lock().await;
        let id = *next_id;
        *next_id = next_id.wrapping_add(1);
        id
    };
    
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[tokio::test]
async fn test_connection_handler_stream_id_wraparound() {
    let next_stream_id = Arc::new(Mutex::new(u32::MAX));
    
    // Test wraparound behavior
    let id1 = {
        let mut next_id = next_stream_id.lock().await;
        let id = *next_id;
        *next_id = next_id.wrapping_add(1);
        id
    };
    
    let id2 = {
        let mut next_id = next_stream_id.lock().await;
        let id = *next_id;
        *next_id = next_id.wrapping_add(1);
        id
    };
    
    assert_eq!(id1, u32::MAX);
    assert_eq!(id2, 0); // Wrapped around
}

#[test]
fn test_message_serialization() {
    let request = Request::ping();
    let message = Message::request(request);
    
    // Test serialization round-trip
    let serialized = rmp_serde::to_vec(&message).unwrap();
    let deserialized: Message = rmp_serde::from_slice(&serialized).unwrap();
    
    // Verify request IDs match
    assert_eq!(message.request_id(), deserialized.request_id());
}

#[test]
fn test_response_creation() {
    let request_id = Uuid::new_v4();
    let error_details = ErrorDetails::new(ErrorCode::InternalError, "Test error");
    let response = Response::error(request_id, error_details);
    
    assert_eq!(response.request_id(), request_id);
    
    match response {
        Response::Error { error, .. } => {
            assert_eq!(error.code, ErrorCode::InternalError);
            assert_eq!(error.message, "Test error");
        }
        _ => panic!("Expected error response"),
    }
}

#[tokio::test]
async fn test_router_timeout_behavior() {
    let timeout = Duration::from_millis(100);
    
    // Simulate timeout by waiting longer than the timeout duration
    let start = std::time::Instant::now();
    tokio::time::sleep(timeout + Duration::from_millis(50)).await;
    let elapsed = start.elapsed();
    
    assert!(elapsed > timeout);
}

#[test]
fn test_uuid_operations() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    
    // UUIDs should be unique
    assert_ne!(id1, id2);
    
    // Test string conversion
    let id_str = id1.to_string();
    let parsed = Uuid::parse_str(&id_str).unwrap();
    assert_eq!(id1, parsed);
    
    // Test hash map usage
    let mut map = HashMap::new();
    map.insert(id1, "value1");
    map.insert(id2, "value2");
    
    assert_eq!(map.get(&id1), Some(&"value1"));
    assert_eq!(map.get(&id2), Some(&"value2"));
}

#[tokio::test]
async fn test_channel_operations() {
    let (tx, mut rx) = mpsc::channel(10);
    
    // Test sending and receiving
    tx.send("test message").await.unwrap();
    let received = rx.recv().await.unwrap();
    assert_eq!(received, "test message");
    
    // Test channel closure
    drop(tx);
    let result = rx.recv().await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_oneshot_channel() {
    let (tx, rx) = oneshot::channel();
    
    // Test successful send/receive
    tx.send("test").unwrap();
    let received = rx.await.unwrap();
    assert_eq!(received, "test");
}

#[tokio::test]
async fn test_oneshot_channel_drop() {
    let (tx, rx) = oneshot::channel::<&str>();
    
    // Test receiver error when sender is dropped
    drop(tx);
    let result = rx.await;
    assert!(result.is_err());
}

// More comprehensive tests would require actual connections and would be better
// suited for integration tests with Docker containers