// ABOUTME: Protocol message type definitions and serialization
// ABOUTME: Supports client/hello, server/hello, stream/start, etc.

use serde::{Deserialize, Serialize};

/// Top-level protocol message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    /// Client hello handshake message
    #[serde(rename = "client/hello")]
    ClientHello(ClientHello),

    /// Server hello handshake response
    #[serde(rename = "server/hello")]
    ServerHello(ServerHello),

    /// Client time synchronization request
    #[serde(rename = "client/time")]
    ClientTime(ClientTime),

    /// Server time synchronization response
    #[serde(rename = "server/time")]
    ServerTime(ServerTime),

    /// Stream start notification
    #[serde(rename = "stream/start")]
    StreamStart(StreamStart),

    /// Stream clear notification (for seek)
    #[serde(rename = "stream/clear")]
    StreamClear(StreamClear),

    /// Stream end notification
    #[serde(rename = "stream/end")]
    StreamEnd(StreamEnd),

    /// Server command to client
    #[serde(rename = "server/command")]
    ServerCommand(ServerCommand),

    /// Server state update to client
    #[serde(rename = "server/state")]
    ServerState(ServerState),

    /// Group update notification
    #[serde(rename = "group/update")]
    GroupUpdate(GroupUpdate),

    /// Client state update to server
    #[serde(rename = "client/state")]
    ClientState(ClientState),

    /// Client goodbye message
    #[serde(rename = "client/goodbye")]
    ClientGoodbye(ClientGoodbye),

    /// Player state update from client (legacy, use client/state)
    #[serde(rename = "player/update")]
    PlayerUpdate(PlayerUpdate),
}

/// Client hello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// Unique client identifier
    pub client_id: String,
    /// Human-readable client name
    pub name: String,
    /// Protocol version number
    pub version: u32,
    /// List of supported roles (e.g., "player", "metadata")
    pub supported_roles: Vec<String>,
    /// Device information
    pub device_info: DeviceInfo,
    /// Player capabilities (if client supports player role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_support: Option<PlayerSupport>,
    /// Metadata capabilities (if client supports metadata role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_support: Option<MetadataSupport>,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Product name (e.g., "Sendspin-RS Player")
    pub product_name: String,
    /// Manufacturer name
    pub manufacturer: String,
    /// Software version string
    pub software_version: String,
}

/// Player capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSupport {
    /// List of supported codecs (e.g., ["pcm", "opus"])
    pub support_codecs: Vec<String>,
    /// List of supported channel counts (e.g., [1, 2] for mono and stereo)
    pub support_channels: Vec<u8>,
    /// List of supported sample rates (e.g., [44100, 48000, 96000])
    pub support_sample_rates: Vec<u32>,
    /// List of supported bit depths (e.g., [16, 24, 32])
    pub support_bit_depth: Vec<u8>,
    /// List of supported audio formats
    pub support_formats: Vec<AudioFormatSpec>,
    /// Buffer capacity in chunks
    pub buffer_capacity: u32,
    /// List of supported playback commands
    pub supported_commands: Vec<String>,
}

/// Audio format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormatSpec {
    /// Codec name (e.g., "pcm", "opus")
    pub codec: String,
    /// Number of audio channels
    pub channels: u8,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Bit depth per sample
    pub bit_depth: u8,
}

/// Metadata display capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSupport {
    /// Supported picture formats (e.g., "jpeg", "png")
    pub support_picture_formats: Vec<String>,
    /// Display width in pixels
    pub media_width: u32,
    /// Display height in pixels
    pub media_height: u32,
}

/// Server hello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    /// Unique server identifier
    pub server_id: String,
    /// Human-readable server name
    pub name: String,
    /// Protocol version number
    pub version: u32,
    /// Active roles for this client
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_roles: Vec<String>,
    /// Connection reason (for server-initiated connections)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_reason: Option<String>,
}

/// Client time sync message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTime {
    /// Client transmission timestamp (Unix microseconds)
    pub client_transmitted: i64,
}

/// Server time sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTime {
    /// Original client transmission timestamp
    pub client_transmitted: i64,
    /// Server reception timestamp (server loop microseconds)
    pub server_received: i64,
    /// Server transmission timestamp (server loop microseconds)
    pub server_transmitted: i64,
}

/// Stream start message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStart {
    /// Player stream configuration
    pub player: StreamPlayerConfig,
}

/// Stream player configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPlayerConfig {
    /// Audio codec name
    pub codec: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u8,
    /// Bit depth per sample
    pub bit_depth: u8,
    /// Optional codec-specific header (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec_header: Option<String>,
}

/// Server command message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCommand {
    /// Command name (e.g., "play", "pause", "stop")
    pub command: String,
    /// Optional volume level (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    /// Optional mute state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
}

/// Player state update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUpdate {
    /// Current playback state (e.g., "playing", "paused", "stopped")
    pub state: String,
    /// Current volume level (0-100)
    pub volume: u8,
    /// Whether audio is muted
    pub muted: bool,
}

/// Group update message (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupUpdate {
    /// Playback state of the group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_state: Option<String>,
    /// Group identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Group name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
}

/// Client state message (client -> server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientState {
    /// Player state (if client has player role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerState>,
}

/// Player state in client/state message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Current state: "synchronized" or "error"
    pub state: String,
    /// Current volume (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    /// Mute state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
}

/// Stream clear message (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamClear {
    /// Roles to clear buffers for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
}

/// Stream end message (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEnd {
    /// Roles to end streams for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
}

/// Client goodbye message (client -> server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGoodbye {
    /// Reason for disconnect
    pub reason: String,
}

/// Server state message (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    /// Metadata state (if client has metadata role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataState>,
    /// Controller state (if client has controller role)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<ControllerState>,
}

/// Metadata state in server/state message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataState {
    /// Server timestamp for this metadata
    pub timestamp: i64,
    /// Track title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Artist name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// Album name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
}

/// Controller state in server/state message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerState {
    /// Supported commands
    pub supported_commands: Vec<String>,
    /// Group volume (0-100)
    pub volume: u8,
    /// Group mute state
    pub muted: bool,
}
