//! Connection routing and multiplexing

use crate::{Result, MitoxideError};
use mitoxide_proto::{Message, Response, Frame, FrameCodec, FrameFlags};
use mitoxide_proto::message::{ErrorDetails, ErrorCode};
use mitoxide_ssh::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock, Mutex};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Connection router for managing multiple connections and request/response correlation
pub struct Router {
    /// Pending requests waiting for responses
    pending_requests: Arc<RwLock<HashMap<Uuid, oneshot::Sender<Response>>>>,
    /// Message sender to the connection handler
    message_tx: mpsc::Sender<Message>,
    /// Shutdown sender
    shutdown_tx: mpsc::Sender<()>,
    /// Request timeout
    request_timeout: Duration,
}

impl Router {
    /// Create a new router with connection
    pub async fn new(
        connection: Connection,
        max_streams: u32,
        timeout: Duration,
    ) -> Result<(Self, mpsc::Sender<()>)> {
        let (message_tx, message_rx) = mpsc::channel(max_streams as usize);
        let (_shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let (router_shutdown_tx, _router_shutdown_rx) = mpsc::channel(1);
        
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        
        let router = Self {
            pending_requests: pending_requests.clone(),
            message_tx,
            shutdown_tx: router_shutdown_tx.clone(),
            request_timeout: timeout,
        };
        
        // Start connection handler task
        let connection_handler = ConnectionHandler::new(
            connection,
            message_rx,
            pending_requests,
            shutdown_rx,
        );
        
        tokio::spawn(async move {
            if let Err(e) = connection_handler.run().await {
                error!("Connection handler error: {}", e);
            }
        });
        
        Ok((router, router_shutdown_tx))
    }
    
    /// Send a message and wait for response
    pub async fn send_message(&self, message: Message) -> Result<Response> {
        let request_id = message.request_id()
            .ok_or_else(|| MitoxideError::Protocol("Message has no request ID".to_string()))?;
        
        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();
        
        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id, response_tx);
        }
        
        // Send message
        self.message_tx.send(message).await
            .map_err(|_| MitoxideError::Protocol("Failed to send message".to_string()))?;
        
        // Wait for response with timeout
        let response = timeout(self.request_timeout, response_rx).await
            .map_err(|_| MitoxideError::Timeout { duration: self.request_timeout })?
            .map_err(|_| MitoxideError::Protocol("Response channel closed".to_string()))?;
        
        Ok(response)
    }
    
    /// Shutdown the router
    pub async fn shutdown(&self) -> Result<()> {
        debug!("Shutting down router");
        
        // Send shutdown signal
        self.shutdown_tx.send(()).await
            .map_err(|_| MitoxideError::Protocol("Failed to send shutdown signal".to_string()))?;
        
        // Cancel all pending requests
        let mut pending = self.pending_requests.write().await;
        for (request_id, sender) in pending.drain() {
            let error_response = Response::error(
                request_id,
                ErrorDetails::new(
                    ErrorCode::InternalError,
                    "Router shutdown"
                )
            );
            let _ = sender.send(error_response);
        }
        
        info!("Router shutdown complete");
        Ok(())
    }
}

/// Connection handler manages the actual connection and message processing
struct ConnectionHandler {
    /// Frame codec for the connection
    codec: FrameCodec,
    /// Connection for reading/writing
    connection: Connection,
    /// Message receiver from router
    message_rx: mpsc::Receiver<Message>,
    /// Pending requests map
    pending_requests: Arc<RwLock<HashMap<Uuid, oneshot::Sender<Response>>>>,
    /// Shutdown receiver
    shutdown_rx: mpsc::Receiver<()>,
    /// Next stream ID
    next_stream_id: Arc<Mutex<u32>>,
}

impl ConnectionHandler {
    /// Create a new connection handler
    fn new(
        connection: Connection,
        message_rx: mpsc::Receiver<Message>,
        pending_requests: Arc<RwLock<HashMap<Uuid, oneshot::Sender<Response>>>>,
        shutdown_rx: mpsc::Receiver<()>,
    ) -> Self {
        let codec = FrameCodec::new();
        
        Self {
            codec,
            connection,
            message_rx,
            pending_requests,
            shutdown_rx,
            next_stream_id: Arc::new(Mutex::new(1)),
        }
    }
    
    /// Run the connection handler main loop
    async fn run(mut self) -> Result<()> {
        info!("Starting connection handler");
        
        loop {
            tokio::select! {
                // Handle outgoing messages
                message = self.message_rx.recv() => {
                    match message {
                        Some(msg) => {
                            if let Err(e) = self.send_message(msg).await {
                                error!("Failed to send message: {}", e);
                            }
                        }
                        None => {
                            debug!("Message channel closed");
                            break;
                        }
                    }
                }
                
                // Handle incoming frames
                frame_result = async {
                    if let Some(stdout) = self.connection.stdout() {
                        self.codec.read_frame(stdout).await
                    } else {
                        Err(mitoxide_proto::ProtocolError::Serialization("No stdout available".to_string()))
                    }
                } => {
                    match frame_result {
                        Ok(Some(frame)) => {
                            if let Err(e) = self.handle_incoming_frame(frame).await {
                                error!("Failed to handle incoming frame: {}", e);
                            }
                        }
                        Ok(None) => {
                            debug!("Connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("Failed to read frame: {}", e);
                            break;
                        }
                    }
                }
                
                // Handle shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }
        
        info!("Connection handler stopped");
        Ok(())
    }
    
    /// Send a message over the connection
    async fn send_message(&mut self, message: Message) -> Result<()> {
        debug!("Sending message: {:?}", message);
        
        // Serialize message
        let payload = rmp_serde::to_vec(&message)
            .map_err(|e| MitoxideError::Protocol(format!("Failed to serialize message: {}", e)))?;
        
        // Get next stream ID
        let stream_id = {
            let mut next_id = self.next_stream_id.lock().await;
            let id = *next_id;
            *next_id = next_id.wrapping_add(1);
            id
        };
        
        // Create frame
        let frame = Frame::new(
            stream_id,
            0, // sequence number
            FrameFlags::NONE,
            payload.into(),
        );
        
        // Send frame
        if let Some(stdin) = self.connection.stdin() {
            self.codec.write_frame(stdin, &frame).await
                .map_err(|e| MitoxideError::Protocol(format!("Failed to write frame: {}", e)))?;
        } else {
            return Err(MitoxideError::Protocol("No stdin available".to_string()));
        }
        
        Ok(())
    }
    
    /// Handle an incoming frame
    async fn handle_incoming_frame(&mut self, frame: Frame) -> Result<()> {
        debug!("Received frame: stream_id={}, len={}", frame.stream_id, frame.payload.len());
        
        // Deserialize message
        let message: Message = rmp_serde::from_slice(&frame.payload)
            .map_err(|e| MitoxideError::Protocol(format!("Failed to deserialize message: {}", e)))?;
        
        match message {
            Message::Response(response) => {
                self.handle_response(response).await?;
            }
            Message::Request(_) => {
                warn!("Received unexpected request from remote");
            }
        }
        
        Ok(())
    }
    
    /// Handle a response message
    async fn handle_response(&self, response: Response) -> Result<()> {
        let request_id = response.request_id();
        debug!("Handling response for request: {}", request_id);
        
        // Find pending request
        let sender = {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&request_id)
        };
        
        if let Some(sender) = sender {
            // Send response to waiting caller
            if let Err(_) = sender.send(response) {
                warn!("Failed to send response - receiver dropped");
            }
        } else {
            warn!("Received response for unknown request: {}", request_id);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests;