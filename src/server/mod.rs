// ABOUTME: Server module for Sendspin protocol
// ABOUTME: Provides WebSocket server, client management, and audio streaming

mod audio_engine;
mod audio_source;
mod client_handler;
mod client_manager;
mod clock;
mod config;
mod encoder;
mod group;
mod server;
pub mod tui;

pub use audio_engine::AudioEngine;
pub use audio_source::{AudioSource, FileSource, TestToneSource};
pub use client_handler::handle_client;
pub use client_manager::{ClientManager, ConnectedClient};
pub use clock::ServerClock;
pub use config::ServerConfig;
pub use encoder::{AudioEncoder, FlacEncoder, OpusEncoder, PcmEncoder};
pub use group::{Group, GroupManager};
pub use server::SendspinServer;
pub use tui::{ServerStats, TuiApp};
