//! Frame codec for async streams

use crate::{Frame, ProtocolError};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum frame size (16MB)
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Frame codec for encoding/decoding frames over async streams
pub struct FrameCodec {
    /// Read buffer for incoming data
    read_buf: BytesMut,
    /// Maximum frame size allowed
    max_frame_size: usize,
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameCodec {
    /// Create a new frame codec with default settings
    pub fn new() -> Self {
        Self {
            read_buf: BytesMut::with_capacity(8192),
            max_frame_size: MAX_FRAME_SIZE,
        }
    }
    
    /// Create a new frame codec with custom max frame size
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self {
            read_buf: BytesMut::with_capacity(8192),
            max_frame_size,
        }
    }
    
    /// Encode a frame to bytes with length prefix
    pub fn encode_frame(&self, frame: &Frame) -> Result<Bytes, ProtocolError> {
        // Serialize the frame to MessagePack
        let frame_bytes = frame.to_msgpack()?;
        
        // Check frame size limit
        if frame_bytes.len() > self.max_frame_size {
            return Err(ProtocolError::FrameTooLarge {
                size: frame_bytes.len(),
                max: self.max_frame_size,
            });
        }
        
        // Create buffer with length prefix (4 bytes) + frame data
        let mut buf = BytesMut::with_capacity(4 + frame_bytes.len());
        buf.put_u32(frame_bytes.len() as u32);
        buf.put_slice(&frame_bytes);
        
        Ok(buf.freeze())
    }
    
    /// Write a frame to an async writer
    pub async fn write_frame<W>(&self, writer: &mut W, frame: &Frame) -> Result<(), ProtocolError>
    where
        W: AsyncWrite + Unpin,
    {
        let encoded = self.encode_frame(frame)?;
        writer.write_all(&encoded).await
            .map_err(|e| ProtocolError::Serialization(format!("Write error: {}", e)))?;
        writer.flush().await
            .map_err(|e| ProtocolError::Serialization(format!("Flush error: {}", e)))?;
        Ok(())
    }
    
    /// Read a frame from an async reader
    pub async fn read_frame<R>(&mut self, reader: &mut R) -> Result<Option<Frame>, ProtocolError>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            // Try to decode a frame from the buffer
            if let Some(frame) = self.try_decode_frame()? {
                return Ok(Some(frame));
            }
            
            // Need more data, read from the stream
            let mut temp_buf = [0u8; 8192];
            let n = reader.read(&mut temp_buf).await
                .map_err(|e| ProtocolError::Serialization(format!("Read error: {}", e)))?;
            
            if n == 0 {
                // EOF reached
                if self.read_buf.is_empty() {
                    return Ok(None);
                } else {
                    return Err(ProtocolError::InvalidFrame);
                }
            }
            
            self.read_buf.extend_from_slice(&temp_buf[..n]);
        }
    }
    
    /// Try to decode a frame from the internal buffer
    pub fn try_decode_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        if self.read_buf.len() < 4 {
            // Not enough data for length prefix
            return Ok(None);
        }
        
        // Read the length prefix without consuming it
        let frame_len = (&self.read_buf[..4]).get_u32() as usize;
        
        // Check frame size limit
        if frame_len > self.max_frame_size {
            return Err(ProtocolError::FrameTooLarge {
                size: frame_len,
                max: self.max_frame_size,
            });
        }
        
        // Check if we have the complete frame
        if self.read_buf.len() < 4 + frame_len {
            return Ok(None);
        }
        
        // We have a complete frame, consume the length prefix
        self.read_buf.advance(4);
        
        // Extract the frame data
        let frame_data = self.read_buf.split_to(frame_len);
        
        // Deserialize the frame
        let frame = Frame::from_msgpack(&frame_data)?;
        Ok(Some(frame))
    }
    
    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.read_buf.len()
    }
    
    /// Clear the internal buffer
    pub fn clear_buffer(&mut self) {
        self.read_buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use proptest::prelude::*;
    
    #[tokio::test]
    async fn test_frame_encode_decode() {
        let codec = FrameCodec::new();
        let frame = Frame::data(1, 42, Bytes::from("test payload"));
        
        let encoded = codec.encode_frame(&frame).unwrap();
        assert!(encoded.len() > 4); // Should have length prefix
        
        let mut codec2 = FrameCodec::new();
        let mut cursor = Cursor::new(encoded);
        let decoded = codec2.read_frame(&mut cursor).await.unwrap().unwrap();
        
        assert_eq!(frame.stream_id, decoded.stream_id);
        assert_eq!(frame.sequence, decoded.sequence);
        assert_eq!(frame.flags, decoded.flags);
        assert_eq!(frame.payload, decoded.payload);
    }
    
    #[tokio::test]
    async fn test_write_read_frame() {
        let codec = FrameCodec::new();
        let frame = Frame::end_stream(123, 456);
        
        let mut buffer = Vec::new();
        codec.write_frame(&mut buffer, &frame).await.unwrap();
        
        let mut codec2 = FrameCodec::new();
        let mut cursor = Cursor::new(buffer);
        let decoded = codec2.read_frame(&mut cursor).await.unwrap().unwrap();
        
        assert_eq!(frame.stream_id, decoded.stream_id);
        assert_eq!(frame.sequence, decoded.sequence);
        assert!(decoded.is_end_stream());
    }
    
    #[tokio::test]
    async fn test_partial_frame_reading() {
        let codec = FrameCodec::new();
        let frame = Frame::data(1, 1, Bytes::from("test"));
        let encoded = codec.encode_frame(&frame).unwrap();
        
        // Test the try_decode_frame method directly with partial data
        let mut codec2 = FrameCodec::new();
        
        // Add partial data to the buffer
        let mid = encoded.len() / 2;
        codec2.read_buf.extend_from_slice(&encoded[..mid]);
        
        // Should return None (incomplete)
        let result1 = codec2.try_decode_frame().unwrap();
        assert!(result1.is_none());
        
        // Add the rest of the data
        codec2.read_buf.extend_from_slice(&encoded[mid..]);
        
        // Should now return the complete frame
        let result2 = codec2.try_decode_frame().unwrap().unwrap();
        
        assert_eq!(frame.stream_id, result2.stream_id);
        assert_eq!(frame.payload, result2.payload);
    }
    
    #[tokio::test]
    async fn test_multiple_frames_in_buffer() {
        let codec = FrameCodec::new();
        let frame1 = Frame::data(1, 1, Bytes::from("first"));
        let frame2 = Frame::data(2, 2, Bytes::from("second"));
        
        let encoded1 = codec.encode_frame(&frame1).unwrap();
        let encoded2 = codec.encode_frame(&frame2).unwrap();
        
        // Combine both frames in one buffer
        let mut combined = BytesMut::new();
        combined.extend_from_slice(&encoded1);
        combined.extend_from_slice(&encoded2);
        
        let mut codec2 = FrameCodec::new();
        let mut cursor = Cursor::new(combined.freeze());
        
        // Read first frame
        let decoded1 = codec2.read_frame(&mut cursor).await.unwrap().unwrap();
        assert_eq!(frame1.stream_id, decoded1.stream_id);
        assert_eq!(frame1.payload, decoded1.payload);
        
        // Read second frame
        let decoded2 = codec2.read_frame(&mut cursor).await.unwrap().unwrap();
        assert_eq!(frame2.stream_id, decoded2.stream_id);
        assert_eq!(frame2.payload, decoded2.payload);
        
        // No more frames
        let result3 = codec2.read_frame(&mut cursor).await.unwrap();
        assert!(result3.is_none());
    }
    
    #[tokio::test]
    async fn test_frame_too_large() {
        let codec = FrameCodec::with_max_frame_size(100);
        let large_payload = Bytes::from(vec![0u8; 200]);
        let frame = Frame::data(1, 1, large_payload);
        
        let result = codec.encode_frame(&frame);
        assert!(matches!(result, Err(ProtocolError::FrameTooLarge { .. })));
    }
    
    #[tokio::test]
    async fn test_invalid_frame_data() {
        let mut codec = FrameCodec::new();
        
        // Create invalid frame data (valid length prefix but invalid MessagePack)
        let mut invalid_data = BytesMut::new();
        invalid_data.put_u32(4); // Length prefix
        invalid_data.put_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Invalid MessagePack
        
        let mut cursor = Cursor::new(invalid_data.freeze());
        let result = codec.read_frame(&mut cursor).await;
        
        assert!(matches!(result, Err(ProtocolError::Serialization(_))));
    }
    
    #[tokio::test]
    async fn test_empty_stream() {
        let mut codec = FrameCodec::new();
        let mut cursor = Cursor::new(Vec::<u8>::new());
        
        let result = codec.read_frame(&mut cursor).await.unwrap();
        assert!(result.is_none());
    }
    
    proptest! {
        #[test]
        fn test_codec_roundtrip_properties(
            stream_id in any::<u32>(),
            sequence in any::<u32>(),
            payload in prop::collection::vec(any::<u8>(), 0..1000)
        ) {
            tokio_test::block_on(async {
                let codec = FrameCodec::new();
                let frame = Frame::data(stream_id, sequence, Bytes::from(payload));
                
                let encoded = codec.encode_frame(&frame)?;
                
                let mut codec2 = FrameCodec::new();
                let mut cursor = Cursor::new(encoded);
                let decoded = codec2.read_frame(&mut cursor).await?.unwrap();
                
                prop_assert_eq!(frame.stream_id, decoded.stream_id);
                prop_assert_eq!(frame.sequence, decoded.sequence);
                prop_assert_eq!(frame.flags, decoded.flags);
                prop_assert_eq!(frame.payload, decoded.payload);
                
                Ok(())
            })?;
        }
    }
}