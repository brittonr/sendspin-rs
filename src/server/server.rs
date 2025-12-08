// ABOUTME: Main Sendspin server implementation
// ABOUTME: Provides WebSocket endpoint and coordinates all server components

use crate::server::audio_engine::spawn_audio_engine;
use crate::server::audio_source::{AudioSource, TestToneSource};
use crate::server::client_handler::handle_client;
use crate::server::client_manager::ClientManager;
use crate::server::clock::ServerClock;
use crate::server::config::ServerConfig;
use crate::server::group::GroupManager;
use axum::{
    extract::ws::WebSocketUpgrade,
    extract::State,
    response::IntoResponse,
    routing::any,
    Router,
};
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Server configuration
    pub config: Arc<ServerConfig>,
    /// Client manager
    pub client_manager: Arc<ClientManager>,
    /// Group manager
    pub group_manager: Arc<GroupManager>,
    /// Server clock
    pub clock: Arc<ServerClock>,
}

/// Sendspin server
pub struct SendspinServer {
    /// Server configuration
    config: Arc<ServerConfig>,
    /// Client manager
    client_manager: Arc<ClientManager>,
    /// Group manager
    group_manager: Arc<GroupManager>,
    /// Server clock
    clock: Arc<ServerClock>,
    /// Audio source
    source: Option<Box<dyn AudioSource>>,
}

impl SendspinServer {
    /// Create a new Sendspin server with default configuration
    pub fn new() -> Self {
        Self::with_config(ServerConfig::default())
    }

    /// Create a new Sendspin server with custom configuration
    pub fn with_config(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
            client_manager: Arc::new(ClientManager::new()),
            group_manager: Arc::new(GroupManager::new()),
            clock: Arc::new(ServerClock::new()),
            source: None,
        }
    }

    /// Set the audio source
    pub fn with_source(mut self, source: Box<dyn AudioSource>) -> Self {
        self.source = Some(source);
        self
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get the client manager
    pub fn client_manager(&self) -> Arc<ClientManager> {
        Arc::clone(&self.client_manager)
    }

    /// Get the group manager
    pub fn group_manager(&self) -> Arc<GroupManager> {
        Arc::clone(&self.group_manager)
    }

    /// Run the server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.clone();
        let client_manager = self.client_manager.clone();
        let group_manager = self.group_manager.clone();
        let clock = self.clock.clone();

        // Start audio engine
        let source = self.source.unwrap_or_else(|| {
            Box::new(TestToneSource::new(440.0, config.default_sample_rate))
        });

        let (audio_handle, audio_shutdown) = spawn_audio_engine(
            source,
            client_manager.clone(),
            clock.clone(),
            config.chunk_interval_ms,
            config.buffer_ahead_ms,
        );

        // Build application state
        let state = AppState {
            config: config.clone(),
            client_manager,
            group_manager,
            clock,
        };

        // Build router
        let app = Router::new()
            .route(&config.ws_path, any(ws_handler))
            .with_state(state);

        // Bind and serve
        let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
        log::info!(
            "Sendspin server listening on {} (endpoint: {})",
            config.bind_addr,
            config.ws_path
        );

        // Setup graceful shutdown
        let shutdown_signal = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl-C");
            log::info!("Received shutdown signal");
        };

        // Run server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await?;

        // Shutdown audio engine
        let _ = audio_shutdown.send(true);
        let _ = audio_handle.await;

        log::info!("Server shutdown complete");
        Ok(())
    }
}

impl Default for SendspinServer {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_client(
            socket,
            state.client_manager,
            state.group_manager,
            state.clock,
            state.config,
        )
    })
}
