//! Error types for the Mitoxide library

use thiserror::Error;
use std::time::Duration;

/// Main error type for Mitoxide operations
#[derive(Debug, Error)]
pub enum MitoxideError {
    /// Transport-related errors
    #[error("Transport error: {0}")]
    Transport(String),
    
    /// Protocol-related errors  
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Agent-related errors
    #[error("Agent error: {0}")]
    Agent(String),
    
    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),
    
    /// Timeout errors
    #[error("Timeout after {duration:?}")]
    Timeout { 
        /// Duration that was exceeded
        duration: Duration 
    },
    
    /// WASM execution errors
    #[cfg(feature = "wasm")]
    #[error("WASM execution error: {0}")]
    Wasm(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// Session errors
    #[error("Session error: {0}")]
    Session(String),
}

impl From<mitoxide_ssh::TransportError> for MitoxideError {
    fn from(err: mitoxide_ssh::TransportError) -> Self {
        match err {
            mitoxide_ssh::TransportError::Connection(msg) => Self::Connection(msg),
            mitoxide_ssh::TransportError::Bootstrap(msg) => Self::Agent(msg),
            mitoxide_ssh::TransportError::Protocol(msg) => Self::Protocol(msg),
            mitoxide_ssh::TransportError::Io(e) => Self::Io(e),
            mitoxide_ssh::TransportError::Authentication(msg) => Self::Auth(msg),
            mitoxide_ssh::TransportError::Timeout => Self::Timeout { duration: Duration::from_secs(30) },
            mitoxide_ssh::TransportError::Configuration(msg) => Self::Protocol(msg),
            mitoxide_ssh::TransportError::CommandFailed { .. } => Self::Agent("Command failed".to_string()),
        }
    }
}

impl From<rmp_serde::encode::Error> for MitoxideError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Self::Serialization(format!("MessagePack encode error: {}", err))
    }
}

impl From<rmp_serde::decode::Error> for MitoxideError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Self::Serialization(format!("MessagePack decode error: {}", err))
    }
}

impl From<serde_json::Error> for MitoxideError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(format!("JSON error: {}", err))
    }
}