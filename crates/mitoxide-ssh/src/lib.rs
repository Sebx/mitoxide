//! # Mitoxide SSH Transport
//!
//! SSH transport layer implementation for Mitoxide.

#![warn(missing_docs)]

/// Transport abstraction and implementations
pub mod transport;

/// SSH connection management
pub mod connection;

/// Connection pool and management
pub mod pool;

/// Agent bootstrap logic
pub mod bootstrap;

/// SSH-specific error types
pub mod error;

pub use transport::{Transport, StdioTransport, SshConfig, ConnectionInfo, TransportType};
pub use connection::Connection;
pub use pool::{ConnectionPool, PoolConfig, PooledConnection};
pub use bootstrap::{Bootstrap, PlatformInfo, BootstrapMethod};
pub use error::TransportError;