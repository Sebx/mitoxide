//! Frame structure and serialization

use serde::{Deserialize, Serialize};
use bytes::Bytes;
use crate::ProtocolError;

/// Frame flags for protocol control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameFlags(pub u8);

impl FrameFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// End of stream flag
    pub const END_STREAM: Self = Self(1);
    /// Error flag
    pub const ERROR: Self = Self(2);
    /// Flow control flag
    pub const FLOW_CONTROL: Self = Self(4);
    
    /// Check if a flag is set
    pub fn has_flag(self, flag: FrameFlags) -> bool {
        (self.0 & flag.0) != 0
    }
    
    /// Set a flag
    pub fn set_flag(&mut self, flag: FrameFlags) {
        self.0 |= flag.0;
    }
    
    /// Clear a flag
    pub fn clear_flag(&mut self, flag: FrameFlags) {
        self.0 &= !flag.0;
    }
}

/// Protocol frame structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// Stream identifier
    pub stream_id: u32,
    /// Sequence number
    pub sequence: u32,
    /// Frame flags
    pub flags: FrameFlags,
    /// Frame payload
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame
    pub fn new(stream_id: u32, sequence: u32, flags: FrameFlags, payload: Bytes) -> Self {
        Self {
            stream_id,
            sequence,
            flags,
            payload,
        }
    }
    
    /// Create a data frame
    pub fn data(stream_id: u32, sequence: u32, payload: Bytes) -> Self {
        Self::new(stream_id, sequence, FrameFlags::NONE, payload)
    }
    
    /// Create an end-of-stream frame
    pub fn end_stream(stream_id: u32, sequence: u32) -> Self {
        Self::new(stream_id, sequence, FrameFlags::END_STREAM, Bytes::new())
    }
    
    /// Create an error frame
    pub fn error(stream_id: u32, sequence: u32, payload: Bytes) -> Self {
        Self::new(stream_id, sequence, FrameFlags::ERROR, payload)
    }
    
    /// Serialize frame to MessagePack bytes
    pub fn to_msgpack(&self) -> Result<Vec<u8>, ProtocolError> {
        rmp_serde::to_vec(self)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))
    }
    
    /// Deserialize frame from MessagePack bytes
    pub fn from_msgpack(bytes: &[u8]) -> Result<Self, ProtocolError> {
        rmp_serde::from_slice(bytes)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))
    }
    
    /// Get the payload size
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }
    
    /// Check if this is an end-of-stream frame
    pub fn is_end_stream(&self) -> bool {
        self.flags.has_flag(FrameFlags::END_STREAM)
    }
    
    /// Check if this is an error frame
    pub fn is_error(&self) -> bool {
        self.flags.has_flag(FrameFlags::ERROR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    #[test]
    fn test_frame_flags() {
        let mut flags = FrameFlags::NONE;
        assert!(!flags.has_flag(FrameFlags::END_STREAM));
        
        flags.set_flag(FrameFlags::END_STREAM);
        assert!(flags.has_flag(FrameFlags::END_STREAM));
        
        flags.clear_flag(FrameFlags::END_STREAM);
        assert!(!flags.has_flag(FrameFlags::END_STREAM));
    }
    
    #[test]
    fn test_frame_creation() {
        let payload = Bytes::from("test payload");
        let frame = Frame::data(1, 42, payload.clone());
        
        assert_eq!(frame.stream_id, 1);
        assert_eq!(frame.sequence, 42);
        assert_eq!(frame.flags, FrameFlags::NONE);
        assert_eq!(frame.payload, payload);
        assert!(!frame.is_end_stream());
        assert!(!frame.is_error());
    }
    
    #[test]
    fn test_end_stream_frame() {
        let frame = Frame::end_stream(1, 42);
        assert!(frame.is_end_stream());
        assert!(!frame.is_error());
        assert_eq!(frame.payload.len(), 0);
    }
    
    #[test]
    fn test_error_frame() {
        let payload = Bytes::from("error message");
        let frame = Frame::error(1, 42, payload.clone());
        assert!(!frame.is_end_stream());
        assert!(frame.is_error());
        assert_eq!(frame.payload, payload);
    }
    
    #[test]
    fn test_msgpack_serialization_roundtrip() {
        let payload = Bytes::from("test payload data");
        let original = Frame::data(123, 456, payload);
        
        let serialized = original.to_msgpack().unwrap();
        let deserialized = Frame::from_msgpack(&serialized).unwrap();
        
        assert_eq!(original.stream_id, deserialized.stream_id);
        assert_eq!(original.sequence, deserialized.sequence);
        assert_eq!(original.flags, deserialized.flags);
        assert_eq!(original.payload, deserialized.payload);
    }
    
    #[test]
    fn test_empty_payload_serialization() {
        let frame = Frame::end_stream(1, 1);
        let serialized = frame.to_msgpack().unwrap();
        let deserialized = Frame::from_msgpack(&serialized).unwrap();
        
        assert_eq!(frame.stream_id, deserialized.stream_id);
        assert_eq!(frame.sequence, deserialized.sequence);
        assert_eq!(frame.flags, deserialized.flags);
        assert_eq!(frame.payload, deserialized.payload);
        assert!(deserialized.is_end_stream());
    }
    
    proptest! {
        #[test]
        fn test_frame_roundtrip_properties(
            stream_id in any::<u32>(),
            sequence in any::<u32>(),
            flags in any::<u8>(),
            payload in prop::collection::vec(any::<u8>(), 0..1024)
        ) {
            let frame = Frame::new(
                stream_id,
                sequence,
                FrameFlags(flags),
                Bytes::from(payload)
            );
            
            let serialized = frame.to_msgpack().unwrap();
            let deserialized = Frame::from_msgpack(&serialized).unwrap();
            
            prop_assert_eq!(frame.stream_id, deserialized.stream_id);
            prop_assert_eq!(frame.sequence, deserialized.sequence);
            prop_assert_eq!(frame.flags, deserialized.flags);
            prop_assert_eq!(frame.payload, deserialized.payload);
        }
    }
}