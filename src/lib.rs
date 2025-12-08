// ABOUTME: Main library entry point for sendspin-rs
// ABOUTME: Exports public API for Sendspin Protocol client and server

//! # sendspin-rs
//!
//! Hyper-efficient Rust implementation of the Sendspin Protocol for synchronized multi-room audio streaming.
//!
//! This library provides zero-copy audio pipelines, lock-free concurrency, and async I/O
//! for building high-performance audio streaming clients and servers.
//!
//! ## Features
//!
//! - **Client**: Connect to Sendspin servers and receive synchronized audio
//! - **Server**: Host a Sendspin server for multi-room audio streaming
//! - **Protocol**: Full implementation of the Sendspin Protocol
//!
//! ## Example: Running a Server
//!
//! ```no_run
//! use sendspin::server::{SendspinServer, ServerConfig, TestToneSource};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ServerConfig::new("My Server")
//!         .bind_addr("0.0.0.0:8927".parse().unwrap());
//!
//!     let server = SendspinServer::with_config(config)
//!         .with_source(Box::new(TestToneSource::new(440.0, 48000)));
//!
//!     server.run().await.unwrap();
//! }
//! ```

#![warn(missing_docs)]

/// Audio types and processing
pub mod audio;
/// Protocol implementation for WebSocket communication
pub mod protocol;
/// Audio scheduler for timed playback
pub mod scheduler;
/// Server implementation for hosting Sendspin services
pub mod server;
/// Clock synchronization utilities
pub mod sync;

pub use protocol::client::ProtocolClient;
pub use protocol::messages::{ClientHello, ServerHello};
pub use scheduler::AudioScheduler;
pub use server::{SendspinServer, ServerConfig};

/// Result type for sendspin operations
pub type Result<T> = std::result::Result<T, error::Error>;

/// Error types for sendspin
pub mod error {
    use thiserror::Error;

    /// Error types for sendspin operations
    #[derive(Error, Debug)]
    pub enum Error {
        /// WebSocket-related error
        #[error("WebSocket error: {0}")]
        WebSocket(String),

        /// Protocol violation or parsing error
        #[error("Protocol error: {0}")]
        Protocol(String),

        /// Invalid message format received
        #[error("Invalid message format")]
        InvalidMessage,

        /// Connection-related error
        #[error("Connection error: {0}")]
        Connection(String),

        /// Audio output error
        #[error("Audio output error: {0}")]
        Output(String),
    }
}
