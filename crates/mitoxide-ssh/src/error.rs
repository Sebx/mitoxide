//! SSH-specific error types

use thiserror::Error;
use std::io;

/// Transport-specific errors
#[derive(Debug, Error)]
pub enum TransportError {
    /// SSH connection error
    #[error("SSH connection error: {0}")]
    Connection(String),
    
    /// Bootstrap error
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
    
    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// Timeout error
    #[error("Operation timed out")]
    Timeout,
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Remote command failed
    #[error("Remote command failed with exit code {code}: {message}")]
    CommandFailed { 
        /// Exit code of the failed command
        code: i32, 
        /// Error message
        message: String 
    },
}