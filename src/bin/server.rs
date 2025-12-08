// ABOUTME: Sendspin server binary
// ABOUTME: Standalone server application for streaming audio

use clap::Parser;
use sendspin::server::{AudioSource, FileSource, SendspinServer, ServerConfig, TestToneSource};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "sendspin-server")]
#[command(author, version, about = "Sendspin streaming audio server", long_about = None)]
struct Args {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0:8927")]
    bind: SocketAddr,

    /// Server name
    #[arg(short, long, default_value = "Sendspin Server")]
    name: String,

    /// WebSocket endpoint path
    #[arg(long, default_value = "/sendspin")]
    path: String,

    /// Audio file to stream (MP3, FLAC, WAV, etc.). If not specified, generates a test tone.
    #[arg(long)]
    file: Option<String>,

    /// Test tone frequency in Hz (only used if no file is specified, 0 for silence)
    #[arg(short, long, default_value = "440.0")]
    frequency: f64,

    /// Sample rate in Hz (only used for test tone)
    #[arg(short, long, default_value = "48000")]
    sample_rate: u32,

    /// Audio chunk interval in milliseconds
    #[arg(long, default_value = "20")]
    chunk_ms: u64,

    /// Buffer ahead time in milliseconds
    #[arg(long, default_value = "500")]
    buffer_ahead_ms: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose {
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

    tracing::info!("Sendspin Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Bind: {}", args.bind);
    tracing::info!("Endpoint: ws://{}{}", args.bind, args.path);

    // Create audio source
    let source: Box<dyn AudioSource> = if let Some(file_path) = args.file {
        match FileSource::new(&file_path) {
            Ok(file_source) => {
                tracing::info!(
                    "Audio: Streaming from file '{}' ({}Hz, {} channels, looping)",
                    file_path,
                    file_source.sample_rate(),
                    file_source.channels()
                );
                Box::new(file_source)
            }
            Err(e) => {
                tracing::error!("Failed to open audio file '{}': {}", file_path, e);
                return Err(format!("Failed to open audio file: {}", e).into());
            }
        }
    } else {
        if args.frequency > 0.0 {
            tracing::info!("Audio: {} Hz test tone at {} Hz sample rate", args.frequency, args.sample_rate);
        } else {
            tracing::info!("Audio: Silence");
        }
        Box::new(TestToneSource::new(args.frequency.max(0.0), args.sample_rate))
    };

    // Create server configuration
    let config = ServerConfig::new(&args.name)
        .bind_addr(args.bind)
        .ws_path(args.path)
        .chunk_interval_ms(args.chunk_ms)
        .buffer_ahead_ms(args.buffer_ahead_ms);

    // Create and run server
    let server = SendspinServer::with_config(config).with_source(source);
    let client_manager = server.client_manager();

    // Spawn a task to periodically report connected clients
    let report_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let count = client_manager.client_count();
            if count > 0 {
                tracing::info!("Connected clients: {}", count);
                client_manager.for_each(|client| {
                    tracing::info!(
                        "  - {} ({}): roles={:?}, volume={}%, muted={}",
                        client.name,
                        client.client_id,
                        client.active_roles,
                        client.volume,
                        client.muted
                    );
                });
            }
        }
    });

    tracing::info!("Press Ctrl+C to stop");

    let result = server.run().await;
    report_task.abort();
    result
}
