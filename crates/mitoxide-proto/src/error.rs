//! Error types for protocol operations

use thiserror::Error;
use crate::message::{ErrorCode, ErrorDetails};

/// Protocol-specific errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Invalid frame format
    #[error("Invalid frame format")]
    InvalidFrame,
    
    /// Frame too large
    #[error("Frame too large: {size} bytes (max: {max})")]
    FrameTooLarge { 
        /// Actual frame size
        size: usize, 
        /// Maximum allowed size
        max: usize 
    },
    
    /// Stream closed
    #[error("Stream closed")]
    StreamClosed,
    
    /// Invalid stream ID
    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(u32),
    
    /// Flow control violation
    #[error("Flow control violation")]
    FlowControlViolation,
}

impl From<ErrorDetails> for ProtocolError {
    fn from(details: ErrorDetails) -> Self {
        match details.code {
            ErrorCode::InvalidRequest => Self::InvalidFrame,
            _ => Self::Serialization(details.message),
        }
    }
}

impl From<ProtocolError> for ErrorDetails {
    fn from(error: ProtocolError) -> Self {
        match error {
            ProtocolError::Serialization(msg) => {
                ErrorDetails::new(ErrorCode::InvalidRequest, msg)
            }
            ProtocolError::InvalidFrame => {
                ErrorDetails::new(ErrorCode::InvalidRequest, "Invalid frame format")
            }
            ProtocolError::FrameTooLarge { size, max } => {
                ErrorDetails::new(
                    ErrorCode::ResourceExhausted,
                    format!("Frame too large: {} bytes (max: {})", size, max)
                )
            }
            ProtocolError::StreamClosed => {
                ErrorDetails::new(ErrorCode::InternalError, "Stream closed")
            }
            ProtocolError::InvalidStreamId(id) => {
                ErrorDetails::new(ErrorCode::InvalidRequest, format!("Invalid stream ID: {}", id))
            }
            ProtocolError::FlowControlViolation => {
                ErrorDetails::new(ErrorCode::InternalError, "Flow control violation")
            }
        }
    }
}