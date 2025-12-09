// ABOUTME: Shared CLI argument parsing and server builder utilities
// ABOUTME: Consolidates common code between server binaries (server.rs, server_tui.rs)

use crate::server::{AudioSource, FileSource, ServerConfig, TestToneSource, UrlSource};
use clap::Args;
use std::net::SocketAddr;

/// Common server arguments shared between all server binaries
///
/// Use with `#[command(flatten)]` in your binary's Args struct:
/// ```ignore
/// #[derive(Parser)]
/// struct MyArgs {
///     #[command(flatten)]
///     server: ServerArgs,
///
///     // Binary-specific args here
/// }
/// ```
#[derive(Args, Debug, Clone)]
pub struct ServerArgs {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0:8927")]
    pub bind: SocketAddr,

    /// Server name
    #[arg(short, long, default_value = "Sendspin Server")]
    pub name: String,

    /// WebSocket endpoint path
    #[arg(long, default_value = "/sendspin")]
    pub path: String,

    /// Audio file to stream (MP3, FLAC, WAV, etc.). Mutually exclusive with --url.
    #[arg(long, conflicts_with = "url")]
    pub file: Option<String>,

    /// HTTP/HTTPS URL to stream audio from (MP3, FLAC, etc.). Mutually exclusive with --file.
    #[arg(long, conflicts_with = "file")]
    pub url: Option<String>,

    /// Test tone frequency in Hz (only used if no file/url is specified, 0 for silence)
    #[arg(short, long, default_value = "440.0")]
    pub frequency: f64,

    /// Sample rate in Hz (only used for test tone)
    #[arg(short, long, default_value = "48000")]
    pub sample_rate: u32,

    /// Audio chunk interval in milliseconds
    #[arg(long, default_value = "20")]
    pub chunk_ms: u64,

    /// Buffer ahead time in milliseconds
    #[arg(long, default_value = "500")]
    pub buffer_ahead_ms: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

impl ServerArgs {
    /// Initialize tracing based on verbosity flag
    pub fn init_tracing(&self) {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let filter = if self.verbose {
            "sendspin=debug,tower_http=debug"
        } else {
            "sendspin=info"
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| filter.into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    /// Log startup information
    pub fn log_startup_info(&self) {
        tracing::info!("Sendspin Server v{}", env!("CARGO_PKG_VERSION"));
        tracing::info!("Bind: {}", self.bind);
        tracing::info!("Endpoint: ws://{}{}", self.bind, self.path);
    }

    /// Create audio source based on args (priority: file > url > test tone)
    ///
    /// Returns the audio source and logs information about what was created.
    pub fn create_audio_source(
        &self,
    ) -> Result<Box<dyn AudioSource>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_path) = &self.file {
            match FileSource::new(file_path) {
                Ok(file_source) => {
                    tracing::info!(
                        "Audio: Streaming from file '{}' ({}Hz, {} channels, looping)",
                        file_path,
                        file_source.sample_rate(),
                        file_source.channels()
                    );
                    Ok(Box::new(file_source))
                }
                Err(e) => {
                    tracing::error!("Failed to open audio file '{}': {}", file_path, e);
                    Err(format!("Failed to open audio file: {}", e).into())
                }
            }
        } else if let Some(url) = &self.url {
            match UrlSource::new(url) {
                Ok(url_source) => {
                    tracing::info!(
                        "Audio: Streaming from URL '{}' ({}Hz, {} channels)",
                        url,
                        url_source.sample_rate(),
                        url_source.channels()
                    );
                    Ok(Box::new(url_source))
                }
                Err(e) => {
                    tracing::error!("Failed to open URL stream '{}': {}", url, e);
                    Err(format!("Failed to open URL stream: {}", e).into())
                }
            }
        } else {
            if self.frequency > 0.0 {
                tracing::info!(
                    "Audio: {} Hz test tone at {} Hz sample rate",
                    self.frequency,
                    self.sample_rate
                );
            } else {
                tracing::info!("Audio: Silence");
            }
            Ok(Box::new(TestToneSource::new(
                self.frequency.max(0.0),
                self.sample_rate,
            )))
        }
    }

    /// Build ServerConfig from these args
    ///
    /// Note: This consumes `path` due to the ServerConfig builder pattern.
    /// Call this after `log_startup_info()` if you need the path for logging.
    pub fn build_config(&self) -> ServerConfig {
        ServerConfig::new(&self.name)
            .bind_addr(self.bind)
            .ws_path(self.path.clone())
            .chunk_interval_ms(self.chunk_ms)
            .buffer_ahead_ms(self.buffer_ahead_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        // Verify default values are sensible
        let args = ServerArgs {
            bind: "0.0.0.0:8927".parse().unwrap(),
            name: "Test Server".to_string(),
            path: "/sendspin".to_string(),
            file: None,
            url: None,
            frequency: 440.0,
            sample_rate: 48000,
            chunk_ms: 20,
            buffer_ahead_ms: 500,
            verbose: false,
        };

        assert_eq!(args.bind.port(), 8927);
        assert_eq!(args.chunk_ms, 20);
        assert_eq!(args.buffer_ahead_ms, 500);
    }

    #[test]
    fn test_build_config() {
        let args = ServerArgs {
            bind: "127.0.0.1:9000".parse().unwrap(),
            name: "Custom Server".to_string(),
            path: "/custom".to_string(),
            file: None,
            url: None,
            frequency: 440.0,
            sample_rate: 48000,
            chunk_ms: 10,
            buffer_ahead_ms: 1000,
            verbose: false,
        };

        let config = args.build_config();
        assert_eq!(config.bind_addr().port(), 9000);
    }
}
