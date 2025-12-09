// ABOUTME: WebSocket client handler
// ABOUTME: Handles individual client connections, handshake, and message routing

use crate::audio::types::{AudioFormat, Codec};
use crate::protocol::messages::{
    ClientHello, ClientTime, Message, ServerHello,
    ServerTime, StreamPlayerConfig, StreamStart,
};
use crate::server::client_manager::{ClientId, ClientManager, ConnectedClient, ServerMessage};
use crate::server::clock::ServerClock;
use crate::server::config::ServerConfig;
use crate::server::group::GroupManager;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Handle a WebSocket client connection
pub async fn handle_client(
    socket: WebSocket,
    client_manager: Arc<ClientManager>,
    group_manager: Arc<GroupManager>,
    clock: Arc<ServerClock>,
    config: Arc<ServerConfig>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Wait for client/hello
    let client_hello = match wait_for_client_hello(&mut ws_rx).await {
        Ok(hello) => hello,
        Err(e) => {
            log::warn!("Failed to receive client/hello: {}", e);
            return;
        }
    };

    log::info!(
        "Client connected: {} ({})",
        client_hello.name,
        client_hello.client_id
    );

    // Negotiate roles
    let active_roles = negotiate_roles(&client_hello.supported_roles);

    // Send server/hello
    let server_hello = Message::ServerHello(ServerHello {
        server_id: config.server_id.clone(),
        name: config.name.clone(),
        version: 1,
        active_roles: active_roles.clone(),
        connection_reason: Some("discovery".to_string()),
    });

    let hello_json = match serde_json::to_string(&server_hello) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize server/hello: {}", e);
            return;
        }
    };

    if ws_tx.send(WsMessage::Text(hello_json.into())).await.is_err() {
        log::warn!("Failed to send server/hello");
        return;
    }

    // Create channel for server->client messages
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Negotiate audio format
    let audio_format = negotiate_audio_format(&client_hello, &config);

    // Create connected client
    let client_id = client_hello.client_id.clone();
    let mut connected_client = ConnectedClient::new(client_id.clone(), client_hello.name.clone(), tx);
    connected_client.active_roles = active_roles.clone();
    connected_client.audio_format = Some(audio_format.clone());

    if let Some(ref player_support) = client_hello.player_support {
        connected_client.buffer_capacity = player_support.buffer_capacity;
    }

    // Register client
    client_manager.add_client(connected_client);

    // Add to default group
    group_manager.add_to_group(&client_id, group_manager.default_group_id());

    // Send stream/start if client is a player
    if active_roles.iter().any(|r| r.starts_with("player@")) {
        let stream_start = create_stream_start(&audio_format);
        let start_json = match serde_json::to_string(&stream_start) {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to serialize stream/start: {}", e);
                client_manager.remove_client(&client_id);
                return;
            }
        };

        log::info!("Sending stream/start to client {}: {}", client_id, start_json);
        if ws_tx.send(WsMessage::Text(start_json.into())).await.is_err() {
            log::warn!("Failed to send stream/start");
            client_manager.remove_client(&client_id);
            return;
        }
        log::info!("stream/start sent successfully to client {}", client_id);
    }

    // Spawn task to forward server messages to WebSocket
    let client_id_send = client_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let ws_msg = match msg {
                ServerMessage::Binary(data) => WsMessage::Binary(data.into()),
                ServerMessage::Text(text) => WsMessage::Text(text.into()),
            };
            if ws_tx.send(ws_msg).await.is_err() {
                log::debug!("Client {} disconnected (send failed)", client_id_send);
                break;
            }
        }
    });

    // Handle incoming messages
    let client_id_recv = client_id.clone();
    let client_manager_recv = client_manager.clone();
    let clock_recv = clock.clone();

    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                handle_text_message(
                    &text,
                    &client_id_recv,
                    &client_manager_recv,
                    &clock_recv,
                )
                .await;
            }
            Ok(WsMessage::Binary(data)) => {
                // Clients don't typically send binary data to server
                log::debug!(
                    "Received binary from client {} ({} bytes)",
                    client_id_recv,
                    data.len()
                );
            }
            Ok(WsMessage::Ping(_)) | Ok(WsMessage::Pong(_)) => {
                // Handled automatically by axum
            }
            Ok(WsMessage::Close(_)) => {
                log::info!("Client {} closed connection", client_id_recv);
                break;
            }
            Err(e) => {
                log::warn!("WebSocket error for client {}: {}", client_id_recv, e);
                break;
            }
        }
    }

    // Cleanup
    client_manager.remove_client(&client_id);
    group_manager.remove_client(&client_id);
    send_task.abort();

    log::info!("Client {} disconnected", client_id);
}

/// Wait for client/hello message
async fn wait_for_client_hello(
    ws_rx: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Result<ClientHello, String> {
    // Wait up to 10 seconds for client/hello
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    match serde_json::from_str::<Message>(&text) {
                        Ok(Message::ClientHello(hello)) => return Ok(hello),
                        Ok(other) => {
                            return Err(format!("Expected client/hello, got {:?}", other));
                        }
                        Err(e) => {
                            return Err(format!("Failed to parse message: {}", e));
                        }
                    }
                }
                Ok(WsMessage::Ping(_)) | Ok(WsMessage::Pong(_)) => continue,
                Ok(WsMessage::Close(_)) => {
                    return Err("Connection closed before hello".to_string());
                }
                Err(e) => {
                    return Err(format!("WebSocket error: {}", e));
                }
                _ => continue,
            }
        }
        Err("Connection closed".to_string())
    });

    match timeout.await {
        Ok(result) => result,
        Err(_) => Err("Timeout waiting for client/hello".to_string()),
    }
}

/// Negotiate active roles based on client's supported roles
fn negotiate_roles(supported_roles: &[String]) -> Vec<String> {
    let mut active = Vec::new();

    // Check for player role (accept "player", "player@v1", etc.)
    for role in supported_roles {
        if role == "player" || role.starts_with("player@") {
            // Normalize to versioned form for consistency
            if role == "player" {
                active.push("player@v1".to_string());
            } else {
                active.push(role.clone());
            }
            break; // Only one player version
        }
    }

    // Check for controller role
    for role in supported_roles {
        if role == "controller" || role.starts_with("controller@") {
            if role == "controller" {
                active.push("controller@v1".to_string());
            } else {
                active.push(role.clone());
            }
            break;
        }
    }

    // Check for metadata role
    for role in supported_roles {
        if role == "metadata" || role.starts_with("metadata@") {
            if role == "metadata" {
                active.push("metadata@v1".to_string());
            } else {
                active.push(role.clone());
            }
            break;
        }
    }

    active
}

/// Negotiate audio format based on client capabilities
fn negotiate_audio_format(client_hello: &ClientHello, config: &ServerConfig) -> AudioFormat {
    // Default format
    let mut format = AudioFormat {
        codec: Codec::Pcm,
        sample_rate: config.default_sample_rate,
        channels: config.default_channels,
        bit_depth: config.default_bit_depth,
        codec_header: None,
    };

    // Check client's supported formats
    if let Some(ref player_support) = client_hello.player_support {
        // Try to find PCM format first (most compatible)
        for fmt in &player_support.supported_formats {
            if fmt.codec == "pcm" {
                format.sample_rate = fmt.sample_rate;
                format.channels = fmt.channels;
                format.bit_depth = fmt.bit_depth;
                return format;
            }
        }

        // Fall back to first supported format (client's preferred)
        if let Some(fmt) = player_support.supported_formats.first() {
            format.codec = match fmt.codec.as_str() {
                "opus" => Codec::Opus,
                "flac" => Codec::Flac,
                "mp3" => Codec::Mp3,
                _ => Codec::Pcm,
            };
            format.sample_rate = fmt.sample_rate;
            format.channels = fmt.channels;
            format.bit_depth = fmt.bit_depth;
        }
    }

    format
}

/// Create stream/start message
fn create_stream_start(format: &AudioFormat) -> Message {
    Message::StreamStart(StreamStart {
        player: StreamPlayerConfig {
            codec: match format.codec {
                Codec::Pcm => "pcm".to_string(),
                Codec::Opus => "opus".to_string(),
                Codec::Flac => "flac".to_string(),
                Codec::Mp3 => "mp3".to_string(),
            },
            sample_rate: format.sample_rate,
            channels: format.channels,
            bit_depth: format.bit_depth,
            codec_header: format.codec_header.as_ref().map(|h| base64_encode(h)),
        },
    })
}

/// Handle incoming text message from client
async fn handle_text_message(
    text: &str,
    client_id: &ClientId,
    client_manager: &ClientManager,
    clock: &ServerClock,
) {
    let msg = match serde_json::from_str::<Message>(text) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse message from {}: {}", client_id, e);
            return;
        }
    };

    match msg {
        Message::ClientTime(client_time) => {
            handle_client_time(client_id, client_time, client_manager, clock);
        }
        Message::ClientState(state) => {
            // Handle spec-compliant client/state message with player object
            if let Some(player) = state.player {
                log::debug!(
                    "Player {} state: {}, volume: {:?}, muted: {:?}",
                    client_id,
                    player.state,
                    player.volume,
                    player.muted
                );
                // Update volume if provided (both must be present per spec when supported)
                if let (Some(volume), Some(muted)) = (player.volume, player.muted) {
                    client_manager.update_volume(client_id, volume, muted);
                }
            }
        }
        Message::ClientGoodbye(goodbye) => {
            // Per spec: client is gracefully disconnecting
            // Reasons: 'another_server', 'shutdown', 'restart', 'user_request'
            log::info!(
                "Client {} sent goodbye with reason: {}",
                client_id,
                goodbye.reason
            );
            // The client will be removed when the WebSocket closes
            // Server can use reason to determine auto-reconnect behavior:
            // - 'restart': auto-reconnect expected
            // - 'another_server', 'shutdown', 'user_request': no auto-reconnect
        }
        Message::StreamRequestFormat(request) => {
            // Per spec: client requests format change (adaptive streaming)
            log::info!(
                "Client {} requested format change: {:?}",
                client_id,
                request
            );
            // TODO: Implement format negotiation and send new stream/start
            // For now, log the request - full implementation requires per-client encoding
            if let Some(player_req) = request.player {
                log::debug!(
                    "Player format request - codec: {:?}, sample_rate: {:?}, channels: {:?}, bit_depth: {:?}",
                    player_req.codec,
                    player_req.sample_rate,
                    player_req.channels,
                    player_req.bit_depth
                );
            }
        }
        _ => {
            log::debug!("Unhandled message from {}: {:?}", client_id, msg);
        }
    }
}

/// Handle client/time message and respond with server/time
fn handle_client_time(
    client_id: &ClientId,
    client_time: ClientTime,
    client_manager: &ClientManager,
    clock: &ServerClock,
) {
    let server_received = clock.now_micros();
    let server_transmitted = clock.now_micros();

    let response = Message::ServerTime(ServerTime {
        client_transmitted: client_time.client_transmitted,
        server_received,
        server_transmitted,
    });

    let json = match serde_json::to_string(&response) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to serialize server/time: {}", e);
            return;
        }
    };

    client_manager.send_to_client(client_id, &json);
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
