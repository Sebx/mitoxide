//! Agent main loop and frame processing

use anyhow::{Context, Result};
use bytes::Bytes;
use mitoxide_proto::{Frame, FrameCodec, Message, Request, Response};
use mitoxide_proto::message::{ErrorCode, ErrorDetails};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{stdin, stdout, AsyncRead, AsyncWrite};
use tokio::sync::{oneshot, RwLock};
use tracing::{debug, error, info, warn};

/// Handler trait for processing requests
#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    /// Handle a request and return a response
    async fn handle(&self, request: Request) -> Result<Response>;
}

/// Main agent loop for processing frames
pub struct AgentLoop<R, W> 
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    /// Input stream (typically stdin)
    reader: R,
    /// Output stream (typically stdout)
    writer: W,
    /// Frame codec for encoding/decoding
    codec: FrameCodec,
    /// Registered handlers by request type
    handlers: Arc<RwLock<HashMap<String, Arc<dyn Handler>>>>,
    /// Shutdown signal receiver
    shutdown_rx: Option<oneshot::Receiver<()>>,
    /// Shutdown signal sender (kept for graceful shutdown)
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl AgentLoop<tokio::io::Stdin, tokio::io::Stdout> {
    /// Create a new agent loop with stdin/stdout
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Self {
            reader: stdin(),
            writer: stdout(),
            codec: FrameCodec::new(),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
        }
    }
}

impl<R, W> AgentLoop<R, W>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    /// Create a new agent loop with custom reader/writer
    pub fn with_io(reader: R, writer: W) -> Self {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Self {
            reader,
            writer,
            codec: FrameCodec::new(),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_rx: Some(shutdown_rx),
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
    
    /// Run the agent loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting agent loop");
        
        let mut shutdown_rx = self.shutdown_rx.take()
            .context("Shutdown receiver already taken")?;
        
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = &mut shutdown_rx => {
                    info!("Received shutdown signal, stopping agent loop");
                    break;
                }
                
                // Process incoming frames
                frame_result = self.codec.read_frame(&mut self.reader) => {
                    match frame_result {
                        Ok(Some(frame)) => {
                            if let Err(e) = self.process_frame(frame).await {
                                error!("Error processing frame: {}", e);
                                // Continue processing other frames on error
                            }
                        }
                        Ok(None) => {
                            info!("Input stream closed, stopping agent loop");
                            break;
                        }
                        Err(e) => {
                            error!("Error reading frame: {}", e);
                            // Try to continue on protocol errors
                            continue;
                        }
                    }
                }
            }
        }
        
        info!("Agent loop stopped");
        Ok(())
    }
    
    /// Process a single frame
    async fn process_frame(&mut self, frame: Frame) -> Result<()> {
        debug!("Processing frame: stream_id={}, sequence={}, flags={:?}, payload_size={}", 
               frame.stream_id, frame.sequence, frame.flags, frame.payload.len());
        
        // Handle control frames
        if frame.is_error() {
            warn!("Received error frame: stream_id={}, payload={:?}", 
                  frame.stream_id, frame.payload);
            return Ok(());
        }
        
        if frame.is_end_stream() {
            debug!("Received end-of-stream frame: stream_id={}", frame.stream_id);
            return Ok(());
        }
        
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
        
        // Dispatch message
        match message {
            Message::Request(request) => {
                self.handle_request(frame.stream_id, frame.sequence, request).await?;
            }
            Message::Response(_) => {
                warn!("Received unexpected response message on agent");
                // Agents typically don't handle responses, only requests
            }
        }
        
        Ok(())
    }
    
    /// Handle a request message
    async fn handle_request(&mut self, stream_id: u32, sequence: u32, request: Request) -> Result<()> {
        let request_id = request.id();
        debug!("Handling request: id={}, type={:?}", request_id, std::mem::discriminant(&request));
        
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
            let handlers = self.handlers.read().await;
            handlers.get(request_type).cloned()
        };
        
        let response = match handler {
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
        };
        
        // Send response
        self.send_response(stream_id, sequence, response).await?;
        
        Ok(())
    }
    
    /// Send a response message
    async fn send_response(&mut self, stream_id: u32, sequence: u32, response: Response) -> Result<()> {
        let message = Message::response(response);
        let payload = rmp_serde::to_vec(&message)
            .context("Failed to serialize response message")?;
        
        let frame = Frame::data(stream_id, sequence, Bytes::from(payload));
        self.codec.write_frame(&mut self.writer, &frame).await
            .context("Failed to write response frame")?;
        
        debug!("Sent response: stream_id={}, sequence={}", stream_id, sequence);
        Ok(())
    }
    
    /// Send an error frame
    async fn send_error_frame(&mut self, stream_id: u32, sequence: u32, 
                            error_code: ErrorCode, message: String) -> Result<()> {
        let error_payload = rmp_serde::to_vec(&ErrorDetails::new(error_code, message))
            .context("Failed to serialize error details")?;
        
        let frame = Frame::error(stream_id, sequence, Bytes::from(error_payload));
        self.codec.write_frame(&mut self.writer, &frame).await
            .context("Failed to write error frame")?;
        
        debug!("Sent error frame: stream_id={}, sequence={}", stream_id, sequence);
        Ok(())
    }
}

impl<R, W> Default for AgentLoop<R, W>
where
    R: AsyncRead + Unpin + Send + Default,
    W: AsyncWrite + Unpin + Send + Default,
{
    fn default() -> Self {
        Self::with_io(R::default(), W::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mitoxide_proto::{Request, Response};
    use std::io::Cursor;
    use tokio::time::{timeout, Duration};
    use uuid::Uuid;
    
    /// Mock handler for testing
    struct MockHandler {
        response: Response,
    }
    
    #[async_trait::async_trait]
    impl Handler for MockHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            // Echo back the request ID in a pong response
            match request {
                Request::Ping { id, timestamp } => {
                    Ok(Response::pong(id, timestamp))
                }
                _ => Ok(self.response.clone()),
            }
        }
    }
    
    #[tokio::test]
    async fn test_agent_loop_creation() {
        let agent = AgentLoop::new();
        assert!(agent.shutdown_tx.is_some());
        assert!(agent.shutdown_rx.is_some());
    }
    
    #[tokio::test]
    async fn test_handler_registration() {
        let agent = AgentLoop::new();
        let handler = Arc::new(MockHandler {
            response: Response::pong(Uuid::new_v4(), 12345),
        });
        
        agent.register_handler("test".to_string(), handler).await;
        
        let handlers = agent.handlers.read().await;
        assert!(handlers.contains_key("test"));
    }
    
    #[tokio::test]
    async fn test_graceful_shutdown() {
        let input = Cursor::new(Vec::<u8>::new());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        let shutdown_tx = agent.shutdown_sender().unwrap();
        
        // Start the agent loop in a task
        let agent_task = tokio::spawn(async move {
            agent.run().await
        });
        
        // Send shutdown signal
        shutdown_tx.send(()).unwrap();
        
        // Agent should stop gracefully
        let result = timeout(Duration::from_secs(1), agent_task).await;
        assert!(result.is_ok());
        assert!(result.unwrap().unwrap().is_ok());
    }
    
    #[tokio::test]
    async fn test_ping_request_handling() {
        // Create a ping request
        let request = Request::ping();
        let request_id = request.id();
        let message = Message::request(request);
        
        // Serialize message and create frame
        let payload = rmp_serde::to_vec(&message).unwrap();
        let frame = Frame::data(1, 1, Bytes::from(payload.clone()));
        
        // Encode frame
        let codec = FrameCodec::new();
        let encoded_frame = codec.encode_frame(&frame).unwrap();
        
        // Create agent with mock I/O
        let input = Cursor::new(encoded_frame.to_vec());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        // Register ping handler
        let handler = Arc::new(MockHandler {
            response: Response::pong(request_id, 12345),
        });
        agent.register_handler("ping".to_string(), handler).await;
        
        // Process the frame
        let frame_to_process = Frame::data(1, 1, Bytes::from(payload));
        let result = agent.process_frame(frame_to_process).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_invalid_message_handling() {
        // Create frame with invalid payload
        let frame = Frame::data(1, 1, Bytes::from(vec![0xFF, 0xFF, 0xFF, 0xFF]));
        
        let input = Cursor::new(Vec::<u8>::new());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        // Should handle invalid message gracefully
        let result = agent.process_frame(frame).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_unsupported_request_handling() {
        // Create a request without a registered handler
        let request = Request::process_exec(
            vec!["echo".to_string()],
            std::collections::HashMap::new(),
            None,
            None,
            None,
        );
        let message = Message::request(request);
        let payload = rmp_serde::to_vec(&message).unwrap();
        let frame = Frame::data(1, 1, Bytes::from(payload));
        
        let input = Cursor::new(Vec::<u8>::new());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        // Should handle unsupported request gracefully
        let result = agent.process_frame(frame).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_error_frame_handling() {
        let error_frame = Frame::error(1, 1, Bytes::from("test error"));
        
        let input = Cursor::new(Vec::<u8>::new());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        // Should handle error frames gracefully
        let result = agent.process_frame(error_frame).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_end_stream_frame_handling() {
        let end_frame = Frame::end_stream(1, 1);
        
        let input = Cursor::new(Vec::<u8>::new());
        let output = Cursor::new(Vec::<u8>::new());
        let mut agent = AgentLoop::with_io(input, output);
        
        // Should handle end-of-stream frames gracefully
        let result = agent.process_frame(end_frame).await;
        assert!(result.is_ok());
    }
}