//! Bevy Bridge Core
//!
//! A reusable async library for communicating with Bevy Remote Protocol (BRP) over HTTP.
//! Provides structured config, error handling, and high-level operations for interacting
//! with a running Bevy game instance.

pub mod config;
pub mod error;
pub mod client;
pub mod ops;
pub mod types;

// Re-export commonly used types
pub use config::BrpConfig;
pub use error::BrpError;
pub use client::BrpClient;

/// Result type alias using BrpError
pub type Result<T> = std::result::Result<T, BrpError>;
