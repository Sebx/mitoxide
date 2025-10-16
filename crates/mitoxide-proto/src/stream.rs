//! Stream multiplexing and management

use crate::{Frame, ProtocolError};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Stream multiplexer for managing multiple logical streams
pub struct StreamMultiplexer {
    /// Next stream ID to assign
    next_stream_id: AtomicU32,
    /// Active streams
    streams: Arc<Mutex<HashMap<u32, StreamInfo>>>,
    /// Incoming frame sender
    frame_sender: mpsc::UnboundedSender<Frame>,
    /// Incoming frame receiver
    frame_receiver: Arc<Mutex<mpsc::UnboundedReceiver<Frame>>>,
    /// Global flow control settings
    flow_control_config: FlowControlConfig,
}

/// Flow control configuration
#[derive(Debug, Clone)]
pub struct FlowControlConfig {
    /// Initial window size for new streams
    pub initial_window_size: u32,
    /// Maximum window size
    pub max_window_size: u32,
    /// Connection-level window size
    pub connection_window_size: u32,
}

/// Information about an active stream
#[derive(Debug)]
struct StreamInfo {
    /// Stream state
    state: StreamState,
    /// Sender for frames to this stream
    frame_sender: mpsc::UnboundedSender<Frame>,
    /// Next expected sequence number
    next_sequence: u32,
    /// Request ID if this is a request stream
    request_id: Option<Uuid>,
    /// Flow control state
    flow_control: FlowControlState,
}

/// Flow control state for a stream
#[derive(Debug)]
struct FlowControlState {
    /// Send window (credits we can send)
    send_window: u32,
    /// Receive window (credits we can receive)
    recv_window: u32,
    /// Initial window size
    initial_window_size: u32,
    /// Bytes sent but not yet acknowledged
    bytes_in_flight: u32,
    /// Bytes received but not yet processed
    bytes_buffered: u32,
}

/// Stream state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is open and active
    Open,
    /// Stream is half-closed (local end closed)
    HalfClosed,
    /// Stream is fully closed
    Closed,
}

/// Handle to a specific stream
pub struct StreamHandle {
    /// Stream ID
    stream_id: u32,
    /// Frame receiver for this stream
    frame_receiver: mpsc::UnboundedReceiver<Frame>,
    /// Reference to the multiplexer for sending frames
    multiplexer: Arc<StreamMultiplexer>,
    /// Next sequence number for outgoing frames
    next_sequence: AtomicU32,
    /// Stream state
    state: StreamState,
}

impl Default for FlowControlConfig {
    fn default() -> Self {
        Self {
            initial_window_size: 65536, // 64KB
            max_window_size: 1048576,   // 1MB
            connection_window_size: 1048576, // 1MB
        }
    }
}

impl FlowControlState {
    fn new(initial_window_size: u32) -> Self {
        Self {
            send_window: initial_window_size,
            recv_window: initial_window_size,
            initial_window_size,
            bytes_in_flight: 0,
            bytes_buffered: 0,
        }
    }
    
    /// Check if we can send data of the given size
    fn can_send(&self, size: u32) -> bool {
        self.send_window >= size && self.bytes_in_flight + size <= self.initial_window_size
    }
    
    /// Consume send credits
    fn consume_send_credits(&mut self, size: u32) -> Result<(), ProtocolError> {
        if !self.can_send(size) {
            return Err(ProtocolError::FlowControlViolation);
        }
        
        self.send_window -= size;
        self.bytes_in_flight += size;
        Ok(())
    }
    
    /// Add receive credits (when data is processed)
    fn add_recv_credits(&mut self, size: u32) {
        self.recv_window += size;
        self.bytes_buffered = self.bytes_buffered.saturating_sub(size);
    }
    
    /// Consume receive credits (when data is received)
    fn consume_recv_credits(&mut self, size: u32) -> Result<(), ProtocolError> {
        if self.recv_window < size {
            return Err(ProtocolError::FlowControlViolation);
        }
        
        self.recv_window -= size;
        self.bytes_buffered += size;
        Ok(())
    }
    
    /// Update send window (when receiving window updates)
    fn update_send_window(&mut self, delta: u32) {
        self.send_window += delta;
        self.bytes_in_flight = self.bytes_in_flight.saturating_sub(delta);
    }
}

impl StreamMultiplexer {
    /// Create a new stream multiplexer
    pub fn new() -> Self {
        Self::with_config(FlowControlConfig::default())
    }
    
    /// Create a new stream multiplexer with custom flow control config
    pub fn with_config(config: FlowControlConfig) -> Self {
        let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
        
        Self {
            next_stream_id: AtomicU32::new(1),
            streams: Arc::new(Mutex::new(HashMap::new())),
            frame_sender,
            frame_receiver: Arc::new(Mutex::new(frame_receiver)),
            flow_control_config: config,
        }
    }
    
    /// Create a new stream
    pub async fn create_stream(&self, request_id: Option<Uuid>) -> Result<StreamHandle, ProtocolError> {
        let stream_id = self.next_stream_id.fetch_add(1, Ordering::SeqCst);
        let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
        
        let stream_info = StreamInfo {
            state: StreamState::Open,
            frame_sender,
            next_sequence: 0,
            request_id,
            flow_control: FlowControlState::new(self.flow_control_config.initial_window_size),
        };
        
        {
            let mut streams = self.streams.lock().await;
            streams.insert(stream_id, stream_info);
        }
        
        Ok(StreamHandle {
            stream_id,
            frame_receiver,
            multiplexer: Arc::new(self.clone()),
            next_sequence: AtomicU32::new(0),
            state: StreamState::Open,
        })
    }
    
    /// Route an incoming frame to the appropriate stream
    pub async fn route_frame(&self, frame: Frame) -> Result<(), ProtocolError> {
        let stream_id = frame.stream_id;
        
        let mut streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get_mut(&stream_id) {
            // Check sequence number
            if frame.sequence != stream_info.next_sequence {
                return Err(ProtocolError::InvalidFrame);
            }
            
            stream_info.next_sequence += 1;
            
            // Handle end-of-stream
            if frame.is_end_stream() {
                stream_info.state = StreamState::Closed;
            }
            
            // Send frame to stream
            if let Err(_) = stream_info.frame_sender.send(frame) {
                // Stream receiver dropped, clean up
                streams.remove(&stream_id);
            }
        } else {
            // Unknown stream ID
            return Err(ProtocolError::InvalidStreamId(stream_id));
        }
        
        Ok(())
    }
    
    /// Close a stream
    pub async fn close_stream(&self, stream_id: u32) -> Result<(), ProtocolError> {
        let mut streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get_mut(&stream_id) {
            stream_info.state = StreamState::Closed;
            streams.remove(&stream_id);
            Ok(())
        } else {
            Err(ProtocolError::InvalidStreamId(stream_id))
        }
    }
    
    /// Get the number of active streams
    pub async fn stream_count(&self) -> usize {
        let streams = self.streams.lock().await;
        streams.len()
    }
    
    /// Get stream state
    pub async fn stream_state(&self, stream_id: u32) -> Option<StreamState> {
        let streams = self.streams.lock().await;
        streams.get(&stream_id).map(|info| info.state)
    }
    
    /// Process incoming frames (should be called in a loop)
    pub async fn process_frames(&self) -> Result<(), ProtocolError> {
        let mut receiver = self.frame_receiver.lock().await;
        
        while let Some(frame) = receiver.recv().await {
            self.route_frame(frame).await?;
        }
        
        Ok(())
    }
    
    /// Send a frame through the multiplexer
    pub fn send_frame(&self, frame: Frame) -> Result<(), ProtocolError> {
        self.frame_sender.send(frame)
            .map_err(|_| ProtocolError::StreamClosed)
    }
    
    /// Check if a stream can send data of the given size
    pub async fn can_send_data(&self, stream_id: u32, size: u32) -> Result<bool, ProtocolError> {
        let streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get(&stream_id) {
            Ok(stream_info.flow_control.can_send(size))
        } else {
            Err(ProtocolError::InvalidStreamId(stream_id))
        }
    }
    
    /// Update flow control window for a stream
    pub async fn update_window(&self, stream_id: u32, delta: u32) -> Result<(), ProtocolError> {
        let mut streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get_mut(&stream_id) {
            stream_info.flow_control.update_send_window(delta);
            Ok(())
        } else {
            Err(ProtocolError::InvalidStreamId(stream_id))
        }
    }
    
    /// Process received data and update flow control
    pub async fn process_received_data(&self, stream_id: u32, size: u32) -> Result<(), ProtocolError> {
        let mut streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get_mut(&stream_id) {
            stream_info.flow_control.consume_recv_credits(size)?;
            Ok(())
        } else {
            Err(ProtocolError::InvalidStreamId(stream_id))
        }
    }
    
    /// Acknowledge processed data and return credits
    pub async fn ack_processed_data(&self, stream_id: u32, size: u32) -> Result<(), ProtocolError> {
        let mut streams = self.streams.lock().await;
        
        if let Some(stream_info) = streams.get_mut(&stream_id) {
            stream_info.flow_control.add_recv_credits(size);
            Ok(())
        } else {
            Err(ProtocolError::InvalidStreamId(stream_id))
        }
    }
}

impl Clone for StreamMultiplexer {
    fn clone(&self) -> Self {
        Self {
            next_stream_id: AtomicU32::new(self.next_stream_id.load(Ordering::SeqCst)),
            streams: Arc::clone(&self.streams),
            frame_sender: self.frame_sender.clone(),
            frame_receiver: Arc::clone(&self.frame_receiver),
            flow_control_config: self.flow_control_config.clone(),
        }
    }
}

impl Default for StreamMultiplexer {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamHandle {
    /// Get the stream ID
    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }
    
    /// Send a data frame on this stream
    pub async fn send_data(&mut self, payload: Bytes) -> Result<(), ProtocolError> {
        if self.state == StreamState::Closed {
            return Err(ProtocolError::StreamClosed);
        }
        
        let payload_size = payload.len() as u32;
        
        // Check flow control
        if !self.multiplexer.can_send_data(self.stream_id, payload_size).await? {
            return Err(ProtocolError::FlowControlViolation);
        }
        
        let sequence = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        let frame = Frame::data(self.stream_id, sequence, payload);
        
        // Consume flow control credits
        {
            let mut streams = self.multiplexer.streams.lock().await;
            if let Some(stream_info) = streams.get_mut(&self.stream_id) {
                stream_info.flow_control.consume_send_credits(payload_size)?;
            }
        }
        
        self.multiplexer.send_frame(frame)
    }
    
    /// Send an end-of-stream frame
    pub async fn send_end_stream(&mut self) -> Result<(), ProtocolError> {
        if self.state == StreamState::Closed {
            return Err(ProtocolError::StreamClosed);
        }
        
        let sequence = self.next_sequence.fetch_add(1, Ordering::SeqCst);
        let frame = Frame::end_stream(self.stream_id, sequence);
        
        self.state = StreamState::HalfClosed;
        self.multiplexer.send_frame(frame)
    }
    
    /// Receive the next frame on this stream
    pub async fn recv_frame(&mut self) -> Option<Frame> {
        self.frame_receiver.recv().await
    }
    
    /// Close this stream
    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        if self.state != StreamState::Closed {
            self.send_end_stream().await?;
            self.state = StreamState::Closed;
            self.multiplexer.close_stream(self.stream_id).await?;
        }
        Ok(())
    }
    
    /// Get the current stream state
    pub fn state(&self) -> StreamState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_stream_creation() {
        let multiplexer = StreamMultiplexer::new();
        let stream = multiplexer.create_stream(None).await.unwrap();
        
        assert_eq!(stream.stream_id(), 1);
        assert_eq!(stream.state(), StreamState::Open);
        assert_eq!(multiplexer.stream_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_multiple_streams() {
        let multiplexer = StreamMultiplexer::new();
        
        let stream1 = multiplexer.create_stream(None).await.unwrap();
        let stream2 = multiplexer.create_stream(None).await.unwrap();
        let stream3 = multiplexer.create_stream(None).await.unwrap();
        
        assert_eq!(stream1.stream_id(), 1);
        assert_eq!(stream2.stream_id(), 2);
        assert_eq!(stream3.stream_id(), 3);
        assert_eq!(multiplexer.stream_count().await, 3);
    }
    
    #[tokio::test]
    async fn test_frame_routing() {
        let multiplexer = StreamMultiplexer::new();
        let mut stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Send a frame to the stream
        let frame = Frame::data(stream_id, 0, Bytes::from("test data"));
        multiplexer.route_frame(frame.clone()).await.unwrap();
        
        // Receive the frame from the stream
        let received = timeout(Duration::from_millis(100), stream.recv_frame())
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(received.stream_id, frame.stream_id);
        assert_eq!(received.payload, frame.payload);
    }
    
    #[tokio::test]
    async fn test_stream_send_receive() {
        let multiplexer = StreamMultiplexer::new();
        let mut stream1 = multiplexer.create_stream(None).await.unwrap();
        let _stream2 = multiplexer.create_stream(None).await.unwrap();
        
        // Send data from stream1
        let payload = Bytes::from("hello world");
        stream1.send_data(payload).await.unwrap();
        
        // The frame should be available through the multiplexer's frame sender
        // In a real scenario, this would be handled by the frame processing loop
    }
    
    #[tokio::test]
    async fn test_stream_close() {
        let multiplexer = StreamMultiplexer::new();
        let mut stream = multiplexer.create_stream(None).await.unwrap();
        let _stream_id = stream.stream_id();
        
        assert_eq!(multiplexer.stream_count().await, 1);
        
        stream.close().await.unwrap();
        
        assert_eq!(stream.state(), StreamState::Closed);
        assert_eq!(multiplexer.stream_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_invalid_stream_id() {
        let multiplexer = StreamMultiplexer::new();
        
        let frame = Frame::data(999, 0, Bytes::from("test"));
        let result = multiplexer.route_frame(frame).await;
        
        assert!(matches!(result, Err(ProtocolError::InvalidStreamId(999))));
    }
    
    #[tokio::test]
    async fn test_sequence_number_validation() {
        let multiplexer = StreamMultiplexer::new();
        let stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Send frame with wrong sequence number
        let frame = Frame::data(stream_id, 5, Bytes::from("test"));
        let result = multiplexer.route_frame(frame).await;
        
        assert!(matches!(result, Err(ProtocolError::InvalidFrame)));
    }
    
    #[tokio::test]
    async fn test_end_stream_handling() {
        let multiplexer = StreamMultiplexer::new();
        let stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Send end-of-stream frame
        let frame = Frame::end_stream(stream_id, 0);
        multiplexer.route_frame(frame).await.unwrap();
        
        // Stream should be marked as closed
        assert_eq!(multiplexer.stream_state(stream_id).await, Some(StreamState::Closed));
    }
    
    #[tokio::test]
    async fn test_flow_control_basic() {
        let config = FlowControlConfig {
            initial_window_size: 1000,
            max_window_size: 2000,
            connection_window_size: 5000,
        };
        let multiplexer = StreamMultiplexer::with_config(config);
        let mut stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Should be able to send data within window
        assert!(multiplexer.can_send_data(stream_id, 500).await.unwrap());
        
        // Send some data
        let payload = Bytes::from(vec![0u8; 500]);
        stream.send_data(payload).await.unwrap();
        
        // Should still be able to send more
        assert!(multiplexer.can_send_data(stream_id, 500).await.unwrap());
        
        // But not more than the window
        assert!(!multiplexer.can_send_data(stream_id, 600).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_flow_control_violation() {
        let config = FlowControlConfig {
            initial_window_size: 100,
            max_window_size: 200,
            connection_window_size: 500,
        };
        let multiplexer = StreamMultiplexer::with_config(config);
        let mut stream = multiplexer.create_stream(None).await.unwrap();
        
        // Try to send data larger than window
        let large_payload = Bytes::from(vec![0u8; 200]);
        let result = stream.send_data(large_payload).await;
        
        assert!(matches!(result, Err(ProtocolError::FlowControlViolation)));
    }
    
    #[tokio::test]
    async fn test_window_update() {
        let config = FlowControlConfig {
            initial_window_size: 100,
            max_window_size: 200,
            connection_window_size: 500,
        };
        let multiplexer = StreamMultiplexer::with_config(config);
        let mut stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Send data to consume window
        let payload = Bytes::from(vec![0u8; 100]);
        stream.send_data(payload).await.unwrap();
        
        // Should not be able to send more
        assert!(!multiplexer.can_send_data(stream_id, 50).await.unwrap());
        
        // Update window
        multiplexer.update_window(stream_id, 50).await.unwrap();
        
        // Should now be able to send
        assert!(multiplexer.can_send_data(stream_id, 50).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_receive_flow_control() {
        let multiplexer = StreamMultiplexer::new();
        let stream = multiplexer.create_stream(None).await.unwrap();
        let stream_id = stream.stream_id();
        
        // Process received data
        multiplexer.process_received_data(stream_id, 1000).await.unwrap();
        
        // Acknowledge processed data
        multiplexer.ack_processed_data(stream_id, 500).await.unwrap();
        
        // Should be able to receive more data
        multiplexer.process_received_data(stream_id, 500).await.unwrap();
    }
    
    // Property-based tests
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_stream_id_generation_properties(
            num_streams in 1usize..100
        ) {
            tokio_test::block_on(async {
                let multiplexer = StreamMultiplexer::new();
                let mut stream_ids = Vec::new();
                
                for _ in 0..num_streams {
                    let stream = multiplexer.create_stream(None).await?;
                    stream_ids.push(stream.stream_id());
                }
                
                // All stream IDs should be unique
                stream_ids.sort();
                stream_ids.dedup();
                prop_assert_eq!(stream_ids.len(), num_streams);
                
                // Stream IDs should be sequential starting from 1
                for (i, &stream_id) in stream_ids.iter().enumerate() {
                    prop_assert_eq!(stream_id, (i + 1) as u32);
                }
                
                Ok(())
            })?;
        }
        
        #[test]
        fn test_flow_control_invariants(
            initial_window in 100u32..10000,
            data_sizes in prop::collection::vec(1u32..1000, 1..20)
        ) {
            tokio_test::block_on(async {
                let config = FlowControlConfig {
                    initial_window_size: initial_window,
                    max_window_size: initial_window * 2,
                    connection_window_size: initial_window * 5,
                };
                let multiplexer = StreamMultiplexer::with_config(config);
                let mut stream = multiplexer.create_stream(None).await?;
                let stream_id = stream.stream_id();
                
                let mut total_sent = 0u32;
                
                for &size in &data_sizes {
                    let can_send = multiplexer.can_send_data(stream_id, size).await?;
                    
                    if can_send && total_sent + size <= initial_window {
                        // Should be able to send
                        let payload = Bytes::from(vec![0u8; size as usize]);
                        stream.send_data(payload).await?;
                        total_sent += size;
                    } else {
                        // Should not be able to send
                        let payload = Bytes::from(vec![0u8; size as usize]);
                        let result = stream.send_data(payload).await;
                        prop_assert!(result.is_err());
                    }
                }
                
                // Total sent should not exceed initial window
                prop_assert!(total_sent <= initial_window);
                
                Ok(())
            })?;
        }
        
        #[test]
        fn test_concurrent_stream_operations(
            num_streams in 1usize..10,
            operations_per_stream in 1usize..10
        ) {
            tokio_test::block_on(async {
                let multiplexer = StreamMultiplexer::new();
                let mut streams = Vec::new();
                
                // Create streams
                for _ in 0..num_streams {
                    let stream = multiplexer.create_stream(None).await?;
                    streams.push(stream);
                }
                
                prop_assert_eq!(multiplexer.stream_count().await, num_streams);
                
                // Perform operations on each stream
                for stream in &mut streams {
                    for i in 0..operations_per_stream {
                        let payload = Bytes::from(format!("data-{}", i));
                        // Some operations might fail due to flow control, that's ok
                        let _ = stream.send_data(payload).await;
                    }
                }
                
                // Close all streams
                for stream in &mut streams {
                    stream.close().await?;
                }
                
                prop_assert_eq!(multiplexer.stream_count().await, 0);
                
                Ok(())
            })?;
        }
        
        #[test]
        fn test_sequence_number_properties(
            num_frames in 1usize..50
        ) {
            tokio_test::block_on(async {
                let multiplexer = StreamMultiplexer::new();
                let stream = multiplexer.create_stream(None).await?;
                let stream_id = stream.stream_id();
                
                // Send frames with correct sequence numbers
                for i in 0..num_frames {
                    let frame = Frame::data(stream_id, i as u32, Bytes::from("test"));
                    multiplexer.route_frame(frame).await?;
                }
                
                // Try to send frame with wrong sequence number
                let wrong_frame = Frame::data(stream_id, (num_frames + 5) as u32, Bytes::from("wrong"));
                let result = multiplexer.route_frame(wrong_frame).await;
                prop_assert!(result.is_err());
                
                Ok(())
            })?;
        }
        
        #[test]
        fn test_window_update_properties(
            initial_window in 100u32..1000,
            updates in prop::collection::vec(1u32..500, 1..10)
        ) {
            tokio_test::block_on(async {
                let config = FlowControlConfig {
                    initial_window_size: initial_window,
                    max_window_size: initial_window * 10,
                    connection_window_size: initial_window * 10,
                };
                let multiplexer = StreamMultiplexer::with_config(config);
                let mut stream = multiplexer.create_stream(None).await?;
                let stream_id = stream.stream_id();
                
                // Consume initial window
                let payload = Bytes::from(vec![0u8; initial_window as usize]);
                stream.send_data(payload).await?;
                
                // Should not be able to send more
                prop_assert!(!multiplexer.can_send_data(stream_id, 1).await?);
                
                // Apply window updates
                let mut total_updates = 0u32;
                for &update in &updates {
                    multiplexer.update_window(stream_id, update).await?;
                    total_updates += update;
                    
                    // Should be able to send up to the updated amount
                    if total_updates > 0 {
                        prop_assert!(multiplexer.can_send_data(stream_id, 1).await?);
                    }
                }
                
                Ok(())
            })?;
        }
    }
}