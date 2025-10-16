//! # Mitoxide
//!
//! A Rust library for remote execution and automation inspired by Mitogen.
//!
//! Mitoxide provides a client-agent architecture for executing commands and transferring
//! files over SSH connections with multiplexed binary RPC protocol.

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use mitoxide_proto as proto;

/// Error types for the Mitoxide library
pub mod error;

/// Session management and connection handling
pub mod session;

/// Execution context for remote operations
pub mod context;

/// Connection routing and multiplexing
pub mod router;

pub use error::MitoxideError;
pub use session::{Session, SessionBuilder, ConnectedSession};
pub use context::Context;
pub use router::Router;

/// Result type alias for Mitoxide operations
pub type Result<T> = std::result::Result<T, MitoxideError>;