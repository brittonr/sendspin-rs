// ABOUTME: Client connection manager
// ABOUTME: Thread-safe registry of connected clients with broadcast capabilities

use crate::audio::types::{AudioFormat, Codec};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Unique client identifier
pub type ClientId = String;

/// Message types that can be sent to clients
#[derive(Debug, Clone)]
pub enum ServerMessage {
    /// JSON text message
    Text(String),
    /// Binary audio chunk (already formatted with type + timestamp + data)
    Binary(Vec<u8>),
}

/// A connected client
#[derive(Debug)]
pub struct ConnectedClient {
    /// Unique client identifier
    pub client_id: ClientId,
    /// Human-readable client name
    pub name: String,
    /// Active roles for this client (e.g., ["player@v1"])
    pub active_roles: Vec<String>,
    /// Negotiated audio format for player role
    pub audio_format: Option<AudioFormat>,
    /// Channel to send messages to this client
    pub tx: mpsc::UnboundedSender<ServerMessage>,
    /// Group this client belongs to
    pub group_id: Option<String>,
    /// Client's current volume (0-100)
    pub volume: u8,
    /// Whether client is muted
    pub muted: bool,
    /// Buffer capacity in bytes
    pub buffer_capacity: u32,
}

impl ConnectedClient {
    /// Create a new connected client
    pub fn new(
        client_id: ClientId,
        name: String,
        tx: mpsc::UnboundedSender<ServerMessage>,
    ) -> Self {
        Self {
            client_id,
            name,
            active_roles: Vec::new(),
            audio_format: None,
            tx,
            group_id: None,
            volume: 100,
            muted: false,
            buffer_capacity: 0,
        }
    }

    /// Check if client has player role
    pub fn is_player(&self) -> bool {
        self.active_roles
            .iter()
            .any(|r| r.starts_with("player@"))
    }

    /// Send a message to this client
    pub fn send(&self, msg: ServerMessage) -> Result<(), mpsc::error::SendError<ServerMessage>> {
        self.tx.send(msg)
    }
}

/// Manages all connected clients
#[derive(Debug)]
pub struct ClientManager {
    /// Map of client_id to client
    clients: Arc<RwLock<HashMap<ClientId, ConnectedClient>>>,
}

impl ClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a client to the manager
    pub fn add_client(&self, client: ConnectedClient) {
        let client_id = client.client_id.clone();
        self.clients.write().insert(client_id.clone(), client);
        log::info!("Client {} added, total clients: {}", client_id, self.client_count());
    }

    /// Remove a client from the manager
    pub fn remove_client(&self, client_id: &str) -> Option<ConnectedClient> {
        let client = self.clients.write().remove(client_id);
        if client.is_some() {
            log::info!("Client {} removed, total clients: {}", client_id, self.client_count());
        }
        client
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.read().len()
    }

    /// Update a client's audio format
    pub fn update_audio_format(&self, client_id: &str, format: AudioFormat) {
        if let Some(client) = self.clients.write().get_mut(client_id) {
            client.audio_format = Some(format);
        }
    }

    /// Update a client's volume
    pub fn update_volume(&self, client_id: &str, volume: u8, muted: bool) {
        if let Some(client) = self.clients.write().get_mut(client_id) {
            client.volume = volume;
            client.muted = muted;
        }
    }

    /// Broadcast a binary message to all player clients
    pub fn broadcast_audio(&self, message: &[u8]) {
        let clients = self.clients.read();
        for client in clients.values() {
            if client.is_player() {
                let _ = client.send(ServerMessage::Binary(message.to_vec()));
            }
        }
    }

    /// Broadcast a text message to all clients
    pub fn broadcast_text(&self, message: &str) {
        let clients = self.clients.read();
        for client in clients.values() {
            let _ = client.send(ServerMessage::Text(message.to_string()));
        }
    }

    /// Send a text message to a specific client
    pub fn send_to_client(&self, client_id: &str, message: &str) -> bool {
        if let Some(client) = self.clients.read().get(client_id) {
            client.send(ServerMessage::Text(message.to_string())).is_ok()
        } else {
            false
        }
    }

    /// Send stream/clear to all player clients
    /// Per spec: instructs clients to clear buffers without ending stream (for seek)
    pub fn broadcast_stream_clear(&self, roles: Option<Vec<String>>) {
        use crate::protocol::messages::{Message, StreamClear};

        let msg = Message::StreamClear(StreamClear { roles });
        if let Ok(json) = serde_json::to_string(&msg) {
            let clients = self.clients.read();
            for client in clients.values() {
                if client.is_player() {
                    let _ = client.send(ServerMessage::Text(json.clone()));
                }
            }
            log::debug!("Broadcast stream/clear to {} player clients", clients.values().filter(|c| c.is_player()).count());
        }
    }

    /// Send stream/end to all player clients
    /// Per spec: ends the stream for specified roles, clients should stop output and clear buffers
    pub fn broadcast_stream_end(&self, roles: Option<Vec<String>>) {
        use crate::protocol::messages::{Message, StreamEnd};

        let msg = Message::StreamEnd(StreamEnd { roles });
        if let Ok(json) = serde_json::to_string(&msg) {
            let clients = self.clients.read();
            for client in clients.values() {
                if client.is_player() {
                    let _ = client.send(ServerMessage::Text(json.clone()));
                }
            }
            log::debug!("Broadcast stream/end to {} player clients", clients.values().filter(|c| c.is_player()).count());
        }
    }

    /// Send server/command with player command to a specific client
    /// Per spec: command must be one of supported_commands from client/hello
    pub fn send_player_command(&self, client_id: &str, command: &str, volume: Option<u8>, mute: Option<bool>) -> bool {
        use crate::protocol::messages::{Message, ServerCommand, PlayerCommand};

        let msg = Message::ServerCommand(ServerCommand {
            player: Some(PlayerCommand {
                command: command.to_string(),
                volume,
                mute,
            }),
        });

        if let Ok(json) = serde_json::to_string(&msg) {
            self.send_to_client(client_id, &json)
        } else {
            false
        }
    }

    /// Broadcast server/command with player command to all player clients
    pub fn broadcast_player_command(&self, command: &str, volume: Option<u8>, mute: Option<bool>) {
        use crate::protocol::messages::{Message, ServerCommand, PlayerCommand};

        let msg = Message::ServerCommand(ServerCommand {
            player: Some(PlayerCommand {
                command: command.to_string(),
                volume,
                mute,
            }),
        });

        if let Ok(json) = serde_json::to_string(&msg) {
            let clients = self.clients.read();
            for client in clients.values() {
                if client.is_player() {
                    let _ = client.send(ServerMessage::Text(json.clone()));
                }
            }
        }
    }

    /// Get a list of all client IDs
    pub fn client_ids(&self) -> Vec<ClientId> {
        self.clients.read().keys().cloned().collect()
    }

    /// Get a client's audio format
    pub fn get_audio_format(&self, client_id: &str) -> Option<AudioFormat> {
        self.clients.read().get(client_id)?.audio_format.clone()
    }

    /// Iterate over all clients with a closure
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&ConnectedClient),
    {
        let clients = self.clients.read();
        for client in clients.values() {
            f(client);
        }
    }

    /// Get default audio format (PCM 48kHz stereo 24-bit)
    pub fn default_audio_format() -> AudioFormat {
        AudioFormat {
            codec: Codec::Pcm,
            sample_rate: 48000,
            channels: 2,
            bit_depth: 24,
            codec_header: None,
        }
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ClientManager {
    fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
        }
    }
}
