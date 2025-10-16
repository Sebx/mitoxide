//! Agent-side routing for multiplexed streams

use crate::agent::Handler;
use anyhow::{Context, Result};
use bytes::Bytes;
use mitoxide_proto::{Frame, FrameCodec, Message, Request, Response};
use mitoxide_proto::message::{ErrorCode, ErrorDetails};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWrite;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Stream information for tracking active streams
#[derive(Debug, Clone)]
struct StreamInfo {
    /// Stream ID
    stream_id: u32,
    /// Current sequence number
    sequence: u32,
    /// Request ID being processed
    request_id: Option<Uuid>,
}

/// Agent-side router for handling multiplexed streams and request dispatch
pub struct AgentRouter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    /// Output writer for sending responses
    writer: Arc<tokio::sync::Mutex<W>>,
    /// Frame codec for encoding responses
    codec: FrameCodec,
    /// Active streams
    streams: Arc<RwLock<HashMap<u32, StreamInfo>>>,
    /// Registered handlers by request type
    handlers: Arc<RwLock<HashMap<String, Arc<dyn Handler>>>>,
    /// Channel for sending requests to be processed
    request_tx: mpsc::UnboundedSender<(u32, u32, Request)>,
    /// Channel for receiving requests to process
    request_rx: Option<mpsc::UnboundedReceiver<(u32, u32, Request)>>,
    /// Shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl<W> AgentRouter<W>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    /// Create a new agent router
    pub fn new(writer: W) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = oneshot::channel();
        
        Self {
            writer: Arc::new(tokio::sync::Mutex::new(writer)),
            codec: FrameCodec::new(),
            streams: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            request_tx,
            request_rx: Some(request_rx),
            shutdown_tx: Some(shutdown_tx),
        }
    }
    
    /// Register a handler for a specific request type
    pub async fn register_handler(&self, request_type: String, handler: Arc<dyn Handler>) {
        let mut handlers = self.handlers.write().await;
        debug!("Registered handler for request type: {}", request_type);
        handlers.insert(request_type, handler);
    }
    
    /// Get shutdown sender for graceful shutdown
    pub fn shutdown_sender(&mut self) -> Option<oneshot::Sender<()>> {
        self.shutdown_tx.take()
    }
    
    /// Route an incoming frame to the appropriate handler
    pub async fn route_frame(&self, frame: Frame) -> Result<()> {
        debug!("Routing frame: stream_id={}, sequence={}, flags={:?}", 
               frame.stream_id, frame.sequence, frame.flags);
        
        // Handle control frames
        if frame.is_error() {
            warn!("Received error frame: stream_id={}, payload={:?}", 
                  frame.stream_id, frame.payload);
            return Ok(());
        }
        
        if frame.is_end_stream() {
            debug!("Received end-of-stream frame: stream_id={}", frame.stream_id);
            self.close_stream(frame.stream_id).await;
            return Ok(());
        }
        
        // Update stream info
        self.update_stream_info(frame.stream_id, frame.sequence).await;
        
        // Deserialize message from frame payload
        let message = match rmp_serde::from_slice::<Message>(&frame.payload) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
                self.send_error_frame(frame.stream_id, frame.sequence, 
                                    ErrorCode::InvalidRequest, 
                                    format!("Invalid message format: {}", e)).await?;
                return Ok(());
            }
        };
        
        // Route message
        match message {
            Message::Request(request) => {
                // Send request for processing
                if let Err(e) = self.request_tx.send((frame.stream_id, frame.sequence, request)) {
                    error!("Failed to send request for processing: {}", e);
                }
            }
            Message::Response(_) => {
                warn!("Received unexpected response message on agent router");
            }
        }
        
        Ok(())
    }
    
    /// Start the request processing loop
    pub async fn start_processing(&mut self) -> Result<()> {
        let mut request_rx = self.request_rx.take()
            .context("Request receiver already taken")?;
        
        let handlers = Arc::clone(&self.handlers);
        let writer = Arc::clone(&self.writer);
        
        info!("Starting request processing loop");
        
        while let Some((stream_id, sequence, request)) = request_rx.recv().await {
            let handlers = Arc::clone(&handlers);
            let writer = Arc::clone(&writer);
            
            // Process request in a separate task
            tokio::spawn(async move {
                let response = Self::process_request(request, &handlers).await;
                let codec = FrameCodec::new(); // Create new codec instance
                
                if let Err(e) = Self::send_response(stream_id, sequence, response, &writer, &codec).await {
                    error!("Failed to send response: {}", e);
                }
            });
        }
        
        info!("Request processing loop stopped");
        Ok(())
    }
    
    /// Process a single request using registered handlers
    async fn process_request(request: Request, handlers: &Arc<RwLock<HashMap<String, Arc<dyn Handler>>>>) -> Response {
        let request_id = request.id();
        debug!("Processing request: id={}, type={:?}", request_id, std::mem::discriminant(&request));
        
        // Determine request type for handler lookup
        let request_type = match &request {
            Request::ProcessExec { .. } => "process_exec",
            Request::FileGet { .. } => "file_get",
            Request::FilePut { .. } => "file_put",
            Request::DirList { .. } => "dir_list",
            Request::WasmExec { .. } => "wasm_exec",
            Request::JsonCall { .. } => "json_call",
            Request::Ping { .. } => "ping",
            Request::PtyExec { .. } => "pty_exec",
        };
        
        // Look up handler
        let handler = {
            let handlers_guard = handlers.read().await;
            handlers_guard.get(request_type).cloned()
        };
        
        match handler {
            Some(handler) => {
                // Execute handler
                match handler.handle(request).await {
                    Ok(response) => response,
                    Err(e) => {
                        error!("Handler error for request {}: {}", request_id, e);
                        Response::error(
                            request_id,
                            ErrorDetails::new(ErrorCode::InternalError, format!("Handler error: {}", e))
                        )
                    }
                }
            }
            None => {
                warn!("No handler registered for request type: {}", request_type);
                Response::error(
                    request_id,
                    ErrorDetails::new(ErrorCode::Unsupported, format!("Unsupported request type: {}", request_type))
                )
            }
        }
    }
    
    /// Send a response back to the client
    async fn send_response(
        stream_id: u32, 
        sequence: u32, 
        response: Response,
        writer: &Arc<tokio::sync::Mutex<W>>,
        codec: &FrameCodec
    ) -> Result<()> {
        let message = Message::response(response);
        let payload = rmp_serde::to_vec(&message)
            .context("Failed to serialize response message")?;
        
        let frame = Frame::data(stream_id, sequence, Bytes::from(payload));
        
        let mut writer_guard = writer.lock().await;
        codec.write_frame(&mut *writer_guard, &frame).await
            .context("Failed to write response frame")?;
        
        debug!("Sent response: stream_id={}, sequence={}", stream_id, sequence);
        Ok(())
    }
    
    /// Send an error frame
    async fn send_error_frame(&self, stream_id: u32, sequence: u32, 
                            error_code: ErrorCode, message: String) -> Result<()> {
        let error_payload = rmp_serde::to_vec(&ErrorDetails::new(error_code, message))
            .context("Failed to serialize error details")?;
        
        let frame = Frame::error(stream_id, sequence, Bytes::from(error_payload));
        
        let mut writer = self.writer.lock().await;
        self.codec.write_frame(&mut *writer, &frame).await
            .context("Failed to write error frame")?;
        
        debug!("Sent error frame: stream_id={}, sequence={}", stream_id, sequence);
        Ok(())
    }
    
    /// Update stream information
    async fn update_stream_info(&self, stream_id: u32, sequence: u32) {
        let mut streams = self.streams.write().await;
        streams.insert(stream_id, StreamInfo {
            stream_id,
            sequence,
            request_id: None,
        });
    }
    
    /// Close a stream
    async fn close_stream(&self, stream_id: u32) {
        let mut streams = self.streams.write().await;
        if streams.remove(&stream_id).is_some() {
            debug!("Closed stream: {}", stream_id);
        }
    }
    
    /// Get active stream count
    pub async fn active_stream_count(&self) -> usize {
        let streams = self.streams.read().await;
        streams.len()
    }
    
    /// Get list of active stream IDs
    pub async fn active_streams(&self) -> Vec<u32> {
        let streams = self.streams.read().await;
        streams.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::PingHandler;
    use mitoxide_proto::{Request, Response};
    use std::collections::HashMap;
    use std::io::Cursor;

    
    #[tokio::test]
    async fn test_router_creation() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        assert_eq!(router.active_stream_count().await, 0);
        assert!(router.active_streams().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_handler_registration() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        let handler = Arc::new(PingHandler);
        router.register_handler("ping".to_string(), handler).await;
        
        let handlers = router.handlers.read().await;
        assert!(handlers.contains_key("ping"));
    }
    
    #[tokio::test]
    async fn test_stream_management() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        // Update stream info
        router.update_stream_info(1, 42).await;
        assert_eq!(router.active_stream_count().await, 1);
        assert_eq!(router.active_streams().await, vec![1]);
        
        // Close stream
        router.close_stream(1).await;
        assert_eq!(router.active_stream_count().await, 0);
        assert!(router.active_streams().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_ping_request_routing() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        // Register ping handler
        let handler = Arc::new(PingHandler);
        router.register_handler("ping".to_string(), handler).await;
        
        // Create ping request
        let request = Request::ping();
        let message = Message::request(request);
        let payload = rmp_serde::to_vec(&message).unwrap();
        let frame = Frame::data(1, 1, Bytes::from(payload));
        
        // Route the frame
        let result = router.route_frame(frame).await;
        assert!(result.is_ok());
        
        // Check that stream was created
        assert_eq!(router.active_stream_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_invalid_message_routing() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        // Create frame with invalid payload
        let frame = Frame::data(1, 1, Bytes::from(vec![0xFF, 0xFF, 0xFF, 0xFF]));
        
        // Should handle invalid message gracefully
        let result = router.route_frame(frame).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_error_frame_routing() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        let error_frame = Frame::error(1, 1, Bytes::from("test error"));
        
        // Should handle error frames gracefully
        let result = router.route_frame(error_frame).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_end_stream_frame_routing() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        // Create a stream first
        router.update_stream_info(1, 1).await;
        assert_eq!(router.active_stream_count().await, 1);
        
        // Send end-of-stream frame
        let end_frame = Frame::end_stream(1, 2);
        let result = router.route_frame(end_frame).await;
        assert!(result.is_ok());
        
        // Stream should be closed
        assert_eq!(router.active_stream_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_process_request_with_handler() {
        let handlers: Arc<RwLock<HashMap<String, Arc<dyn Handler>>>> = Arc::new(RwLock::new(HashMap::new()));
        
        // Register ping handler
        let handler: Arc<dyn Handler> = Arc::new(PingHandler);
        handlers.write().await.insert("ping".to_string(), handler);
        
        // Process ping request
        let request = Request::ping();
        let request_id = request.id();
        let response = AgentRouter::<Cursor<Vec<u8>>>::process_request(request, &handlers).await;
        
        match response {
            Response::Pong { request_id: resp_id, .. } => {
                assert_eq!(resp_id, request_id);
            }
            _ => panic!("Expected Pong response"),
        }
    }
    
    #[tokio::test]
    async fn test_process_request_without_handler() {
        let handlers: Arc<RwLock<HashMap<String, Arc<dyn Handler>>>> = Arc::new(RwLock::new(HashMap::new()));
        
        // Process request without registered handler
        let request = Request::ping();
        let request_id = request.id();
        let response = AgentRouter::<Cursor<Vec<u8>>>::process_request(request, &handlers).await;
        
        match response {
            Response::Error { request_id: resp_id, error } => {
                assert_eq!(resp_id, request_id);
                assert_eq!(error.code, ErrorCode::Unsupported);
            }
            _ => panic!("Expected Error response"),
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_request_processing() {
        let output = Cursor::new(Vec::<u8>::new());
        let router = AgentRouter::new(output);
        
        // Register handlers
        let ping_handler: Arc<dyn Handler> = Arc::new(PingHandler);
        router.register_handler("ping".to_string(), ping_handler).await;
        
        // Create multiple ping requests and route them sequentially
        for i in 0..5 {
            let request = Request::ping();
            let message = Message::request(request);
            let payload = rmp_serde::to_vec(&message).unwrap();
            let frame = Frame::data(i + 1, 1, Bytes::from(payload));
            
            let result = router.route_frame(frame).await;
            assert!(result.is_ok());
        }
        
        // Check that all streams were processed
        assert_eq!(router.active_stream_count().await, 5);
    }
}