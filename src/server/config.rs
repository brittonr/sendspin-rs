// ABOUTME: Server configuration
// ABOUTME: Defines configurable parameters for the Sendspin server

use std::net::SocketAddr;

/// Server configuration
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Address to bind the server to
    pub bind_addr: SocketAddr,
    /// WebSocket endpoint path
    pub ws_path: String,
    /// Server name for client discovery
    pub name: String,
    /// Unique server identifier
    pub server_id: String,
    /// Audio chunk interval in milliseconds (typically 20ms)
    pub chunk_interval_ms: u64,
    /// Buffer ahead time in milliseconds (how far ahead to send audio)
    pub buffer_ahead_ms: u64,
    /// Default sample rate in Hz
    pub default_sample_rate: u32,
    /// Default number of channels
    pub default_channels: u8,
    /// Default bit depth
    pub default_bit_depth: u8,
}

impl ServerConfig {
    /// Create a new server configuration with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the bind address
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Set the WebSocket path
    pub fn ws_path(mut self, path: impl Into<String>) -> Self {
        self.ws_path = path.into();
        self
    }

    /// Set the chunk interval in milliseconds
    pub fn chunk_interval_ms(mut self, ms: u64) -> Self {
        self.chunk_interval_ms = ms;
        self
    }

    /// Set the buffer ahead time in milliseconds
    pub fn buffer_ahead_ms(mut self, ms: u64) -> Self {
        self.buffer_ahead_ms = ms;
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8927".parse().unwrap(),
            ws_path: "/sendspin".to_string(),
            name: "Sendspin Rust Server".to_string(),
            server_id: uuid::Uuid::new_v4().to_string(),
            chunk_interval_ms: 20,
            buffer_ahead_ms: 500,
            default_sample_rate: 48000,
            default_channels: 2,
            default_bit_depth: 24,
        }
    }
}
