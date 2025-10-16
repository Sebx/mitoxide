//! # Mitoxide Agent
//!
//! Core agent functionality for remote execution.

#![warn(missing_docs)]

/// Agent main loop and frame processing
pub mod agent;

/// Request handlers for different operation types
pub mod handlers;

/// Agent-side routing for multiplexed streams
pub mod router;

/// Agent bootstrap and platform detection
pub mod bootstrap;