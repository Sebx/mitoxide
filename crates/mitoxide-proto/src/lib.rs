//! # Mitoxide Protocol
//!
//! Protocol definitions, message types, and codec for the Mitoxide RPC system.

#![warn(missing_docs)]

/// Frame structure and serialization
pub mod frame;

/// Message types and enums
pub mod message;

/// Frame codec for async streams
pub mod codec;

/// Stream multiplexing and management
pub mod stream;

/// Error types for protocol operations
pub mod error;

pub use frame::{Frame, FrameFlags};
pub use message::{Message, Request, Response};
pub use codec::FrameCodec;
pub use stream::{StreamMultiplexer, StreamHandle, StreamState};
pub use error::ProtocolError;